# Epic P1 Comprehensive Test Specification

## Overview

This specification defines what "comprehensive" means for Epic P1 (Priority 1 High Priority Epic) testing in bead-forge. Epic P1 refers to issues with `issue_type: epic` and `priority: 1` (P1/HIGH).

**Target Test Coverage: 95%+** for all Epic P1 related code paths in:
- `src/model.rs` (Issue struct, enums, validation)
- `src/storage/sqlite.rs` (CRUD operations, queries)
- `src/jsonl.rs` (import/export)
- `tests/p1_epic_creation.rs` (integration tests)

---

## 1. Test Scenarios

### 1.1 Creation Tests

**Scenario: Create Epic with P1 Priority**
- Create an epic with `issue_type: Epic` and `priority: HIGH` (value 1)
- Verify creation succeeds
- Verify all fields are stored correctly
- Verify ID generation follows bead-forge scheme

**Scenario: Create Epic with All Fields Populated**
- Create epic with every optional field set
- Verify storage preserves all fields
- Verify retrieval returns identical data

**Scenario: Create Multiple P1 Epics**
- Create 10+ epics with P1 priority
- Verify all are stored independently
- Verify no data corruption between epics

### 1.2 Storage Tests

**Scenario: SQLite Persistence**
- Create epic → write to SQLite
- Close and reopen database
- Verify epic is retrievable and identical

**Scenario: Concurrent Creation**
- Create 5 epics simultaneously (simulated workers)
- Verify all are stored without corruption
- Verify no `SQLITE_BUSY` failures

**Scenario: Update Operations**
- Create epic, update various fields
- Verify updates persist correctly
- Verify `updated_at` timestamp changes

### 1.3 Retrieval Tests

**Scenario: Point Lookup by ID**
- Retrieve epic by exact ID
- Verify all fields match
- Verify `get_issue()` returns `Some(Issue)`

**Scenario: List All P1 Epics**
- Create mix of priorities (P0, P1, P2, P3, P4)
- Query for epics with `priority: HIGH`
- Verify only P1 epics returned

**Scenario: Filtered Queries**
- Query with `IssueFilter { priority: Some(1), issue_type: Some(Epic) }`
- Verify results match both criteria
- Test combined filters (priority + status + assignee)

**Scenario: Retrieve Epic with Dependencies**
- Create epic with child tasks
- Retrieve epic with populated dependencies
- Verify `dependencies` vector is complete

### 1.4 Serialization Tests

**Scenario: JSON Serialization (Epic)**
- Serialize epic with P1 to JSON
- Verify `"issue_type": "epic"` is present
- Verify `"priority": 1` is present
- Verify all fields serialize correctly

**Scenario: JSON Deserialization**
- Deserialize JSON epic back to struct
- Verify all fields match original
- Verify `Priority::HIGH` and `IssueType::Epic` parse correctly

**Scenario: Roundtrip Integrity**
- Serialize → deserialize → serialize again
- Compare JSON strings (should be identical)
- Verify no data loss in roundtrip

**Scenario: JSONL Export**
- Create multiple P1 epics
- Run `bf sync --flush`
- Verify `issues.jsonl` contains one line per epic
- Verify JSON format matches br export

**Scenario: JSONL Import**
- Start with empty database
- Import `issues.jsonl` with P1 epics
- Verify all epics restored correctly
- Verify priorities and types preserved

### 1.5 Edge Case Tests

**Scenario: Empty/Minimal Epic**
- Create epic with only required fields (id, title, timestamps)
- Verify serialization skips `None` fields
- Verify empty vectors (`labels`, `dependencies`, `comments`) are skipped

**Scenario: Unicode in Fields**
- Create epic with Unicode characters in title, description (emoji, CJK, Arabic)
- Verify UTF-8 survives roundtrip
- Verify JSON escaping works correctly

**Scenario: Special Characters**
- Use quotes, newlines, backslashes in text fields
- Verify JSON escaping/unescaping
- Verify no injection vulnerabilities

**Scenario: Long Strings**
- Title at 500 char limit
- Description at 10,000+ chars
- Verify storage and retrieval handle long fields

**Scenario: Invalid Priority Value**
- Attempt to create epic with `priority: 5` or `priority: -1`
- Verify validation catches error
- Verify database constraints reject

