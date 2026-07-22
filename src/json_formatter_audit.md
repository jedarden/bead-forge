# JSON Output Implementation Audit

**Bead:** bf-xmwq  
**Date:** 2026-07-03  
**Purpose:** Document current JSON output format for each command with `--format json` flag

---

## Bead bf-5haf: Focused Audit of `list` and `ready` Commands

**Date:** 2026-07-03  
**Purpose:** Detailed audit of JSON output implementations for list and ready commands

### Summary

| Command | Formatter Used | Output Format | Array Wrapper | Implementation Pattern |
|---------|---------------|---------------|---------------|----------------------|
| `list`  | `JsonFormatter.format_issues()` | JSONL (newline-delimited) | NO | **Formatter system** with custom loop (`.iter().map().join("\n")`) |
| `ready` | None (bypasses formatter) | JSON array | YES | **Direct serialization** with `serde_json::to_string()` |

### List Command (`bf list --format json`)

**Implementation:** `src/cli/mod.rs:995-1079`

**Uses the Formatter system:**
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**Formatter implementation:** `src/format/json.rs:17-29`
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

**Output format:** JSONL (newline-delimited JSON objects)
```json
{"id":"bf-123","title":"First bead","status":"open","priority":2,...}
{"id":"bf-124","title":"Second bead","status":"open","priority":1,...}
```

**Key characteristics:**
- Uses `JsonFormatter.format_issues()` method
- Outputs JSONL format (objects separated by newlines)
- NO array wrapper
- Strips `dependencies` and `comments` before serialization
- Empty result produces no output (empty string)

### Ready Command (`bf ready --format json`)

**Implementation:** `src/cli/mod.rs:1229-1271`

**Bypasses the Formatter system entirely:**
```rust
match format {
    "json" => {
        // Output as JSON array: [] for empty, [candidate] for single, [c1, c2, ...] for multiple
        println!("{}", serde_json::to_string(&candidates)?);
    }
    "toon" => { /* custom */ }
    _ => { /* custom */ }
}
```

**Output format:** Standard JSON array
```json
[
  {"id":"bf-123","title":"First bead","priority":2,"downstream_impact":5,"critical_float":0.0},
  {"id":"bf-124","title":"Second bead","priority":1,"downstream_impact":3,"critical_float":1.0}
]
```

**Key characteristics:**
- Does NOT use the formatter system
- Direct `serde_json::to_string()` call on `Vec<ScoredBead>`
- Outputs proper JSON array
- Empty result produces `[]`
- Returns `ScoredBead` objects (not full `Issue` objects)

### Why Ready Bypasses the Formatter

The `ready` command returns `ScoredBead` objects (from `src/claim.rs`), which differ from `Issue` objects:

**ScoredBead structure:**
```rust
pub struct ScoredBead {
    pub id: String,
    pub title: String,
    pub status: String,
    pub priority: i32,
    pub downstream_impact: i64,
    pub critical_float: f64,
    pub created_at: String,
}
```

The `Formatter` trait is designed for `Issue` objects and cannot handle `ScoredBead` directly. A separate formatter would be needed for consistency.

### Inconsistency Issues

1. **Different output formats:**
   - `list`: JSONL (newline-delimited objects)
   - `ready`: JSON array
   
2. **Empty case handling:**
   - `list`: No output (empty string)
   - `ready`: `[]`

3. **Single item handling:**
   - `list`: Single JSON object (no array)
   - `ready`: Array with one element `[{...}]`

---

## Bead bf-2x0p: Focused Audit of `search` and `claim` Commands

**Date:** 2026-07-22
**Purpose:** Detailed audit of JSON output implementations for search and claim commands
**Method:** Verified against current source (`src/cli/mod.rs`) **and** empirically against the live `target/debug/bf` binary — line numbers below are current as of this date (the comprehensive audit below uses stale 2026-07-03 line numbers).

### Summary

