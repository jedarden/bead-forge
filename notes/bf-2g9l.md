# bf-2g9l: Verify all commands use formatter for JSON output

Umbrella/verification bead (depends on **bf-1cx9** — `ready` migrated to
formatter — and **bf-2nhb** — the JSON-consistency verification suite, both
closed). Acceptance criteria:

- All commands with `--format json` use `get_formatter().format_issues()`
- No custom `println!` loops outputting individual JSON objects
- Consistent array format across all commands
- Tested with: `list`, `ready`, `search`, `claim`, `stats`, `velocity`

## Finding — all six commands route through the formatter

Verified three independent ways (source, automated test, live runtime). The
`Formatter` trait (`src/format/mod.rs:109-123`) carries a dedicated method per
output shape, and every in-scope command calls the matching one. **No custom
`println!` JSON loop remains in any of the six.**

| Command   | Formatter call site (`src/cli/mod.rs`) | Trait method | Output shape |
|-----------|----------------------------------------|--------------|--------------|
| `list`    | `:1540-1541`                           | `format_issues`      | JSONL (one `Issue`/line) |
| `ready`   | `:1750, :1758`                         | `format_issues`      | JSONL; `[]` on empty (`:1756`) |
| `search`  | `:2659-2660`                           | `format_issues`      | JSONL (one `Issue`/line) |
| `claim`   | `:1819` formatter; `:1891/1918/1942/1964/1989` + no-claim `:1893/1921/1967/1992` | `format_claim_result` / `format_no_claim` | single object (all 4 branches) |
| `stats`   | `:2711-2712`                           | `format_stats`       | single object; breakdowns folded in |
| `velocity`| `:2943-2944`                           | `format_velocity`    | JSON array |

Notes:
- `list`/`search`/`ready` resolve to full `Issue` records and share
  `format_issues`, so for the same bead their JSON is **byte-identical** (this
  is the cornerstone assertion in the test below). `ready`'s only divergence is
  the deliberate `[]` empty placeholder (bf-1cx9 contract).
- `claim` creates one `get_formatter(output_format)` at `:1819` and every branch
  (dry-run / any / fallback-any / normal) emits through
  `format_claim_result`/`format_no_claim` — never a hand-rolled `serde_json`
  literal. The `ClaimResultOutput` projection is what the trait renders.
- `stats` builds a `StatsOutput` and renders via `format_stats`. The
  `--by-type`/`--by-priority`/`--by-assignee`/`--by-label` breakdowns are folded
  **into** the object as nested maps, so `bf stats --format json --by-type` stays
  a single valid JSON document (this was the §5.1 defect in
  `src/json_formatter_audit.md`, now fixed).
- `velocity` renders a `Vec<VelocityStats>` via `format_velocity`.

## Verification (this bead)

1. **Source inspection** — read each of the six `cmd_*` functions end-to-end;
   confirmed each terminal output statement is a `format_*` call. The only
   `serde_json` calls near these commands belong to *other* commands
   (`cmd_schema` at `:2723`, `cmd_show`, etc.), not the six in scope.
2. **Automated suite** — `tests/json_formatter_verification.rs` (bf-2nhb) covers
   all six. 10/10 pass:
   ```
   cargo test --test json_formatter_verification   →   10 passed; 0 failed
   ```
   The strongest assertion is `issue_array_commands_share_formatter`: for one
   bead, `list`/`ready`/`search` emit byte-identical JSON — only possible if all
   three share `format_issues`. Any residual `println!` loop would diverge and
   fail here.
3. **Live runtime** — fresh workspace, `bf create` one bead, then each command
   with `--format json`. Confirmed:
   - `list`/`ready`/`search` print the **same** one-line JSON object for the
     bead.
   - `claim --dry-run` → `{"bead_id":…,"assignee":…,"workspace":…,"title":…,
     "priority":…,"downstream_impact":…,"dry_run":true}` (single object).
   - `stats` → `{"total":1,"open":1,"in_progress":0,"closed":0}`;
     `stats --by-type` → same object with `,"by_type":{"task":1}` folded in
     (parses as one JSON value — no trailing text).
   - `velocity` → `[]` (valid JSON array).
   - Every command's stdout parsed cleanly as JSON.

## Broader scope — "all commands"

The strict criterion ("all `--format json` commands use `format_issues`") holds
for every command that emits **`Issue` records**: `list`, `ready`, `search`, and
`recent` (`:3357-3358`). Commands emitting non-`Issue` data use the dedicated
trait method added for that shape (`format_claim_result` / `format_stats` /
`format_velocity`), which is the correct routing — the audit
(`src/json_formatter_audit.md` §7, "Non-recommendations") explicitly leaves the
remaining struct-serializing commands (`show`, `log`, `dep tree`, `labels`,
`schema`, `mitosis`, `critical-path`) on direct `serde_json` because their data
types have no `Issue` analogue and no cross-format benefit. No regression there.

## Files

- new: `notes/bf-2g9l.md` (this file)
- no source changes — all six commands already route through the formatter;
  the existing test suite and live output confirm it.
