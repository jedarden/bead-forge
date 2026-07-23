# Epic Label Listing and Querying Verification

## Bead ID
bf-641les

## Verification Date
2026-07-23

## Acceptance Criteria Verified

### 1. Use 'bf labels <bead-id>' to list epic labels ✓
**Status:** VERIFIED
- Command: `bf labels <bead-id>` works correctly
- Tested on epic bead: `bf-yitu4`
- Output format: One label per line (text mode) or JSON array (json mode)

### 2. Verify labels output in JSON array format ✓
**Status:** VERIFIED
```bash
$ bf labels bf-yitu4 --format json
[
  "epic-test",
  "failure-count:4",
  "test",
  "umbrella"
]
```
- Output is a valid JSON array
- Each label is a separate string element
- Properly formatted with `serde_json::to_string_pretty`

### 3. Confirm per-row DB storage (no comma-joined artifacts) ✓
**Status:** VERIFIED

**Database Schema:**
```sql
CREATE TABLE IF NOT EXISTS labels (
    issue_id TEXT NOT NULL,
    label TEXT NOT NULL,
    PRIMARY KEY (issue_id, label),
    FOREIGN KEY (issue_id) REFERENCES issues(id) ON DELETE CASCADE
);
```

**Direct DB Query:**
```bash
$ sqlite3 .beads/beads.db "SELECT issue_id, label FROM labels WHERE issue_id = 'bf-yitu4' ORDER BY label;"
bf-yitu4|epic-test
bf-yitu4|failure-count:4
bf-yitu4|test
bf-yitu4|umbrella
```

**Issues Table Verification:**
- No `labels` column exists on the `issues` table
- Labels are stored exclusively in the separate `labels` table
- Each label is a separate row (issue_id, label pair)

### 4. Verify labels persist after flush checkpoint (db → JSONL) ✓
**Status:** VERIFIED

**Round-trip Test:**
1. Created bead `bf-5m5uk8` with labels: `test-label-1`, `test-label-2`
2. Verified in DB: `SELECT label FROM labels WHERE issue_id = 'bf-5m5uk8'`
   - Result: Both labels present as separate rows
3. Flushed to JSONL: `bf sync --flush-only`
   - Result: "Flushed 1133 beads to JSONL"
4. Verified in JSONL: `jq -r 'select(.id=="bf-5m5uk8") | .labels' .beads/issues.jsonl`
   - Result: `["test-label-1", "test-label-2"]`
5. Verified via `bf show`: Labels correctly loaded from DB

**Verification on Existing Epic:**
```bash
$ bf labels bf-yitu4 --format json
["epic-test", "failure-count:4", "test", "umbrella"]

$ jq -r 'select(.id=="bf-yitu4") | .labels' .beads/issues.jsonl
["epic-test", "failure-count:4", "test", "umbrella"]
```
Labels in DB match labels in JSONL exactly.

### 5. Test label query on live epic bead ✓
**Status:** VERIFIED

**Test Epic:** `bf-yitu4` (Test epic with labels - open)

**Query Results:**
- Text format: 4 labels, one per line
- JSON format: Valid JSON array with 4 label strings
- Direct DB query: 4 rows in labels table for this bead
- JSONL verification: Labels match exactly

**Additional Search Query Tests:**
```bash
# Single label query on epic type
$ bf search --label epic-test --type epic --format json
# Returns 20+ epic beads with epic-test label, including bf-yitu4

# Multi-label query (OR logic)
$ bf search --label epic-test --label test --type epic --format json
# Returns 25 epic beads with EITHER label (OR logic correctly implemented)
```

## Implementation Details

### Command Handler
Location: `src/cli/mod.rs:2786` (`fn cmd_labels`)

```rust
fn cmd_labels(beads_dir: &PathBuf, id: &str, format: &str) -> Result<()> {
    let metadata = load_metadata(beads_dir)?;
    let db_path = beads_dir.join(&metadata.database);
    let storage = Storage::open(&db_path)?;
    let labels = storage.get_labels(id)?;
    if format == "json" {
        println!("{}", serde_json::to_string_pretty(&labels)?);
    } else {
        for label in &labels {
            println!("{}", label);
        }
    }
    Ok(())
}
```

### Storage Layer
Location: `src/storage/sqlite.rs:1290` (`pub fn get_labels`)

- Calls `load_labels` which performs: `SELECT label FROM labels WHERE issue_id = ?1`
- Returns `Result<Vec<String>>`
- Direct SELECT query - efficient single-query implementation

### CLI Definition
Location: `src/cli/mod.rs:520-527` (`Labels` command)

```rust
/// List labels for a specific issue (direct SELECT, efficient)
///
/// A lightweight single-SELECT variant of `bf label list` for one bead.
/// Prints one label per line, or JSON with --format json.
Labels {
    /// Bead ID
    id: String,

    /// Output format (text, json)
    #[arg(short, long, default_value = "text")]
    format: String,
},
```

## Conclusion

All acceptance criteria for bead bf-641les have been successfully verified:

1. ✓ `bf labels <bead-id>` command works correctly
2. ✓ Labels output in JSON array format when `--format json` is specified
3. ✓ Labels are stored per-row in the `labels` table (no comma-joined artifacts)
4. ✓ Labels persist correctly through DB → JSONL flush checkpoint
5. ✓ Label queries work correctly on live epic beads

The implementation correctly separates label storage into a dedicated table with proper normalization (issue_id, label pairs as the primary key), ensuring efficient queries and clean data management without comma-joined artifacts.
