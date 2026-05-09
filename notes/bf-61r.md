# bf-61r: Implement bf schema command

## Status: Already Implemented

The `bf schema` command was already fully implemented in `src/cli/mod.rs` (lines 1761-1809).

### Implementation Details

1. **No argument (or "all")**: Prints the SQLite schema DDL for all bf tables
   - Uses `crate::storage::schema::SCHEMA_SQL` constant
   - Outputs in JSON or text format based on `--format` flag

2. **With bead ID**: Prints the bead's full JSON representation including annotations
   - Loads the issue from SQLite or archives
   - Loads annotations via `storage.get_annotations(bead_id)`
   - Outputs pretty-printed JSON

### Verification

```bash
# Show full schema
bf schema

# Show specific bead with annotations
bf schema bf-5se

# Text format
bf schema --format text
```

The implementation correctly:
- Prints the complete SQLite DDL for all 20+ tables
- Includes bead annotations when querying a specific bead
- Supports both JSON and text output formats
