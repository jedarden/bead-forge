# bf-x7hy5c — test-label-export

**Task:** Verify that bead labels export correctly to `issues.jsonl`.

## Result: VERIFIED ✅

Label export to JSONL works correctly across unit tests, the real test bead, and an
end-to-end CLI round-trip. No source changes were required — the feature is already
implemented and tested in `src/jsonl.rs`.

## How labels are stored

Labels are NOT a column on `issues` (adding one would trip br's
`issues_column_order_matches()` and trigger a destructive `rebuild_issues_table()`,
per CLAUDE.md). They live in two parallel tables:

- `labels` (issue_id, label) — br-compatible
- `bead_labels` (bead_id, label) — bf annotation table

Both are populated identically. At export time, labels are attached to the `Issue`
struct and serialized as a JSON array via `serde` (`skip_serializing_if` empty).

## Verification performed

### 1. Unit tests (src/jsonl.rs) — all pass
`cargo test --lib jsonl::` → **43 passed, 0 failed**, including the 5 label-specific tests:

- `labels_are_exported_to_jsonl` — labels survive merge export
- `labels_roundtrip_through_jsonl` — labels survive full export + parse
- `empty_labels_array_skipped_in_jsonl` — no labels ⇒ `"labels"` key omitted
- `debug_label_export_import` — labels survive merge export (auto-flush path)
- `import_jsonl_with_extra_fields` — labels parsed on import

### 2. Real test bead (bf-x7hy5c itself)
The bead carries labels `critical, phase-1, storage`. In the production
`issues.jsonl` it is exported as:
```json
"labels": ["critical", "phase-1", "storage"]
```
Matching the `labels` + `bead_labels` tables in the live DB.

### 3. End-to-end CLI round-trip (fresh /tmp workspace)
```
bf create --title "End-to-end label test" --type task \
  --label phase-2 --label critical --label storage
bf sync --flush-only
```
→ JSONL contains `"labels": ["critical", "phase-2", "storage"]`. ✅

### 4. Code review: from-empty-file path
`export_jsonl_merge` (src/jsonl.rs:107) correctly creates the JSONL from scratch when
given upserts but no pre-existing file — the empty-file no-op guard (line 115) only
fires when there is nothing to write.

## Note (side observation, not a defect)
On a freshly `bf init`'d workspace, `bf create` does not bootstrap `issues.jsonl`
until the first explicit `bf sync --flush-only`. The auto-flush dirty-merge path needs
an active checkpoint to merge into; a brand-new workspace has none. In the production
workspace (checkpoint pre-existing) auto-flush exports labels on every mutation as
documented. This is unrelated to label serialization and is by design.
