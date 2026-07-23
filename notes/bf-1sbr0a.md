# bf-1sbr0a: Labels Export to JSONL Verification

## Finding
Labels export to JSONL during sync flush is **already fully implemented** and working correctly.

## Implementation Details

### Storage Layer (src/storage/sqlite.rs)
- `list_all_issues()` uses `GROUP_CONCAT(bl.label) AS labels` with LEFT JOIN to bead_labels
- `list_dirty_issues()` uses the same pattern for incremental exports
- `row_to_issue_conn()` parses the GROUP_CONCAT result into `labels: Vec<String>`

### Model Layer (src/model.rs)
- `Issue` struct has `labels: Vec<String>` field with proper serde serialization
- `#[serde(skip_serializing_if = "Vec::is_empty", default)]` ensures empty arrays are skipped

### Export Flow
1. `flush()` calls `storage.list_all_issues()` → queries labels
2. `flush_dirty()` calls `storage.list_dirty_issues()` → queries labels
3. `export_jsonl()` serializes the full Issue struct including labels
4. Labels are written to JSONL in standard array format: `"labels":["label1","label2"]`

## Verification Test
```
Created bead: bf-1bm
Labels: critical, phase-1, storage
Flushed 1 beads to JSONL
JSONL output: {"labels":["critical","phase-1","storage"]}
```

## Acceptance Criteria (All Met)
- ✓ Labels queried from bead_labels table during export
- ✓ Labels included in JSONL output format for each bead
- ✓ All labels exported (no truncation)
- ✓ Export atomic with rest of bead data
- ✓ JSONL format backward-compatible with br

## Related Commits
- c6800c5 docs(bf-3ou3re): Verify labels export to JSONL is already implemented
- 27944bd test(bf-4wmjb2): Add label export/import round-trip verification
- 44f8def feat(bf-49v3i6): Query labels from bead_labels table during JSONL export
- ce84ead feat(bf-3l64k2): Add labels field to Issue JSON serialization
