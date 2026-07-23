# Test Failure Grouping by Response Construction Path

**Bead:** bf-1uum9  
**Date:** 2026-07-23  
**Total Failing Tests:** 16  
**Working Commands:** create, list, show, stats

## Executive Summary

All 16 failing tests stem from 4 distinct code path patterns in `src/cli/mod.rs`:

1. **Pattern A: Envelope parameter ignored entirely** (8 tests)
2. **Pattern B: JSONL passed to envelope parser** (7 tests)
3. **Pattern C: Incorrect array nesting** (1 test)

---

## Pattern A: Envelope Parameter Ignored (8 tests)

### Commands Affected
- **claim** (6 failing tests)
- **search** (2 failing tests)

### Root Cause
These commands accept an `envelope: bool` parameter but never use it in their JSON output logic. They output raw JSON/JSONL directly without envelope wrapping.

### Code Locations

#### Claim Command: `src/cli/mod.rs:1926-2133`
- **Function signature (line 1937):** `envelope: bool` parameter exists
- **Output paths (lines 2026, 2053, 2077, 2099, 2124):** 
  ```rust
  println!("{}", formatter.format_claim_result(&out));
  println!("{}", formatter.format_no_claim());
  ```
- **Problem:** The `envelope` parameter is never checked. Output is always raw JSON.

#### Search Command: `src/cli/mod.rs:2829-2882`
- **Function signature (line 2840):** `envelope: bool` parameter exists
- **Output path (lines 2869-2874):**
  ```rust
  OutputFormat::Json => {
      let jsonl = formatter.format_issues(&issues);
      if !jsonl.is_empty() {
          println!("{}", jsonl);
      }
  }
  ```
- **Problem:** The `envelope` parameter is never checked. Output is raw JSONL.

### Failing Tests in this Category

**Claim (6 tests):**
1. `envelope_claim_bead_id_is_valid` - Claims envelope data is missing bead_id
2. `envelope_claim_and_stats_consistent_structure` - Claims version field missing
3. `envelope_claim_json_has_metadata_fields` - Claims version metadata missing
4. `envelope_claim_json_returns_claim_result` - Claims envelope version is None
5. `envelope_claim_no_beads_returns_empty_object` - Claims envelope version is None
6. `envelope_claim_reflects_assignee` - Claims envelope missing assignee data

**Search (2 tests):**
7. `envelope_search_empty_emits_empty_array` - No JSON output (EOF error)
8. `envelope_search_command_has_stable_structure` - Claims envelope version is None

### Why These Fail Together
Both commands have identical logic: they receive the `envelope` flag but the JSON output branch never conditionally wraps based on it. The fix is identical for both: check `if envelope { println!("{}", formatter.format_with_envelope(...)); }`

---

## Pattern B: JSONL Passed to Envelope Parser (7 tests)

### Commands Affected
- **batch** (2 failing tests)
- **recent** (2 failing tests)
- **ready** (1 failing test)

### Root Cause
These commands use `formatter.format_with_envelope()` but pass **JSONL** (newline-separated JSON objects) instead of a valid JSON array. The envelope parser (src/format/json.rs:81-90) tries to parse the data string as JSON and fails, treating it as a string instead.

### Code Locations

#### Batch Command: `src/cli/mod.rs:2420-2481`
- **Output path (lines 2452-2458):**
  ```rust
  let jsonl = results
      .iter()
      .map(|r| serde_json::to_string(r))
      .collect::<Result<Vec<_>, _>>()
      .unwrap_or_default()
      .join("\n");  // ← Creates JSONL, not JSON array
  println!("{}", formatter.format_with_envelope("batch", &jsonl));
  ```
- **Problem:** `jsonl` is `"{}{}\n{}"` (newline-separated), not `"[{},{},{}]"` (JSON array)

#### Ready Command: `src/cli/mod.rs:1860-1924`
- **Output path (lines 1874-1893):**
  ```rust
  let jsonl = formatter.format_issues(&issues);
  if envelope {
      let data = if jsonl.is_empty() { "[]".to_string() } else { jsonl };
      println!("{}", formatter.format_with_envelope("ready", &data));
  }
  ```
- **Problem:** When non-empty, `jsonl` is JSONL (from format_issues), not a JSON array

#### Recent Command: `src/cli/mod.rs:3538-3615`
- **Output path (lines 3605-3607):**
  ```rust
  let json_str = formatter.format_issues(&issues);
  println!("{}", formatter.format_with_envelope("recent", &json_str));
  ```
- **Problem:** `json_str` is JSONL from format_issues, not a JSON array

### Why JSONL Breaks the Envelope Parser

