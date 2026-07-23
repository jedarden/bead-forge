# Refactored readonly_commands tests to parametric macro approach

## What was changed

Replaced 25 repetitive individual test functions in `tests/readonly_commands.rs` with three parametric macros that generate tests from command specifications:

1. **`test_readonly_command!`** - Basic macro for single-command tests
2. **`test_readonly_command_with_exit!`** - For commands that use `process::exit` (like commit-check)
3. **`test_readonly_variants!`** - For testing multiple command variants in a single test

## Benefits

- **Maintainability**: Adding new read-only commands now requires just one line
- **DRY principle**: Test setup logic is defined once in the macro, not repeated 25 times
- **Clarity**: The test specifications are concise data declarations at the bottom
- **Coverage**: All existing test cases preserved and verified passing

## Test structure

- **14 basic single-variant tests** (e.g., `test_critical_path`, `test_doctor`)
- **1 special exit-handling test** (`test_commit_check` - handles process::exit)
- **8 multi-variant tests** covering commands with multiple invocations (list, show, ready, stats, velocity, labels, config, status)

Total: **23 test functions**, all passing `cargo test --test readonly_commands`