| Command | Formatter Used | Output Format | Array Wrapper | Implementation Pattern |
|---------|---------------|---------------|---------------|----------------------|
| `search` | `JsonFormatter.format_issues()` | JSONL (newline-delimited) | NO | **Formatter system** via `get_formatter().format_issues()` |
| `claim`  | None (bypasses formatter) | Single JSON object | NO | **Direct** `serde_json::json!({...})` + `println!` Display |

### Search Command (`bf search --format json`)

**Implementation:** `src/cli/mod.rs:2531-2572`

**Uses the Formatter system** (identical pattern to `list`):
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

The `issues` vec comes from `storage.search_issues(...)` — full `Issue` objects.

**Formatter method used:** `Formatter::format_issues()` → `JsonFormatter::format_issues()` (`src/format/json.rs:17-29`). `search` does **not** call `format_issue` (singular) or `format_error`.

**Formatter implementation:**
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

**Output format:** JSONL (newline-delimited JSON objects), one full `Issue` per line. **Empirically verified** — each line parses independently as valid JSON and contains all br-canonical `Issue` fields (`id`, `title`, `description`, `design`, `acceptance_criteria`, `notes`, `status`, `priority`, `issue_type`, `assignee`, `created_at`, `updated_at`, `closed_at`, `close_reason`, `closed_by_session`, `source_repo`, `compaction_level`, `labels`).

**Key characteristics:**
- Uses `JsonFormatter.format_issues()` method (the only formatter trait method invoked)
- Outputs JSONL (objects separated by `\n`) — **not** a JSON array
- NO array wrapper
- Strips `dependencies` and `comments` before serialization (br compatibility)
- Empty result produces **no output** (empty string via `.join("\n")` on an empty vec)
- Uses compact `serde_json::to_string` (single-line objects, key order follows struct field order)
- Emitted with `print!` (no trailing newline beyond the JSONL separators)

### Claim Command (`bf claim --format json`)

**Implementation:** `src/cli/mod.rs:1674-1929`

**Bypasses the Formatter system entirely.** Output is built with the `serde_json::json!` macro into a `serde_json::Value`, then printed via `Value`'s `Display` impl:
```rust
let output = serde_json::json!({
    "bead_id": candidate.id,
    "title": candidate.title,
    ...
});
println!("{}", output);
```

The handler has **four mutually exclusive branches**, each with its own JSON object shape. Which branch runs depends on `--dry-run`, `--any`, and `--fallback`:

| Branch | Condition | Source lines | Extra fields vs. base |
|--------|-----------|--------------|----------------------|
| **dry-run** | `--dry-run` | 1697-1780 | `title`, `priority`, `downstream_impact`, `workspace`, `dry_run: true` |
| **any** | `--any` (no dry-run) | 1781-1822 | `reclaimed`, `workspace` |
| **fallback-any** | `--fallback any` (no dry-run, no any) | 1823-1891 | `reclaimed`, plus `workspace` on the fallback sub-branch |
| **normal** | none of the above | 1892-1926 | `reclaimed` only |

**Field sets per branch (verified):**

dry-run (`serde_json::json!` at line 1755):
```json
{"bead_id":"bf-2hgh8","title":"...","priority":2,"downstream_impact":1,"assignee":"test-worker","workspace":".","dry_run":true}
```

any (line 1799) and fallback-any fallback sub-branch (line 1866):
```json
{"bead_id":"bf-123","reclaimed":0,"assignee":"test-worker","workspace":"/abs/path"}
```

fallback-any first-try sub-branch (line 1838) and normal single-workspace (line 1907):
```json
{"bead_id":"bf-123","reclaimed":0,"assignee":"test-worker"}
```

**Common object fields:** every branch emits at least `bead_id` + `assignee`.

