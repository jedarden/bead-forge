# JSON Output Audit — Consolidated Summary

**Consolidating beads:** bf-xmwq (original comprehensive audit), bf-5haf (`list`/`ready`),
bf-2x0p (`search`/`claim`), bf-20da (`stats`/`velocity`), bf-4w48 (this consolidation)
**Date:** 2026-07-22
**Scope:** Every `bf` subcommand that accepts `--format json` (i.e. exposes a `format` field
in the `Commands` enum).
**Method:** Findings below are verified against the **current** source (`src/cli/mod.rs` — 3325
lines, `src/format/*.rs`, `src/log.rs`, `src/velocity.rs`, `src/storage/sqlite.rs`) and, where
noted, the live `target/debug/bf` binary. Earlier focused audits (bf-5haf in particular) are
**reconciled here**: `ready` has since been migrated onto the shared formatter, so several of
their claims are now stale and are corrected below.

---

## 1. At a glance

14 subcommands produce JSON output. They split cleanly into two implementation families:

| Family | Commands | Count | Mechanism |
|--------|----------|-------|-----------|
| **A. Shared formatter** (`get_formatter().format_issues()`) | `list`, `ready`, `search`, `recent` | 4 | routes `&[Issue]` through `JsonFormatter` → JSONL |
| **B. Custom serialization** (bypasses the formatter) | `show`, `claim`, `stats`, `velocity`, `log`, `dep tree`, `labels`, `schema`, `mitosis`, `critical-path` | 10 | inline `serde_json` calls on non-`Issue` data |

Commands with **no** `--format json` (out of scope): `count` (prints a bare integer),
`comment add/list`, `annotate *`, `config *`, `label add/remove/list` (singular `label`; the
plural `labels` command *does* have JSON), `dep add/remove/list`, and all mutating/op commands
(`init`, `create`, `update`, `close`, `reopen`, `delete`, `sync`, `doctor`, `merge-jsonl`,
`batch`, `rotate`, `migrate`, `commit-check`).

### Master table

| Command | Family | Source (cmd fn) | Serialization | Container shape | Compact? | Empty case | Trailing newline |
|---------|--------|-----------------|---------------|-----------------|----------|------------|------------------|
| `list` | A | `cmd_list` `:1358` | `format_issues()` | JSONL | yes | *no output* | `print!` (none extra) |
| `search` | A | `cmd_search` `:2531` | `format_issues()` | JSONL | yes | *no output* | `print!` (none extra) |
| `recent` | A | `cmd_recent` `:3256` | `format_issues()` | JSONL | yes | *no output* | `print!` (none extra) |
| `ready` | A* | `cmd_ready` `:1618` | `format_issues()` + `[]` special-case | JSONL (or `[]`) | yes | `[]` | `print!`/`println!` |
| `show` | B | `cmd_show` `:1440` | `to_string(&vec![out])` | single-element array `[{…}]` | yes | n/a (errors if missing) | `println!` |
| `claim` | B | `cmd_claim` `:1674` | `json!({...})` | single object `{…}` | yes | `{}` | `println!` |
| `stats` | B | `cmd_stats` `:2574` | `to_string_pretty(&Stats)` | object `{…}` | **no** (pretty) | always object (zeros) | `println!` |
| `velocity` | B | `cmd_velocity` `:2846` | `to_string_pretty(&Vec<VelocityStats>)` | array `[…]` | **no** (pretty) | `[]` | `println!` |
| `log` | B | `cmd_log` `:2958` | `format_events_json()` → `to_string_pretty` | array `[…]` | **no** (pretty) | `[]` | `println!` |
| `dep tree` | B | `cmd_dep` `:2313` | `json!({...})` → `to_string_pretty` | object w/ `nodes[]` | **no** (pretty) | always object | `println!` |
| `labels` | B | `cmd_labels` `:2489` | `to_string_pretty(&Vec<String>)` | string array `[…]` | **no** (pretty) | `[]` | `println!` |
| `schema` | B | `cmd_schema` `:2635` | `json!`/`to_string_pretty` | varies by target | **no** (pretty) | always object | `println!` |
| `mitosis` | B | `cmd_mitosis` `:2201` | `to_string_pretty(&Vec<BatchResult>)` | array `[…]` | **no** (pretty) | `[]` | `println!` |
| `critical-path` | B | `cmd_critical_path` `:3091` | `to_string_pretty(&result)` | object w/ `beads[]` | **no** (pretty) | always object | `println!` |

