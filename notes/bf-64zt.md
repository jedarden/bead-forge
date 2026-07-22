# bf-64zt: Ready Command JSON Output via Shared Formatter

## Status: Complete

The `cmd_ready` JSON path was routed through the shared formatter in commit
`1c78bc9` (`fix(bf-64zt): route bf ready --format json through shared formatter`),
which is already pushed to `origin/main`. This note documents the verification
run against all three acceptance criteria.

## Implementation (`src/cli/mod.rs`, `cmd_ready`)

The `"json"` arm (≈lines 1636-1648):

```rust
let formatter = get_formatter(OutputFormat::Json);
let issues: Vec<Issue> = candidates
    .iter()
    .filter_map(|c| storage.get_issue(&c.id).ok().flatten())
    .collect();
if issues.is_empty() {
    println!("[]");
} else {
    print!("{}", formatter.format_issues(&issues));
}
```

- **Uses `get_formatter()`** — `get_formatter(OutputFormat::Json)` instead of a
  custom `serde_json::to_string`/`to_string_pretty` call. Output is JSONL (one
  `Issue` per line), consistent with `list`/`search`/`recent`.
- **Converts `ReadyCandidate` → `Issue`** — each scored candidate is resolved to
  its full `Issue` record via `storage.get_issue(&c.id)` so the formatter has
  every field (`description`, `status`, `priority`, `labels`, …).
- **Empty array when no candidates** — `issues.is_empty()` short-circuits to
  `println!("[]")`.

## Verification Tests Run

```bash
$ cargo build            # clean, no errors

# 1. With candidates — full Issue records via shared formatter:
$ ./target/debug/bf ready --format json | head -1
{"id":"bf-3cu1k","title":"Write bf-checkpoint.sh flush/diff/commit script (deploy/ + scripts/)","description":"...","status":"open","priority":2,"issue_type":"task","labels":["deferred","failure-count:2","split-child"]}
# (one Issue object per line, exit 0)

# 2. Empty workspace — empty array:
$ ./target/debug/bf init --workspace "$TMP"
Initialized bead-forge workspace in ".../​.beads"
$ ./target/debug/bf ready --format json --workspace "$TMP"
[]
# (exit 0)
```

All three acceptance criteria pass. No code change was needed this session —
the fix was already committed and pushed; this session verified it.
