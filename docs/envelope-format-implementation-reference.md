# Envelope Format Implementation Reference

**Purpose:** This document provides a comprehensive reference for implementing and validating JSON envelope format across all `bf` commands. It maps expected vs. actual formats, identifies specific code paths, and provides fix patterns for each failure category.

**Last Updated:** 2026-08-14

**Related Documents:**
- `docs/envelope_format_spec.md` - Full envelope specification
- `envelope-format-failure-report.md` - Detailed failure analysis
- `docs/envelope-failures.md` - Original failure documentation

---

## Quick Reference: Envelope Structure

### Expected Envelope Format (All Commands)

```json
{
  "version": 1,
  "kind": "<command-name>",
  "data": <command-specific-data>,
  "warning": "<optional-warning-message>"
}
```

**Field Requirements:**
- `version`: Always present, always integer `1`
- `kind`: Always present, string matching command name
- `data`: Always present, type varies by command (object, array, string, etc.)
- `warning`: Present only when non-fatal error occurs (e.g., auto-flush failure)

---

## Category 1: Claim Command (14 failing tests)

### Expected Envelope Structure

**Successful Claim:**
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

**No Beads Available:**
```json
{
  "version": 1,
  "kind": "claim",
  "data": {}
}
```

### Actual Response Structure (Current Implementation)

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

### Code Path Analysis

**Files Involved:**
- `src/cli/mod.rs` - Command dispatch (line ~1277)
- `src/format/json.rs` - JSON formatter implementation
- `src/claim.rs` - Claim logic

**Current Code Path (INCORRECT):**
```rust
// src/cli/mod.rs around line 2198-2298
// cmd_claim function calls formatter.format_claim_result(&out)
// which directly serializes ClaimResult without envelope

pub fn cmd_claim(...) -> Result<()> {
    // ... claim logic ...
    let out = claim(...)?;
    let formatter = get_formatter(OutputFormat::Json);
    
    // ❌ WRONG: Direct serialization, no envelope
    println!("{}", formatter.format_claim_result(&out));
    Ok(())
}
```

**Current Implementation in `src/format/json.rs` (lines 66-72):**
```rust
fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
    // ❌ WRONG: Direct serialization without envelope wrapper
    serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string())
}
```

**Required Fix Pattern:**
```rust
// In cmd_claim function, replace format_claim_result with:
let json_str = serde_json::to_string(&out)?;
println!("{}", formatter.format_with_envelope("claim", &json_str));
```

**Or update `src/format/json.rs`:**
```rust
fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
    let json_str = serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string());
    // ✅ CORRECT: Wrap in envelope
    self.format_with_envelope("claim", &json_str)
}
```

### Failing Tests

1. `envelope_claim_bead_id_is_valid` - bead_id field at wrong path
2. `envelope_claim_and_stats_consistent_structure` - missing version field
3. `envelope_claim_json_has_metadata_fields` - missing version/kind fields
4. `envelope_claim_json_returns_claim_result` - envelope structure missing
5. `envelope_claim_no_beads_returns_empty_object` - missing version field
6. `envelope_claim_command_has_stable_structure` - envelope not an object
7. `envelope_claim_reflects_assignee` - assignee at wrong nesting level
8. `claim_envelope_empty_workspace` - version field missing
9. `claim_envelope_data_fields` - data field not an object
10. `claim_envelope_has_stable_structure` - version field missing
11. `claim_envelope_kind_matches_command` - kind field missing
12. `claim_envelope_metadata_fields` - version field missing
13. `claim_envelope_structure_consistency` - wrong structure
14. `claim_envelope_successful_case` - version field missing
15. `claim_envelope_version_always_one` - version field missing

---

## Category 2: List-like Commands (6 failing tests)

**Commands Affected:** `ready`, `recent`, `search`, `velocity`

### Expected Envelope Structure

