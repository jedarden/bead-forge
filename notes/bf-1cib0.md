# Envelope Test Survey (bf-1cib0)

## Summary
Surveyed all envelope test modules and their current status as of 2026-07-23.

## Compilation Status
✅ **No compilation errors** - `cargo build` completes cleanly

## Test Modules

### 1. `tests/envelope/` - Organized envelope test directory

#### `tests/envelope/mod.rs`
- Declares 3 submodules: `claim_stats`, `text_format`, `toon_format`
- No `non_json` module exists (contrary to bead bf-16y4t expectations)

#### `tests/envelope/claim_stats.rs` 
**Status: ✅ PASSING**
- Tests for `stats` and `claim` commands with JSON envelope wrapping
- Tests verify stable envelope structure with `{version, kind, data}` fields
- All tests passing

#### `tests/envelope/text_format.rs`
**Status: ✅ PASSING (19 tests)**
- Tests that `--format text` ignores envelope wrapping and outputs plain text
- Covers: `stats`, `claim`, `list`, `ready`, `show` commands
- Tests verify text format is same with and without `--envelope` flag
- All tests passing

#### `tests/envelope/toon_format.rs`
**Status: ✅ PASSING (18 tests)**
- Tests that `--format toon` ignores envelope wrapping and outputs plain text  
- Covers: `stats`, `claim`, `list`, `ready`, `show` commands
- Tests verify toon format is same with and without `--envelope` flag
- All tests passing

**Total for envelope/ module: 53 passing tests**

### 2. `tests/envelope_coverage.rs`
**Status: ❌ 7 FAILING tests (out of 41 total)**

#### Passing tests (34):
- ✅ `envelope_claim_*` (claim command envelope tests)
- ✅ `envelope_list_*` (list command envelope tests)  
- ✅ `envelope_ready_*` (ready command envelope tests)
- ✅ `envelope_show_*` (show command envelope tests)
- ✅ `envelope_stats_*` (stats command envelope tests)
- ✅ `envelope_velocity_*` (velocity command envelope tests)

#### Failing tests (7):
- ❌ `envelope_batch_command_has_stable_structure` - "data must be an array"
- ❌ `envelope_batch_empty_emits_empty_array` - "data must be an array"
- ❌ `envelope_create_command_has_stable_structure` - "data must be an array"
- ❌ `envelope_recent_command_has_stable_structure` - "data must be an array"
- ❌ `envelope_recent_empty_emits_empty_array` - "data must be an array"
- ❌ `envelope_search_command_has_stable_structure` - "data must be an array"
- ❌ `envelope_search_empty_emits_empty_array` - "data must be an array"

**Issue:** These 7 commands (`batch`, `create`, `recent`, `search`) appear to not properly implement envelope wrapping. The tests expect the envelope `data` field to contain an array, but it's not in array format.

### 3. Other envelope test files

#### `tests/envelope_integration_tests.rs`
- Minimal file that just declares `mod envelope;`
- This is the entry point for tests in the `tests/envelope/` directory

#### `tests/envelope_helpers.rs`
- Helper utilities for envelope testing

#### `tests/test_envelope_helpers_usage.rs`
- Tests for the helper utilities

## Test Structure by Format

### JSON Format (envelope wrapping expected)
- **Commands with working envelope support:**
  - `claim` ✅
  - `list` ✅
  - `ready` ✅
  - `show` ✅
  - `stats` ✅
  - `velocity` ✅

- **Commands with broken/incomplete envelope support:**
  - `batch` ❌
  - `create` ❌
  - `recent` ❌
  - `search` ❌

### Text Format (envelope ignored, plain text output)
- All commands tested ✅
- `stats`, `claim`, `list`, `ready`, `show` all properly ignore `--envelope` flag

### Toon Format (envelope ignored, plain text output)
- All commands tested ✅
- `stats`, `claim`, `list`, `ready`, `show` all properly ignore `--envelope` flag

## Key Finding: `envelope::non_json` vs actual module structure

Bead `bf-16y4t` references tests at `envelope::non_json`, but:
- ❌ No `envelope::non_json` module exists
- ✅ Tests are actually in `envelope::text_format` and `envelope::toon_format`
- ✅ The command `cargo test envelope::non_json` will NOT work
- ✅ The correct commands are:
  - `cargo test envelope::text_format`
  - `cargo test envelope::toon_format`

## Issues Identified

1. **bf-16y4t acceptance criteria issue**: Bead expects `cargo test envelope::non_json` but that module doesn't exist
2. **7 commands with incomplete envelope support**: `batch`, `create`, `recent`, `search` don't properly wrap their output in envelope format
3. **Inconsistent envelope implementation**: Some commands properly wrap array data in envelope `data` field, others don't

## Recommendations

1. Rename bead bf-16y4t's test reference from `envelope::non_json` to specific modules
2. Fix envelope support for the 7 failing commands to properly wrap array outputs
3. Consider adding envelope::non_json as an alias module if backward compatibility needed