\* `ready` is in family A but carries an explicit `[]` empty-case branch (see §4).

---

## 2. Commands using `get_formatter().format_issues()` (Family A)

Four commands route `&[Issue]` through the shared `JsonFormatter`. All four resolve to **full
`Issue` records** (every br-canonical field) and emit **JSONL** — one compact object per line,
no array wrapper, joined with `\n`.

| Command | Call site | Notes |
|---------|-----------|-------|
| `list` | `src/cli/mod.rs:1434` | canonical pattern |
| `search` | `src/cli/mod.rs:2568` | identical pattern to `list` |
| `recent` | `src/cli/mod.rs:3321` | newest list command; identical pattern to `list` |
| `ready` | `src/cli/mod.rs:1632` | **migrated** onto the formatter (see below) |

The shared call shape:
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

### `ready` was migrated onto the formatter (corrects bf-5haf)

bf-5haf recorded `ready` as bypassing the formatter and emitting a `serde_json::to_string`
array of `ScoredBead`. **That is no longer true.** The current `cmd_ready`
(`src/cli/mod.rs:1627-1642`) resolves each `ScoredBead` to its full `Issue` and routes through
`get_formatter(OutputFormat::Json).format_issues()` — explicitly "for consistency with
`list`/`search`" per the in-source comment. This **resolves** the old list-vs-ready JSONL/array
split (inconsistency #1 from bf-5haf/bf-xmwq).

One wrinkle remains: `ready` special-cases the empty result with `println!("[]")`
(`src/cli/mod.rs:1637-1638`) *before* falling through to the formatter. So:

- `bf ready --format json` (non-empty) → JSONL (same as `list`/`search`/`recent`)
- `bf ready --format json` (empty) → `[]`

`list`/`search`/`recent` print **nothing** on empty (the formatter returns `""` from
`.join("\n")` on an empty vec). This is the residual empty-case inconsistency — see §4.

---

## 3. Commands using custom `println!` serialization (Family B)

Ten commands serialize non-`Issue` data inline. This is largely **justified**: the `Formatter`
trait is keyed to `&Issue`/`&[Issue]` and has no method for `Stats`, `VelocityStats`, `Event`,
batch results, claim results, or dependency trees. They fall into three serialization styles:

**`to_string_pretty(&T)` on a `Serialize` struct** (struct field order, 2-space indent):
- `stats` → `Stats` object (`:2589`)
- `velocity` → `Vec<VelocityStats>` array (`:2862`)
- `log` → `Vec<Event>` array via `crate::log::format_events_json()` (`src/log.rs:142-144`, called at `:3070`)
- `labels` → `Vec<String>` array (`:2495`)
- `mitosis` → `Vec<BatchResult>` array (`:2224`)
- `critical-path` → `CriticalPath` object with nested `beads[]` (`:3101`)
- `schema` → `Issue` object or `{"schema": SQL}` (`:2674`/`:2677`/`:2644`)

**`serde_json::json!({...})` literal → `Display`/`to_string_pretty`** (keys **sorted
alphabetically**, see §4):
- `claim` → single result object, hand-picked projection (`:1755`/`:1799`/`:1838`/`:1866`/`:1907`)
- `dep tree` → object with `nodes[]` (`:2398`/`:2408`)
- `schema` `"all"` target → `{"schema": SQL}` (`:2641`)

**`to_string(&vec![out])` compact** (struct field order, single line):
- `show` → single-element array `[{…}]` (`:1464`) — array-wrapped for NEEDLE compatibility

### `claim` detail (carried from bf-2x0p, still accurate)

`claim` bypasses the formatter because it emits a **single** hand-picked projection mixing
fields from `ScoredBead`, `ClaimResult`, and the caller's `assignee`/`workspace` — not an
`Issue`, and not a struct's `Serialize`. Four mutually exclusive branches (dry-run / any /
fallback-any / normal) each emit a different object shape; every branch emits at least
`bead_id` + `assignee`, and every branch's empty/no-candidate case emits the literal `{}` via
`println!("{{}}")` (`:1777`/`:1817`/`:1884`/`:1920`). Always a single object, never an array.

