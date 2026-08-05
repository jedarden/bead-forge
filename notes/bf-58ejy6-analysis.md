# Batch Operations Format Fix Specification

**Bead ID:** bf-2fwnk3  
**Date:** 2026-08-05  
**Status:** COMPLETE  
**Combined from:** bf-310kxd, bf-3w72ms, bf-50bh66, bf-3xofss

---

## Executive Summary

The batch operations output format is **correctly implemented** according to the envelope specification. The perceived "mismatch" stems from a **fundamental design divergence** between what test expectations assume (output should contain bead data for verification) versus what the implementation provides (operation metadata reporting success/failure).

**Root Cause:** Different mental models
- **Test model:** "Run batch → See created beads" (query-oriented)
- **Implementation model:** "Run batch → Get operation report" (action-oriented)

**Resolution Required:** Choose between three options (see Section 8)

---

## 1. Current Implementation (ACTUAL FORMAT)

### 1.1 Code Locations

**Primary Data Structure**
- **File:** `src/batch.rs:113-122`
- **Struct:** `BatchResult`
```rust
pub struct BatchResult {
    pub op: usize,              // Operation index in batch
    pub status: String,         // "ok" or "error"
    pub id: Option<String>,     // Created bead ID (for create ops)
    pub error: Option<String>,  // Error message (for failed ops)
    pub message: Option<String>, // Human-readable result message
}
```

**Core Execution Function**
- **File:** `src/batch.rs:191-435`
- **Function:** `execute_batch(storage, ops, workspace_dir, no_auto_flush) -> Result<Vec<BatchResult>>`
- **Characteristics:**
  - Executes all operations atomically under `with_immediate_transaction`
  - Returns `Vec<BatchResult>` - one result per operation
  - Fail-fast on first error (transaction rollback)

**CLI Output Formatting**
- **File:** `src/cli/mod.rs:2713-2794`
- **Function:** `cmd_batch()`

**Text Format (lines 2726-2742)**
```
[op 0] ok: bf-xxx
[op 1] ok: bf-yyy
[op 2] error: Bead not found
```

**JSON Format (lines 2719-2723)**
```rust
crate::format::OutputFormat::Json => {
    let formatter = get_formatter(output_format);
    let json_array = serde_json::to_string(&results).unwrap_or_default();
    println!("{}", formatter.format_with_envelope("batch", &json_array));
}
```

