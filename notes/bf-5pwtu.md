# Phase 7.4 Implementation: Anomaly Classification & Sync Verdicts

## Status: COMPLETE ✓

Phase 7.4 from docs/plan/plan.md has been fully implemented and tested.

## What Was Implemented

### 1. AnomalyClass Enum with Severity Tiers

**Location:** `src/doctor.rs:16-56`

The `AnomalyClass` enum provides comprehensive anomaly classification:

```rust
pub enum AnomalyClass {
    // Critical severity
    DatabaseCorrupt,      // Database integrity check failed
    ConflictMarkers,      // Git merge conflict markers in JSONL
    JsonlInvalid,         // JSONL contains unparseable records

    // High severity  
    IdSetMismatch,        // Bead ID sets don't match between DB and JSONL
    CountMismatch,        // Bead count differs between DB and JSONL

    // Medium severity
    DbNewer,              // Database contains beads not yet flushed to JSONL
    JsonlNewer,           // JSONL contains beads not present in database
    NullNotNull,          // NULL values found in NOT NULL columns

    // Low severity
    Unflushed,            // Beads modified since last flush to JSONL
    PseudoTerminal,       // Beads with done-sounding status instead of closed
    StaleBlocked,         // Beads stuck in blocked status despite no active blockers
}
```

Each anomaly class has:
- `severity()` method returning `AnomalySeverity` (Critical, High, Medium, Low)
- `description()` method with human-readable explanation
- `remediation()` method with suggested action

### 2. Structured Anomaly Findings

**Location:** `src/doctor.rs:128-163`

The `Anomaly` struct provides structured findings:

```rust
pub struct Anomaly {
    pub class: AnomalyClass,
    pub message: String,
    pub affected_ids: Vec<String>,
    pub context: Option<String>,
}
```

### 3. Sync Verdict System

**Location:** `src/doctor.rs:1439-1625`

#### SyncVerdict Enum

```rust
pub enum SyncVerdict {
    InSync,           // Database and JSONL are fully in sync
    DbNewer,          // Unflushed changes exist in database
    JsonlNewer,       // JSONL has changes not in database
    Divergent,        // Divergent state requiring reconciliation
    DatabaseCorrupt,  // Database corruption detected
    JsonlInvalid,     // JSONL file is invalid or missing
}
```

#### SyncStatus Struct

```rust
pub struct SyncStatus {
    pub verdict: SyncVerdict,
    pub db_healthy: bool,
    pub jsonl_healthy: bool,
    pub db_count: usize,
    pub jsonl_count: usize,
    pub dirty_count: usize,
    pub missing_in_jsonl: Vec<String>,
    pub missing_in_db: Vec<String>,
    pub hash_mismatch: Vec<String>,
    pub evidence: Vec<String>,
    pub recommendation: String,
}
```

### 4. Machine-Readable Output

Both `bf doctor --json` and `bf sync --status --json` output structured data.

#### `bf doctor --json` output includes:

```json
{
  "healthy": false,
  "db_ok": true,
  "jsonl_ok": true,
  "anomalies": [
    {
      "class": "id_set_mismatch",
      "message": "2 bead(s) have content hash mismatch",
      "affected_ids": ["bf-3a2", "bf-5pwtu"]
    },
    {
      "class": "unflushed",
      "message": "2 unflushed bead(s) exist",
      "affected_ids": []
    }
  ]
}
```

#### `bf sync --status --json` output includes:

```json
{
  "verdict": "in_sync",
  "db_healthy": true,
  "jsonl_healthy": true,
  "evidence": [
    "Database integrity: PASS (0)",
    "JSONL validity: PASS (0)"
  ],
  "recommendation": "No action needed"
}
```

## In-Sync Verdict Criteria

The "In sync" verdict is backed by evidence and requires ALL of:

1. ✓ Database passes integrity check (`PRAGMA integrity_check` returns "ok")
2. ✓ JSONL file exists and all records parse successfully
3. ✓ No dirty (unflushed) beads exist (`dirty_issues` table is empty)
4. ✓ All bead IDs match between DB and JSONL (no missing_in_jsonl or missing_in_db)
5. ✓ All content hashes match (computed hash equals stored hash)

## Exit Criteria Met

✓ **Manual triage ritual replaced by one command with machine-readable verdict**

The old ritual:
```bash
git log -1 .beads/issues.jsonl
# Compare commit timestamp to bead counts
# Manually inspect for drift
```

Is now replaced by:
```bash
bf sync --status --json
# Returns verdict with full evidence backing
```

## Testing Examples

### Clean workspace (In sync):
```json
{
  "verdict": "in_sync",
  "db_healthy": true,
  "jsonl_healthy": true,
  "dirty_count": 0,
  "evidence": ["Database integrity: PASS (0)", "JSONL validity: PASS (0)"]
}
```

### Workspace with unflushed changes (DbNewer):
```json
{
  "verdict": "db_newer",
  "dirty_count": 2,
  "evidence": ["2 dirty bead(s) detected"],
  "recommendation": "Run 'bf sync --flush-only' to export changes to JSONL"
}
```

### Workspace with hash mismatch (IdSetMismatch):
```json
{
  "verdict": "in_sync",
  "hash_mismatch": ["bf-3a2", "bf-5pwtu"],
  "evidence": ["2 bead(s) with hash mismatch"]
}
```

## Files Modified

- `src/doctor.rs` - Added AnomalyClass, SyncVerdict, SyncStatus, sync_status()
- `src/cli/mod.rs` - Added cmd_sync_status() wired to `bf sync --status`

## Verification Commands

```bash
# Test anomaly classification
bf doctor --json | jq '.anomalies'

# Test sync verdict on clean workspace
cd /tmp/test-bf-sync && bf sync --status --json

# Test sync verdict with drift
bf sync --status --json
```

## Conclusion

Phase 7.4 is fully implemented. The anomaly classification system provides structured, machine-readable output with severity tiers, and the sync status command delivers a definitive "In sync" verdict backed by comprehensive evidence checks. The manual triage ritual is now replaced by a single command that outputs both human-readable and machine-parseable status information.