---

## 4. Inconsistencies in array / object formatting

After `ready`'s migration, the old headline inconsistency (list/search JSONL vs. ready array)
is gone. What remains:

1. **Container shape for "list of beads" is JSONL, but `show` is a 1-element array, and
   `ready`'s empty case is `[]`.**
   - `list`/`search`/`recent`/`ready`(non-empty) → JSONL (no array, no `[…]`)
   - `ready` (empty) → `[]`
   - `show` → always `[{…}]` (even though it is logically a single bead)
   - The remaining list-style commands that *do* use arrays (`velocity`, `log`, `mitosis`,
     `labels`) emit arrays of non-`Issue` rows, so the JSONL-vs-array split is now strictly
     "Issue lists = JSONL, everything else = array." That is a defensible rule, but it is
     **undocumented** and breaks the reasonable expectation that `--format json` yields a
     parseable JSON document (JSONL is not a single JSON value).

2. **Empty-case handling differs across list-style commands.**
   - `list`/`search`/`recent` → *no output at all* (empty string). A consumer reading stdout
     gets `EOF`/parse error, not `[]`.
   - `ready` → `[]`
   - `claim` → `{}` (single-object shape; appropriate)
   - `velocity`/`log`/`mitosis`/`labels` → `[]`
   - `stats`/`critical-path`/`dep tree` → always emit a full object (never truly "empty")

3. **Compact vs. pretty printing.**
   - Compact (`to_string` / JSONL): `list`, `search`, `recent`, `ready`, `show`, `claim`
   - Pretty (`to_string_pretty`, 2-space indent): `stats`, `velocity`, `log`, `dep tree`,
     `labels`, `schema`, `mitosis`, `critical-path`
   - No command lets the user choose; it is hard-coded per command.

4. **Trailing newline.**
   - Family A uses `print!` (the only newlines are the JSONL separators; no trailing newline
     after the last object).
   - Family B uses `println!` (trailing newline).
   - A consumer doing `bf search --format json | jq` is fine, but byte-for-byte stable
     consumers notice.

5. **Key ordering.**
   - Struct-`Serialize` (`list`/`search`/`recent`/`ready`/`show`, `stats`, `velocity`, `log`,
     `labels`, `mitosis`, `critical-path`) → **struct declaration order**.
   - `json!` macro (`claim`, `dep tree`, `schema` `"all"`) → **alphabetical**. `serde_json`
     in `Cargo.toml` does **not** enable `preserve_order`, so `json!` builds a `BTreeMap`-backed
     `Map` and `Value::Display` sorts keys. (Empirically verified for `claim` in bf-2x0p.)
   - Harmless to generic-object parsers; differs byte-for-byte.

---

## 5. Notable defects / surprises

1. **⚠ `stats --format json` + any breakdown flag emits invalid combined output** (bf-20da,
   still present). The `--by-type` / `--by-priority` / `--by-assignee` / `--by-label` blocks
   (`src/cli/mod.rs:2599-2630`) run *after* the format match and always use plain `println!`
   text regardless of `--format`. So `bf stats --format json --by-type` prints the JSON object
   **followed by** un-ignorable text ("By type: …"). The combined stdout is not valid JSON.
   **Fix:** when `format == "json"`, fold breakdowns into the object as nested maps
   (`by_type`/`by_priority`/…) instead of printing text.

2. **`show` wraps a single bead in an array** (`:1464`). Intentional and now documented in a
   code comment: NEEDLE's `parse_single_bead` expects `Vec<Bead>` and takes the first element.
   Surprising for a user expecting a bare object, but correct for the NEEDLE contract. **Action:
   surface this in `--help` / README**, not change the behavior.

3. **`ready`'s `[]` empty case diverges from its JSONL siblings.** Minor, but it means
   `bf ready --format json` on an empty board yields a valid JSON value while `bf list --format
   json` on an empty board yields an empty string. Either make `list`/`search`/`recent` emit
   `[]` on empty, or drop `ready`'s special case.

---

## 6. The `JsonFormatter` itself

**Location:** `src/format/json.rs:17-29` (trait + `get_formatter` in `src/format/mod.rs`).

