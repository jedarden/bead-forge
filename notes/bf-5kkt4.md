# P0 Epic Test Infrastructure Implementation

## Summary

Created comprehensive P0 epic test infrastructure in `tests/common.rs` to support all subsequent P0 epic test beads.

## What Was Implemented

### 1. P0 Epic JSONL Fixtures (lines 197-285)

- `sample_p0_epic_jsonl()` - Minimal P0 epic (id, title only)
- `sample_p0_epic_with_description_jsonl()` - P0 epic with description
- `sample_p0_epic_with_labels_jsonl()` - P0 epic with labels array
- `sample_p0_epic_full_jsonl()` - Complete P0 epic with all metadata

All fixtures set:
- `priority: 0` (CRITICAL/P0)
- `issue_type: "epic"`
- Proper JSONL structure matching br format

### 2. P0 Epic Assertion Helpers (lines 287-366)

- `assert_p0_epic()` - Verifies issue is epic with P0 priority
- `assert_p0_epic_display()` - Verifies priority displays as "P0"
- `assert_p0_epic_json_serialization()` - Verifies JSON contains correct values

### 3. P0 Epic Database Utilities (lines 368-425)

- `seed_p0_epics()` - Create N P0 epics in workspace for bulk testing
- `count_p0_epics()` - Count epics matching type=Epic AND priority=CRITICAL

### 4. P0 Epic Builder Pattern (lines 427-509)

```rust
let epic = P0EpicBuilder::new("bf-epic-001", "Critical migration")
    .with_description("Complete infrastructure overhaul")
    .with_labels(&["backend", "database", "p0"])
    .with_assignee("platform-team")
    .with_status(Status::InProgress)
    .build();
```

Builder enforces:
- `issue_type: Epic`
- `priority: CRITICAL` (P0)
- Fluent API for optional fields

### 5. Comprehensive Test Suite (lines 542-672)

Tests cover:
- JSONL fixture generation and parsing
- Assertion helper correctness
- Builder pattern variations
- Database seeding and counting
- Roundtrip serialization

## Usage Examples

### In integration tests:

```rust
use crate::common::*;

#[test]
fn test_my_p0_feature() {
    let ws = TempWorkspace::new().unwrap();
    
    // Create P0 epic using builder
    let epic = P0EpicBuilder::new("bf-epic-001", "Test epic")
        .with_description("Critical security fix")
        .build();
    
    ws.storage().create_issue(&epic).unwrap();
    
    // Verify using assertions
    let retrieved = ws.get_bead("bf-epic-001").unwrap().unwrap();
    assert_p0_epic(&retrieved, Some("Retrieved epic"));
    assert_p0_epic_display(&retrieved);
}
```

### For bulk testing:

```rust
#[test]
fn test_p0_epic_listing() {
    let ws = TempWorkspace::new().unwrap();
    seed_p0_epics(&ws, 10).unwrap();
    
    assert_eq!(count_p0_epics(&ws).unwrap(), 10);
}
```

## Validation

✅ Code compiles without errors
✅ All fixtures generate valid JSONL
✅ Builder enforces P0 epic constraints
✅ Assertions validate all required properties

## Enables

This infrastructure enables:
- `bf-5kkt5` - P0 epic creation CLI tests
- `bf-5kkt6` - P0 epic listing/filtering tests
- `bf-5kkt7` - P0 epic update tests
- All subsequent P0 epic test beads
