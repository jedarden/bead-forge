# bf-51aps: Acceptance-Criteria Update Logic Verification

## Status: ✅ COMPLETE (Already Implemented)

The acceptance-criteria update logic was already fully implemented in the codebase.

## What Was Verified

### 1. CLI Flag Handling
**Location:** `src/cli/mod.rs:188`
```rust
/// New acceptance criteria
#[arg(long)]
acceptance_criteria: Option<String>,
```
✅ The `--acceptance-criteria` flag is properly defined in the Update command.

### 2. Update Command Wiring
**Location:** `src/cli/mod.rs:1666-1717`
```rust
fn cmd_update(
    ...
    acceptance_criteria: Option<String>,
    ...
) -> Result<()> {
    ...
    let changes = IssueChanges {
        ...
        acceptance_criteria,
        ...
    };
    storage.update_issue(id, &changes)?;
```
✅ The CLI parameter flows through to storage via IssueChanges struct.

### 3. Storage Update Logic
**Location:** `src/storage/sqlite.rs:519-522`
```rust
if let Some(ref acceptance_criteria) = changes.acceptance_criteria {
    updates.push("acceptance_criteria = ?");
    params.push(Box::new(acceptance_criteria.clone()));
}
```
✅ The acceptance_criteria is properly added to the UPDATE statement.

### 4. Database Schema
**Location:** `src/storage/schema.rs:18`
```sql
acceptance_criteria TEXT NOT NULL DEFAULT '',
```
✅ The acceptance_criteria column exists in the issues table.

### 5. Model Definition
**Location:** `src/model.rs:449-450, 884`
```rust
pub struct Issue {
    ...
    /// Acceptance criteria.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_criteria: Option<String>,
    ...
}

pub struct IssueChanges {
    ...
    pub acceptance_criteria: Option<String>,
    ...
}
```
✅ Both Issue and IssueChanges structs include acceptance_criteria.

## Test Results

### Test 1: Create bead with acceptance criteria
```bash
bf create --title "Test acceptance criteria"
# Output: bf-3cx
```

### Test 2: Update with multiline acceptance criteria
```bash
bf update bf-3cx --acceptance-criteria "Test criteria line 1

Line 2
Line 3"
# Output: Updated bead bf-3cx
```

### Test 3: Verify database storage
```sql
SELECT acceptance_criteria FROM issues WHERE id='bf-3cx';
# Result:
# Test criteria line 1
#
# Line 2
# Line 3
```
✅ Multiline text is correctly preserved with newlines.

### Test 4: Verify JSON output
```bash
bf show bf-3cx --format json | jq '.[0].acceptance_criteria'
# Result: "Test criteria line 1\n\nLine 2\nLine 3"
```
✅ JSON output correctly represents newlines as `\n`.

## Conclusion

All acceptance criteria for bead bf-51aps are met:
- ✅ Update command handles --acceptance-criteria flag
- ✅ Acceptance criteria is written to the issue's acceptance_criteria field in storage
- ✅ Uses the actual update path (same as other field updates via IssueChanges)
- ✅ Proper handling of multi-line acceptance criteria text

The implementation is complete and functional. No code changes were required.