**Scenario: Priority Boundary Values**
- Test P0 (0), P1 (1), P2 (2), P3 (3), P4 (4)
- Verify ordering: P0 < P1 < P2 < P3 < P4
- Verify display: "P0", "P1", "P2", "P3", "P4"

**Scenario: Custom Status Types**
- Create epic with `Status::Custom("in-review")`
- Verify serialization preserves custom value
- Verify roundtrip maintains custom status

**Scenario: Duplicate Labels**
- Add label "urgent" twice to same epic
- Verify deduplication occurs
- Verify storage normalizes to single instance

**Scenario: Epic with No Children**
- Create orphan epic (no parent-child dependencies)
- Verify it's valid
- Verify `EpicStatus.total_children == 0`

**Scenario: Epic with All Children Closed**
- Create epic with 5 child tasks
- Close all children
- Verify `eligible_for_close == true`
- Verify epic can be closed

**Scenario: Timestamp Edge Cases**
- Set `created_at` and `updated_at` to same value
- Set `closed_at` before `created_at` (should this be allowed?)
- Verify timezone handling (UTC)

---

## 2. Field Coverage Matrix

All fields from the `Issue` struct must be tested in Epic P1 context:

| Field | Test Coverage | Notes |
|-------|---------------|-------|
| `id` | ✓ | ID generation, uniqueness, format |
| `title` | ✓ | Required field, 1-500 chars, Unicode |
| `description` | ✓ | Optional, long text, Markdown |
| `design` | ✓ | Optional, long text |
| `acceptance_criteria` | ✓ | Optional, list format |
| `notes` | ✓ | Optional, freeform text |
| `status` | ✓ | All statuses, transitions, custom |
| `priority` | ✓ | **P1 specifically**, all priorities, ordering |
| `issue_type` | ✓ | **Epic specifically**, all types, custom |
| `assignee` | ✓ | Optional string, worker IDs |
| `owner` | ✓ | Optional string |
| `estimated_minutes` | ✓ | Optional i32, validation |
| `created_at` | ✓ | DateTime<Utc>, auto-set |
| `created_by` | ✓ | Optional string |
| `updated_at` | ✓ | DateTime<Utc>, auto-update |
| `closed_at` | ✓ | Optional DateTime, set on close |
| `close_reason` | ✓ | Optional string |
| `closed_by_session` | ✓ | Optional string |
| `due_at` | ✓ | Optional DateTime |
| `defer_until` | ✓ | Optional DateTime |
| `external_ref` | ✓ | Optional string (e.g., JIRA-123) |
| `source_system` | ✓ | Optional string |
| `source_repo` | ✓ | Optional string, default "." |
| `deleted_at` | ✓ | Optional DateTime (tombstone) |
| `deleted_by` | ✓ | Optional string |
| `delete_reason` | ✓ | Optional string |
| `original_type` | ✓ | Optional string |
| `compaction_level` | ✓ | Optional i32, **serializes as 0 when None** |
| `compacted_at` | ✓ | Optional DateTime |
| `compacted_at_commit` | ✓ | Optional string (commit hash) |
| `original_size` | ✓ | Optional i32 |
| `sender` | ✓ | Optional string |
| `ephemeral` | ✓ | Bool, default false |
| `pinned` | ✓ | Bool, default false |
| `is_template` | ✓ | Bool, default false |
| `labels` | ✓ | Vec<String>, deduplication |
| `dependencies` | ✓ | Vec<Dependency>, all dep types |
| `comments` | ✓ | Vec<Comment>, ordering |
| `annotations` | ✓ | BTreeMap<String, String>, bf-only |

**Total: 40+ fields**

---

## 3. Edge Cases Catalog

### 3.1 String Edge Cases

| Case | Input | Expected Behavior |
|------|-------|------------------|
| Empty string | `""` | Stored as-is, serialized as `""` |
| Whitespace only | `"   "` | Stored as-is, trimmed on display? |
| Single char | `"x"` | Valid, stored |
| Max length | 500 chars for title | Valid, stored |
| Overflow | 501 chars for title | **Reject** with error |
| Newlines | `"line1\nline2"` | Valid, JSON-escaped |
| Tabs | `"col1\tcol2"` | Valid, JSON-escaped |
| Quotes | `"say \"hello\""` | Valid, JSON-escaped |
| Backslashes | `"path\\to\\file"` | Valid, JSON-escaped |
| Null byte | `"\0"` | **Reject** or escape |
| Emoji | `"🚀 🔥"` | Valid UTF-8 |
| CJK | `"中文日本語"` | Valid UTF-8 |
| RTL | `"مرحبا"` | Valid UTF-8 |
| Zero-width | `"a​b"` | Valid, displayed correctly |
| Combining marks | `"é"` | Valid, NFC normalized? |

