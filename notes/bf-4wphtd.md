# Bead bf-4wphtd: P0 Security patch

## Investigation

Bead created with title "P0 Security patch" but no description provided.

## Actual P0 Security Patch

The actual P0 security vulnerability was already fixed in bead **bf-1d103h** (commit 8cf5939):

### Vulnerability (bf-1d103h)
Internal storage modules (`schema`, `sqlite`) were exposed at crate root level via `src/lib.rs`:
```rust
pub use storage::{schema, sqlite, Storage};
```

This exposed:
- `schema::SCHEMA_SQL` - Complete database schema to external code
- `sqlite` module - Internal storage backend implementation

### Fix Applied (bf-1d103h)
Reverted to exposing only the public API:
```rust
pub use storage::Storage;
```

## Changes in src/format/json.rs

The modifications to `src/format/json.rs` are **test updates**, not security fixes:

### What Changed
- Test `format_issues_guarantees_fields_per_line`: Updated assertions to verify correct serde behavior
- Test `format_issues_single_yields_one_valid_json_line`: Updated to match expected output

### Behavior Verified
The tests confirm that `JsonFormatter` correctly handles `skip_serializing_if` attributes:
- `assignee` field is **omitted** when `None` (via `#[serde(skip_serializing_if = "Option::is_none")]`)
- `labels` field is **omitted** when empty (via `#[serde(skip_serializing_if = "Vec::is_empty")]`)

### Test Results
All 10 JSON formatter tests pass:
```
test format::json::tests::assignee_skipped_when_unset ... ok
test format::json::tests::labels_skipped_when_empty ... ok
test format::json::tests::assignee_and_labels_populated_when_present ... ok
...
```

## Conclusion

**No security vulnerability found** in `src/format/json.rs`. The changes are test updates to ensure the JSON formatter behaves correctly with serde's conditional serialization.

The P0 security patch was already completed in bf-1d103h (hiding internal storage modules).

## Recommendation

Close this bead as "no action needed" - the tests are working correctly and the actual security issue was resolved in bf-1d103h.