### 1.2 Actual JSON Output Format

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-abc123",
      "error": null,
      "message": "Created bead bf-abc123"
    },
    {
      "op": 1,
      "status": "ok",
      "id": "bf-def456",
      "error": null,
      "message": "Created bead bf-def456"
    }
  ],
  "warning": null
}
```

**Purpose:** Report whether each operation succeeded/failed  
**Domain:** Operation result metadata  
**Fields:** `op`, `status`, `id`, `error`, `message`  
**Envelope:** `version`, `kind`, `data`, `warning`

---

## 2. Test Expectations (EXPECTED FORMAT)

### 2.1 Test Behavior Analysis

**Test File:** `tests/test_p0_multilabel_cli.rs:370-419`

**What the test does:**
1. Creates batch operations (lines 382-386):
```json
[
  {"op": "create", "title": "P0 batch test 1", "type": "task", "priority": 0, "labels": ["critical", "batch"]},
  {"op": "create", "title": "P0 batch test 2", "type": "bug", "priority": 0, "labels": ["urgent", "batch"]},
  {"op": "create", "title": "P0 batch test 3", "type": "feature", "priority": 0, "labels": ["critical", "hotfix"]}
]
```

2. Calls `bf batch --stdin` WITHOUT `--format json` flag (line 389)
3. Only verifies command succeeds (`output.status.success()`)
4. Verifies bead creation via SEPARATE `bf list` command (lines 406-419)

**Critical Finding:** The test does NOT examine batch command output format at all. It validates bead data from the `list` command, not from `batch`.

### 2.2 What Test Would Expect (If It Checked Batch Output)

The test expects batch output to contain **bead data** for verification:

```json
{
  "data": [
    {
      "priority": 0,
      "labels": ["critical", "batch"],
      "title": "P0 batch test 1",
      "type": "task",
      // ... other bead fields
    },
    {
      "priority": 0,
      "labels": ["urgent", "batch"],
      "title": "P0 batch test 2",
      "type": "bug",
      // ... other bead fields
    }
  ]
}
```

**Purpose:** Verify the beads that were created  
**Domain:** Bead data records  
**Fields:** Bead properties (`priority`, `labels`, `title`, `type`, `description`, `status`, etc.)

---

## 3. Structural Differences

### 3.1 Envelope Structure

| Aspect | Expected | Actual | Impact |
|--------|----------|--------|--------|
| **Structure** | Object with envelope | Object with envelope | ✅ Match |
| **Fields** | `{data}` only | `{version, kind, data, warning}` | ⚠️ Superset |
| **Versioning** | None | `version: 1` | ⚠️ Extra field |
| **Typing** | None | `kind: "batch"` | ⚠️ Extra field |
| **Warnings** | None | `warning: null` | ⚠️ Extra field |

### 3.2 Array Element Structure (Critical Mismatch)

| Aspect | Expected (Test) | Actual (Implementation) | Compatibility |
|--------|-----------------|------------------------|--------------|
| **Element type** | Bead object | BatchResult object | ❌ Incompatible |
| **Purpose** | Verify created beads | Report operation success | ❌ Different goals |
| **Fields** | Bead properties | Operation metadata | ❌ Disjoint sets |
| **Source** | Bead store/database | Execution results | ❌ Different sources |

### 3.3 Field-by-Field Comparison

**Expected Output Fields (Bead Objects):**
| Field | Type | Domain | Present in Actual? |
|-------|------|--------|-------------------|
| `priority` | number | Bead data | ❌ No |
| `labels` | array of strings | Bead data | ❌ No |
| `title` | string | Bead data | ❌ No |
| `type` | string | Bead data | ❌ No |
| `description` | string | Bead data | ❌ No |
| `status` | string | Bead workflow state | ⚠️ Name collision |
| `id` | string | Bead identifier | ⚠️ Name collision |

**Actual Output Fields (BatchResult Objects):**
| Field | Type | Domain | Present in Expected? |
|-------|------|--------|---------------------|
| `op` | number | Operation index | ❌ No |
| `status` | string | Operation result | ⚠️ Name collision |
| `id` | string or null | Created bead ID | ⚠️ Name collision |
| `error` | string or null | Error message | ❌ No |
| `message` | string or null | Success message | ❌ No |

**Field Name Collisions:**
- `status`: Bead workflow state ("in_progress") vs operation result ("ok")
- `id`: Bead identifier (always present) vs created ID (conditional)

---

## 4. Field Naming and Conventions

### 4.1 Naming Convention Analysis

✅ **No naming convention conflict**

Both formats consistently use:
- **snake_case** for field names
- **kebab-case** for ID values (e.g., `bf-abc123`)

**Expected format field naming:**
- `priority`, `labels`, `title`, `type`, `description`, `status`, `id` → all snake_case

**Actual format field naming:**
- `op`, `status`, `id`, `error`, `message`, `version`, `kind`, `data`, `warning` → all snake_case

### 4.2 Field Set Disjointness

❌ **Zero field overlap (except ambiguous name collisions)**

Expected bead fields are **completely disjoint** from actual operation result fields:
- Bead domain: data properties of a bead record
- Result domain: execution metadata of an operation

The only shared field names (`status`, `id`) have different meanings in each domain.

---

## 5. Type and Cardinality Differences

### 5.1 Type Compatibility

✅ **JSON-level types are compatible**

Both formats use primitive JSON types (numbers, strings, arrays, nulls). The incompatibility is **semantic**, not syntactic.

### 5.2 Cardinality Differences

**Expected Format:**
- One bead per array element
- Bead ID always present
- `status` has multiple enum values (workflow states)
- Fields consistently present (non-nullable)

**Actual Format:**
- One operation result per array element
- Bead ID conditionally present (create ops only)
- `status` has two enum values (`ok`, `error`)
- Fields conditionally present based on operation type

### 5.3 Nullability Patterns

**Expected Format:**
- Most fields non-nullable
- `labels` may be empty array but typically present
- No conditional nullability

**Actual Format:**
- `id` nullable (present for create, null otherwise)
- `error` nullable (present on error, null on success)
- `message` nullable (present on success, null on error)
- `warning` nullable (present only on auto-flush failure)

---

## 6. Semantic Domain Analysis

### 6.1 Semantic Domains

| Aspect | Expected Domain | Actual Domain | Compatible? |
|--------|----------------|---------------|------------|
| **Array element** | Bead (data record) | BatchResult (operation result) | ❌ |
| **priority** | Bead property | N/A | ❌ |
| **labels** | Bead property | N/A | ❌ |
| **op** | N/A | Operation index | ❌ |
| **error** | N/A | Error message | ❌ |
| **message** | N/A | Success message | ❌ |
| **status** (bead) | Workflow state | N/A | ❌ |
| **status** (result) | N/A | Operation result | ❌ |
| **id** (bead) | Bead identifier | N/A | ⚠️ Partial |
| **id** (result) | N/A | Created bead ID | ⚠️ Partial |
| **version** | N/A | Envelope version | ❌ |
| **kind** | N/A | Command type | ❌ |
| **warning** | N/A | Auto-flush warning | ❌ |

**Conclusion:** Zero semantic overlap except for ambiguous field name collisions.

### 6.2 Mental Model Divergence

**Test Mental Model:**
> *"I run a batch command to create beads, and I want to see the beads that were created."*
- **Orientation:** Query-oriented
- **Parallel to:** `SELECT * FROM beads WHERE ...`
- **Output purpose:** Verify created bead data

**Implementation Mental Model:**
> *"I run a batch command, and I want to know if each operation succeeded."*
- **Orientation:** Action-oriented
- **Parallel to:** CLI exit codes or HTTP response status
- **Output purpose:** Report operation success/failure

---

## 7. Complete Side-by-Side Comparison

### 7.1 Input Format

```json
[
  {"op": "create", "title": "P0 batch test 1", "type": "task", "priority": 0, "labels": ["critical", "batch"]},
  {"op": "create", "title": "P0 batch test 2", "type": "bug", "priority": 0, "labels": ["urgent", "batch"]},
  {"op": "create", "title": "P0 batch test 3", "type": "feature", "priority": 0, "labels": ["critical", "hotfix"]}
]
```

### 7.2 Expected Output (What Test Would Want)

```json
{
  "data": [
    {
      "priority": 0,
      "labels": ["critical", "batch"],
      "title": "P0 batch test 1",
      "type": "task",
      "status": "open",
      "id": "bf-abc123"
    },
    {
      "priority": 0,
      "labels": ["urgent", "batch"],
      "title": "P0 batch test 2",
      "type": "bug",
      "status": "open",
      "id": "bf-def456"
    },
    {
      "priority": 0,
      "labels": ["critical", "hotfix"],
      "title": "P0 batch test 3",
      "type": "feature",
      "status": "open",
      "id": "bf-ghi789"
    }
  ]
}
```

### 7.3 Actual Output (What Implementation Provides)

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-abc123",
      "error": null,
      "message": "Created bead bf-abc123"
    },
    {
      "op": 1,
      "status": "ok",
      "id": "bf-def456",
      "error": null,
      "message": "Created bead bf-def456"
    },
    {
      "op": 2,
      "status": "ok",
      "id": "bf-ghi789",
      "error": null,
      "message": "Created bead bf-ghi789"
    }
  ],
  "warning": null
}
```

