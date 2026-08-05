# Dependency Formatting Implementation (bf-2jfm3x)

## Summary
Implemented dependency formatting function for text display output.

## Implementation

### Added Function
- `format_dependencies(dependencies: &[Dependency]) -> String` in `src/format/text.rs`
- Exported via `src/format/mod.rs` for use across the codebase
- Also available in `src/format/toon.rs` for consistency

### Format Specification
- **Format**: `"Depends: bf-xxx (Title) (blocks), bf-yyy (Title)"`
- **Empty dependencies**: Returns empty string
- **Blocking indicator**: Only shows `(blocks)` for blocking dependency types
- **Non-blocking dependencies**: Shows without `(blocks)` suffix
- **Unknown titles**: Defaults to `"Unknown"` when title is `None`

### Blocking Dependency Types
The following types are considered blocking (affect ready work):
- `Blocks`
- `ParentChild`
- `ConditionalBlocks`
- `WaitsFor`

Non-blocking types like `Related`, `RelatesTo`, etc. don't show the `(blocks)` suffix.

### Testing
Added comprehensive test suite covering:
- Empty dependency list
- Single blocking dependency
- Single non-blocking dependency
- Mixed blocking and non-blocking dependencies
- Unknown title handling
- Multiple blocking dependencies

All tests pass successfully.

## Usage Example
```rust
use crate::format::format_dependencies;

let deps = vec![
    Dependency {
        depends_on_id: "bf-blocker".to_string(),
        title: Some("Blocker task".to_string()),
        dep_type: DependencyType::Blocks,
        // ... other fields
    },
    Dependency {
        depends_on_id: "bf-related".to_string(),
        title: Some("Related task".to_string()),
        dep_type: DependencyType::Related,
        // ... other fields
    },
];

let formatted = format_dependencies(&deps);
// Result: "Depends: bf-blocker (Blocker task) (blocks), bf-related (Related task)"
```

## Files Modified
- `src/format/text.rs` - Added `format_dependencies` function and tests
- `src/format/mod.rs` - Exported `format_dependencies`
- `src/format/toon.rs` - Added wrapper function for consistency

## Acceptance Criteria Met
✅ Function to format dependencies as text string
✅ Format: 'Depends: bf-xxx (Title) (blocks), bf-yyy (Title)'
✅ (blocks) indicator only shown for blocking dependencies
✅ Handles empty dependency list gracefully
✅ Returns structured string for show command output
