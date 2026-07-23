# Envelope Format Test Failures - Investigation Report

**Bead:** bf-3texk
**Date:** 2026-07-23
**Investigator:** claude-code-glm-4.7

## Executive Summary

**16 integration tests are failing** because the `--envelope` flag is being **completely ignored** by most commands. Commands output raw JSON without wrapping it in the standard envelope structure `{version, kind, data, warning?}`.

## Expected Envelope Format

All commands with `--json --envelope` (or `--format json --envelope`) should emit:

```json
{
  "version": 1,
  "kind": "<command_name>",
  "data": <command_specific_data>,
  "warning": "<optional_warning_message>"
}
```

### Command-Specific Data Shapes

| Command | `data` Shape | Empty Result | Example |
|---------|-------------|--------------|---------|
| `create` | `{"id": "bf-xxx"}` | N/A (always succeeds) | `{"data":{"id":"bf-123"}}` |
| `claim` | `{"bead_id": "...", "assignee": "...", ...}` | `{}` | `{"data":{"bead_id":"bf-1","assignee":"worker"}}` |
| `stats` | `{total: N, open: N, ...}` | N/A | `{"data":{"total":42,"open":10,...}}` |
| `velocity` | `[{...}, {...}]` | `[]` | `{"data":[{velocity_obj1},{velocity_obj2}]}` |
| `list` | `[{...}, {...}]` | `[]` | `{"data":[{issue1},{issue2}]}` |
| `ready` | `[{...}, {...}]` | `[]` | `{"data":[{ready_issue1}]}` |
| `search` | `[{...}, {...}]` | `[]` | `{"data":[{matching_issue}]}` |
| `recent` | `[{...}, {...}]` | `[]` | `{"data":[{recent_issue}]}` |
| `show` | `{...}` (single issue) | error | `{"data":{issue_object}}` |
| `batch` | `[{op:...,result:...}]` | `[]` | `{"data":[{op1},{op2}]}` |

## Actual Behavior (Current)

### Example: `bf claim --json --envelope`

**Expected:**
```json
{"version":1,"kind":"claim","data":{"bead_id":"bf-bziwd","assignee":"test-worker","reclaimed":0}}
```

**Actual:**
```json
{"bead_id":"bf-bziwd","assignee":"test-worker","reclaimed":0}
```

The envelope wrapper is **completely missing** - the command outputs raw JSON.

## Root Cause Analysis

### 1. Global Envelope Flag Not Implemented

The `--envelope` flag is defined in `src/cli/mod.rs`:

```rust
#[arg(long, global = true, help = "...")]
envelope: bool,
```

But the handler only calls a no-op:

```rust
// src/cli/mod.rs:1064-1068
if cli.envelope {
    crate::format::json::JsonFormatter::with_envelope_enabled();
}
```

**`JsonFormatter::with_envelope_enabled()` is a no-op** - it returns `JsonFormatter` without storing any state:

```rust
// src/format/json.rs:10-13
pub fn with_envelope_enabled() -> Self {
    JsonFormatter  // Does nothing!
}
```

### 2. Command Handlers Don't Check Envelope Flag

Individual command handlers like `cmd_claim`, `cmd_batch`, `cmd_ready`, etc. **do not check** `cli.envelope` when outputting JSON. They call:

```rust
formatter.format_claim_result(&out)  // Returns raw JSON, not envelope-wrapped
```

Instead of:

```rust
formatter.format_with_envelope("claim", &json_str)  // Would wrap in envelope
```

### 3. Why `create` Works

The `create` command **explicitly** wraps its output using `format_with_envelope_and_warning`:

```rust
// src/cli/mod.rs:1540-1544
if json {
    let formatter = get_formatter(OutputFormat::Json);
    let data = serde_json::json!({ "id": id });
    let json_str = serde_json::to_string(&data)?;
    println!("{}", formatter.format_with_envelope_and_warning("create", &json_str, warning.as_deref()));
}
```

This is why `create --json --envelope` tests pass but all other commands fail.

## Failing Tests by Command

### 1. `claim` Command (6 tests failing)

**Failing tests:**
- `envelope_claim_command_has_stable_structure`
- `envelope_claim_no_bead_emits_empty_object`
- `claim_stats::envelope_claim_and_stats_consistent_structure`
- `claim_stats::envelope_claim_bead_id_is_valid`
- `claim_stats::envelope_claim_json_has_metadata_fields`
- `claim_stats::envelope_claim_json_returns_claim_result`
- `claim_stats::envelope_claim_no_beads_returns_empty_object`
- `claim_stats::envelope_claim_reflects_assignee`

**Current output:** `{"bead_id":"bf-xxx","assignee":"...",...}`

**Expected output:** `{"version":1,"kind":"claim","data":{"bead_id":"bf-xxx",...}}`

**Code location:** `src/cli/mod.rs:2020-2130` (claim command handler)

### 2. `batch` Command (2 tests failing)

**Failing tests:**
- `envelope_batch_command_has_stable_structure`
- `envelope_batch_empty_emits_empty_array`

**Current output:** JSONL (one line per operation) or raw array
**Expected output:** `{"version":1,"kind":"batch","data":[...]}`

**Code location:** `src/cli/mod.rs:1826-1929` (batch command handler)

### 3. `ready` Command (2 tests failing)

**Failing tests:**
- `envelope_ready_command_has_stable_structure`

