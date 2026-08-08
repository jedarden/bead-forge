# Envelope Format Test Failure Report

**Date:** 2026-08-07  
**Total Failing Tests:** 28  
**Test Suite:** ~1,472 total tests (~98.1% pass rate)  
**Primary Issue:** JSON envelope format not consistently implemented across commands

---

## Executive Summary

All 28 failing tests share the same root cause: commands are emitting raw JSON without the required envelope wrapper structure specified in `docs/envelope_format_spec.md`.

### Expected Envelope Structure

All commands with `--json` or `--format json` MUST emit:

```json
{
  "version": 1,
  "kind": "<command-name>",
  "data": <command-specific-data>,
  "warning": "<optional-warning-message>"
}
```

### Current Reality

Most commands emit raw JSON without the `version`, `kind`, and `data` wrapper.

---

## Failure Categories by Response Construction Code Path

### Category 1: Claim Command (14 tests)
**Code Path:** `src/cli/claim.rs` → `src/format/json.rs`  
**Total Failures:** 14

#### Expected Envelope for Claim

```json
{
  "version": 1,
  "kind": "claim",
  "data": {
    "bead_id": "bf-abc123",
    "assignee": "worker-7",
    "reclaimed": 0,
    "title": "Implement auth flow",
    "priority": 2,
    "downstream_impact": 5
  }
}
```

#### Actual Output (No Envelope)

```json
{
  "bead_id": "bf-abc123",
  "assignee": "worker-7",
  "reclaimed": 0,
  "title": "Implement auth flow",
  "priority": 2,
  "downstream_impact": 5
}
```

#### Mismatch Fields

- **Missing:** `version` (required)
- **Missing:** `kind` (required, should be "claim")
- **Missing:** `data` wrapper (claim result should be nested inside)
- **Incorrect:** Top-level fields are at wrong nesting level

#### Failing Tests

1. `envelope_claim_bead_id_is_valid` - Looking for bead_id in wrong location
2. `envelope_claim_and_stats_consistent_structure` - version field missing
3. `envelope_claim_json_has_metadata_fields` - version field missing
4. `envelope_claim_json_returns_claim_result` - version field missing
5. `envelope_claim_no_beads_returns_empty_object` - version field missing
6. `envelope_claim_command_has_stable_structure` - version field missing
7. `envelope_claim_reflects_assignee` - assignee field not found at expected path
8. `claim_envelope_empty_workspace` - version field missing
9. `claim_envelope_data_fields` - data field not an object (wrong structure)
10. `claim_envelope_has_stable_structure` - version field missing
11. `claim_envelope_kind_matches_command` - kind field missing
12. `claim_envelope_metadata_fields` - version field missing
13. `claim_envelope_structure_consistency` - version field missing
14. `claim_envelope_successful_case` - version field missing
15. `claim_envelope_version_always_one` - version field missing

#### Required Fix Location

**File:** `src/cli/claim.rs` or `src/format/json.rs`

**Current Code Pattern (incorrect):**
```rust
// Emits raw JSON
let output = ClaimResult {
    bead_id: "...",
    assignee: "...",
    // ...
};
serde_json::to_string(&output)?
```

**Required Code Pattern:**
```rust
let claim_result = ClaimResult { /* ... */ };
let envelope = JsonEnvelope {
    version: 1,
    kind: "claim".to_string(),
    data: serde_json::to_value(&claim_result)?,
    warning: None, // or Some(message) if warning exists
};
serde_json::to_string(&envelope)?
```

---

### Category 2: List-like Commands (6 tests)
**Code Path:** `src/cli/ready.rs`, `src/cli/recent.rs`, `src/cli/search.rs`, `src/cli/velocity.rs` → `src/format/json.rs`  
**Total Failures:** 6

#### Commands Affected

- `bf ready --format json`
- `bf recent --format json`
- `bf search "query" --format json`
- `bf velocity --format json`

#### Expected Envelope for List Commands