### 3.2 Numeric Edge Cases

| Case | Input | Expected Behavior |
|------|-------|------------------|
| Zero priority | `0` | Valid (P0 Critical) |
| Min priority | `0` | Valid (P0) |
| Max priority | `4` | Valid (P4 Backlog) |
| Underflow min | `-1` | **Reject** with error |
| Overflow max | `5` | **Reject** with error |
| Negative estimated | `-60` | **Reject** or allow? |
| Zero estimated | `0` | Valid (no estimate) |
| Large estimated | `999999` | Valid (16,666 hours) |

### 3.3 DateTime Edge Cases

| Case | Input | Expected Behavior |
|------|-------|------------------|
| Unix epoch | `1970-01-01T00:00:00Z` | Valid |
| Far future | `2100-01-01T00:00:00Z` | Valid |
| Microseconds | `2026-01-01T12:00:00.123456Z` | Truncate to milliseconds? |
| Monotonic | `created_at == updated_at` | Valid |
| Out of order | `closed_at < created_at` | **Reject** or allow? |
| Timezone | `2026-01-01T12:00:00+05:00` | Convert to UTC |

### 3.4 Enum Edge Cases

| Case | Input | Expected Behavior |
|------|-------|------------------|
| Custom status | `Status::Custom("any-string")` | Valid, roundtrips |
| Custom type | `IssueType::Custom("spike")` | Valid, roundtrips |
| Custom dep type | `DependencyType::Custom("related-to")` | Valid, roundtrips |
| Empty custom | `Custom("")` | Valid? |
| Case insensitive | `"IN_PROGRESS"` | Parses to `InProgress` |
| Invalid value | `"not-a-status"` | **Reject** with error |

### 3.5 Collection Edge Cases

| Case | Input | Expected Behavior |
|------|-------|------------------|
| Empty labels | `[]` | Skipped in serialization |
| Single label | `["urgent"]` | Valid |
| Duplicate labels | `["a", "a", "b"]` | Deduplicate to `["a", "b"]` |
| Many labels | 100+ labels | Valid |
| Empty dependencies | `[]` | Skipped in serialization |
| Circular deps | `A→B→A` | **Reject** or allow? |
| Self dep | `A→A` | **Reject** |
| Empty comments | `[]` | Skipped in serialization |
| Comment ordering | Create C2 then C1 | Return sorted by created_at |

### 3.6 Database Edge Cases

| Case | Scenario | Expected Behavior |
|------|----------|------------------|
| Duplicate ID | Create same ID twice | **Reject** with constraint error |
| Missing ID | Get non-existent ID | Return `Ok(None)` |
| Concurrent write | Two workers update same epic | One wins, one gets `SQLITE_BUSY` |
| Transaction rollback | Create epic, fail mid-tx | No partial state |
| Database corruption | Invalid WAL file | `br doctor --repair` rebuilds |
| Full disk | Cannot write | Return `Err`, no corruption |

---

## 4. Expected Test Coverage

### 4.1 Coverage Targets

| Component | Target Coverage | Rationale |
|-----------|-----------------|------------|
| `src/model.rs` | 95%+ | Core data structures, must be bulletproof |
| `src/storage/sqlite.rs` | 90%+ | Database layer, critical path |
| `src/jsonl.rs` | 90%+ | Import/export, data integrity |
| `tests/p1_epic_creation.rs` | 100% | All test functions must pass |

### 4.2 Coverage Measurement

Run with `tarpaulin`:

```bash
cargo tarpaulin --out Html --output-dir coverage --verbose --timeout 120 --exclude-files 'src/main.rs'
```

**Threshold:** CI fails if Epic P1 coverage drops below 90%.

### 4.3 Missing Coverage Alerts

If any of these paths are untested, the test suite is **incomplete**:

- [ ] Epic creation with ALL optional fields populated
- [ ] Epic update operations (status transitions, priority changes)
- [ ] Epic deletion (tombstone status)
- [ ] JSONL import/export roundtrip
- [ ] Concurrent claim operations
- [ ] Dependency relationship management
- [ ] Label add/remove operations
- [ ] Comment add operations
- [ ] Annotation CRUD operations
- [ ] Filter queries (priority, type, status, assignee, labels)

---

## 5. Test Organization

### 5.1 Unit Tests (in `src/model.rs`)

- Field serialization/deserialization
- Enum roundtrips
- `sync_equals()` comparisons
- `is_expired_tombstone()` logic

### 5.2 Integration Tests (in `tests/`)

- `tests/p1_epic_creation.rs` - Core Epic P1 tests
- `tests/epic_comprehensive.rs` - Epic type and relationships
- `tests/storage_epic.rs` - SQLite persistence (TODO)
- `tests/jsonl_epic.rs` - Import/export (TODO)

### 5.3 Property-Based Tests (future)

Use `proptest` crate:
- Generate random epics, verify roundtrip integrity
- Generate random strings, verify UTF-8 handling
- Generate random label sets, verify deduplication

---

## 6. Acceptance Criteria

A test suite is **comprehensive** when:

1. ✅ All 40+ Issue fields are tested in Epic P1 context
2. ✅ All edge cases from §3 are covered
3. ✅ Coverage targets (§4) are met
4. ✅ All tests pass consistently
5. ✅ No flaky tests (random failures)
6. ✅ Tests run in <10 seconds total
7. ✅ JSONL roundtrip produces identical bead
8. ✅ Database integrity verified after each test
9. ✅ Error paths tested (invalid inputs, constraints)
10. ✅ Documentation updated with any new test patterns

---

## 7. Test Execution

### 7.1 Run All Tests

```bash
# Standard test run
cargo test

# With output
cargo test -- --nocapture --test-threads=1

# Only Epic P1 tests
cargo test --test p1_epic_creation
```

### 7.2 Run with Coverage

```bash
cargo tarpaulin --exclude-files 'src/main.rs' --verbose
```

### 7.3 CI Integration

```yaml
# .github/workflows/test.yml (example)
- name: Run tests
  run: cargo test --verbose

- name: Check coverage
  run: |
    cargo tarpaulin --out Json --output-dir coverage
    # Fail if coverage < 90%
```

---

## 8. Maintenance

### 8.1 When to Update This Spec

- Adding new Issue fields
- Changing Epic/P1 validation rules
- Adding new edge cases
- Coverage targets change
- New test files created

### 8.2 Version History

| Date | Version | Changes |
|------|---------|---------|
| 2026-07-06 | 1.0 | Initial specification |

---

## 9. References

- `src/model.rs` - Core data structures
- `tests/p1_epic_creation.rs` - Existing P1 tests
- `tests/epic_comprehensive.rs` - Epic relationship tests
- `docs/plan/plan.md` - Implementation plan
- `docs/README.md` - User-facing documentation

---

## Appendix A: Quick Reference

### Minimal Epic P1 Test

```rust
let epic = Issue {
    id: "bf-test".to_string(),
    title: "Test Epic P1".to_string(),
    issue_type: IssueType::Epic,
    priority: Priority::HIGH, // P1
    ..Default::default()
};
storage.create_issue(&epic).unwrap();
```

### Full Epic P1 Test

```rust
let epic = Issue {
    id: "bf-full".to_string(),
    title: "Full Epic P1".to_string(),
    description: Some("Full description".to_string()),
    design: Some("Design notes".to_string()),
    acceptance_criteria: Some("- [ ] Test 1\n- [ ] Test 2".to_string()),
    notes: Some("Additional notes".to_string()),
    status: Status::Open,
    priority: Priority::HIGH, // P1
    issue_type: IssueType::Epic,
    assignee: Some("worker-1".to_string()),
    labels: vec!["epic".to_string(), "p1".to_string()],
    ..Default::default()
};
storage.create_issue(&epic).unwrap();
```

### JSON Roundtrip Test

```rust
let json = serde_json::to_string(&epic).unwrap();
let roundtrip: Issue = serde_json::from_str(&json).unwrap();
assert_eq!(epic, roundtrip);
```

---

**End of Specification**