```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| {
            let mut stripped = issue.clone();
            stripped.dependencies = vec![];
            stripped.comments = vec![];
            serde_json::to_string(&stripped)
        })
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")
}
```

Behavior worth recording:
- Returns **JSONL**, not a JSON array.
- **Strips `dependencies` and `comments`** before serializing (br/NEEDLE compatibility).
- Empty input → `""` (no output) — because `.join("\n")` on an empty vec is empty.
- Compact objects; key order = `Issue` struct field order.
- `format_issue` (singular) and `format_error` exist on the trait but are **not called** by any
  command's JSON path (verified: the only `format_*` call sites are the four `format_issues()`
  uses in §2).

`OutputFormat` also has `Toon` (text-art) and `Text` variants; `ToonFormatter`/`TextFormatter`
share the trait. `velocity` is the only command with an explicit `"toon"` arm beyond `"json"`.

---

## 7. Recommendations for standardization

Listed roughly in order of value-to-effort.

1. **Decide and document the JSONL-vs-array rule.** The de-facto rule is already coherent —
   *Issue lists emit JSONL; all other structured output emits a JSON value* — but it is
   unwritten. Either (a) document it prominently (README + per-command `--help`) and accept
   JSONL, or (b) standardize on **real JSON arrays** by giving `JsonFormatter` a
   `format_issues_array()` (or a `--json-mode array|jsonl` flag). JSONL is the more surprising
   choice for `--format json` consumers; option (b) is the cleaner long-term target but is a
   **breaking change** for any consumer that today splits `list`/`search` output on newlines.