### 7.4 Test Behavior (What Actually Happens)

The test does NOT examine batch output. It validates bead creation via the `list` command:

```rust
// Test calls `bf list` to verify beads
let list_output = Command::new("bf")
    .args(["list", "--format", "json"])
    .output()
    .expect("Failed to run bf list");

// List command returns bead data in expected format
let list_json: Value = serde_json::from_slice(&list_output.stdout).unwrap();
let beads = list_json["data"].as_array().unwrap();

// Test verifies bead properties from list output
assert_eq!(beads[0]["priority"], 0);
assert_eq!(beads[0]["labels"], ["critical", "batch"]);
```

---

## 8. Resolution Options

This is a **design choice**, not a bug. Three options exist:

### Option A: Change the Test ✅ RECOMMENDED

**Action:** Rewrite test to validate operation results instead of bead data from batch output

**Implementation:**
```rust
// After batch command, validate operation results
let batch_json: Value = serde_json::from_slice(&batch_output.stdout).unwrap();
let results = batch_json["data"].as_array().unwrap();

assert_eq!(results[0]["status"], "ok");
assert_eq!(results[0]["op"], 0);
assert!(results[0]["id"].as_str().unwrap().starts_with("bf-"));

// Use separate `bf list` command for bead data verification
```

**Pros:**
- ✅ Maintains consistency with other batch-style commands
- ✅ Follows standard CLI patterns (operation reporting)
- ✅ No changes to implementation required
- ✅ Preserves envelope structure with versioning

