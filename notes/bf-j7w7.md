# bf-j7w7 — claim JSON output already routes through the shared formatter

Task asked to migrate `cmd_claim` from 6 hand-written `serde_json::json!`
blocks (described as lines 1351–1375, 1394–1419, 1434–1486, 1503–1522) to the
`get_formatter()` pattern, emitting `{}` when no beads are available.

## Finding: already implemented in the committed tree

The migration is **already complete**. The bead's line references describe an
older revision — in the current tree `cmd_claim` lives at `src/cli/mod.rs:1692`
and already routes every output path through the shared formatter:

- `get_formatter(output_format)` is called once at the top (`cli/mod.rs:1708`).
- Each branch builds a `ClaimResultOutput` (`format/mod.rs:22`) and prints
  `formatter.format_claim_result(&out)`; the no-beads path prints
  `formatter.format_no_claim()`.
- No `serde_json::json!` macros remain anywhere in `cmd_claim` (grep confirmed).

The plumbing was introduced in `f17edfa feat(phase-3): complete CLI commands
with format module and cleanup`, which added the `Formatter::format_claim_result`
/ `format_no_claim` methods and `ClaimResultOutput`, and wired `cmd_claim` to
them. `src/cli/mod.rs` is clean (unmodified vs HEAD).

## Verification

- `cargo build` — clean, no errors/warnings.
- `cargo test --lib format::` — 7 passed, including the claim-specific tests:
  `claim_dry_run_emits_only_preview_keys`,
  `claim_single_workspace_omits_workspace_key`, `no_claim_is_empty_object`.
- Runtime smoke (`./target/debug/bf claim --assignee X --dry-run`):
  - JSON → `{"bead_id":"...","assignee":"...","workspace":".","title":"...","priority":N,"downstream_impact":N,"dry_run":true}` (reclaimed correctly omitted)
  - Toon / Text → `bf-31p84 (priority=3, impact=1, workspace=.)`
  - no-claim JSON path → `{}` (covered by unit test)

## Outcome

No code changes required — produced this notes file only (per bead instructions
when work yields no file changes). Bead closed.