2. **Standardize the empty case for list-style commands.** Pick one: either all Issue-list
   commands emit `[]` on empty (change `format_issues` to return `[]` for empty input, which
   also subsumes `ready`'s special case), or none do. Today's split (ready → `[]`, others →
   nothing) is the worst of both. Cheapest win in the audit.

3. **Fix the `stats` breakdown defect (§5.1).** This is a real correctness bug, not a style
   choice: `--format json --by-*` produces non-JSON stdout. Fold breakdowns into the object as
   nested maps when `format == "json"`.

4. **Standardize compact-vs-pretty.** Either pretty-print everything (more readable, what most
   "pretty" CLIs do) or add `--pretty`/`--compact`. The current per-command split is arbitrary.

5. **Standardize trailing newline.** Trivial: route Family A through `println!` (or document
   that list commands emit no trailing newline). Aligns byte-for-byte output across commands.

6. **Document the `show` array-wrap contract** (`§5.2`) in user-facing docs so NEEDLE
   compatibility is visible rather than surprising.

7. **(Optional) Enable `serde_json` `preserve_order`** if byte-stable key order for `claim`/
   `dep tree`/`schema` matters; otherwise leave as-is (alphabetical is deterministic).

### Non-recommendations (leave alone)

- Family B commands that bypass the formatter are **correct to do so** — they serialize
  non-`Issue` data the trait was never designed for. Forcing them through a formatter would
  require a trait method per data type (`format_stats`, `format_velocity`, …) with no
  cross-format benefit. The split in §1 is the right architecture; only the *output shape*
  conventions need harmonizing (recs 1–5), not the routing.

---

## 8. Appendix: Deep dive on `search` and `claim` commands (bf-2x0p)

This appendix provides detailed implementation analysis of the `search` and `claim` commands'
JSON output mechanisms, as specified in bead bf-2x0p.

### 8.1 `search` command (Family A — Shared formatter)

**Implementation location:** `src/cli/mod.rs:2796-2837` (`cmd_search`)

**How it outputs JSON:**
1. Uses **shared formatter** via `get_formatter(OutputFormat::Json).format_issues(&issues)`
2. Follows canonical Family A pattern:
   ```rust
   let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
   let formatter = get_formatter(output_format);
   print!("{}", formatter.format_issues(&issues));
   ```

**Formatter methods used:**
- `format_issues(&[Issue])` → returns JSONL (newline-separated JSON objects)
- Does NOT use `format_issue` (singular) or `format_error`

**Array/object structure patterns:**
- **Container shape:** JSONL (one JSON object per line, no array wrapper)
- **Object structure:** Full `Issue` records with `dependencies` and `comments` stripped
- **Format:** Compact (no pretty-printing)
- **Empty case:** No output (empty string, not even `[]`)
- **Trailing newline:** None (uses `print!`, not `println!`)

**Data flow:**
1. `storage.search_issues()` returns `Vec<Issue>` with applied filters
2. Issues are passed directly to `format_issues()` 
3. `JsonFormatter::format_issues()` strips dependencies/comments, serializes each to JSON string, joins with `\n`
4. Result printed via `print!()` (no trailing newline)

**Example output:**
```json
{"id":"bf-abc","title":"Fix bug","status":"open","priority":2,"assignee":null,"labels":[]}
{"id":"bf-def","title":"Add feature","status":"in_progress","priority":1,"assignee":"worker","labels":["urgent"]}
```

### 8.2 `claim` command (Family B — Custom serialization)

**Implementation location:** `src/cli/mod.rs:1789-2148` (`cmd_claim`)

**How it outputs JSON:**
- **Custom loop** — bypasses `JsonFormatter` entirely
- Uses `serde_json::json!({...})` macro for object construction
- Four mutually exclusive execution paths, each with separate JSON handling:
  1. Dry-run mode (single workspace)
  2. Dry-run mode (multi-workspace with `--any` or `--fallback any`)
  3. Normal claim from any workspace (`--any`)
  4. Fallback mode (`--fallback any`): tries current workspace, falls back to any
  5. Normal single-workspace claim

**Formatter methods used:**
- None (custom `serde_json::json!` macro serialization)

**Array/object structure patterns:**
- **Container shape:** Single JSON object `{…}` (never array)
- **Object structure:** Hand-picked projection mixing fields from `ScoredBead`, `ClaimResult`, worker metadata
- **Format:** Generally compact, but `to_string_pretty` used when `flush_warning` present
- **Empty case:** Literal `{}` via `println!("{{}}")` in all five branches
- **Trailing newline:** Yes (uses `println!`)
- **Key ordering:** Alphabetical (due to `json!` macro using `BTreeMap`)

**Detailed object shapes by execution path:**

1. **Dry-run (single workspace):**
   ```json
   {
     "assignee": "...",
     "bead_id": "...",
     "dry_run": true,
     "downstream_impact": 5,
     "priority": 2,
     "title": "...",
     "workspace": "..."
   }
   ```

2. **Dry-run (multi-workspace):** Same as single-workspace dry-run

3. **Normal claim from any workspace (`--any`):**
   ```json
   {
     "assignee": "...",
     "bead_id": "...",
     "reclaimed": false,
     "workspace": "/path/to/workspace"
   }
   ```
   Plus optional `flush_warning` key if auto-flush had warnings (uses `to_string_pretty` in this case)

4. **Fallback mode (current workspace success):**
   ```json
   {
     "assignee": "...",
     "bead_id": "...",
     "reclaimed": false
   }
   ```
   Plus optional `flush_warning` key

5. **Fallback mode (fallback to any workspace):** Same as normal `--any` claim

**Data flow:**
1. For dry-run: `get_ready_candidates()` returns `Vec<ScoredBead>`, selects top 1, constructs JSON from fields
2. For normal/fallback claims: `claim()` or `claim_any()` returns `Option<ClaimResult>`, constructs JSON from result fields
3. Optionally adds `flush_warning` from auto-flush result
4. Uses `json!` macro → compact output, or `to_string_pretty` when flush warning present

**Key differences from `search`:**
- Custom serialization vs. shared formatter
- Single object vs. JSONL array  
- Always emits at least `{}` on empty vs. no output on empty
- `println!` (trailing newline) vs. `print!` (no trailing newline)
- Alphabetical key ordering vs. struct field order
- Hand-picked projections vs. full `Issue` records
- Sometimes uses pretty-printing (when flush warning present) vs. always compact

**Implementation complexity note:**
The `claim` command has significant implementation complexity due to:
- Five mutually exclusive code paths
- Different data structures (`ScoredBead` vs. `ClaimResult`)
- Conditional inclusion of `flush_warning` field
- Multi-workspace discovery logic
- All embedded within a single 360-line function

This complexity justifies bypassing the shared formatter — the output shapes are not `Issue` records and vary significantly per execution path.