**Current output:** JSONL (one issue per line)
**Expected output:** `{"version":1,"kind":"ready","data":[{issue1},{issue2},...]}`

**Code location:** `src/cli/mod.rs:1673-1759` (ready command handler)

### 4. `recent` Command (2 tests failing)

**Failing tests:**
- `envelope_recent_command_has_stable_structure`
- `envelope_recent_empty_emits_empty_array`

**Current output:** JSONL (one issue per line)
**Expected output:** `{"version":1,"kind":"recent","data":[{issue1},...]}`

**Code location:** `src/cli/mod.rs:1364-1389` (recent command handler)

### 5. `search` Command (2 tests failing)

**Failing tests:**
- `envelope_search_command_has_stable_structure`
- `envelope_search_empty_emits_empty_array`

**Current output:** JSONL (one issue per line)
**Expected output:** `{"version":1,"kind":"search","data":[{match1},...]}`

**Code location:** `src/cli/mod.rs:1694-1779` (search command handler)

### 6. `velocity` Command (2 tests failing)

**Failing tests:**
- `envelope_velocity_empty_emits_empty_array`
- `envelope_velocity_command_has_stable_structure`

**Current output:** `[velocity_obj1,velocity_obj2,...]` (JSON array)
**Expected output:** `{"version":1,"kind":"velocity","data":[velocity_obj1,...]}`

**Code location:** `src/cli/mod.rs:1274-1295` (velocity command handler)

## Commands That Pass Tests

These commands **already** implement envelope wrapping correctly:

1. **`create`** - Passes all tests (2 tests)
2. **`show`** - Passes all tests (2 tests) 
3. **`list`** - Passes all tests (2 tests)
4. **`stats`** - Passes all tests (2 tests)

## Code Path Analysis

### The Formatter Interface

The `Formatter` trait defines two methods for envelope wrapping:

```rust
// src/format/mod.rs:127-130
fn format_with_envelope(&self, kind: &str, data: &str) -> String;
fn format_with_envelope_and_warning(&self, kind: &str, data: &str, warning: Option<&str>) -> String;
```

**JsonFormatter implements these correctly:**

```rust
// src/format/json.rs:81-106
fn format_with_envelope(&self, kind: &str, data: &str) -> String {
    let json_value: Value = serde_json::from_str(data)
        .unwrap_or_else(|_| Value::String(data.to_string()));
    JsonEnvelope::new(kind, json_value)
        .to_json_compact()
        .unwrap_or_else(|_| "{}".to_string())
}

fn format_with_envelope_and_warning(&self, kind: &str, data: &str, warning: Option<&str>) -> String {
    let json_value: Value = serde_json::from_str(data)
        .unwrap_or_else(|_| Value::String(data.to_string()));
    let envelope = JsonEnvelope::new(kind, json_value);
    let envelope_with_warning = match warning {
        Some(w) => envelope.with_warning(w),
        None => envelope,
    };
    envelope_with_warning
        .to_json_compact()
        .unwrap_or_else(|_| "{}".to_string())
}
```

**But command handlers don't call these methods!** They use `format_claim_result`, `format_issues`, etc., which return raw JSON.

## Required Fix Pattern

Each command handler needs to:

1. Capture the raw JSON output
2. If `cli.envelope` is true, wrap it using `format_with_envelope` or `format_with_envelope_and_warning`
3. Print the wrapped result

### Example Fix for `claim`:

```rust
// Current (line ~2026):
println!("{}", formatter.format_claim_result(&out));

// Should be:
let json_str = formatter.format_claim_result(&out);
if cli.envelope {
    println!("{}", formatter.format_with_envelope("claim", &json_str));
} else {
    println!("{}", json_str);
}
```

## Impact Summary

| Command | Current Behavior | Tests Failing | Lines to Change |
|---------|------------------|---------------|-----------------|
| claim | Raw JSON | 6 | ~10 lines (multiple code paths) |
| batch | Raw JSON/JSONL | 2 | ~5 lines |
| ready | Raw JSONL | 1 | ~3 lines |
| recent | Raw JSONL | 2 | ~3 lines |
| search | Raw JSONL | 2 | ~3 lines |
| velocity | Raw JSON array | 2 | ~3 lines |
| **create** | **Envelope-wrapped** | **0 (passing)** | **0 (already fixed)** |
| **show** | **Envelope-wrapped** | **0 (passing)** | **0 (already fixed)** |
| **list** | **Envelope-wrapped** | **0 (passing)** | **0 (already fixed)** |
| **stats** | **Envelope-wrapped** | **0 (passing)** | **0 (already fixed)** |

## Test Coverage Evidence

The tests in `tests/envelope_coverage.rs` comprehensively verify:

1. **Envelope structure**: `verify_envelope_structure` checks for `version`, `kind`, `data` fields
2. **Data shape**: Array vs object, empty vs populated
3. **Metadata**: All required fields present
4. **Command consistency**: Same structure across all commands

The tests are **well-designed** and **correctly failing** - the implementation is missing, not the tests.

## Conclusion

**All 16 failing tests are valid bugs.** The `--envelope` flag infrastructure exists (formatter methods, JsonEnvelope type, CLI flag) but is **not wired into the command handlers** for `claim`, `batch`, `ready`, `recent`, `search`, and `velocity`.

**Fix scope:** ~30 lines across 6 command handlers in `src/cli/mod.rs`.

**Next bead:** Implement envelope wrapping for all failing commands.
