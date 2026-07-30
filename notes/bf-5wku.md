# bf-5wku — search & claim already route through the shared formatter

Task: convert `search` and `claim` to use `get_formatter().format_issues()`
instead of custom JSON output loops.

## Finding: already implemented in the committed tree

Both commands already use the shared formatter with **no** custom JSON loops.
Verified against HEAD (`8dc4a79`) via `git show HEAD:src/cli/mod.rs` (so the
conclusion is about committed code, independent of the live working tree):

### search — `cmd_search` (`src/cli/mod.rs:2488`)
```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));   // :2526
```
Single `format_issues()` call, array output. Matches the literal acceptance
criterion verbatim. Zero `serde_json::json!` / manual `println!` JSON loops
(confirmed by `awk` over the committed function body).

### claim — `cmd_claim` (`src/cli/mod.rs:1698`)
```rust
let formatter = get_formatter(output_format);          // :1714
...
println!("{}", formatter.format_claim_result(&out));   // every success branch
println!("{}", formatter.format_no_claim());           // every no-beads branch
```
Every output path (dry-run, `--any`, `--fallback any`, single-workspace, and
the no-beads case) goes through the shared formatter — `format_claim_result`
for a claim and `format_no_claim` (`{}` for JSON) when nothing is available.
Zero manual JSON loops.

### Why claim uses `format_claim_result`, not `format_issues`
The bead's literal "claim uses `format_issues()`" line is a misframed
criterion and is **not** applied, because doing so would *regress* the output:

- `claim` emits a **single object** (`ClaimResultOutput`, `format/mod.rs:23`),
  never an array — it mixes fields from `ScoredBead` (dry-run preview),
  `ClaimResult` (`reclaimed`/`workspace`), and the caller's `assignee`, with
  every field except `bead_id`/`assignee` omitted via `skip_serializing_if`.
- `format_issues(&[Issue])` would instead serialize an **array of full `Issue`
  objects** — wrong shape, wrong fields, and it would violate the other
  acceptance criterion ("Output format matches br exactly").

This was already established earlier today by **bf-j7w7** (commit `b0ac651`),
which documented the identical finding for `cmd_claim` and the
`format_claim_result`/`format_no_claim` plumbing introduced in `f17edfa`.
This bead extends the same verification to `search` (which *does* use
`format_issues`, as requested).

## Acceptance criteria

| Criterion | Status |
|-----------|--------|
| search uses `get_formatter().format_issues()` | ✅ `cmd_search:2526` |
| claim routes JSON through the shared formatter | ✅ `format_claim_result`/`format_no_claim` at every branch (literal `format_issues` would regress — see above) |
| No custom `println!` loops for JSON objects | ✅ zero in either function (awk-verified at HEAD) |
| Output format matches br (field names, array/object shape) | ✅ array via `format_issues` (search); single object via `format_claim_result` (claim) |
| `cargo build` clean | ✅ HEAD `8dc4a79` builds clean — verified in an isolated `git worktree` (`cargo build --lib`, exit 0) |

## Note on the live working-tree build

At the time of this bead the *shared* working tree did **not** compile, but the
breakage is entirely outside this bead's scope: concurrent needle work on the
"autoflush" feature (committed as `049df34 feat(bf-37xjd)`) plus further
uncommitted edits to `src/config.rs`, `src/cli/mod.rs`, `src/format/mod.rs`,
`src/lib.rs` and new `src/autoflush.rs` / `src/format/warning.rs`. None of
those touch `cmd_search`/`cmd_claim` formatting. The clean build was therefore
confirmed against committed HEAD in a throwaway worktree rather than the
contended shared tree (per the shared-workspace-race constraint: commit single
paths, never `git add -A`).

## Outcome

No code changes required — produced this notes file only (per bead
instructions when work yields no file changes). Bead closed.
