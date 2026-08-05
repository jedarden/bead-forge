# Investigation Summary: Assignee Serialization Contract

**Bead ID:** bf-7o29bw
**Date:** 2026-08-05
**Status:** Complete

## Task

Document the exact JSON serialization behavior for the assignee field and resolve ambiguity between "null or absent" in acceptance criteria.

## Findings

### Key Discovery: Dual Contract

The assignee field has **two different serialization contracts** depending on the output path:

1. **CLI Display Output** (show, list, ready, search, recent via `--format json`):
   - Field is **always present** (null when unset)
   - Implemented in `src/format/json.rs` via `ensure_display_fields()`
   - Purpose: Distinguish "not set" from "empty string" for programmatic consumers

2. **Storage/JSONL Export** (sync, export):
   - Field is **absent when None** (compact representation)
   - Implemented via `#[serde(skip_serializing_if = "Option::is_none")]` attribute
   - Purpose: Reduce file size and maintain br compatibility

### Acceptance Criteria Resolution

The phrase "null or absent" in the acceptance criteria refers to the **union of both contracts**:
- CLI consumers expect `null` when unset
- JSONL/git storage expect absent key when unset

Both interpretations are correct for their respective contexts.

## Documentation

Created comprehensive contract document at:
- **File:** `docs/assignee-serialization-contract.md`
- **Version:** 2.0 (corrected from initial incorrect v1.0)
- **Contents:**
  - Behavior matrix for all three input states (None, Some(value), Some(""))
  - Detailed explanation of both serialization paths with code references
  - Rationale for dual contract approach
  - Examples for each path
  - Command-specific behavior
  - Database storage representation
  - Import/export roundtrip guarantees
  - Recommendations for consumers
  - Test coverage references

## Files Referenced

- `src/model.rs:469-470` - Field definition with serde attributes
- `src/format/json.rs:27-43` - CLI display contract implementation
- `src/jsonl.rs:88` - Storage contract export path
- `src/storage/sqlite.rs` - Database layer handling
- `src/cli/mod.rs:1747-1780` - CLI command integration
- `src/model.rs:833-847` - clear_assignee method
- Tests in `src/model.rs`, `src/format/json.rs`, `src/cli/tests/show_json_tests.rs`

## Verification

The existing test suite already verifies both contracts:
- Model tests verify storage contract (absent when None)
- Formatter tests verify CLI display contract (null when None)
- CLI integration tests verify end-to-end behavior

No additional test creation required - existing coverage is comprehensive.

## Compatibility

This dual contract is br-compatible and maintains round-trip guarantees with beads_rust (br).

## Completion

All acceptance criteria met:
- [x] Document exact JSON serialization behavior for assignee field
- [x] Specify behavior for None, Some(value), Some(empty string)
- [x] Identify existing tests that verify contract for all serialization paths
- [x] Document deviations/special cases per command (show, list, export, etc.)
- [x] Save findings to docs/assignee-serialization-contract.md