**Cons:**
- ⚠️ Test becomes slightly more complex
- ⚠️ Requires two commands for full verification (batch + list)

### Option B: Change the Implementation

**Action:** Return bead data in `data` array, move operation results to different field

**Implementation:**
```rust
// New envelope structure
{
  "version": 1,
  "kind": "batch",
  "beads": [...],        // Created bead data
  "results": [...],       // Operation results
  "warning": null
}
```

**Pros:**
- ✅ Test expectations met without changes
- ✅ Provides both bead data and operation results

**Cons:**
- ❌ Breaks consistency with other batch commands
- ❌ Requires significant implementation changes
- ❌ Doubles output size (beads + results)
- ❌ Complex to handle non-create operations (dep_add, close, etc.)

### Option C: Hybrid Approach

**Action:** Add optional flag to return bead data instead of operation results

**Implementation:**
```bash
bf batch --stdin --format json --output-beads
```

**Output:**
```json
{
  "version": 1,
  "kind": "batch-beads",  // Different kind
  "data": [...],           // Bead data
  "results": [...],        // Optional operation results
  "warning": null
}
```

**Pros:**
- ✅ Maintains backward compatibility
- ✅ Supports both use cases
- ✅ Flexible for different scenarios

**Cons:**
- ⚠️ Adds CLI complexity
- ⚠️ Requires maintaining two output formats
- ⚠️ More code to test and document

---

## 9. Specification for Implementation Bead

### 9.1 Recommended Approach: Option A (Change Test)

**Rationale:** The implementation is correct and follows standard CLI patterns. The test should validate what the command actually provides.

**Required Changes:**
1. **Test File:** `tests/test_p0_multilabel_cli.rs`
2. **Function:** `test_p0_batch_operations_with_labels` (lines 370-419)

**Implementation Steps:**

1. Add JSON format flag to batch command:
```rust
let batch_output = Command::new("bf")
    .args(["batch", "--stdin", "--format", "json"])
    .stdin(Stdio::piped())
    // ...
```

2. Parse and validate batch output:
```rust
let batch_json: Value = serde_json::from_slice(&batch_output.stdout).unwrap();

// Validate envelope structure
assert_eq!(batch_json["version"], 1);
assert_eq!(batch_json["kind"], "batch");

// Validate operation results
let results = batch_json["data"].as_array().unwrap();
assert_eq!(results.len(), 3);

for (i, result) in results.iter().enumerate() {
    assert_eq!(result["op"], i);
    assert_eq!(result["status"], "ok");
    assert!(result["id"].as_str().unwrap().starts_with("bf-"));
    assert_eq!(result["error"], serde_json::Value::Null);
}
```

3. Keep existing `bf list` validation for bead properties:
```rust
// Existing validation is correct - no changes needed
let list_output = Command::new("bf")
    .args(["list", "--format", "json"])
    .output()
    .expect("Failed to run bf list");
```

### 9.2 Code Locations for Test Changes

**File:** `tests/test_p0_multilabel_cli.rs:370-419`

**Current Code (lines 389-403):**
```rust
let output = Command::new("bf")
    .args(["batch", "--stdin"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap()
    .stdin
    .unwrap()
    .write_all(batch_input.as_bytes())
    .unwrap();
```

**Modified Code:**
```rust
let output = Command::new("bf")
    .args(["batch", "--stdin", "--format", "json"])  // Add --format json
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap()
    .stdin
    .unwrap()
    .write_all(batch_input.as_bytes())
    .unwrap();

// Add batch output validation
let batch_json: Value = serde_json::from_slice(&output.stdout).unwrap();
// ... validation code as shown above
```

### 9.3 No Changes Required to Implementation

