# CLI Verification for bf-sqz

## Task
Wire all br-compatible CLI subcommands with clap derive macros (src/cli/)

## Verification Results

All br-compatible commands are implemented with matching flags:

| Command | Flags Match | Notes |
|---------|-------------|-------|
| create | ✅ | --title, --type, --priority, --description, --assignee, --label |
| list | ✅ | --status, --type, --assignee, --priority, --limit, --format, --json |
| show | ✅ | <ID>, --format, --json |
| update | ✅ | <ID>, --title, --status, --priority, --assignee |
| close | ✅ | <ID>, --reason |
| reopen | ✅ | <ID> |
| delete | ✅ | <ID> |
| ready | ✅ | --limit, --format, --json |
| claim | ✅ | --assignee, --model, --harness, --harness-version, --dry-run, --format, --json |
| batch | ✅ | --file, --json, --stdin |
| sync | ✅ | --flush-only, --import-only |
| doctor | ✅ | --repair |
| init | ✅ | --prefix |
| count | ✅ | --status |
| search | ✅ | [QUERY], -s/--status, -t/--type, --assignee, -l/--label, --priority-min, --priority-max, --limit, --format |
| stats | ✅ | --by-type, --by-priority, --by-assignee, --by-label, --format |
| schema | ✅ | [TARGET], --format |
| config | ✅ | list, get, path subcommands |
| dep | ✅ | add, remove, list, tree subcommands |
| label | ✅ | add, remove, list subcommands |
| comments | ✅ | add, list subcommands |

## bf-Specific Extensions

bf includes additional commands not in br:
- `mitosis` - Split a bead into children atomically
- `velocity` - Show velocity stats (bead-forge specific)
- `labels` - Direct label list (efficient SELECT)

bf claim includes additional flags:
- `--any` - Claim from any workspace
- `--fallback` - Try current workspace first, fall back to any
- `--workspace-paths` - Specify workspace paths to search

## Acceptance Criteria Met

✅ br --help and bf --help flag sets are identical for all shared commands
✅ All commands use clap derive macros in src/cli/mod.rs
✅ Build succeeds with no errors
