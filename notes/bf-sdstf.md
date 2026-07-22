# Epic CLI Labels Test Results

## Bead: bf-sdstf

### Test: Epic Creation with Labels via CLI

**Date:** 2026-07-06

### Test Summary

✅ **PASS** - Epic creation with labels via CLI works correctly

### Commands Tested

#### 1. Create Epic with Multiple Labels
```bash
bf create --title "Test Epic with CLI Labels" --type epic --label phase-1 --label test --label cli-labels
```
**Result:** ✅ Success
- Created: `bf-31l74`
- Type: epic
- Labels applied: `cli-labels`, `phase-1`, `test` (alphabetically sorted)

#### 2. Show Epic Details
```bash
bf show bf-31l74
```
**Result:** ✅ Success
- Labels display correctly in show output

#### 3. List Labels (Direct Query)
```bash
bf labels bf-31l74
```
**Result:** ✅ Success
- Returns: One label per line
- Output: `cli-labels`, `phase-1`, `test`

#### 4. Add Label to Epic
```bash
bf label add bf-31l74 --label integration-test
```
**Result:** ✅ Success
- Label added: `integration-test`
- Verification: 4 labels now present

#### 5. Remove Label from Epic
```bash
bf label remove bf-31l74 --label cli-labels
```
**Result:** ✅ Success
- Label removed: `cli-labels`
- Verification: 3 labels remaining (`integration-test`, `phase-1`, `test`)

#### 6. List Epics with Type Filter
```bash
bf list --type epic | grep bf-31l74
```
**Result:** ✅ Success
- Epic appears in filtered list
- Format: `[bf-31l74] Test Epic with CLI Labels - open (P2)`

### Final State

**Epic:** `bf-31l74`
- Title: Test Epic with CLI Labels
- Type: epic
- Status: open
- Priority: P2
- Labels: `integration-test`, `phase-1`, `test`

### Implementation Notes

1. **Label Storage:** Labels are stored in the `bead_annotations` table, NOT as a column on `issues`
2. **CLI Syntax:** Label commands require `--label` flag syntax:
   - Add: `bf label add <id> --label <label>`
   - Remove: `bf label remove <id> --label <label>`
3. **Label Sorting:** Labels are displayed alphabetically in output
4. **Multiple Labels:** Can specify multiple `--label` flags during creation

### All Tests: ✅ PASS

---

### Re-verification (2026-07-22)

Independently re-ran the full flow on a fresh temp epic (`bf-5vktf`, deleted after):

- `br create --type epic --label verify-a --label verify-b` → epic created with both labels
- `br show` / `br labels` / `br label list` all agree on the label set
- `br label add --label verify-c` → added
- `br label remove --label verify-a` → removed
- Confirmed positional label args are rejected — the `--label` flag is required for
  `label add`/`label remove` (`error: unexpected argument`)

Result: ✅ all epic-with-CLI-labels operations still pass.
