# bead-forge Envelope Format Test Failures

**Date:** 2026-07-23  
**Total Failing Tests:** 39  
**Test Suite:** ~1,472 total tests (~97.4% pass rate)  
**Primary Issue:** JSON envelope format not consistently implemented across commands

---

## Executive Summary

### Summary Statistics

| Category | Failing Tests | Root Cause |
|----------|---------------|------------|
| **Envelope Format (version/kind/data)** | 28 | Missing envelope wrapper structure |
| **Secret Scanning** | 6 | Detection patterns not rejecting tokens |
| **Label Removal Edge Case** | 1 | Non-existent bead handling |
| **Description File Error Handling** | 1 | Empty string vs None handling |
| **Auto-flush JSON Output** | 3 | Missing ID in envelope data |

**Total:** 39 tests failing across 10 test files

### Key Finding

The envelope format specification (`docs/envelope-format-spec.md`) documents that all JSON commands must output:

```json
{
  "version": 1,
  "kind": "<command>",
  "data": <command-specific>,
  "warning": "<optional>"
}
```

**Current Reality:** Most commands are not implementing this envelope wrapper. Instead, they output raw JSON without the `version`, `kind`, and wrapper structure.

---

## Envelope Format Specification (Expected)

### Required Structure

All commands with `--json` or `--format json` MUST emit:

```json
{
  "version": 1,                    // Always present, always 1
  "kind": "<command-name>",         // Command identifier (e.g., "list", "claim")
  "data": <command-specific>,       // Can be object, array, or other JSON value
  "warning": "<optional-message>"   // Present ONLY when non-fatal problem occurs
}
```

### Command-Specific `data` Shapes

| Command Type | Commands | `data` Shape | Example |
|--------------|----------|---------------|---------|
| **List-like** | `list`, `ready`, `search`, `recent`, `velocity` | Array `[{...}]` | `"data": [{"id": "bf-123", ...}, ...]` |
| **Single-object** | `show`, `claim`, `stats` | Object `{...}` | `"data": {"bead_id": "bf-123", ...}` |
| **Mutation** | `create`, `update`, `close`, `reopen`, `delete` | Ack object `{"id": "..."}` | `"data": {"id": "bf-new123"}` |
| **Batch** | `batch` | Array of results `[{op, status, ...}]` | `"data": [{...}, {...}]` |

### Special Cases

**Empty results (list commands):** Return empty array `"data": []`

**No beads available (claim):** Return empty object `"data": {}`

**Auto-flush failure:** Include warning field:
```json
{
  "version": 1,
  "kind": "create",
  "data": {"id": "bf-new789"},
  "warning": "auto-flush failed: write error..."
}
```

---

## Detailed Failure Analysis by Category

### 1. Envelope Format Failures (28 tests)

#### 1.1 Claim Command Envelope Failures (14 tests)

**Test Module:** `tests/envelope_coverage.rs` - `claim_stats`  
**Test Module:** `tests/envelope/claim_stats.rs` (integration tests)

**Expected Output:**
```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-abc123",
    "assignee": "worker-7",
    "title": "...",
    "priority": 2
  }
}
```

**Current Output:**
```json
{
  "bead_id": "bf-abc123",
  "assignee": "worker-7",
  "title": "...",
  "priority": 2
}
```

**Missing Fields:**
- `version` (required)
- `kind` (required, should be "claim")
- `data` wrapper (required - claim result should be nested inside)

**Failing Tests:**
1. `envelope_claim_bead_id_is_valid` - bead_id field not found (looks in wrong location)
2. `envelope_claim_and_stats_consistent_structure` - version field missing
3. `envelope_claim_json_has_metadata_fields` - version field missing
4. `envelope_claim_json_returns_claim_result` - version field missing
5. `envelope_claim_no_beads_returns_empty_object` - version field missing
6. `envelope_claim_command_has_stable_structure` - version field missing
7. `envelope_claim_reflects_assignee` - assignee not found (wrong structure)
8. `claim_envelope_empty_workspace` - version field missing
9. `claim_envelope_data_fields` - claim data not an object (wrong location)
10. `claim_envelope_has_stable_structure` - version field missing
11. `claim_envelope_kind_matches_command` - 'claim' kind missing
12. `claim_envelope_metadata_fields` - version field missing
13. `claim_envelope_structure_consistency` - version field missing
14. `claim_envelope_successful_case` - version field missing
15. `claim_envelope_version_always_one` - version field missing

**What Needs to Change:**
- File: `src/format/envelope.rs` or `src/cli/claim.rs`
- Wrap claim result in envelope: `{"version": 1, "kind": "claim", "data": {<claim_result>}}`

---

#### 1.2 List-like Command Envelope Failures (6 tests)

**Commands Affected:** `ready`, `recent`, `search`, `velocity`

**Expected Output:**
```json
{
  "version": 1,
  "kind": "ready",
  "data": [
    {"id": "bf-123", "title": "...", ...},
    {"id": "bf-456", "title": "...", ...}
  ]
}
```

