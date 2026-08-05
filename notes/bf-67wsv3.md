# Fix Unused Variables - bf-67wsv3

## Task
Remove or prefix unused variables in src/cli/mod.rs and src/commit_check.rs.

## Changes Made

### src/cli/mod.rs
1. **Line 2398 (`check`)**: Prefixed with `_check` - parameter is part of `cmd_doctor` API but not currently used (health check runs by default when no other flag is given)
2. **Line 2411 (`db_path`)**: Removed entirely - variable was created but never used; doctor operations use `workspace_dir` instead
3. **Line 3156 (`envelope`)**: Prefixed with `_envelope` - parameter is part of `cmd_search` API but envelope wrapping is not yet implemented for search results

### src/commit_check.rs
1. **Line 74 (`beads_dir`)**: Prefixed with `_beads_dir` in `parse_diff_and_scan` - function only parses diff string output, doesn't access filesystem
2. **Line 145 (`beads_dir`)**: Prefixed with `_beads_dir` in `scan_staged_files` - function uses `git show` to read staged content, doesn't access beads_dir

## Verification
```bash
cargo build 2>&1 | grep -E "unused variable.*src/(cli/mod\.rs|commit_check\.rs)"
# No output - all unused variable warnings in these files are resolved
```

## Classification
- **Genuinely unused**: `db_path` (line 2411) - removed entirely
- **False positives / API parameters**: `check`, `envelope` - prefixed with underscore since they're part of the function signatures
- **Unused function parameters**: Both `beads_dir` parameters in commit_check.rs - prefixed with underscore; kept for API consistency with caller

## Commit Message
```
fix(cli,commit): Remove/prefix unused variables (bf-67wsv3)

- Remove unused db_path variable in cmd_doctor (function uses workspace_dir)
- Prefix check/_envelope/_beads_dir with underscore (API parameters, not yet implemented)

All unused variable warnings in src/cli/mod.rs and src/commit_check.rs resolved.
```