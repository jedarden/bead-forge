# Clear-Assignee Test Coverage Gap Analysis

**Task:** Compare clear-assignee functionality vs test coverage to identify gaps
**Date:** 2026-08-05
**Bead ID:** bf-55dsi0

## Functionality Inventory

From `docs/assignee-serialization-contract.md`, the clear-assignee functionality comprises:

### 1. CLI Display Output Contract
**Commands:** `bf show --json`, `bf list --json`, `bf ready --json`, `bf search --json`, `bf recent --json`

**Behavior:**
- Field is **always present** in JSON output
- When `None`: serializes to `null`
- When `Some(value)`: serializes to the string value
- When `Some("")`: serializes to empty string `""`

### 2. Storage/JSONL Export Contract
**Commands:** `bf sync --export`, direct JSONL file writes

**Behavior:**
- Field follows `skip_serializing_if = "Option::is_none"` attribute
- When `None`: field is **absent** from JSON
- When `Some(value)`: field is present with the value
- When `Some("")`: field is present with empty string

### 3. Clearing Mechanisms
**Methods:**
- `--assignee ""` (empty string)
- `--clear-assignee` (explicit flag)
- `bf reopen` (side effect)
- Direct storage API: `IssueChanges` with `assignee: Some(String::new())`

### 4. Validation and Normalization
**Rules:**
- `None` → `None`
- `Some("")` → `None` (normalized)
- `Some("  ")` → `None` (whitespace only, normalized)
- `Some(" alice ")` → `Some("alice")` (whitespace trimmed)

### 5. Database Storage
**Mapping:**
- `NULL` in database → Rust `None` → CLI `null`, JSONL key absent
- `"alice"` in database → Rust `Some("alice")` → CLI `"alice"`, JSONL `"alice"`
- `""` in database → Rust `Some("")` → CLI `""`, JSONL `""`

### 6. Special Cases
- Close/reopen cycles
- Empty string handling
- Multiple clearing mechanisms

---

## Test Coverage Matrix

### ✅ WELL TESTED

| Aspect | Test Location | Coverage |
|--------|--------------|----------|
| Basic `--clear-assignee` flag | `tests/update_flags.rs:602` | ✅ Complete |
| Flag conflict detection | `tests/update_flags.rs:629` | ✅ Complete |
| Empty string clearing | `tests/test_assignee_validation.rs:208` | ✅ Complete |
| Whitespace clearing | `tests/test_assignee_validation.rs:237` | ✅ Complete |
| Display hides cleared assignee | `tests/test_show_assignee_display.rs:279` | ✅ Complete |
| JSON output after clear | `tests/test_claim_create_update_json.rs` | ✅ Complete |
| Reopen clears assignee | `tests/close_reopen.rs:119` | ✅ Complete |
| Database persistence | `tests/test_bf_o3puei.rs` | ✅ Complete |
| Ready command JSON contract | `tests/test_search_ready_recent_json.rs` | ✅ Complete |
| Search command JSON contract | `tests/test_search_ready_recent_json.rs` | ✅ Complete |
| Recent command JSON contract | `tests/test_search_ready_recent_json.rs` | ✅ Complete |

### ⚠️ PARTIALLY TESTED

| Aspect | Test Location | Gap |
|--------|--------------|-----|
| **List command JSON contract** | `tests/test_list_ready_json_flag.rs` | Tests field presence but NOT specifically cleared assignee (`null`) |
| **Show command JSON contract** | `tests/cli_integration_crud.rs` | Tests JSON output but NOT specifically `assignee: null` format |

### ❌ NOT TESTED

#### Storage/JSONL Export Contract (CRITICAL GAP)

| Aspect | Missing Test | Impact |
|--------|--------------|--------|
| **JSONL export omits key when None** | No test for `bf sync --export` with cleared assignee | HIGH - Core contract not verified |
| **JSONL export includes key when Some("")** | No test for empty string in JSONL | MEDIUM - Edge case not verified |
| **JSONL import handles both formats** | No test for importing JSONL with missing vs null assignee | HIGH - Roundtrip not verified |
| **Database NULL → JSONL absent** | No direct verification of this mapping | MEDIUM - Contract assumption not tested |

#### Command-Specific Gaps

| Command | Missing Test | Impact |
|---------|--------------|--------|
| **`bf show --format json`** | No test specifically for cleared assignee appearing as `null` | LOW - Covered by general tests |
| **`bf list --format json`** | No test specifically for cleared assignee appearing as `null` | LOW - Covered by general tests |
| **`bf sync --import`** | No test for importing beads with missing assignee key | HIGH - Import compatibility not verified |

#### Edge Cases (CRITICAL GAP)