**Files to NOT modify:**
- `src/batch.rs` - BatchResult struct is correct
- `src/cli/mod.rs:cmd_batch()` - Output formatting is correct
- `src/format/` - Envelope implementation is correct

**Rationale:** The implementation correctly follows the envelope specification and provides operation result reporting, which is the standard pattern for batch operations in CLI tools.

---

## 10. Verification Plan

### 10.1 Test Validation

After implementing Option A, verify:

1. **Batch command output format:**
   - Envelope structure is correct (`version`, `kind`, `data`, `warning`)
   - Operation results contain correct fields (`op`, `status`, `id`, `error`, `message`)
   - Field types match specification
   - Nullability rules are followed

2. **List command output format:**
   - Existing validation continues to work
   - Bead properties are correctly verified
   - Labels are correctly parsed

3. **Integration:**
   - Batch creates beads successfully
   - List retrieves created beads
   - End-to-end flow works

### 10.2 Backward Compatibility

- Text output format unchanged: `[op 0] ok: bf-xxx`
- JSON output format unchanged (envelope with operation results)
- Existing scripts using batch output continue to work

---

## 11. Summary of Findings

### 11.1 What Works Correctly

✅ **Batch operations implementation:**
- Core execution logic (`execute_batch()`)
- Atomic transaction handling
- Operation result tracking
- Error reporting

✅ **Output formatting:**
- Text format is clear and human-readable
- JSON format follows envelope specification
- Field naming conventions are consistent (snake_case)

✅ **Test infrastructure:**
- List command validation is correct
- Bead creation verification works
- Test isolation and cleanup

### 11.2 What Needs Clarification

⚠️ **Test expectations vs implementation:**
- Test doesn't validate batch output format (currently passes without checking)
- If test were to validate batch output, it would expect bead data
- Implementation provides operation results, not bead data

⚠️ **Design decision needed:**
- Should batch output bead data or operation results?
- Current answer: operation results (standard CLI pattern)
- Test should validate operation results, not bead data, from batch output

### 11.3 What Does NOT Need Changing

❌ **Implementation is correct:**
- BatchResult struct is appropriate for operation reporting
- Envelope structure matches specification
- Field naming is consistent
- Output format follows CLI conventions

❌ **No bugs found:**
- Implementation correctly implements the envelope spec
- Text and JSON formats work as intended
- Operation results are accurate and complete

---

## 12. Related Documentation

**Child Bead Documentation:**
- `notes/bf-310kxd.md` - Code location analysis
- `notes/bf-3w72ms-findings.md` - Format mismatch documentation  
- `notes/bf-50bh66.md` - Structural differences analysis
- `notes/bf-3xofss.md` - Field naming and discrepancies

**Implementation Files:**
- `src/batch.rs:113-122` - BatchResult struct
- `src/batch.rs:191-435` - execute_batch function
- `src/cli/mod.rs:2713-2794` - cmd_batch function
- `src/format/envelope.rs:51-61` - Envelope struct

**Test Files:**
- `tests/test_p0_multilabel_cli.rs:370-419` - Batch operations test

**Project Documentation:**
- `docs/plan/plan.md` - Implementation plan
- `docs/README.md` - Command reference

---

## 13. Implementation Bead Requirements

**For the implementation bead that follows this specification:**

**Bead Title:** Fix batch operations test to validate operation results

**Required Changes:**
1. Modify `tests/test_p0_multilabel_cli.rs:370-419`
2. Add `--format json` to batch command invocation
3. Add validation of batch JSON output (envelope + operation results)
4. Keep existing list command validation unchanged

**Acceptance Criteria:**
- Test validates envelope structure (`version`, `kind`, `data`, `warning`)
- Test validates operation result fields (`op`, `status`, `id`, `error`, `message`)
- Test continues to validate bead properties via list command
- Test passes with correct implementation
- No changes to implementation code required

**Verification:**
```bash
cargo test test_p0_batch_operations_with_labels
```

---

**Specification Complete**

This specification combines all findings from child beads (bf-310kxd, bf-3w72ms, bf-50bh66, bf-3xofss) into a comprehensive analysis. The recommended resolution is **Option A: Change the test** to validate operation results rather than bead data, as the implementation is correct and follows standard CLI patterns for batch operations.