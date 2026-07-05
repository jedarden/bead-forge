# bf-2xuoe: Invalid type test verification

## Overview
Verified that bead-forge correctly handles custom/invalid issue types through the existing test suite at `tests/test_invalid_type.rs`.

## Test Coverage
The test suite verifies:
1. **Custom type creation** - Creating beads with non-standard types like "spike"
2. **Multiple custom types** - Various custom types (spike, investigation, refactor, hotfix)
3. **Special characters** - Custom types with hyphens and other special chars
4. **JSON roundtrip** - Custom types persist through JSON serialization/deserialization
5. **Mixed types** - Standard and custom types coexist correctly
6. **Edge cases** - Empty/whitespace handling

## Manual Verification Results

### Test 1: Create bead with custom type
```bash
bf create --title "Test invalid type" --type "invalid-type" --priority 2
# Output: bf-5p8pl
```

### Test 2: Verify type preserved in show output
```bash
bf show bf-5p8pl
# Output shows: Type: invalid-type
```

### Test 3: Verify type preserved in JSON output
```bash
bf show bf-5p8pl --json
# JSON contains: "issue_type":"invalid-type"
```

### Test 4: Database verification
```bash
sqlite3 .beads/beads.db "SELECT id, issue_type FROM issues WHERE id = 'bf-2xuoe'"
# Output: bf-2xuoe|invalid_type
```

## Implementation Details

The `IssueType` enum in `src/model.rs` uses `#[serde(untagged)]` for the Custom variant:

```rust
pub enum IssueType {
    #[default]
    Task,
    Bug,
    Feature,
    Epic,
    Chore,
    Docs,
    Question,
    #[serde(untagged)]
    Custom(String),
}
```

This allows any string that doesn't match the standard variants to be captured as a Custom variant, preserving the original value.

## Conclusion
The invalid/custom type handling is working correctly. The test suite at `tests/test_invalid_type.rs` provides comprehensive coverage of the functionality.
