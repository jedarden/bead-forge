# Batch Operations Output Format Analysis

## Task: Document batch operations output format mismatch

**Bead ID:** bf-3w72ms  
**Date:** 2026-08-05  
**Status:** COMPLETED

## Summary

After analyzing the test expectations in `tests/test_p0_multilabel_cli.rs:370-419` and comparing them with the actual batch command implementation in `src/cli/mod.rs:2689-2746`, this document confirms the batch operations output format specifications and identifies the differences between test expectations and actual output.

## Test Expectations vs Actual Output

### Test Input Format (lines 382-386)

The test provides batch operations as a JSON array:

```json
[
  {"op": "create", "title": "P0 batch test 1", "type": "task", "priority": 0, "labels": ["critical", "batch"]},
  {"op": "create", "title": "P0 batch test 2", "type": "bug", "priority": 0, "labels": ["urgent", "batch"]},
  {"op": "create", "title": "P0 batch test 3", "type": "feature", "priority": 0, "labels": ["critical", "hotfix"]}
]
```

### Test Execution (lines 389-403)

The test calls `bf batch --stdin` without any JSON format flags, then only verifies:
1. Command succeeds (`output.status.success()`)
2. Beads were created via separate `list` command

**Key Finding:** The test does NOT verify the batch command output format directly.

### Actual Batch Output Format

Based on the implementation in `src/cli/mod.rs:2716-2743`, batch produces two output formats:

#### 1. Text Format (Default - when no `--format json` flag)

```
[op 0] ok: bf-xxx
[op 1] ok: bf-yyy  
[op 2] ok: bf-zzz
```

#### 2. JSON Format (when `--format json` is specified)

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-xxx",
      "error": null,
      "message": "Created bead bf-xxx"
    },
    {
      "op": 1,
      "status": "ok",
      "id": "bf-yyy",
      "error": null,
      "message": "Created bead bf-yyy"
    },
    {
      "op": 2,
      "status": "ok",
      "id": "bf-zzz",
      "error": null,
      "message": "Created bead bf-zzz"
    }
  ]
}
```

## Format Differences Summary

### Map vs Sequence Structure

**Expected in JSON mode:** ✅ Map/Object structure with envelope  
**Actual output:** ✅ Map/Object structure with envelope  
**Status:** MATCH - When `--format json` is used, output is a map/object, not a sequence

### Envelope Structure Presence/Absence

**Expected:** ✅ Envelope with `version`, `kind`, `data` fields  
**Actual:** ✅ Envelope present when `--format json` is used  
**Status:** MATCH - Envelope is correctly applied in JSON mode

**Note:** The test calls `bf batch --stdin` without `--format json`, so it receives TEXT output, not JSON output. The test does not examine batch output format.

### Field Naming Differences

**Envelope fields:** ✅ MATCH
- `version`: Always 1
- `kind`: Always "batch" for batch command  
- `data`: Array of BatchResult objects
- `warning`: Optional string (only present on auto-flush failure)

**BatchResult fields:** ✅ MATCH
- `op`: Number (zero-based operation index)
- `status`: String ("ok" or "error")
- `id`: String or null (bead ID for successful creates)
- `error`: String or null (error message when status is "error")
- `message`: String or null (success message when status is "ok")

### List Command Output (Verified by Test)

The test DOES verify the list command output format (lines 406-419):

```json
{
  "data": [
    {
      "id": "...",
      "title": "...", 
      "priority": 0,
      "labels": ["critical", "batch"],
      ...
    }
  ]
}
```

**Status:** ✅ MATCH - List command correctly wraps bead objects in envelope `data` array

## Data Structure Comparison

### Input vs Output Data Types

| Aspect | Input Format | Output Format | Status |
|--------|--------------|---------------|--------|
| Top-level | Array of operations | Object with envelope | Different (expected) |
| Operation data | Operation spec | Result object | Different (expected) |
| Field naming | "op", "title", "type", etc. | "op", "status", "id", etc. | Different (expected) |

### Envelope Structure Compliance

✅ **COMPLIANT** - When using `--format json`, the batch command correctly implements the envelope structure defined in `src/format/envelope.rs`

## Conclusion

**NO MISMATCH FOUND** - The batch operations output format is correctly implemented:

1. **Text mode (default):** Human-readable format `[op 0] ok: bf-xxx`
2. **JSON mode (`--format json`):** Properly formatted envelope with version, kind, and data array

The test `test_p0_batch_operations_with_labels` does not examine batch output format - it only verifies bead creation via the list command. The list command output format is verified and matches expectations.

## Recommendations

1. **Test Enhancement:** Consider adding a test that directly verifies batch JSON output format when `--format json` is used
2. **Documentation:** The existing documentation in `notes/bf-1dzcxb.md` and `notes/bf-3w72ms.md` accurately describes the output format
3. **No Code Changes Required:** The implementation is correct and matches the envelope specification

## Related Documentation

- `notes/bf-3w72ms.md` - Original envelope specification and format analysis
- `notes/bf-1dzcxb.md` - Detailed batch operations output format documentation
- `src/format/envelope.rs:51-61` - Envelope struct definition
- `src/cli/mod.rs:2689-2746` - Batch command implementation
- `tests/test_p0_multilabel_cli.rs:378-427` - Test expectations