```json
{
  "version": 1,
  "kind": "ready",
  "data": [
    {
      "id": "bf-abc123",
      "title": "Unblocked task",
      "status": "open",
      "priority": 1,
      "issue_type": "task",
      "assignee": null,
      "labels": ["urgent"],
      "created_at": "2026-07-22T14:00:00Z",
      "updated_at": "2026-07-22T14:00:00Z"
    },
    {
      "id": "bf-def456",
      "title": "Another task",
      "status": "open",
      "priority": 2,
      "issue_type": "task",
      "assignee": null,
      "labels": [],
      "created_at": "2026-07-22T14:00:00Z",
      "updated_at": "2026-07-22T14:00:00Z"
    }
  ]
}
```

#### Expected Envelope for Empty List

```json
{
  "version": 1,
  "kind": "ready",
  "data": []
}
```

#### Actual Output (No Envelope)

```json
[
  {
    "id": "bf-abc123",
    "title": "Unblocked task",
    "status": "open",
    "priority": 1,
    "issue_type": "task",
    "assignee": null,
    "labels": ["urgent"],
    "created_at": "2026-07-22T14:00:00Z",
    "updated_at": "2026-07-22T14:00:00Z"
  }
]
```

#### Mismatch Fields

- **Missing:** `version` (required)
- **Missing:** `kind` (required)
- **Missing:** `data` wrapper (array returned directly instead of wrapped)
- **Incorrect:** Output is array, not object

#### Failing Tests

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

#### Required Fix Location

**Files:** 
- `src/cli/ready.rs`
- `src/cli/recent.rs`
- `src/cli/search.rs`
- `src/cli/velocity.rs`
- `src/format/json.rs`

**Current Code Pattern (incorrect):**
```rust
// Emits raw JSON array
let beads = fetch_beads()?;
serde_json::to_string(&beads)?
```

**Required Code Pattern:**
```rust
let beads = fetch_beads()?;
let envelope = JsonEnvelope {
    version: 1,
    kind: "ready".to_string(), // or "recent", "search", "velocity"
    data: serde_json::to_value(&beads)?,
    warning: None,
};
serde_json::to_string(&envelope)?
```

---

### Category 3: Batch Command (2 tests)
**Code Path:** `src/cli/batch.rs` → `src/format/json.rs`  
**Total Failures:** 2

#### Expected Envelope for Batch

```json
{
  "version": 1,
  "kind": "batch",
  "data": [
    {
      "op": 0,
      "status": "ok",
      "id": "bf-new456",
      "message": "Created bead bf-new456"
    },
    {
      "op": 1,
      "status": "ok",
      "id": "bf-new789",
      "message": "Created bead bf-new789"
    },
    {
      "op": 2,
      "status": "ok",
      "message": "ok: bf-parent blocked by bf-new456"
    },
    {
      "op": 3,
      "status": "ok",
      "message": "Closed bead bf-parent"
    }
  ]
}
```

#### Expected Envelope for Empty Batch

```json
{
  "version": 1,
  "kind": "batch",
  "data": []
}
```

#### Actual Output (No Envelope)

```json
[
  {
    "op": 0,
    "status": "ok",
    "id": "bf-new456",
    "message": "Created bead bf-new456"
  },
  {
    "op": 1,
    "status": "ok",
    "id": "bf-new789",
    "message": "Created bead bf-new789"
  }
]
```

#### Mismatch Fields

- **Missing:** `version` (required)
- **Missing:** `kind` (required, should be "batch")
- **Missing:** `data` wrapper (array returned directly)
- **Incorrect:** Output is array, not object

#### Failing Tests

1. `envelope_batch_command_has_stable_structure` - batch data not an array (looking in wrong place)
2. `envelope_batch_empty_emits_empty_array` - batch data not an array

#### Required Fix Location

**File:** `src/cli/batch.rs` or `src/format/json.rs`

