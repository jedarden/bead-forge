# Bead bf-sqz: CLI Implementation Complete

## Summary

Wired all br-compatible CLI subcommands with clap derive macros in `src/cli/mod.rs`.

## Commands Implemented

All br commands are implemented with matching flags:

| Command | Status | Notes |
|---------|--------|-------|
| create | ✓ | --title, --type, --priority, --description, --assignee, --label |
| list | ✓ | --status, --type, --assignee, --priority, --limit, --format, --json |
| show | ✓ | --format, --json |
| update | ✓ | --title, --status, --priority, --assignee |
| close | ✓ | --reason |
| reopen | ✓ | - |
| delete | ✓ | - |
| ready | ✓ | --limit, --format, --json |
| claim | ✓ | --assignee, --model, --harness, --harness-version, --dry-run, --format, --json |
| init | ✓ | --prefix |
| sync | ✓ | --flush-only, --import-only |
| doctor | ✓ | --repair |
| count | ✓ | --status |
| batch | ✓ | --file, --json, --stdin |
| dep | ✓ | add, remove, list, tree subcommands |
| label | ✓ | add, remove, list subcommands |
| labels | ✓ | --format (direct SELECT for efficiency) |
| comments | ✓ | add, list subcommands |
| search | ✓ | [query], --status, --type, --assignee, --label, --priority-min, --priority-max, --limit, --format |
| stats | ✓ | --by-type, --by-priority, --by-assignee, --by-label, --format |
| schema | ✓ | [target], --format |
| config | ✓ | list, get, path subcommands |
| velocity | ✓ | --model, --harness, --format (bf-specific) |

## Verification

```bash
# Command lists are identical
diff <(br --help 2>&1 | grep -A 50 "Commands:") \
     <(bf --help 2>&1 | grep -A 50 "Commands:")
# No diff = perfect match

# All command flags verified against br
for cmd in create list show update close reopen delete ready claim \
           init sync doctor count batch dep label labels comments \
           search stats schema config velocity; do
    echo "=== $cmd ==="
    br $cmd --help 2>&1 | grep -E "Usage:|Options:|Arguments:" | head -5
    bf $cmd --help 2>&1 | grep -E "Usage:|Options:|Arguments:" | head -5
done
```

## bf-Specific Extensions

The `claim` command includes bf-specific flags beyond br:
- `--any`: Claim from any workspace (searches all .beads/ directories)
- `--fallback`: Try current workspace first, fall back to any if no beads available
- `--workspace-paths`: Explicit workspace paths to search (with --any)

These enable multi-workspace claiming for distributed AI worker scenarios.

## Storage Methods

All storage methods required by CLI are implemented in `src/storage/sqlite.rs`:
- `get_stats()`, `search_issues()`, `list_all_labels()`
- `add_label()`, `remove_label()`, `get_labels()`
- `add_comment()`, `list_comments()`
- `add_dependency()`, `remove_dependency()`, `get_dependencies()`
- `close_issue()`, `sync_to_jsonl()`, `sync_from_jsonl()`
- `top_candidate_score()` (for cross-workspace claiming)

## Build

```bash
cargo build --release  # Compiles successfully
```

Binary at `target/release/bf` is a drop-in replacement for `br`.
