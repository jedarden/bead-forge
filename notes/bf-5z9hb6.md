# Analysis of incremental_flush() Implementation

## Task Summary
Read and analyze the current `incremental_flush()` function structure in `src/jsonl.rs`.

## Key Findings

### Function Signature
```rust
pub fn incremental_flush(storage: &crate::storage::sqlite::Storage, path: &Path) -> Result<FlushResult>
```

### Return Type
`Result<FlushResult>` where `FlushResult` contains:
- `flushed: usize` - number of beads successfully flushed
- `warnings: Vec<String>` - warnings about failures

### Implementation Structure (lines 241-299)

1. **Query dirty IDs** - Uses `storage.query_dirty_issues()`
2. **Early return** - No-op if nothing dirty (returns empty result)
3. **List dirty issues closure** - SQL query with INNER JOIN to dirty_issues, LEFT JOIN to bead_labels
4. **Clear dirty marks closure** - DELETE from dirty_issues table
5. **Execute export** - Calls `export_jsonl_dirty(path, list_dirty, clear_dirty)`
6. **Return result** - Constructs FlushResult with count and empty warnings

### Key Design Points

- **Surgical line replacement** via `export_jsonl_merge()` - only rewrites dirty beads
- **Byte-for-byte preservation** of untouched beads in JSONL file
- **Transactional safety** - clear_dirty only runs after successful write
- **Atomic write pattern** - temp file + rename in `export_jsonl_merge`
- **Orphan preservation** - unparseable lines kept and appended

### Supporting Functions
- `export_jsonl_dirty` (202-219) - Orchestrates list/call pattern
- `export_jsonl_merge` (118-183) - Core surgical merge with BTreeMap
- `get_dirty_issue_ids` (225-237) - Utility for querying dirty IDs

### Test Coverage
Comprehensive tests at lines 1653-2194 covering:
- Basic success cases
- No-op when nothing dirty
- Surgical replacement verification
- Multiple dirty beads
- Failure handling
- Related data (labels, dependencies, comments, events)
- Orphan line preservation

## Acceptance Criteria Met
- ✅ Current incremental_flush() implementation understood
- ✅ Key code locations identified (function, supporting functions, tests)
- ✅ No code changes made (read-only analysis)