**Current Code Pattern (incorrect):**
```rust
// Emits raw JSON array of operation results
let results = execute_batch()?;
serde_json::to_string(&results)?
```

**Required Code Pattern:**
```rust
let results = execute_batch()?;
let envelope = JsonEnvelope {
    version: 1,
    kind: "batch".to_string(),
    data: serde_json::to_value(&results)?,
    warning: None,
};
serde_json::to_string(&envelope)?
```

---

### Category 4: Auto-flush JSON Output (3 tests)
**Code Path:** `src/cli/create.rs`, `src/sync.rs` → `src/format/json.rs`  
**Total Failures:** 3

These are envelope format issues - the tests verify that when auto-flush fails, the warning field is included.

#### Expected Envelope for Create with Flush Failure

```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new123"
  },
  "warning": "auto-flush failed: 3 beads not exported to JSONL. Run 'bf sync --flush-only' to retry."
}
```

#### Actual Output (No Envelope, No Warning)

```json
{
  "id": "bf-new123"
}
```

#### Mismatch Fields

- **Missing:** `version` (required)
- **Missing:** `kind` (required, should be "create")
- **Missing:** `data` wrapper (id should be nested inside)
- **Missing:** `warning` (required when flush fails)

#### Failing Tests

1. `create_json_succeeds_warns_retains_dirty_and_recovers` - ID not in envelope structure, warning missing
2. `flush_failure_nonfatal_json_warning_and_dirty_retained` - create --json missing id envelope
3. `flush_failure_does_not_fail_mutation_and_warns_json` - ID not in envelope structure
4. `flush_failure_surfaces_warning_in_json_output` - ID not in envelope structure
5. `flush_failure_carries_json_warning` - ID not in envelope structure

#### Required Fix Location

**Files:** 
- `src/cli/create.rs`
- `src/sync.rs` (auto-flush warning generation)

**Current Code Pattern (incorrect):**
```rust
// Emits raw JSON, ignores warnings
let bead_id = create_bead()?;
let result = json!({"id": bead_id});
serde_json::to_string(&result)?
```

**Required Code Pattern:**
```rust
let bead_id = create_bead()?;
let warning = if flush_failed {
    Some("auto-flush failed: 3 beads not exported to JSONL. Run 'bf sync --flush-only' to retry.".to_string())
} else {
    None
};
let envelope = JsonEnvelope {
    version: 1,
    kind: "create".to_string(),
    data: json!({"id": bead_id}),
    warning,
};
serde_json::to_string(&envelope)?
```

---

### Category 5: Other Envelope-Related Failures (3 tests)
**Code Path:** Various  
**Total Failures:** 3

#### Expected Envelope for Stats

```json
{
  "version": 1,
  "kind": "stats",
  "data": {
    "total": 100,
    "open": 50,
    "in_progress": 30,
    "closed": 20,
    "by_type": {
      "task": 60,
      "bug": 20,
      "feature": 20
    },
    "by_priority": {
      "0": 10,
      "1": 20,
      "2": 40,
      "3": 20,
      "4": 10
    }
  }
}
```

#### Actual Output (No Envelope)

```json
{
  "total": 100,
  "open": 50,
  "in_progress": 30,
  "closed": 20,
  "by_type": {
    "task": 60,
    "bug": 20,
    "feature": 20
  },
  "by_priority": {
    "0": 10,
    "1": 20,
    "2": 40,
    "3": 20,
    "4": 10
  }
}
```

#### Mismatch Fields

- **Missing:** `version` (required)
- **Missing:** `kind` (required, should be "stats")
- **Missing:** `data` wrapper

#### Failing Tests

1. `envelope_stats_command_has_stable_structure` - stats data not nested correctly
2. `envelope_show_command_full_bead_structure` - show output missing envelope
3. `envelope_update_mutation_returns_id` - update output missing envelope

#### Required Fix Location

**Files:**
- `src/cli/stats.rs`
- `src/cli/show.rs`
- `src/cli/update.rs`

