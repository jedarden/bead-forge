# Envelope Test Infrastructure Survey

**Bead:** bf-1tugi  
**Date:** 2026-07-23  
**Purpose:** Survey existing envelope test infrastructure to understand patterns and helpers that can be reused for text/toon formats.

---

## Overview

The envelope test infrastructure provides reusable helpers for validating JSON envelope structure across all `bf` commands. The system is well-organized and modular, making it straightforward to extend for text/toon formats.

## File Structure

```
tests/
├── envelope_helpers.rs              # Core reusable validation helpers
├── test_envelope_helpers_usage.rs   # Usage examples and documentation
├── envelope_coverage.rs              # Comprehensive integration tests
├── envelope_integration_tests.rs    # Entry point (delegates to envelope/)
└── envelope/
    ├── mod.rs                        # Module organization
    └── claim_stats.rs                # Command-specific integration tests
```

---

## Envelope Structure (Current JSON)

All `bf --json` commands emit:

```json
{
  "version": 1,
  "kind": "<command>",
  "data": <command-specific data>,
  "warning": "<optional warning message>"
}
```

---

## Core Helper Functions

### 1. Core Envelope Validation (`tests/envelope_helpers.rs`)

| Function | Purpose |
|----------|---------|
| `validate_envelope_structure()` | Validates version=1, kind matches, data present |
| `validate_metadata_fields()` | Validates version and kind fields only |
| `validate_warning_present()` | Validates warning field contains expected text |
| `validate_no_warning()` | Ensures no warning field present |

### 2. Parsing and Extraction

| Function | Purpose |
|----------|---------|
| `parse_envelope()` | Parse JSON string → serde_json::Value |
| `get_data()` | Extract data field reference |
| `get_kind()` | Extract command kind string |
| `get_version()` | Extract version number |

### 3. Data Type Checkers

| Function | Purpose |
|----------|---------|
| `data_is_array()` | Check if data is array (list, ready, search, etc.) |
| `data_is_object()` | Check if data is object (show, create, claim, stats) |
| `data_is_null()` | Check if data is null |
| `data_is_string()` | Check if data is string |
| `data_is_number()` | Check if data is numeric |
| `data_is_boolean()` | Check if data is boolean |

### 4. Data Type Assertions

| Function | Purpose |
|----------|---------|
| `assert_data_is_array()` | Panic if not array (with context) |
| `assert_data_is_object()` | Panic if not object (with context) |

### 5. Array Helpers

| Function | Purpose |
|----------|---------|
| `data_array_length()` | Get array length |
| `assert_data_array_length()` | Assert array has expected length |
| `assert_data_array_empty()` | Assert array is empty |
| `assert_data_array_non_empty()` | Assert array is non-empty |

### 6. Object Helpers

| Function | Purpose |
|----------|---------|
| `assert_data_object_has_key()` | Assert object has key |
| `assert_data_object_has_value()` | Assert key has expected value |

---

## Integration Test Patterns

All integration tests follow this pattern (see `tests/envelope/claim_stats.rs`):

```rust
fn bf_path() -> String {
    std::env::var("CARGO_BIN_EXE_bf")
        .unwrap_or_else(|_| "./target/debug/bf".to_string())
}

fn init_workspace() -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    let out = Command::new(bf_path())
        .args(["init", "--prefix", "test"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to init workspace");
    // ... validate success
    temp_dir
}

fn run_envelope_command(workspace: &std::path::Path, args: &[&str]) -> serde_json::Value {
    let mut full_args = vec!["--envelope"];
    full_args.extend_from_slice(args);

    let out = Command::new(bf_path())
        .args(&full_args)
        .current_dir(workspace)
        .output()
        .expect("Failed to run bf command");

    serde_json::from_str(&stdout).unwrap()
}

#[test]
fn stats_envelope_has_stable_structure() {
    let ws = init_workspace();
    create_bead(ws.path(), "test");

    let envelope = run_envelope_command(ws.path(), &["stats", "--format", "json"]);

    verify_envelope_structure(&envelope, "stats");
    // ... command-specific validations
}
```

