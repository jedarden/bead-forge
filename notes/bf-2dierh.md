# bf-2dierh: test_remove_label_basic verification

## Task
Add test_remove_label_basic to tests/storage_labels.rs

## Findings
The test already exists in `tests/storage_labels.rs` (lines 7-49) and is fully implemented per the acceptance criteria.

## Test Implementation
The `test_remove_label_basic` function:
1. Creates an issue with 3 labels: "keep1", "remove-me", "keep2"
2. Verifies initial labels are all present (3 total)
3. Removes one label ("remove-me")
4. Verifies removed label is gone and other labels remain unchanged (2 total: "keep1", "keep2")
5. Verifies global label list is updated

## Verification
Test passes successfully:
```bash
cargo test --test storage_labels
test test_remove_label_basic ... ok
test result: ok. 10 passed; 0 failed
```

All acceptance criteria met:
- ✅ Function `test_remove_label_basic` exists
- ✅ Creates issue with labels, removes one, verifies removal
- ✅ Verifies other labels remain unchanged
- ✅ Test passes with `cargo test`
- ✅ Follows pattern of existing add_label tests