**Key characteristics:**
- Does NOT use the formatter system, `ScoredBead`, or `ClaimResult` serialization directly. It hand-picks fields into a `serde_json::json!` literal (so the JSON shape is a custom projection, not a struct's `Serialize`).
- Always a **single JSON object** — never an array, regardless of branch.
- **Empty / no-candidate case:** every branch emits the literal object `{}` via `println!("{{}}")` (lines 1777, 1817, 1884, 1920). Confirmed in source.
- Uses `println!` (adds a trailing newline), unlike `search`'s `print!`.
- **`reclaimed`** is a `usize` (count of reclaimed stale beads) from `ClaimResult` (`src/claim.rs:18-22`), serialized as a JSON number.
- **`workspace`** is emitted via `workspace_path.map(|p| p.display().to_string())` — the key is **always present** in branches that include it, with value `null` when `workspace_path` is `None` (e.g. certain `claim_any` results). In dry-run, `workspace` is always a string (defaults to `.` or the absolute workspace path).
- **Key order is alphabetical, not source order.** `serde_json = "1"` in `Cargo.toml:13` does **not** enable the `preserve_order` feature, so `json!` builds a `BTreeMap`-backed `Map` and `Value::Display` emits keys sorted (e.g. `assignee, bead_id, downstream_impact, dry_run, priority, title, workspace`). **Empirically verified.** This differs from `search`/`list`, whose `Issue` serialization preserves struct field order.

**Why claim bypasses the Formatter:**
- `Formatter` is designed for `&[Issue]` / `&Issue`. Claim emits a **single** result object that is a hand-picked projection (mixing fields from `ScoredBead`, `ClaimResult`, and the caller's `assignee`/`workspace`), not an `Issue`. The formatter trait has no `format_claim_result` method, so the command serializes inline.

### Inconsistency Notes (search vs. claim)

1. **Formatter usage:** `search` routes through the shared `JsonFormatter`; `claim` inlines its own `json!` projection. A change to JSON conventions in the formatter (e.g. switching `list`/`search` to arrays) would not affect `claim`.

2. **Trailing newline:** `search` uses `print!` (no extra newline after the last JSONL line); `claim` uses `println!` (trailing newline).

3. **Empty case:** `search` → empty string (no output); `claim` → `{}`. Both are reasonable for their shapes (list vs. single object), but differ.

4. **Key ordering:** `search` preserves `Issue` struct field order; `claim` emits alphabetically-sorted keys (no `preserve_order` feature). Consumers parsing either as a generic object are unaffected, but byte-for-byte expectations differ.

5. **No array path:** neither command ever wraps results in a JSON array. `search` is JSONL; `claim` is a single object. (Contrast with `ready`, which does emit an array — see bf-5haf above.)

---

## Bead bf-20da: Focused Audit of `stats` and `velocity` Commands

**Date:** 2026-07-22
**Purpose:** Detailed audit of JSON output implementations for stats and velocity commands
**Method:** Verified against current source (`src/cli/mod.rs`, `src/velocity.rs`, `src/storage/sqlite.rs`) **and** empirically against the live `target/debug/bf` binary.

### Summary

| Command | Formatter Used | Output Format | Array Wrapper | Implementation Pattern |
|---------|---------------|---------------|---------------|----------------------|
| `stats`    | None (bypasses formatter) | Single JSON object | NO | **Direct** `serde_json::to_string_pretty(&Stats)` + `println!` |
| `velocity` | None (bypasses formatter) | JSON array of objects | YES | **Direct** `serde_json::to_string_pretty(&Vec<VelocityStats>)` + `println!` |

### Stats Command (`bf stats --format json`)

**Implementation:** `src/cli/mod.rs:2574-2633`. JSON branch is the match arm at `src/cli/mod.rs:2587-2590`:

```rust
let stats = storage.get_stats()?;
match format {
    "json" => {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    }
    _ => { /* text: Total / Open / In Progress / Closed */ }
}
```

**Formatter methods used:** NONE. `stats` does not touch the `Formatter` trait / `JsonFormatter` at all — it does not call `format_issues`, `format_issue`, or `format_error`. It serializes the `Stats` struct directly.

**Data source:** `storage.get_stats()` → `Stats` (`src/storage/sqlite.rs:1529-1556`). `get_stats` runs four `SELECT COUNT(*) FROM issues WHERE ... AND deleted_at IS NULL` queries (total, open, in_progress, closed) and casts the `i64` counts to `usize`.

**Serialized struct:** `Stats` (`src/storage/sqlite.rs:1980-1986`):
```rust
#[derive(Debug, Clone, serde::Serialize)]
pub struct Stats {
    pub total: usize,
    pub open: usize,
    pub in_progress: usize,
    pub closed: usize,
}
```

**Output format:** a single JSON object, pretty-printed (2-space indent), with a trailing newline from `println!`. Key order follows struct declaration order (`total`, `open`, `in_progress`, `closed`) — NOT alphabetical (the `Serialize` derive preserves field order; contrast with `claim`'s `json!` macro, which sorts keys — see bf-2x0p).

**Empirically verified** against `target/debug/bf`:
```json
{
  "total": 964,
  "open": 102,
  "in_progress": 5,
  "closed": 662
}
```

**Key characteristics:**
- Bypasses the formatter system; serializes the `Stats` struct via its `Serialize` derive
- Single object — never an array
- Pretty-printed (`to_string_pretty`), unlike the compact `to_string` used by `list`/`search`/`show`
- No "empty" case to speak of: `get_stats` always returns an object (counts may be `0`). On an empty db it would emit `{"total":0,"open":0,"in_progress":0,"closed":0}`
- Uses `println!` (trailing newline)

**⚠ Inconsistency / latent bug — breakdown flags ignore `--format json`:** The `--by-type` / `--by-priority` / `--by-assignee` / `--by-label` breakdowns are emitted by `if by_type { ... }` blocks (`src/cli/mod.rs:2599-2630`) that run *after* the format match and always use plain `println!` text, regardless of `format`. So `bf stats --format json --by-type` produces the JSON object **followed by text**:

```
{
  "total": 964,
  "open": 102,
  "in_progress": 5,
  "closed": 662
}

By type:
  task (794)
  epic (106)
  ...
```

This is not valid JSON as a whole — a machine consumer gets the object and then garbage. (Empirically verified.) If JSON consumers ever need breakdowns, the breakdown data would need to be folded into the JSON object (e.g. nested `by_type`/`by_priority` maps) rather than printed as text.

### Velocity Command (`bf velocity --format json`)

**Implementation:** `src/cli/mod.rs:2846-2915`. JSON branch is the match arm at `src/cli/mod.rs:2860-2863`:

```rust
let stats = storage.with_immediate_transaction(|tx| {
    crate::velocity::get_velocity_stats(tx, model.as_deref(), harness.as_deref())
})?;
match format {
    "json" => {
        println!("{}", serde_json::to_string_pretty(&stats)?);
    }
    "toon" => { /* per-stat text block */ }
    _ => { /* columnar table */ }
}
```

**Formatter methods used:** NONE. Like `stats`, `velocity` does not invoke any `Formatter`/`JsonFormatter` method; it serializes the `Vec<VelocityStats>` directly. (`velocity` is the only command in this file that also has a `"toon"` arm in addition to `"json"`/default.)

**Data source:** `crate::velocity::get_velocity_stats(tx, model, harness)` → `Vec<VelocityStats>` (`src/velocity.rs:343-378`), run inside `storage.with_immediate_transaction`. Optional `--model` / `--harness` filters narrow the (model, harness, issue_type) tuples returned.

**Serialized struct:** `VelocityStats` (`src/velocity.rs:49-59`):
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityStats {
    pub model: String,
    pub harness: String,
    pub issue_type: String,
    pub sample_count: i64,
    pub p50_seconds: Option<i64>,
    pub p90_seconds: Option<i64>,
    pub avg_seconds: Option<f64>,
    pub last_updated: Option<String>,
}
```

**Output format:** a JSON **array** of `VelocityStats` objects, pretty-printed (2-space indent), trailing newline from `println!`. Key order follows struct declaration order. `Option` fields serialize as `null` when absent (insufficient samples for a given percentile/avg). Empty result → `[]`.

**Empirically verified** empty case against `target/debug/bf` (this workspace's db has no velocity rows):
```json
[]
```

(A non-empty row would be a single object inside the array with all eight fields above — `p50_seconds`/`p90_seconds` as integers-or-`null`, `avg_seconds` as a float-or-`null`, `last_updated` as an RFC3339 string-or-`null`. The field set is confirmed from the `Serialize` derive since no populated velocity rows were available to sample.)

**Key characteristics:**
- Bypasses the formatter system; serializes `Vec<VelocityStats>` via the struct's `Serialize` derive
- JSON array (matching `ready`/`log`/`mitosis`/`labels`, unlike `list`/`search`'s JSONL)
- Pretty-printed; trailing newline via `println!`
- Empty case → `[]` (unlike `list`/`search`, which emit nothing)
- Key order = struct field order (not alphabetical)

**Correction to the original comprehensive audit (bf-xmwq, §6 velocity):** that example is **stale**. It listed only 7 fields and omitted `last_updated`, and showed `p50_seconds`/`p90_seconds`/`avg_seconds` as non-Option (`"p50_seconds": 120`). The current `VelocityStats` struct has **8** fields, with `p50_seconds`/`p90_seconds` as `Option<i64>`, `avg_seconds` as `Option<f64>`, and `last_updated` as `Option<String>` (all of which serialize to `null` when `None`).

### Inconsistency Notes (stats vs. velocity, and vs. the rest)

1. **Formatter usage:** neither uses the formatter — both serialize a derived-`Serialize` struct directly. This is appropriate: both return aggregate/non-`Issue` data (`Stats`, `Vec<VelocityStats>`), so the `Formatter` trait (designed for `&[Issue]`/`&Issue`) has no fitting method. Consistent with `claim`'s rationale (bf-2x0p).

2. **Container shape differs by data, not by whim:** `stats` is a single summary object → object output; `velocity` is a list of rows → array output. Both are the natural shape for their data (contrast with `list` vs. `ready`, where two bead-*list* commands disagree on JSONL-vs-array).

3. **Pretty-printing + trailing newline:** both use `to_string_pretty` + `println!`, consistent with each other and with `log`/`labels`/`schema`/`mitosis`/`critical-path`/`dep tree`, but unlike the compact `to_string` of `list`/`search`/`show`.

4. **Key ordering:** both preserve struct field order (via `Serialize` derive), consistent with `list`/`search`/`show`/`ready` — and unlike `claim`/`schema`/`dep tree`, which sort keys alphabetically through the `json!` macro.

5. **The real defect is in `stats`, not `velocity`:** `stats --format json` plus any breakdown flag (`--by-type`, `--by-priority`, `--by-assignee`, `--by-label`) emits JSON followed by un-ignorable plain-text lines, so the combined stdout is not valid JSON. `velocity` has no such secondary output and is clean.

---

## Original Comprehensive Audit (bf-xmwq)

---

## Command-by-Command Analysis

### 1. `list` (cmd_list)
**Location:** `src/cli/mod.rs:995-1079`

**Current Method:** `get_formatter().format_issues()`
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Output Format:** JSONL (one JSON object per line, newline-separated)
- Empty: `(no output)`
- Single: `{"id":"bf-123",...}`
- Multiple: `{"id":"bf-123",...}\n{"id":"bf-456",...}`

**Notes:** Uses `JsonFormatter::format_issues()` which returns newline-separated JSON objects (NOT a JSON array)

**Needs Fix:** NO

---

### 2. `ready` (cmd_ready)
**Location:** `src/cli/mod.rs:1229-1271`

**Current Method:** Custom `println!` with `serde_json::to_string()`
```rust
println!("{}", serde_json::to_string(&candidates)?);
```

**JSON Output Format:** JSON array
- Empty: `[]`
- Single: `[{...}]`
- Multiple: `[{...},{...}]`

**Output Structure:**
```json
[{
  "id": "bf-123",
  "title": "...",
  "priority": 2,
  "downstream_impact": 5,
  "critical_float": 0
}]
```

**Needs Fix:** **YES** - inconsistent with `list`/`search` (returns JSON array instead of JSONL for bead-like objects)

---

### 3. `claim` (cmd_claim)
**Location:** `src/cli/mod.rs:1273-1526`

**Current Method:** Custom `println!` with `serde_json::json!`
```rust
let output = serde_json::json!({
    "bead_id": candidate.id,
    "title": candidate.title,
    "priority": candidate.priority,
    "downstream_impact": candidate.downstream_impact,
    "assignee": assignee,
    "workspace": path.display().to_string(),
    "dry_run": true
});
println!("{}", output);
```

**JSON Output Format:** Single JSON object (never array)
- Success: `{"bead_id":"bf-123",...}`
- Empty: `{}`

**Output Structure:**
```json
{
  "bead_id": "bf-123",
  "reclaimed": false,
  "assignee": "worker-name",
  "workspace": "/path/to/workspace"
}
```

**Needs Fix:** NO - claim returns a single bead result, object format is appropriate

---

### 4. `search` (cmd_search)
**Location:** `src/cli/mod.rs:2051-2092`

**Current Method:** `get_formatter().format_issues()`
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

**JSON Output Format:** JSONL (newline-separated objects, same as `list`)

**Needs Fix:** NO - consistent with `list` command

---

### 5. `stats` (cmd_stats)
**Location:** `src/cli/mod.rs:2094-2153`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&stats)?);
```

**JSON Output Format:** Single JSON object with nested stats
```json
{
  "total": 42,
  "open": 10,
  "in_progress": 5,
  "closed": 27
}
```

**Needs Fix:** NO - stats is a different data structure, object format is appropriate

---

### 6. `velocity` (cmd_velocity)
**Location:** `src/cli/mod.rs:2345-2414`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&stats)?);
```

**JSON Output Format:** JSON array of velocity stat objects
```json
[
  {
    "model": "claude-sonnet-5",
    "harness": "claude-code",
    "issue_type": "task",
    "sample_count": 15,
    "p50_seconds": 120,
    "p90_seconds": 300,
    "avg_seconds": 180.5
  }
]
```

**Needs Fix:** NO - velocity is a different data structure

---

### 7. `log` (cmd_log)
**Location:** `src/cli/mod.rs:2457-2588`  
**Helper:** `src/log.rs:142-144`

**Current Method:** Custom `println!` with `crate::log::format_events_json()`
```rust
println!("{}", crate::log::format_events_json(&events)?);
// In log.rs:
pub fn format_events_json(events: &[Event]) -> Result<String> {
    Ok(serde_json::to_string_pretty(events)?)
}
```

**JSON Output Format:** JSON array
```json
[
  {
    "id": 1,
    "issue_id": "bf-123",
    "event_type": "StatusChanged",
    "actor": "claude",
    "old_value": "open",
    "new_value": "in_progress",
    "comment": null,
    "created_at": "2026-07-03T12:00:00Z"
  }
]
```

**Needs Fix:** NO - events are a different data structure

---

### 8. `dep tree` (cmd_dep -> DepCommands::Tree)
**Location:** `src/cli/mod.rs:1890-1962`

**Current Method:** Custom `println!` with `serde_json::json!`
```rust
let output = serde_json::json!({
    "root_id": id,
    "direction": direction,
    "max_depth": max_depth,
    "nodes": nodes
});
println!("{}", serde_json::to_string_pretty(&output)?);
```

**JSON Output Format:** Single JSON object
```json
{
  "root_id": "bf-123",
  "direction": "down",
  "max_depth": 10,
  "nodes": [
    {
      "id": "bf-456",
      "depth": 1,
      "dep_type": "blocks",
      "status": "open",
      "priority": 2,
      "title": "...",
      "path": "bf-123 > bf-456"
    }
  ]
}
```

**Needs Fix:** NO - dep tree is a different data structure

---

### 9. `show` (cmd_show)
**Location:** `src/cli/mod.rs:1081-1142`

**Current Method:** Custom `println!` with `serde_json::to_string()`
```rust
let mut out = issue;
out.dependencies = vec![];
out.comments = vec![];
// Wrap in array so NEEDLE's parse_single_bead (Vec<Bead> → first) works
println!("{}", serde_json::to_string(&vec![out])?);
```

**JSON Output Format:** **ALWAYS a JSON array with single element**
```json
[{
  "id": "bf-123",
  "title": "...",
  "status": "open",
  "priority": 2,
  ...
}]
```

**Needs Fix:** **MAYBE** - wraps single bead in array for NEEDLE compatibility. This is intentional but should be documented.

---

### 10. `labels` (cmd_labels)
**Location:** `src/cli/mod.rs:2009-2022`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&labels)?);
```

**JSON Output Format:** JSON array of strings
```json
["bug", "enhancement", "phase-1"]
```

**Needs Fix:** NO - labels are a different data structure

---

### 11. `schema` (cmd_schema)
**Location:** `src/cli/mod.rs:2155-2203`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
// For "all" target:
let output = serde_json::json!({"schema": crate::storage::schema::SCHEMA_SQL});
println!("{}", serde_json::to_string_pretty(&output)?);

// For specific bead:
println!("{}", serde_json::to_string_pretty(&issue)?);
```

**JSON Output Format:** Varies by target
- `target="all"`: `{"schema": "CREATE TABLE..."}`
- `target="<bead-id>"`: Full issue object with annotations

**Needs Fix:** NO - schema output is special-purpose

---

### 12. `mitosis` (cmd_mitosis)
**Location:** `src/cli/mod.rs:1737-1778`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&results)?);
```

**JSON Output Format:** JSON array of batch operation results
```json
[
  {
    "op": "create",
    "id": "bf-456",
    "status": "ok",
    "error": null
  },
  {
    "op": "dep_add_blocker",
    "id": null,
    "status": "ok",
    "error": null
  }
]
```

**Needs Fix:** NO - mitosis output is batch results, not issue data

---

### 13. `critical-path` (cmd_critical_path)
**Location:** `src/cli/mod.rs:2590-2631`

**Current Method:** Custom `println!` with `serde_json::to_string_pretty()`
```rust
println!("{}", serde_json::to_string_pretty(&result)?);
```

**JSON Output Format:** Single JSON object
```json
{
  "root_id": "bf-123",
  "min_remaining": 3,
  "longest_chain": ["bf-456", "bf-789", "bf-123"],
  "beads": [
    {
      "bead_id": "bf-456",
      "float": 0
    }
  ]
}
```

**Needs Fix:** NO - critical path is a different data structure

---

## Consolidated Table

| Command | Current Method | Output Type | Array Format | Needs Fix? |
|---------|---------------|-------------|--------------|------------|
| `list` | `get_formatter().format_issues()` | Issue list | JSONL (newline-separated) | NO |
| `ready` | Custom `serde_json::to_string()` | ReadyCandidate array | `[{...}]` | **YES** - inconsistent |
| `claim` | Custom `serde_json::json!()` | Single ClaimResult | Object `{...}` | NO |
| `search` | `get_formatter().format_issues()` | Issue list | JSONL (newline-separated) | NO |
| `stats` | Custom `serde_json::to_string_pretty()` | Stats object | Object `{...}` | NO |
| `velocity` | Custom `serde_json::to_string_pretty()` | VelocityStat array | `[{...}]` | NO |
| `log` | Custom `format_events_json()` | Event array | `[{...}]` | NO |
| `dep tree` | Custom `serde_json::json!()` | DepTree object | Object with nodes array | NO |
| `show` | Custom `serde_json::to_string()` | Issue array (single) | `[{...}]` | **MAYBE** - intentional for NEEDLE |
| `labels` | Custom `serde_json::to_string_pretty()` | String array | `["a","b"]` | NO |
| `schema` | Custom `serde_json::to_string_pretty()` | Schema/Issue object | Varies | NO |
| `mitosis` | Custom `serde_json::to_string_pretty()` | BatchResult array | `[{...}]` | NO |
| `critical-path` | Custom `serde_json::to_string_pretty()` | CriticalPath object | Object with beads array | NO |

---

## Expected JSON Array Format for Consistency

### Issue List Commands (list, search, ready)

**Current (list/search - JSONL):**
```json
{"id":"bf-123","title":"First","status":"open"}
{"id":"bf-456","title":"Second","status":"closed"}
```

**Current (ready - JSON array):**
```json
[{"id":"bf-123","title":"First","priority":2}]
```

**Expected (for array consistency):**
```json
[
  {"id":"bf-123","title":"First","status":"open"},
  {"id":"bf-456","title":"Second","status":"closed"}
]
```

**Rationale:** 
- JSONL is non-standard and harder to parse with standard JSON tools
- Most CLIs output proper JSON arrays for list results
- `ready` already uses JSON arrays, creating inconsistency

### Single Issue Commands (show)

**Current:**
```json
[{"id":"bf-123","title":"..."}]
```

**Expected (if not for NEEDLE compatibility):**
```json
{"id":"bf-123","title":"..."}
```

**Rationale:** Wrapping single items in arrays is unusual. The current behavior is intentional for NEEDLE compatibility (`parse_single_bead` expects `Vec<Bead>`) but should be documented with a comment.

---

## JsonFormatter Behavior

**Location:** `src/format/json.rs`

**Key behavior:**
- `format_issues()` returns newline-separated JSON (JSONL), NOT a JSON array
- Strips `dependencies` and `comments` from issues before serializing (br compatibility)
- Empty result returns `""` (no output)

**Code:**
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

---

## Key Findings

### Commands using `get_formatter().format_issues()`
1. `list` (line 1076)
2. `search` (line 2089)

### Commands using custom JSON serialization
1. `ready` - `serde_json::to_string()` (candidate array)
2. `claim` - `serde_json::json!()` (claim result object)
3. `stats` - `serde_json::to_string_pretty()` (stats object)
4. `velocity` - `serde_json::to_string_pretty()` (velocity array)
5. `log` - `format_events_json()` → `serde_json::to_string_pretty()` (event array)
6. `dep tree` - `serde_json::json!()` → `serde_json::to_string_pretty()` (dep tree object)
7. `show` - `serde_json::to_string()` (single-issue array)
8. `labels` - `serde_json::to_string_pretty()` (label string array)
9. `schema` - `serde_json::json!()` or `serde_json::to_string_pretty()`
10. `mitosis` - `serde_json::to_string_pretty()` (batch result array)
11. `critical-path` - `serde_json::to_string_pretty()` (critical path object)

### Inconsistencies Identified

1. **`list`/`search` use JSONL, `ready` uses JSON array**
   - All three return bead-like objects
   - Inconsistent format makes parsing harder for consumers

2. **`show` wraps single issue in array for NEEDLE**
   - Not documented in code
   - Surprising behavior for users expecting a single object

3. **Pretty-printing inconsistency**
   - Some commands use `to_string_pretty()` (formatted)
   - Others use compact `to_string()` (single line)
   - `list`/`search` use JSONL (not an array at all)

---

## Recommendations

1. **Standardize issue list output format:**
   - Option A: Change `list`/`search` to JSON arrays (breaking change, more standard)
   - Option B: Change `ready` to JSONL (consistent with current approach)
   - Option C: Add a `--array` flag to choose between JSONL and array formats

2. **Document `show` NEEDLE compatibility:**
   ```rust
   // Wrap in array so NEEDLE's parse_single_bead (Vec<Bead> → first) works
   println!("{}", serde_json::to_string(&vec![out])?);
   ```

3. **Consider adding `JsonArrayFormatter`:**
   - New formatter type that returns proper JSON arrays
   - Allows users to choose between JSONL and array formats via `--format json-array`

4. **Document expected behavior in CLI help:**
   - Add notes to `--help` output about JSON formats
   - Document in user-facing README