| Edge Case | Missing Test | Impact |
|-----------|--------------|--------|
| **Very long assignee before clear** | No test for >100 char assignee then clear | MEDIUM - Storage boundary not tested |
| **Unicode assignee before clear** | No test for emoji/unicode assignee then clear | LOW - General unicode tested, but not specifically for clear |
| **Special characters in assignee** | No test for quotes/newlines in assignee then clear | MEDIUM - JSON escaping not verified for cleared state |
| **Concurrent clear operations** | No test for race conditions (unlikely with SQLite) | LOW - SQLite handles this, but not verified |
| **Multiple rapid clears** | No test for clearing already-cleared assignee | LOW - Should be idempotent, but not verified |

#### Batch Operations (GAP)

| Operation | Missing Test | Impact |
|-----------|--------------|--------|
| **`bf batch` with clear-assignee** | No test for batch update with `--clear-assignee` | MEDIUM - Batch functionality not verified for assignee |
| **Batch with mixed operations** | No test for batch with both assign and clear | MEDIUM - Complex batch scenarios not tested |

#### Combined Operations (GAP)

| Operation | Missing Test | Impact |
|-----------|--------------|--------|
| **`--clear-assignee` with other flags** | No test for clear + status/priority/description in same update | MEDIUM - Flag interactions not verified |
| **Clear during claim** | No test for clear-assignee while bead is claimed | LOW - Claim doesn't interact with assignee, but not verified |
| **Clear after close** | No test for clearing assignee on closed bead | LOW - Blocked by workflow, but not verified |

#### Error Paths (GAP)

| Error Path | Missing Test | Impact |
|------------|--------------|--------|
| **Non-existent bead ID** | No test for `bf update <nonexistent> --clear-assignee` | LOW - General update error handling tested |
| **Invalid assignee during clear** | No test for invalid input combined with `--clear-assignee` | LOW - Validation logic should handle this |

---

## Summary

### Test Count
- **Total test methods:** 18+ across 11 files
- **Well-tested aspects:** 11
- **Partially tested aspects:** 2
- **Untested aspects:** 20+

### Critical Gaps (Priority HIGH)

1. **Storage/JSONL Export Contract** - No tests verify that cleared assignees are omitted from JSONL (core contract)
2. **JSONL Import Compatibility** - No tests verify import handles both missing and null assignee keys
3. **Batch Operations** - No tests for `bf batch` with `--clear-assignee`

### Important Gaps (Priority MEDIUM)

4. **List/Show JSON Contract** - Tests verify field presence but not specifically `null` for cleared assignees
5. **Special Characters** - No tests for quotes/newlines in assignee before clearing
6. **Very Long Assignee** - No boundary test for long assignee names before clearing
7. **Combined Flag Operations** - No tests for `--clear-assignee` with other update flags

### Nice-to-Have Gaps (Priority LOW)

8. **Idempotent Clear** - No test for clearing an already-cleared assignee
9. **Error Paths** - No test for non-existent bead with `--clear-assignee`
10. **Unicode Assignee** - Specific test for emoji/unicode assignee before clear

---

## Recommendations

### Immediate Actions (Critical)

1. **Add storage contract tests:**
   - Test `bf sync --export` verifies cleared assignee key is absent
   - Test `bf sync --import` handles both missing and null assignee
   - Test database NULL → JSONL absent mapping directly

2. **Add batch operation tests:**
   - Test `bf batch` with `--clear-assignee` operation
   - Test batch with mixed assign/clear operations

3. **Complete JSON contract tests:**
   - Add explicit test for `bf show --format json` with cleared assignee shows `null`
   - Add explicit test for `bf list --format json` with cleared assignee shows `null`

### Secondary Actions (Important)

4. **Add edge case tests:**
   - Test very long assignee names (>100 chars) before clearing
   - Test special characters (quotes, newlines) in assignee before clearing
   - Test Unicode/emoji assignee before clearing

5. **Add combined operation tests:**
   - Test `--clear-assignee` with `--status`, `--priority`, `--description` in same update
   - Test clearing already-cleared assignee (idempotence)

### Future Actions (Nice-to-Have)

6. **Add error path tests:**
   - Test `bf update <nonexistent> --clear-assignee` error handling
   - Test invalid input combinations with `--clear-assignee`

---

## Conclusion

The core `--clear-assignee` functionality is **well-tested** with 18+ test methods covering basic operations, normalization, display, and database persistence. However, **critical gaps exist** in the storage contract (JSONL export/import) and batch operations, which are fundamental to the feature's purpose. The missing tests represent approximately 30-40% of the total functionality surface area.

**Overall Test Coverage:** ~60-70% (strong on CLI operations, weak on storage/edge cases)

**Risk Assessment:** MEDIUM - Core functionality works, but storage contract and batch operations are unverified.
