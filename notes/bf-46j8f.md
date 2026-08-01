# P0 Epic JSONL Export Verification (bf-46j8f)

## Test Summary

Verified that P0 epics are correctly exported to JSONL and can be round-tripped through import/export operations.

## Test Procedure

1. **Created P0 Epic**
   - ID: `bf-3xtcth`
   - Title: "Test P0 Epic JSONL Export"
   - Type: `epic`
   - Priority: `0` (P0)
   - Status: `open`

2. **Export to JSONL**
   - Command: `bf sync --flush-only`
   - Result: Exported 1495 beads successfully
   - Verification: Confirmed JSONL contains proper `issue_type: "epic"` and `priority: 0`

3. **Round-trip Import Test**
   - Backed up database, cleared it, and imported from JSONL
   - Command: `bf sync --import-only`
   - Result: Imported 1495 beads successfully
   - Verification: All P0 epic fields preserved correctly

## Verification Results

### JSONL Export Verification
```json
{
  "id": "bf-3xtcth",
  "title": "Test P0 Epic JSONL Export",
  "issue_type": "epic",
  "priority": 0,
  "status": "open"
}
```

### Post-Import Verification
```json
{
  "id": "bf-3xtcth",
  "title": "Test P0 Epic JSONL Export",
  "description": "This is a test epic to verify P0 priority and epic type are correctly exported to JSONL",
  "issue_type": "epic",
  "priority": 0,
  "status": "open"
}
```

## Acceptance Criteria Met

✅ **'bf sync --flush-only' exports P0 epic to JSONL correctly**
- The flush command successfully exported the P0 epic to the JSONL file
- Epic was present in the exported JSONL with all fields intact

✅ **JSONL file contains the epic with proper issue_type: 'epic'**
- Confirmed via `jq` that the JSONL contains `"issue_type": "epic"`
- The epic type is serialized correctly as `"epic"` in snake_case

✅ **JSONL file contains priority: 0 for P0 epics**
- Confirmed via `jq` that the JSONL contains `"priority": 0`
- P0 priority (CRITICAL) is correctly serialized as integer `0`

✅ **JSONL round-trip preserves all P0 epic fields**
- All fields preserved: id, title, description, issue_type, priority, status
- No data loss or corruption during export/import cycle
- Content hash comparison correctly identifies unchanged beads

✅ **'bf sync --import-only' reimports P0 epic correctly**
- Import command successfully loaded the P0 epic from JSONL
- All 1495 beads imported without errors
- P0 epic fields correctly reconstructed in the database

## Technical Details

### Serde Configuration
The `Issue` struct in `src/model.rs` has proper serde attributes for epic and priority:
```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    #[serde(default)]
    pub issue_type: IssueType,  // Serializes as "epic" for Epic variant
    #[serde(default)]
    pub priority: Priority,      // Serializes as 0 for CRITICAL
    // ... other fields
}
```

### IssueType Enum
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    #[default]
    Task,
    Bug,
    Feature,
    Epic,  // ← Serializes as "epic"
    Chore,
    Docs,
    Question,
    #[serde(untagged)]
    Custom(String),
}
```

### Priority Enum
```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Priority(pub i32);  // ← Serializes as 0 for P0/CRITICAL
```

## Conclusion

All acceptance criteria for P0 epic JSONL export/import functionality are met. The implementation correctly handles:
- Epic type serialization to `"epic"` in JSONL
- P0 priority serialization to `0` in JSONL  
- Full round-trip preservation of all epic fields
- Import/export operations at scale (1495 beads)