From `src/format/json.rs:81-90`:
```rust
fn format_with_envelope(&self, kind: &str, data: &str) -> String {
    let json_value: Value = serde_json::from_str(data)  // ← Fails on JSONL
        .unwrap_or_else(|_| Value::String(data.to_string()));  // ← Falls back to string
    // ...
}
```

When `serde_json::from_str()` receives JSONL like `{"id":"a"}\n{"id":"b"}`, parsing fails and the data becomes a string, not an array.

### Failing Tests in this Category

**Batch (2 tests):**
9. `envelope_batch_command_has_stable_structure` - Claims "batch data must be an array"
10. `envelope_batch_empty_emits_empty_array` - Claims "batch data must be an array"

**Ready (1 test):**
11. `envelope_ready_command_has_stable_structure` - Claims "ready data must be an array"

**Recent (2 tests):**
12. `envelope_recent_empty_emits_empty_array` - Claims "recent data must be an array"
13. `envelope_recent_command_has_stable_structure` - Claims "recent data must be an array"

### Why These Fail Together
All three commands use `format_issues()` which returns JSONL (line 52-59 in src/format/json.rs):
```rust
fn format_issues(&self, issues: &[Issue]) -> String {
    issues.iter()
        .map(|issue| serde_json::to_string(&issue_to_value(issue)))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_default()
        .join("\n")  // ← JSONL format
}
```

The fix requires converting the issues to a JSON array before passing to `format_with_envelope()`.

---

## Pattern C: Incorrect Array Nesting (1 test)

### Commands Affected
- **velocity** (1 failing test)

### Root Cause
The velocity command DOES handle envelope wrapping correctly (lines 3178-3186), but the data structure doesn't match the test's expectation. The test expects `data` to be `[[]]` (nested array) for empty results.

### Code Location

#### Velocity Command: `src/cli/mod.rs:3160-3193`
- **Output path (lines 3178-3186):**
  ```rust
  OutputFormat::Json => {
      let jsonl = formatter.format_velocity(&stats);
      if envelope {
          let data = if jsonl.is_empty() { "[]".to_string() } else { jsonl };
          println!("{}", formatter.format_with_envelope("velocity", &data));
      } else {
          println!("{}", jsonl);
      }
  }
  ```

### The Issue

From `src/format/json.rs:77-79`:
```rust
fn format_velocity(&self, stats: &[VelocityStats]) -> String {
    serde_json::to_string(stats).unwrap_or_else(|_| "[]".to_string())
}
```

`serde_json::to_string(&[])` produces `"[]"` (a flat array), but the test at `tests/envelope_coverage.rs:310` expects:
```rust
let inner = data.as_array().unwrap().first().expect("velocity data must contain inner array");
```

This expects the envelope data to be `[[]]` (nested array), not `[]` (flat array). The test expectation may be incorrect.

### Failing Tests in this Category

**Velocity (1 test):**
14. `envelope_velocity_empty_emits_empty_array` - Claims "velocity data must contain inner array"

---

## Summary Table

| Category | Commands | Test Count | Code Pattern | Fix Required |
|----------|----------|------------|--------------|--------------|
| **Pattern A** | claim, search | 8 | Envelope param ignored | Add `if envelope` branch |
| **Pattern B** | batch, ready, recent | 7 | JSONL → envelope parser | Convert to JSON array |
| **Pattern C** | velocity | 1 | Array structure mismatch | Investigate test expectation |

## Correct Implementation Examples

These commands already handle envelopes correctly:

### Show Command: `src/cli/mod.rs:1738-1791`
- Has `envelope: bool` parameter
- Uses it to conditionally wrap output
- Passes valid JSON (not JSONL) to envelope

### List Command: `src/cli/mod.rs:1075-1119`
- Has `envelope: bool` parameter
- Uses it to conditionally wrap output
- Properly constructs JSON array for envelope

### Stats Command: `src/cli/mod.rs:2884-2955`
- Has `envelope: bool` parameter
- Uses it to conditionally wrap output
- Passes valid JSON object to envelope

---

## Implementation Notes

### Pattern A Fix (claim, search)
Add conditional envelope wrapping in the JSON output branch:
```rust
if envelope {
    println!("{}", formatter.format_with_envelope("claim", &formatted_result));
} else {
    println!("{}", formatted_result);
}
```

### Pattern B Fix (batch, ready, recent)
Convert issues to JSON array before wrapping:
```rust
// Instead of format_issues() which returns JSONL
let json_array = serde_json::to_string(&issues)?;
println!("{}", formatter.format_with_envelope("ready", &json_array));
```

### Pattern C Investigation (velocity)
Verify whether the test expectation or the implementation is correct. The test may be expecting legacy behavior that should be updated.
