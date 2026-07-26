# bf-5sf0m5 — Search with spaces

**Outcome:** Verification bead — no code change needed. The feature already
works as designed and is fully covered by tests.

## What the bead asked

The title "Search   with    spaces" maps directly to the fixture string in
`src/cli/tests/search_json_tests.rs::test_search_json_whitespace_in_query`
(`fixtures::create_bead("Search   with    spaces")`).

## How `bf search` handles spaces (design)

- CLI positional: `query: Vec<String>` with `#[arg(num_args = 0..)]`, joined
  with single spaces (`query.join(" ")`) in `src/cli/mod.rs` (~line 1299) so
  `bf search foo bar` == `bf search "foo bar"`.
- Storage: the joined string is matched with a literal `LIKE '%query%'`
  substring against `title`/`description` (`src/storage/sqlite.rs`,
  `search_issues`, ~line 1522). LIKE is whitespace-sensitive, so the query
  must contain the *exact* run of spaces found in the stored text.

## Verification

- `cargo test --lib search_json` → **54 passed; 0 failed**, including:
  - `test_search_json_whitespace_in_query` — exact multi-space substring
    matches, single-space variant does not, single word matches.
  - `test_search_json_multiword_unquoted_query` — multiple unquoted positional
    args are joined with a single space (regression for the old
    "unexpected argument 'bar' found" error).

- Direct CLI demo (`target/debug/bf`), bead titled `Search   with    spaces`:
  - `search "Search   with"` → 1 match (exact substring) ✓
  - `search "Search with"`  → 0 matches (spacing mismatch, expected) ✓
  - `search "spaces"`       → 1 match (single word) ✓

## Why the prior attempt failed

The previous run (metadata.json: `exit_code 1`) terminated with
`terminal_reason: max_turns` / "Reached maximum number of turns (30)" — the
agent ran out of turns exploring and never committed. It was not a real
failure of the feature. This run completes the verification efficiently.
