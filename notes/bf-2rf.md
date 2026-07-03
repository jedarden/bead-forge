# bf-2rf: Implement bf list command

## Verification

The `bf list` command was already fully implemented in the codebase at `src/cli/mod.rs:995-1079`.

### Functionality Verified

1. **SQLite Database Integration**: ✅
   - Command reads from SQLite database via `Storage::list_issues()`
   - Located in `src/storage/sqlite.rs:171-239`

2. **JSON Format**: ✅
   - `bf list --format json` returns JSONL array (one JSON object per line)
   - Tested: `./target/debug/bf list --format json | head -5` works correctly

3. **Text Format (Default)**: ✅
   - `bf list` returns formatted text table with `[ID] Title - status (P{priority})` format
   - Tested: `./target/debug/bf list` displays all beads correctly

4. **Filtering**: ✅
   - `--status <STATUS>`: Filters by status (tested with `--status open`)
   - `--type <TYPE>`: Filters by issue type
   - `--assignee <ASSIGNEE>`: Filters by assignee
   - `--priority <PRIORITY>`: Filters by priority level (tested with `--priority 0`)
   - `--annotation <KEY=VALUE>`: Filters by annotations
   - `--limit <LIMIT>`: Limits results (tested with `--limit 3`)

5. **Additional Features**: ✅
   - `--all`: Includes archived beads from archive files
   - `--json`: Alias for `--format json`
   - `--toon`: Alternative output format

### Implementation Details

The command uses the `IssueFilter` struct from `src/model.rs:863-873` to build filter queries, then uses the formatter system from `src/format/` to output in different formats.

No changes were needed - the implementation was already complete and functional.