---

## Command Data Shapes

| Command | `data` shape | Empty result |
|---------|-------------|--------------|
| `create` | `{"id": "bf-xxx"}` | N/A (always succeeds) |
| `list` | `[{...}, {...}]` | `[]` |
| `ready` | `[{...}, {...}]` | `[]` |
| `show` | `{...}` | error (not found) |
| `claim` | `{bead_id, assignee, reclaimed}` | `{}` (no bead available) |
| `update` | `{id: "..."}` | N/A |
| `close` | `{id: "..."}` | N/A |
| `stats` | `{total, by_status, ...}` | N/A |
| `search` | `[{...}, {...}]` | `[]` |

---

## Plan for Extending to Text/Toon Formats

### Text Format Envelope (`--format text --envelope`)

**Structure:**
```
=== ENVELOPE ===
version: 1
kind: <command>
warning: <optional>
=== END HEADER ===

<data content>
```

**New Helpers Needed:**
1. `parse_text_envelope()` - Parse text envelope string → (header_map, body_content)
2. `validate_text_envelope_header()` - Validate `=== ENVELOPE ===` delimiters
3. `get_text_envelope_body()` - Extract content after header
4. `assert_text_body_contains()` - Assert body contains expected text
5. `assert_text_body_matches_regex()` - Assert body matches pattern

**Reusable Patterns:**
- `init_workspace()` - Same as JSON
- `run_envelope_command()` - Same with `--format text` instead of `--format json`
- `bf_path()` - Identical
- `create_bead()` - Identical

### Toon Format Envelope (`--format toon --envelope`)

**Structure:**
```
┌────────────────────────────────────┐
│ version: 1                          │
│ kind: <command>                     │
│ warning: <optional>                 │
└────────────────────────────────────┘

<toon-rendered content>
```

**New Helpers Needed:**
1. `parse_toon_envelope()` - Parse toon envelope (handle box-drawing chars)
2. `validate_toon_box_structure()` - Validate `┌─┐` box drawing
3. `get_toon_envelope_content()` - Extract content after box
4. `assert_toon_content_contains()` - Assert toon output contains expected text
5. `assert_toon_box_dimensions()` - Optional: validate box width

**Reusable Patterns:**
- Same workspace/init patterns as JSON
- Same `run_envelope_command()` with `--format toon`

---

## Recommended Implementation Steps

1. **Add format-specific helpers to `envelope_helpers.rs`:**
   - Keep existing JSON helpers unchanged
   - Add text envelope functions
   - Add toon envelope functions
   - All share same validation philosophy (structure → metadata → data)

2. **Create `tests/envelope/text_format.rs` and `tests/envelope/toon_format.rs`:**
   - Follow pattern from `claim_stats.rs`
   - Test each command with `--format text --envelope`
   - Test each command with `--format toon --envelope`

3. **Add coverage tracking to `envelope_coverage.rs`:**
   - Extend existing coverage tests to include text and toon
   - Ensure all commands are tested with all three formats

4. **Create usage examples:**
   - Add `test_envelope_text_helpers_usage.rs`
   - Add `test_envelope_toon_helpers_usage.rs`
   - Follow pattern from `test_envelope_helpers_usage.rs`

---

## Key Design Insights

1. **Separation of Concerns:** Helpers validate structure, integration tests run commands
2. **Modular Organization:** Easy to add new formats without breaking existing tests
3. **Consistent Patterns:** Same init/run/verify pattern across all tests
4. **Context-Rich Errors:** All helpers include context strings for debugging
5. **Type Safety:** Use `Option<&str>` context for optional error messages

---

## Next Steps

This survey provides the foundation for implementing text and toon envelope tests. The existing JSON envelope infrastructure is well-designed and can be extended with minimal changes to support new formats.

**Bead ready to close.**