**Required Code Pattern:**
```rust
let result = execute_command()?;
let envelope = JsonEnvelope {
    version: 1,
    kind: "<command>".to_string(),
    data: serde_json::to_value(&result)?,
    warning: None,
};
serde_json::to_string(&envelope)?
```

---

## Summary by Code Path

| Code Path | Files to Modify | Tests Affected | Required Change |
|-----------|----------------|----------------|------------------|
| **Claim output** | `src/cli/claim.rs`, `src/format/json.rs` | 14 | Wrap claim result in envelope |
| **List outputs** | `src/cli/ready.rs`, `src/cli/recent.rs`, `src/cli/search.rs`, `src/cli/velocity.rs`, `src/format/json.rs` | 6 | Wrap arrays in envelope with data field |
| **Batch output** | `src/cli/batch.rs`, `src/format/json.rs` | 2 | Wrap batch results in envelope |
| **Create with warnings** | `src/cli/create.rs`, `src/sync.rs`, `src/format/json.rs` | 3 | Wrap result in envelope, add warning field |
| **Stats/Show/Update** | `src/cli/stats.rs`, `src/cli/show.rs`, `src/cli/update.rs`, `src/format/json.rs` | 3 | Wrap results in envelope |

**Total:** 28 tests, 11 files to modify

---

## Root Cause Analysis

### Why This Happened

The envelope format specification (`docs/envelope_format_spec.md`) was written to describe a stable, machine-readable JSON contract for all commands, but the implementation was never completed.

### The Fix Pattern

All JSON-emitting commands need to follow this pattern:

```rust
// OLD (incorrect):
let output = command_result?;
Ok(serde_json::to_string(&output)?)

// NEW (correct):
let command_result = execute_command()?;  // or whatever the command does
let envelope = JsonEnvelope {
    version: 1,
    kind: "<command-name>".to_string(),
    data: serde_json::to_value(&command_result)?,
    warning: warning_option,  // None or Some(message)
};
Ok(serde_json::to_string(&envelope)?)
```

### Centralized vs Distributed Implementation

**Option A (Centralized):** Fix in `src/format/json.rs` only
- Pro: Single fix location
- Con: May need to detect command context, hard to get warnings

**Option B (Distributed):** Fix in each CLI command file
- Pro: Clear command-specific logic, easy to add warnings
- Con: 11 files to modify

**Recommendation:** Option B - distributed fixes in each command file with shared `JsonEnvelope` struct.

---

## Verification Steps

After implementing the fix, verify with:

```bash
# Claim command
bf claim --assignee test-worker --format json | jq .
# Should see: {"version": 1, "kind": "claim", "data": {...}}

# Ready command (empty)
bf ready --format json | jq .
# Should see: {"version": 1, "kind": "ready", "data": []}

# Create command with auto-flush warning
bf create --title "Test" --format json | jq .
# Should see: {"version": 1, "kind": "create", "data": {"id": "bf-..."}, "warning": "..."}

# Run all envelope tests
cargo test envelope -- --nocapture
```

---

## References

- **Envelope Spec:** `docs/envelope_format_spec.md`
- **Existing Failures Doc:** `docs/envelope-failures.md`
- **Test Files:**
  - `tests/envelope_coverage.rs`
  - `tests/envelope_integration_tests.rs`
  - `tests/autoflush_failure_contract.rs`
  - `tests/autoflush_mutation.rs`
  - `tests/autoflush_wiring.rs`
  - `tests/kill_worker_preserves_beads.rs`
  - `tests/recovery_and_exit_criteria.rs`

---

## Conclusion

All 28 envelope format failures are caused by the same root issue: commands are emitting raw JSON instead of wrapping it in the standardized envelope structure. The fix is straightforward but requires updating 11 command files to wrap their output in the envelope structure defined in `docs/envelope_format_spec.md`.

Once the envelope wrapper is consistently applied, all 28 tests will pass, bringing the test suite from ~98.1% to ~100% pass rate.