**With Results:**
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
    }
  ]
}
```

**Empty Results:**
```json
{
  "version": 1,
  "kind": "ready",
  "data": []
}
```

### Actual Response Structure (Current Implementation)

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

### Code Path Analysis

**Files Involved:**
- `src/cli/mod.rs` - Command dispatch (lines ~1277, 1363-1375)
- `src/cli/ready.rs` - Ready command implementation
- `src/format/json.rs` - JSON formatter (lines 53-60)

**Current Code Path (INCORRECT for list commands):**
```rust
// src/format/json.rs lines 53-60
fn format_issues(&self, issues: &[Issue]) -> String {
    issues
        .iter()
        .map(|issue| serde_json::to_string(&issue_to_value(issue)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")  // ❌ WRONG: Returns NDJSON, not array
}
```

**Current Command Implementation (may vary by command):**
```rust
// Some commands correctly use format_with_envelope:
// src/cli/mod.rs line 1781 (list command)
println!("{}", formatter.format_with_envelope("list", &data));

// src/cli/mod.rs line 3981 (recent command)
println!("{}", formatter.format_with_envelope("recent", &json_str));

// But ready/search/velocity may have different implementations
```

**Required Fix Pattern:**
Ensure all list-like commands follow this pattern:
```rust
// Serialize list to JSON array first
let json_array = serde_json::to_string(&items)?;
// Wrap in envelope
println!("{}", formatter.format_with_envelope("<command>", &json_array));
```

**Special Case - Empty Results:**
```rust
// For empty results, explicitly emit "[]" before wrapping:
let json_array = if items.is_empty() {
    "[]".to_string()
} else {
    serde_json::to_string(&items)?
};
println!("{}", formatter.format_with_envelope("<command>", &json_array));
```

### Failing Tests

1. `envelope_ready_command_has_stable_structure` - data not an array
2. `envelope_recent_command_has_stable_structure` - data not an array
3. `envelope_recent_empty_emits_empty_array` - data not an array
4. `envelope_search_empty_emits_empty_array` - invalid JSON/empty case
5. `envelope_search_command_has_stable_structure` - version field missing
6. `envelope_velocity_command_has_stable_structure` - envelope not an object
7. `envelope_velocity_empty_emits_empty_array` - envelope not an object
8. `envelope_data_field_always_present` - missing data field
9. `envelope_kind_matches_command` - missing kind field
10. `envelope_version_is_always_one` - missing version field

---

## Category 3: Batch Command (2 failing tests)

### Expected Envelope Structure

**Successful Batch:**
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

**Empty Batch:**
```json
{
  "version": 1,
  "kind": "batch",
  "data": []
}
```

### Actual Response Structure (Current Implementation)

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

### Code Path Analysis

**Files Involved:**
- `src/cli/mod.rs` - cmd_batch function (line ~2761)
- `src/batch.rs` - Batch execution logic

**Current Code Path (CORRECT - uses envelope):**
```rust
// src/cli/mod.rs line 2761
println!("{}", formatter.format_with_envelope("batch", &json_array));
```

**Status:** ✅ Already correctly implemented

**Note:** Despite being correctly implemented, tests may still fail if the `json_array` string construction is incorrect or if there are edge cases not handled.

### Failing Tests

1. `envelope_batch_command_has_stable_structure` - data not an array
2. `envelope_batch_empty_emits_empty_array` - data not an array

---

## Category 4: Auto-flush Warning Field (5 failing tests)

**Commands Affected:** `create`, `update`, `close` (any mutating command with auto-flush)

### Expected Envelope Structure

**With Auto-flush Failure:**
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

**Without Warnings:**
```json
{
  "version": 1,
  "kind": "create",
  "data": {
    "id": "bf-new123"
  }
}
```

### Actual Response Structure (Current Implementation)

```json
{
  "id": "bf-new123"
}
```

### Code Path Analysis

**Files Involved:**
- `src/cli/mod.rs` - cmd_create, cmd_update, cmd_close functions
- `src/sync.rs` - Auto-flush logic
- `src/format/json.rs` - Envelope with warning support

**Current Code Path (PARTIALLY CORRECT):**
```rust
// src/cli/mod.rs line 1668 (create command)
formatter.format_with_envelope_and_warning("create", &json_str, warning.as_deref())

// src/cli/mod.rs line 2023 (update command)
formatter.format_with_envelope_and_warning("update", &json_str, warning.as_deref())

// src/cli/mod.rs line 2050 (close command)
formatter.format_with_envelope_and_warning("close", &json_str, warning.as_deref())
```

**Status:** ✅ Envelope with warning support is correctly implemented

**Implementation in `src/format/json.rs` (lines 93-112):**
```rust
fn format_with_envelope_and_warning(
    &self,
    kind: &str,
    data: &str,
    warning: Option<&str>,
) -> String {
    let json_value: Value = serde_json::from_str(data)
        .unwrap_or_else(|_| Value::String(data.to_string()));
    
    let envelope = JsonEnvelope::new(kind, json_value);
    let envelope_with_warning = match warning {
        Some(w) => envelope.with_warning(w),
        None => envelope,
    };
    envelope_with_warning.to_json_compact()
        .unwrap_or_else(|_| "{}".to_string())
}
```

### Failing Tests

1. `create_json_succeeds_warns_retains_dirty_and_recovers` - warning field missing
2. `flush_failure_nonfatal_json_warning_and_dirty_retained` - warning missing
3. `flush_failure_does_not_fail_mutation_and_warns_json` - warning missing
4. `flush_failure_surfaces_warning_in_json_output` - warning missing
5. `flush_failure_carries_json_warning` - warning missing

**Note:** These tests may be failing due to test setup issues or auto-flush not being triggered in the test environment, not due to missing envelope implementation.

---

## Category 5: Other Commands (3 failing tests)

**Commands Affected:** `stats`, `show`, `update` (without warnings)

### Expected Envelope Structure

**Stats Command:**
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

**Show Command:**
```json
{
  "version": 1,
  "kind": "show",
  "data": {
    "id": "bf-abc123",
    "title": "Implement auth flow",
    "description": "Add OAuth2 authentication",
    "status": "open",
    "priority": 2,
    "issue_type": "task",
    "assignee": null,
    "labels": ["phase-1", "backend"],
    "created_at": "2026-07-22T15:54:16Z",
    "updated_at": "2026-07-22T15:54:16Z"
  }
}
```

### Actual Response Structure (Current Implementation)

**Stats (without envelope):**
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

**Show (without envelope):**
```json
{
  "id": "bf-abc123",
  "title": "Implement auth flow",
  "description": "Add OAuth2 authentication",
  "status": "open",
  "priority": 2,
  "issue_type": "task",
  "assignee": null,
  "labels": ["phase-1", "backend"],
  "created_at": "2026-07-22T15:54:16Z",
  "updated_at": "2026-07-22T15:54:16Z"
}
```

### Code Path Analysis

**Files Involved:**
- `src/cli/mod.rs` - cmd_stats, cmd_show functions
- `src/format/json.rs` - format_stats method

**Current Code Path (MIXED):**
```rust
// src/cli/mod.rs line 1868 (show command)
println!("{}", formatter.format_with_envelope("show", &json_str));

// src/cli/mod.rs line 3303 (stats command)
println!("{}", formatter.format_with_envelope("stats", &json_str));

// But format_stats in src/format/json.rs (line 74-76):
fn format_stats(&self, stats: &StatsOutput) -> String {
    serde_json::to_string(stats).unwrap_or_else(|_| "{}".to_string())
}
```

**Status:** ⚠️ Partially implemented - some commands use envelope, but formatter methods may not

### Failing Tests

1. `envelope_stats_command_has_stable_structure` - stats not in envelope
2. `envelope_show_command_full_bead_structure` - show not in envelope
3. `envelope_update_mutation_returns_id` - update not in envelope

---

## Implementation Fix Summary

### Required Changes by File

**1. `src/format/json.rs` - Update formatter methods**
```rust
// Replace direct serialization with envelope wrapping:
fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
    let json_str = serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string());
    self.format_with_envelope("claim", &json_str)
}

