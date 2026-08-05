# bf-40zimi: Verify get_dirty_issue_ids test coverage

## Test Location
`src/jsonl.rs:1652-1711` - test function `get_dirty_issue_ids_returns_correct_ids`

## Test Coverage Analysis

### ✅ Acceptance Criteria Met

1. **Test exists at specified location** - Found at line 1652
2. **Basic query functionality** - Covered by lines 1693-1705
3. **Empty result case** - Covered by two scenarios:
   - Initial state with no dirty issues (line 1663)
   - After clearing dirty marks (lines 1708-1710)
4. **Multiple IDs case** - Covered with 3 dirty beads (lines 1676-1700)

### Test Structure

The test comprehensively covers:

```rust
fn get_dirty_issue_ids_returns_correct_ids() {
    // Setup: Create temporary database with schema
    
    // Test 1: Empty result when no dirty issues
    let ids = get_dirty_issue_ids(&conn).unwrap();
    assert_eq!(ids.len(), 0, "should return empty vec when no dirty issues");
    
    // Setup: Create 3 issues and mark them as dirty
    // Insert bf-1, bf-2, bf-3 into dirty_issues table
    
    // Test 2: Query returns correct IDs
    let ids = get_dirty_issue_ids(&conn).unwrap();
    assert_eq!(ids.len(), 3, "should return 3 dirty issue IDs");
    assert!(ids.contains(&"bf-1".to_string()));
    assert!(ids.contains(&"bf-2".to_string()));
    assert!(ids.contains(&"bf-3".to_string()));
    
    // Test 3: Ordering by marked_at (insertion order)
    assert_eq!(ids[0], "bf-1", "first ID should be bf-1 (oldest)");
    assert_eq!(ids[1], "bf-2");
    assert_eq!(ids[2], "bf-3", "third ID should be bf-3 (newest)");
    
    // Test 4: Empty result after clearing dirty marks
    conn.execute("DELETE FROM dirty_issues", []).unwrap();
    let ids = get_dirty_issue_ids(&conn).unwrap();
    assert_eq!(ids.len(), 0, "should return empty vec after clearing dirty marks");
}
```

## Test Execution Status

**⚠️ CANNOT EXECUTE** - Codebase has compilation errors unrelated to this test:

- Type mismatches in `src/sync.rs:216` and `src/doctor.rs:900,1097`
- Missing `From<anyhow::Error>` implementation for `BeadForgeError`
- These errors prevent compilation of the entire project

## Conclusion

The test is **well-designed and comprehensive**, covering all required acceptance criteria:
- ✅ Basic query functionality
- ✅ Empty result cases (initial + post-deletion)
- ✅ Multiple IDs case
- ✅ Ordering verification
- ✅ Correct ID containment

The test follows best practices with clear assertions and descriptive error messages. Once the unrelated compilation errors are fixed, this test should pass successfully.
