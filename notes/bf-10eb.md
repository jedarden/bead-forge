# Bead bf-10eb: Test invalid type

## Summary
Tested how bead-forge handles invalid/non-standard bead types and confirmed the flexible type system design.

## Test Coverage
The test script `test_bf_10eb_invalid_type.sh` comprehensively tests:

1. **Invalid type strings**: Random invalid types like "invalid-type-xyz" are accepted and stored as-is
2. **Empty types**: Empty string types are accepted
3. **Numeric types**: Numeric strings like "12345" are stored as types
4. **Special characters**: Types with special characters (!@#$%^&*()) are accepted
5. **Case variations**: Standard types (task, bug, feature, etc.) are case-insensitive and normalized to lowercase
6. **Filtering by custom types**: Can list beads by custom invalid type
7. **Status filtering with custom types**: Status filtering works correctly with custom types
8. **Unicode types**: Types with emojis and Unicode characters are accepted
9. **JSONL persistence**: Custom types are correctly persisted to issues.jsonl
10. **Mixed standard and custom types**: Handles mix of standard and custom types correctly

## Findings

### Current Behavior (Intentional Design)
- **Accepts ANY type string** and stores it as `Custom(type)` 
- Standard types (task, bug, feature, epic, chore, docs, question) are normalized to lowercase via `FromStr` implementation
- All other types are stored as `IssueType::Custom(String)` without validation
- This is **intentional flexibility** to allow custom bead types for different workflows

### Code Analysis
From `src/model.rs:198-213`:
```rust
impl FromStr for IssueType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(Self::Task),
            "bug" => Ok(Self::Bug),
            "feature" => Ok(Self::Feature),
            "epic" => Ok(Self::Epic),
            "chore" => Ok(Self::Chore),
            "docs" => Ok(Self::Docs),
            "question" => Ok(Self::Question),
            other => Ok(Self::Custom(other.to_string())),  // Accepts anything else
        }
    }
}
```

### Serde Design
From `src/model.rs:154-167`:
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
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
    Custom(String),  // Catches any non-standard value
}
```

The `#[serde(untagged)]` attribute allows the enum to have a catch-all variant that stores any string value.

## Test Results
✅ All 10 test scenarios pass successfully
✅ Custom types are correctly stored and retrieved
✅ Filtering by custom type works
✅ JSONL import/export preserves custom types
✅ Mixing standard and custom types works correctly

## Conclusion
The current behavior is **correct and intentional**. bead-forge is designed to:
1. Support standard types with case-insensitive matching
2. Allow any custom type without validation
3. Provide flexibility for different workflows and naming conventions

If strict type validation were desired, this would need to be considered a feature request, not a bug.

## Files
- Test script: `test_bf_10eb_invalid_type.sh`
- Model implementation: `src/model.rs` (IssueType enum)
- CLI usage: `src/cli/mod.rs:996` (create command)