fn format_stats(&self, stats: &StatsOutput) -> String {
    let json_str = serde_json::to_string(stats).unwrap_or_else(|_| "{}".to_string());
    self.format_with_envelope("stats", &json_str)
}

fn format_velocity(&self, stats: &[VelocityStats]) -> String {
    let json_str = serde_json::to_string(stats).unwrap_or_else(|_| "[]".to_string());
    self.format_with_envelope("velocity", &json_str)
}
```

**2. `src/cli/mod.rs` - Ensure consistent envelope usage**
```rust
// All list commands should serialize to array, then wrap:
let items = fetch_items()?;
let json_array = serde_json::to_string(&items)?;
println!("{}", formatter.format_with_envelope("<command>", &json_array));
```

**3. `src/cli/ready.rs`, `src/cli/recent.rs`, `src/cli/search.rs` - Verify envelope usage**
Ensure each command properly wraps output in envelope.

### Files Requiring Changes

| File | Change Required | Priority |
|------|----------------|----------|
| `src/format/json.rs` | Update 3 formatter methods to use envelope | P0 |
| `src/cli/mod.rs` | Verify claim command uses envelope | P0 |
| `src/cli/ready.rs` | Verify ready output uses envelope | P0 |
| `src/cli/recent.rs` | Verify recent output uses envelope | P0 |
| `src/cli/search.rs` | Verify search output uses envelope | P0 |

### Verification Steps

After implementing fixes:

```bash
# Test claim command envelope
bf claim --assignee test-worker --format json | jq .
# Expected: {"version": 1, "kind": "claim", "data": {...}}

# Test ready command envelope (empty)
bf ready --format json | jq .
# Expected: {"version": 1, "kind": "ready", "data": []}