**Current Output:**
```json
[
  {"id": "bf-123", "title": "...", ...},
  {"id": "bf-456", "title": "...", ...}
]
```

**Missing Fields:**
- `version` (required)
- `kind` (required)
- `data` wrapper (array returned directly instead of wrapped)

**Failing Tests:**
1. `envelope_ready_command_has_stable_structure` - ready data not an array (structure mismatch)
2. `envelope_recent_command_has_stable_structure` - recent data not an array
3. `envelope_recent_empty_emits_empty_array` - recent data not an array
4. `envelope_search_empty_emits_empty_array` - Invalid JSON/EOF (empty case broken)
5. `envelope_search_command_has_stable_structure` - version field missing
6. `envelope_velocity_command_has_stable_structure` - Envelope not an object (raw output)
7. `envelope_velocity_empty_emits_empty_array` - Envelope not an object
8. `envelope_data_field_always_present` - Missing data field for velocity command
9. `envelope_kind_matches_command` - Missing kind for velocity command
10. `envelope_version_is_always_one` - Missing version for velocity command

**What Needs to Change:**
- Files: `src/format/json.rs`, `src/cli/ready.rs`, `src/cli/recent.rs`, `src/cli/search.rs`, `src/cli/velocity.rs`
- Wrap array results: `{"version": 1, "kind": "<command>", "data": [...]}`
- Handle empty case: `"data": []` instead of empty output

---

#### 1.3 Batch Command Envelope Failures (2 tests)

**Expected Output:**
```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {"op": 0, "status": "ok", "id": "bf-new456", "message": "..."},
    {"op": 1, "status": "ok", "message": "..."}
  ]
}
```

**Current Output:**
```json
[
  {"op": 0, "status": "ok", "id": "bf-new456", "message": "..."},
  {"op": 1, "status": "ok", "message": "..."}
]
```

**Missing Fields:**
- `version` (required)
- `kind` (required)
- `data` wrapper

**Failing Tests:**
1. `envelope_batch_command_has_stable_structure` - batch data not an array
2. `envelope_batch_empty_emits_empty_array` - batch data not an array

**What Needs to Change:**
- File: `src/cli/batch.rs` or `src/format/json.rs`
- Wrap batch results: `{"version": 1, "kind": "batch", "data": [...]}`

---

#### 1.4 Auto-flush JSON Output Failures (3 tests)

**These tests are actually envelope format issues in disguise.**

**Expected Output (create with flush failure):**
```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new123"
  },
  "warning": "auto-flush failed: write error..."
}
```

**Current Output:**
```json
{
  "id": "bf-new123"
}
```

**Missing Fields:**
- `version` (required)
- `kind` (required, should be "create")
- `data` wrapper (id should be inside data object)
- `warning` (missing on flush failure)

**Failing Tests:**
1. `create_json_succeeds_warns_retains_dirty_and_recovers` - ID not in envelope structure
2. `flush_failure_nonfatal_json_warning_and_dirty_retained` - create --json missing id envelope
3. `flush_failure_does_not_fail_mutation_and_warns_json` - ID not in envelope structure
4. `flush_failure_surfaces_warning_in_json_output` - ID not in envelope structure
5. `flush_failure_carries_json_warning` - ID not in envelope structure

**What Needs to Change:**
- Files: `src/cli/create.rs`, `src/sync.rs` (auto-flush warning)
- Wrap create result: `{"version": 1, "kind": "create", "data": {"id": "..."}}`
- Add warning field when flush fails after successful mutation

---

### 2. Secret Scanning Failures (6 tests)

**Test File:** `tests/secret_scanning.rs`

**Expected Behavior:**
- `bf create` should reject beads containing secret patterns in any text field
- Should return error before writing to database

**Current Behavior:**
- Beads with secret patterns are accepted
- No error thrown for Azure keys, GitHub tokens, or PATs

**Failing Tests:**
1. `integration_refuses_azure_key` - Azure key not rejected
2. `integration_refuses_github_gho_token` - GitHub gho_ token not rejected  
3. `integration_refuses_github_ghr_token` - GitHub ghr_ token not rejected
4. `integration_refuses_github_ghs_token` - GitHub ghs_ token not rejected
5. `integration_refuses_github_ghu_token` - GitHub ghu_ token not rejected
6. `integration_refuses_github_pat_token` - GitHub PAT token not rejected

**What Needs to Change:**
- File: `src/secrets.rs` or integration in `src/storage/sqlite.rs`
- Verify detection patterns match test cases
- Ensure `create_issue()` checks secrets before INSERT
- Integration test: verify rejection is wired end-to-end

**Test Patterns (likely not matching):**
- Azure keys: `AKIA[0-9A-Z]{16}` or similar
- GitHub tokens: `gho_`, `ghr_`, `ghs_`, `ghu_` prefixes with hex strings
- GitHub PATs: GitHub personal access token pattern

---

### 3. Label Removal Edge Case (1 test)

**Test File:** `tests/label_removal_test.rs`

**Expected Behavior:**
- Removing a label from a non-existent bead should succeed gracefully (no-op)
- Should return `Ok(())` without error

