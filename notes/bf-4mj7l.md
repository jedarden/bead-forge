# bf-4mj7l: Already Completed

This work was completed in commit f769879 on 2026-07-22.

## What Was Done

The commit f769879 implemented all requirements:

1. **src/cli/mod.rs**: Applied `normalize_assignee` to `cmd_create` (line 1346)
   - Removed the old `validate_assignee` call that rejected empty assignees
   - Empty/whitespace assignee now normalizes to `None` before reaching storage

2. **src/validation.rs**: Replaced `validate_assignee` with `normalize_assignee`
   - Changed from rejection validator to pure normalization helper
   - Takes `Option<&str>` → `Option<String>`, trims and collapses empty to `None`
   - Well-documented with examples and usage notes

3. **tests/test_assignee_validation.rs**: Removed all `#[ignore = "aspirational"]` attributes
   - Six tests rewritten from asserting rejection to asserting success
   - All 12 tests pass: 0 ignored, 0 failed

## Verification

```bash
# All acceptance criteria met:
cargo build 2>&1 | grep -E "^error"   # clean
cargo test --test test_assignee_validation  # 12 passed, 0 ignored
cargo fmt --check  # clean

# validation.rs and test_assignee_validation.rs are clippy-clean
# (remaining ~140 clippy findings are pre-existing, out of scope)
```

## Functional Behavior

- `bf create --assignee ''` → creates unassigned bead (no assignee persisted)
- `bf update <id> --assignee ''` → clears assignee (sets to NULL)
- `bf create/update --assignee 'alice'` → works as before (trimmed, persisted)
- `bf create/update` without `--assignee` → works as before (no assignee)

All acceptance criteria satisfied.