# Test stats command envelope
bf stats --format json | jq .
# Expected: {"version": 1, "kind": "stats", "data": {...}}

# Run all envelope tests
cargo test envelope -- --nocapture
```

---

## Design Principles

### 1. Envelope-First JSON Output

All JSON output must be wrapped in envelope, regardless of command type:
- Single objects → wrap in `data` object
- Arrays → wrap in `data` array
- Empty results → `"data": []` or `"data": {}` depending on command

### 2. Formatter Abstraction

The `Formatter` trait should handle envelope wrapping internally, not leave it to individual commands:

```rust
// ❌ WRONG: Command handles envelope
let json = serde_json::to_string(&data)?;
let envelope = JsonEnvelope::new("command", json_value);
println!(envelope.to_json());

// ✅ CORRECT: Formatter handles envelope
formatter.format_with_envelope("command", &data);
```

### 3. Warning Field Discipline

The `warning` field is ONLY for non-fatal errors that don't fail the command:
- Auto-flush failures
- Partial batch completions
- Deprecation notices
- Recovery guidance

Fatal errors should still return error exit codes and error messages.

### 4. Version Field Stability

The `version` field enables future compatibility:
- Current version: `1` (integer, not string)
- Increment only for breaking changes to envelope structure
- Consumers should reject unknown versions
- Producers must never omit the version field

---

## Testing Strategy

### Unit Tests

Test envelope structure at the formatter level:
```rust
#[test]
fn test_envelope_structure() {
    let envelope = JsonEnvelope::new("test", json!({"key": "value"}));
    let json = envelope.to_json().unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();
    
    assert_eq!(parsed["version"], 1);
    assert_eq!(parsed["kind"], "test");
    assert!(parsed["data"].is_object());
    assert!(parsed.get("warning").is_none());
}
```

### Integration Tests

Test complete command execution with envelope:
```rust
#[test]
fn test_claim_envelope() {
    let output = bf_command(&["claim", "--assignee", "worker", "--format", "json"]);
    let envelope: Value = serde_json::from_str(&output).unwrap();
    
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["kind"], "claim");
    assert!(envelope["data"].is_object());
}
```

### Edge Cases

Test envelope handling for:
- Empty results (`"data": []`)
- Null values (`"data": null`)
- Warnings (`warning` field presence)
- Malformed data (graceful degradation)

---

## Migration Guide

### For Command Implementers

When adding a new JSON command:

1. **Define the `data` shape** - What does your command return?
2. **Use envelope wrapper** - Call `format_with_envelope()` or `format_with_envelope_and_warning()`
3. **Add tests** - Verify envelope structure, version, kind, and data fields
4. **Document** - Update this reference with expected vs. actual format

### For Consumers

When parsing `bf` JSON output:

1. **Parse envelope first** - Extract `version`, `kind`, `data`, `warning`
2. **Validate version** - Reject unknown versions
3. **Check warning** - Surface to user even on success
4. **Parse data by kind** - Use `kind` field to determine how to parse `data`

### Example Consumer Code

```python
import json
import sys

def parse_bf_output(output):
    envelope = json.loads(output)
    
    # Validate version
    if envelope.get("version") != 1:
        print(f"Unknown envelope version: {version}", file=sys.stderr)
        return None
    
    # Check for warnings
    if "warning" in envelope:
        print(f"Warning: {envelope['warning']}", file=sys.stderr)
    
    # Parse data based on command kind
    kind = envelope["kind"]
    data = envelope["data"]
    
    if kind == "claim":
        return parse_claim(data)
    elif kind == "list":
        return parse_list(data)
    # ... handle other kinds
    
    return data
```

---

## References

**Implementation Files:**
- `src/format/envelope.rs` - Envelope structure definition
- `src/format/json.rs` - JSON formatter with envelope methods
- `src/cli/mod.rs` - Command dispatch and envelope usage
- `src/cli/ready.rs` - Ready command implementation
- `src/claim.rs` - Claim logic and result structures

**Test Files:**
- `tests/envelope_coverage.rs` - Comprehensive envelope tests
- `tests/envelope_integration_tests.rs` - Integration tests by command
- `tests/envelope/claim_stats.rs` - Claim-specific envelope tests
- `tests/autoflush_*.rs` - Auto-flush warning field tests

**Documentation:**
- `docs/envelope_format_spec.md` - Full envelope specification
- `envelope-format-failure-report.md` - Detailed failure analysis
- `docs/README.md` - User-facing command documentation

---

**End of Reference Document**