**Current Behavior:**
- Returns error, causing test panic on `assert!(result.is_ok())`

**Failing Test:**
1. `test_remove_label_from_nonexistent_issue_fails_gracefully`

**What Needs to Change:**
- File: `src/storage/sqlite.rs` or `src/cli/label.rs`
- Change error handling: treat non-existent bead as no-op instead of error
- OR: change test expectation if error is correct behavior

---

### 4. Description File Error Handling (1 test)

**Test File:** `tests/update_flags.rs`

**Expected Behavior:**
- When `--description-file` points to missing file, description should remain unset (None)
- Should not set description to empty string ""

**Current Behavior:**
- Sets description to empty String("") instead of leaving as None

**Failing Test:**
1. `test_cli_update_description_file_missing_file_errors`

**What Needs to Change:**
- File: `src/cli/update.rs` or file reading logic
- On file read error: keep description as None instead of setting to ""
- Verify error path doesn't overwrite with default value

---

## Test Files Summary

### Files with Failures (10 files)

| File | Failed | Primary Issue |
|------|--------|----------------|
| `tests/autoflush_failure_contract.rs` | 1 | Envelope format |
| `tests/autoflush_mutation.rs` | 1 | Envelope format |
| `tests/autoflush_wiring.rs` | 1 | Envelope format |
| `tests/envelope_coverage.rs` | 20 | Envelope format |
| `tests/envelope_integration_tests.rs` | 8 | Envelope format |
| `tests/kill_worker_preserves_beads.rs` | 1 | Envelope format |
| `tests/label_removal_test.rs` | 1 | Edge case |
| `tests/recovery_and_exit_criteria.rs` | 1 | Envelope format |
| `tests/secret_scanning.rs` | 6 | Secret patterns |
| `tests/update_flags.rs` | 1 | Error handling |

### Files with All Tests Passing (90 files)

All other test files pass completely, including:
- Core library unit tests (272/272 pass)
- Batch operations
- Epic management
- Schema compatibility
- Claim race tests
- JSONL compatibility
- And 80+ more

---

## Implementation Priority

### P0 (Critical) - Envelope Format (28 tests)

**Impact:** Blocking machine-readable JSON output for NEEDLE integration

**Files to Modify:**
1. `src/format/envelope.rs` - Ensure envelope wrapper is applied
2. `src/format/json.rs` - Integrate envelope into JSON formatter
3. `src/cli/claim.rs` - Wrap claim output in envelope
4. `src/cli/ready.rs` - Wrap ready list in envelope
5. `src/cli/recent.rs` - Wrap recent list in envelope
6. `src/cli/search.rs` - Wrap search results in envelope
7. `src/cli/velocity.rs` - Wrap velocity output in envelope
8. `src/cli/batch.rs` - Wrap batch results in envelope
9. `src/cli/create.rs` - Wrap create result in envelope, add warning support
10. `src/sync.rs` - Pass warnings through to envelope

**Implementation Pattern:**
```rust
// Instead of:
let json = serde_json::to_string(&result)?;

// Use:
let envelope = JsonEnvelope {
    version: 1,
    kind: "command_name".to_string(),
    data: serde_json::to_value(&result)?,
    warning: warning_message,
};
let json = serde_json::to_string(&envelope)?;
```

### P1 (High) - Secret Scanning (6 tests)

**Impact:** Security - credentials could be committed to git

**Files to Modify:**
1. `src/secrets.rs` - Verify regex patterns match test cases
2. `src/storage/sqlite.rs::create_issue()` - Reject secrets before INSERT

**Implementation Pattern:**
```rust
// Before INSERT:
validate_no_secrets(&issue)?;
// Returns Error if patterns match
```

### P2 (Medium) - Edge Cases (2 tests)

1. **Label removal** - Graceful no-op for non-existent beads
2. **Description file** - Use None instead of "" on error

---

## Verification Checklist

After implementing fixes, verify:

- [ ] All 28 envelope format tests pass
- [ ] All 6 secret scanning tests pass
- [ ] Label removal edge case test passes
- [ ] Description file error handling test passes
- [ ] Manual verification: `bf claim --json` outputs envelope structure
- [ ] Manual verification: `bf list --json` outputs envelope with array data
- [ ] Manual verification: `bf create --json` outputs envelope with data.id
- [ ] Manual verification: Auto-flush failure includes warning field

---

## Related Documentation

- **Envelope Format Spec:** `docs/envelope-format-spec.md`
- **Implementation Plan:** `docs/plan/plan.md`
- **Test Audit:** `.beads/traces/bf-1mv3p/test_audit.md`

---

## Summary

The envelope format is specified but not implemented. 28 of 39 failing tests are due to missing envelope wrapper structure. The fix is straightforward: wrap all JSON command outputs in `{version: 1, kind: "<command>", data: <result>, warning: <optional>}`.

The remaining failures are:
- 6 secret scanning tests (security feature not working)
- 2 edge case tests (graceful failure handling)

**Root Cause:** Envelope wrapper not consistently applied across JSON output paths.

**Solution:** Apply envelope wrapper in JSON formatter for all commands.
