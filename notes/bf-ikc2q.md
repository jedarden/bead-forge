# Test Epic P0 Creation Verification

**Test ID:** bf-ikc2q
**Date:** 2026-07-05
**Epic Created:** bf-6d5f8

## Test Objective
Verify that epic beads can be created with P0 (critical) priority.

## Test Steps

1. **Flush checkpoint:** `br sync --flush-only` → Flushed 703 beads to JSONL
2. **Create epic command:**
   ```bash
   br create --title "Test Epic: P0 Creation Verification" \
     --type epic --priority 0 \
     --description "Testing epic creation with critical priority (P0)" \
     --label "test-epic" --label "p0-creation"
   ```
3. **Result:** Epic created with ID `bf-6d5f8`

## Verification

### CLI Output
```
ID: bf-6d5f8
Title: Test Epic: P0 Creation Verification
Status: open
Priority: P0
Type: epic
Description: Testing epic creation with critical priority (P0)
Labels: p0-creation, test-epic
```

### Database Verification
```sql
SELECT id, title, status, priority, issue_type FROM issues WHERE id = 'bf-6d5f8';
```
**Result:** `bf-6d5f8|Test Epic: P0 Creation Verification|open|0|epic`

## Test Result
✅ **PASS** - Epic creation with P0 priority works correctly:
- Epic type (`issue_type: epic`) stored correctly
- P0 priority (`priority: 0`) stored correctly
- Labels (`test-epic`, `p0-creation`) stored correctly
- All fields round-trip through CLI → database → CLI

## Notes
- Priority 0 = Critical (P0) per bead-forge design
- Epic is a standard issue type in the model (`IssueType::Epic`)
- The `br create` command correctly handles both `--type epic` and `--priority 0` flags
