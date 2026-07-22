# bf-xmwq: Audit JSON output implementations in all commands (umbrella)

## Task
Identify all commands with `--format json` and document their current JSON output
approach (custom loops vs. shared formatter). Scope: `list`, `ready`, `search`,
`claim`, `stats`, `velocity`. Deliverable: `src/json_formatter_audit.md`.

## Role of this bead
`bf-xmwq` is the **umbrella** audit bead (labels: `umbrella`, `deferred`). The
focused work was done in split-child beads and consolidated into one authoritative
document by **bf-4w48** ("Write JSON formatter audit summary"), which is now
**closed** (commit `d84bbb6`, later amended/`ae8050e`). The consolidated file
`src/json_formatter_audit.md` was produced by bf-4w48 and references bf-xmwq as
the parent. bf-xmwq was blocked-by bf-4w48; that blocker is now resolved, so this
bead's deliverable already exists.

## What I did (this run)
Re-opened the consolidated `src/json_formatter_audit.md` and **independently
re-verified every claim against the current source** (`src/cli/mod.rs` — 3325
lines, `src/format/json.rs`, `src/format/mod.rs`). No Rust was changed; this is a
docs/verify run. All claims hold:

### Formatter-using commands (Family A) — all confirmed
| Command | `format_issues()` call site | Verified |
|---------|----------------------------|----------|
| `list`   | `src/cli/mod.rs:1435` | ✓ |
| `ready`  | `src/cli/mod.rs:1640` (formatter setup `:1632`) | ✓ migrated; `[]` empty-case at `:1637-1638` |
| `search` | `src/cli/mod.rs:2569` | ✓ |
| `recent` | `src/cli/mod.rs:3322` | ✓ |

### Custom-serialization commands (Family B) — key ones confirmed
- `claim` — `serde_json::json!({...})` projection, 5 branches at exact lines
  `:1755`/`:1799`/`:1838`/`:1866`/`:1907`; `{}` empty case at `:1777`/`:1817`/`:1884`/`:1920`.
  Always a single object, never an array. ✓
- `stats` — `serde_json::to_string_pretty(&stats)` at `:2589`; breakdown blocks
  (`By type:` etc.) at `:2599-2630` run regardless of `--format` → the §5.1
  JSON-corruption defect is **confirmed present**. ✓
- `velocity` — `serde_json::to_string_pretty(&stats)` at `:2862` (pretty array). ✓
- `show` — `serde_json::to_string(&vec![out])` single-element array `[{…}]` at
  `:1464` (NEEDLE contract). ✓

### Structural claims confirmed
- `JsonFormatter::format_issues` (`src/format/json.rs:17-29`) strips
  `dependencies`/`comments`, returns **JSONL** (`.join("\n")`), empty → `""`. ✓
- Compact vs. pretty split, empty-case split, trailing-newline split, and the
  `json!`-macro-alphabetical vs. struct-declaration key ordering all verified. ✓

## Result
The deliverable `src/json_formatter_audit.md` exists, is current, and is accurate.
This bead (the umbrella) required no new source or doc changes beyond this
verification record. The audit effort — bf-xmwq and its children
(bf-5haf list/ready, bf-2x0p search/claim, bf-20da stats/velocity, bf-64zt ready
migration, bf-4w48 consolidation) — is complete.

Acceptance criteria for bf-xmwq: all satisfied by the consolidated file
(documented per-command approaches ✓; formatter-users listed ✓;
custom-loop users listed ✓; array/object inconsistencies noted ✓;
`src/json_formatter_audit.md` created ✓).
