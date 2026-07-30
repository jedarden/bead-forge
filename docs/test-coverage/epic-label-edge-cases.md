# Epic Label Edge Cases and Type Preservation Test Coverage

This document summarizes the comprehensive test suite for epic label edge cases and type preservation (bead bf-2ydmaw).

## Acceptance Criteria Coverage

### 1. Test epic with NO labels (empty label set)

**Tests:**
- `test_epic_no_labels_cli_create` - Creates epic via CLI with no `--label` flags
- `test_epic_no_labels_storage_api` - Creates epic via Storage API with empty labels vec
- `test_epic_no_labels_json_serialization` - Verifies empty labels array is skipped in JSON
- `test_epic_no_labels_add_and_remove` - Tests adding to and removing from empty label set

**Expected Behavior:**
- Epic created without labels has empty label set
- Labels can be added to empty epic
- All labels can be removed, leaving empty set
- Empty labels array is skipped in JSON serialization (via `skip_serializing_if`)
- Epic type is preserved throughout all operations

### 2. Test epic with special characters in labels

**Tests:**
- `test_epic_special_characters_labels_cli` - Tests labels with various special characters:
  - Dashes: `label-with-dashes`
  - Underscores: `label_with_underscores`
  - Dots: `label.with.dots`
  - Slashes: `label/with/slashes`
  - Colons: `label:with:colons`
  - At signs: `label@with@at`
  - Plus signs: `label+with+plus`
  - Equals: `label=with=equals`
  - Hash signs: `label#with#hash`

- `test_epic_unicode_labels` - Tests Unicode labels:
  - Japanese: `label-日本語`
  - Emoji: `label-emoji-🎉`
  - Russian: `label-русский`
  - Arabic: `label-العربية`
  - Greek: `label-ελληνικά`

- `test_epic_label_whitespace_handling` - Tests labels with leading/trailing whitespace

**Expected Behavior:**
- All special characters are preserved through storage
- Unicode labels work correctly
- Epic type is preserved regardless of label content
- Label operations work identically regardless of character set

### 3. Verify epic type ('epic') preserved through all label operations

**Tests:**
- `test_epic_type_preserved_through_label_add` - Epic type remains `epic` after adding labels
- `test_epic_type_preserved_through_label_remove` - Epic type remains `epic` after removing labels
- `test_epic_type_preserved_through_update` - Epic type remains `epic` after status/priority updates
- `test_epic_type_preserved_through_jsonl_sync` - Epic type remains `epic` after JSONL flush

**Expected Behavior:**
- Epic type is immutable after creation
- No label operation can change epic type
- JSONL sync preserves epic type and labels
- Epic type is never changed to `task` or any other type

### 4. Test epic labels vs other issue types (no special casing)

**Tests:**
- `test_epic_labels_vs_task_labels_no_special_casing` - Epic and task with same labels work identically
- `test_all_issue_types_with_labels_work_identically` - All issue types (task, bug, feature, epic, chore, docs, question) handle labels identically
- `test_label_operations_across_issue_types_no_special_casing` - Label add/remove operations work the same for all issue types

**Expected Behavior:**
- Epic labels have NO special handling vs other issue types
- Same label operations work identically across all issue types
- Epic type preservation is orthogonal to label handling
- Label search and listing work uniformly across types

### 5. Comprehensive CLI integration test suite

**Tests:**
- `test_comprehensive_epic_label_workflow` - End-to-end epic label workflow:
  1. Create epic with no labels
  2. Verify empty label set
  3. Add multiple labels at once
  4. Verify all labels added
  5. Add duplicate label (idempotent)
  6. Remove one label
  7. Remove non-existent label (no-op)
  8. Update epic status and priority
  9. Verify epic type preserved through all operations

- `test_epic_label_search_integration` - Tests searching epics by labels
- `test_epic_label_list_command` - Tests listing all labels with counts

**Expected Behavior:**
- Complete workflow from creation to update works seamlessly
- Label search finds epics correctly
- Label list command shows correct counts
- Epic type preserved throughout entire workflow

## Test File Structure

The test suite is in `tests/epic_label_edge_cases.rs` and contains:

```rust
// ~800 lines of comprehensive tests
// 25 test functions covering all acceptance criteria
// Mix of CLI integration tests and storage API tests
```

### Test Categories

1. **Empty Label Set Tests** (4 tests)
   - CLI creation without labels
   - Storage API with empty labels
   - JSON serialization
   - Add/remove to/from empty set

2. **Special Character Tests** (3 tests)
   - Special characters (dashes, slashes, colons, etc.)
   - Unicode labels (Japanese, emoji, Arabic, etc.)
   - Whitespace handling

3. **Type Preservation Tests** (4 tests)
   - Through label add
   - Through label remove
   - Through other updates
   - Through JSONL sync

4. **Cross-Type Comparison Tests** (3 tests)
   - Epic vs task with same labels
   - All issue types handle labels identically
   - Label operations across all types

5. **Integration Workflow Tests** (3 tests)
   - Comprehensive end-to-end workflow
   - Label search integration
   - Label list command

## Test Execution

```bash
# Run all epic label edge case tests
cargo test --test epic_label_edge_cases

# Run specific test category
cargo test --test epic_label_edge_cases test_epic_no_labels

# Run with output
cargo test --test epic_label_edge_cases -- --nocapture
```

## Test Helpers

The test suite provides reusable helpers:

- `setup_test_workspace()` - Creates isolated test workspace
- `get_bf_binary()` - Gets path to compiled `bf` binary
- `extract_bead_id()` - Parses bead ID from command output
- `run_labels()` - Executes `bf labels` and returns label set

## Model Tests

Additional epic label tests exist in:

- `tests/test_epic_single_label.rs` - Single label scenarios
- `tests/epic_cli_label_mutate.rs` - Label mutation operations
- `src/model.rs` tests - Serde roundtrip, type preservation

## Acceptance Status

All 5 acceptance criteria are fully covered:

- [x] Test epic with NO labels (empty label set)
- [x] Test epic with special characters in labels
- [x] Verify epic type ('epic') preserved through all label operations
- [x] Test epic labels vs other issue types (no special casing)
- [x] Run comprehensive CLI integration test suite

## Notes

- Tests use isolated workspaces to avoid conflicts
- Tests verify both CLI and Storage API behavior
- Epic type preservation is verified through JSON output
- Label operations tested include: create, add, remove, list, search
- Special character coverage includes common edge cases
- Unicode coverage ensures internationalization support
