# Readonly Commands Test Failures

## Test Run Summary
- Total tests: 23
- Passing tests: 20
- Failing tests: 3
- Hanging tests: 1

## Identified Failures

### 1. test_commit_check - HANGS
**Status:** Test hangs indefinitely  
**Reason:** The `cmd_commit_check` function calls `std::process::exit(0)` on success, which terminates the entire process rather than just the test thread.

**Location:** `src/cli/mod.rs:cmd_commit_check()`
```rust
fn cmd_commit_check(beads_dir: &PathBuf) -> Result<()> {
    let result = scan_staged_beads(beads_dir)?;

    if result.secrets_found.is_empty() {
        // Clean - no output on success (standard for pre-commit hooks)
        std::process::exit(0);  // ← CAUSES TEST TO HANG
    }

    eprintln!("{}", format_scan_results(&result));
    std::process::exit(1);
}
```

**Test macro:** `test_readonly_command_with_exit!` is designed to catch panics from `process::exit`, but the exit call doesn't panic in the test environment - it causes the test to hang.

### 2. test_status_variants - FAILS
**Status:** Test fails with "unrecognized subcommand 'status'"  
**Error:**
```
error: unrecognized subcommand 'status'

  tip: a similar subcommand exists: 'stats'
```

**Reason:** The `bf status` command does not exist in the CLI. The test file includes:
```rust
test_readonly_variants!(
    test_status_variants,
    [
        (["status"], "bf status"),
        (["status", "--format", "json"], "bf status --format json")
    ]
);
```

**Available commands include:** `stats`, `doctor`, `ready`, `recent`, `log`, `count`, etc. - but NO `status` command.

### 3. test_sync_status - FAILS  
**Status:** Test fails with "unexpected argument '--status' found"  
**Error:**
```
error: unexpected argument '--status' found

Usage: bf sync [OPTIONS]
```

**Reason:** The `bf sync --status` command option does not exist. The test file includes:
```rust
test_readonly_command!(test_sync_status, ["sync", "--status"], "bf sync --status");
```

**Available sync options:**
- `--flush-only` - Flush only (SQLite -> JSONL)
- `--import-only` - Import only (JSONL -> SQLite)  
- `--no-auto-flush` - Disable auto-flush for this invocation
- `--envelope` - Wrap JSON output in standard envelope

## Passing Tests (20 tests)

When excluding the 3 failing tests above, the following 20 tests pass successfully:

1. test_annotate_get
2. test_annotate_list
3. test_comments_list
4. test_config_variants
5. test_count
6. test_critical_path
7. test_dep_list
8. test_dep_tree
9. test_label_list
10. test_doctor
11. test_labels_variants
12. test_list_variants
13. test_log
14. test_ready_variants
15. test_recent
16. test_schema
17. test_search
18. test_show_variants
19. test_stats_variants
20. test_velocity_variants

## Test Output Files

- `.beads/traces/bf-qy3lc-test-run.log` - Initial test run (hung on test_commit_check)
- `.beads/traces/bf-qy3lc-test-run-full.log` - Complete test run excluding known failures

## Root Causes

1. **test_commit_check:** Design issue - using `process::exit()` in library code causes tests to hang
2. **test_status_variants:** Test file assumes `bf status` command exists, but it's not implemented
3. **test_sync_status:** Test file assumes `bf sync --status` option exists, but it's not implemented

## Recommendations

1. For `test_commit_check`: Refactor `cmd_commit_check` to return `Result<()>` instead of calling `process::exit()`. Let the main binary handle the exit code.
2. For `test_status_variants` and `test_sync_status`: Either implement the missing commands/options, or remove these test cases if they're not required for the current scope.
