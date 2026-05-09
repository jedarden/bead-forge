# Test Fixtures

This directory contains JSONL snapshot files used for testing bead-forge's compatibility with beads_rust (br).

## Fixture Files

### `simple_bead.jsonl`
A single simple bead with minimal fields. Used for basic round-trip testing.

### `complex_workspace.jsonl`
A realistic workspace with multiple beads of different types:
- Feature bead with labels and assignee
- Bug in progress
- Blocked task with dependency
- Open task
- Closed task with comment

### `edge_cases.jsonl`
Edge cases and special scenarios:
- Unicode characters and emojis
- Very long text fields
- Many labels (20)
- Many dependencies
- Many comments
- Tombstone (deleted) bead
- Escaped characters (quotes, backslashes, newlines)

### `forge-snapshot.jsonl`
**Real workspace snapshot from bead-forge (FORGE).**
Copied from ~/bead-forge/.beads/issues.jsonl. Representative beads showing:
- Open tasks/features/bugs from the plan
- Closed tasks with retrospective notes and close_reason
- In-progress tasks
- Tasks with labels (phase-4b, phase-4c, phase-5, phase-6)
- Tasks with assignees
- Full JSONL schema including design, acceptance_criteria, notes

**Source:** `~/bead-forge/.beads/issues.jsonl` (38 beads total, snapshot has 8)

### `needle-snapshot.jsonl`
**Real workspace snapshot from NEEDLE.**
Copied from ~/NEEDLE/.beads/issues.jsonl. Representative beads showing:
- Diverse statuses (open, closed, in_progress, blocked)
- Multiple issue types (task, bug, feature)
- Beads with labels
- Beads with dependencies
- Beads with comments
- Various complexity levels from simple tasks to complex multi-part features

**Source:** `~/NEEDLE/.beads/issues.jsonl` (562 beads total, snapshot has 50)

## Using Fixtures in Tests

### Manual Copy Method

```rust
use std::fs;

#[test]
fn test_fixture_import() {
    let ws = TempWorkspace::new().unwrap();
    let fixture_path = Path::new("tests/fixtures/complex_workspace.jsonl");

    // Copy fixture to workspace
    fs::copy(fixture_path, &ws.jsonl_path).unwrap();

    // Import and verify
    ws.import_jsonl().unwrap();
    assert_eq!(ws.count_beads().unwrap(), 5);
}
```

### Using `TempWorkspace::from_fixture()` (Recommended)

```rust
#[test]
fn test_forge_snapshot() {
    let ws = TempWorkspace::from_fixture("forge-snapshot.jsonl").unwrap();
    ws.import_jsonl().unwrap();
    assert_eq!(ws.count_beads().unwrap(), 8);
}

#[test]
fn test_needle_snapshot() {
    let ws = TempWorkspace::from_fixture("needle-snapshot.jsonl").unwrap();
    ws.import_jsonl().unwrap();

    // Verify round-trip compatibility
    let exported = ws.export_jsonl(false).unwrap();
    assert!(exported > 0);
}
```

## Fixture Format

All fixtures use the br-compatible JSONL format with the following fields:

- `id`: Bead ID
- `title`: Title
- `description`: Description text
- `design`: Design document
- `acceptance_criteria`: Acceptance criteria
- `notes`: Notes
- `status`: open/closed/in_progress/blocked/tombstone
- `priority`: 0-4 (0 is highest)
- `issue_type`: task/bug/feature/epic
- `assignee`: Assigned worker
- `owner`: Bead owner
- `estimated_minutes`: Time estimate
- `created_at`: ISO 8601 timestamp
- `updated_at`: ISO 8601 timestamp
- `closed_at`: ISO 8601 timestamp (when closed)
- `close_reason`: Reason for closing
- `closed_by_session`: Session that closed the bead
- `due_at`: ISO 8601 timestamp (due date)
- `defer_until`: ISO 8601 timestamp (defer until)
- `external_ref`: External reference ID
- `source_system`: Source system (jira, github, etc.)
- `source_repo`: Source repository path
- `deleted_at`: ISO 8601 timestamp (when deleted)
- `deleted_by`: User who deleted
- `delete_reason`: Reason for deletion
- `original_type`: Original issue type before conversion
- `compaction_level`: Compaction level
- `compacted_at`: ISO 8601 timestamp (when compacted)
- `compacted_at_commit`: Git commit hash of compaction
- `original_size`: Original size before compaction
- `sender`: Message sender
- `ephemeral`: Boolean (true for ephemeral beads)
- `pinned`: Boolean (true for pinned beads)
- `is_template`: Boolean (true for template beads)
- `labels`: Array of label strings
- `dependencies`: Array of dependency objects
- `comments`: Array of comment objects

## Regenerating Fixtures from Real Workspaces

When updating the real workspace snapshots, copy from the actual workspace JSONL files:

### FORGE Snapshot (forge-snapshot.jsonl)

```bash
# Copy a subset of representative beads from FORGE
head -38 ~/bead-forge/.beads/issues.jsonl > tests/fixtures/forge-snapshot.jsonl

# Or manually select specific beads
grep -E 'bf-(1br|45g|8dd|alv|5kn)' ~/bead-forge/.beads/issues.jsonl > tests/fixtures/forge-snapshot.jsonl
```

### NEEDLE Snapshot (needle-snapshot.jsonl)

```bash
# Copy a diverse subset from NEEDLE (562 beads total)
head -200 ~/NEEDLE/.beads/issues.jsonl | python3 -c "
import json, sys
beads = [json.loads(line) for line in sys.stdin.readlines()]
# Select diverse set based on status, type, labels, etc.
# ... selection logic ...
for bead in selected:
    print(json.dumps(bead))
" > tests/fixtures/needle-snapshot.jsonl

# Or take first 50 lines for quick snapshot
head -50 ~/NEEDLE/.beads/issues.jsonl > tests/fixtures/needle-snapshot.jsonl
```

**Important:** These are read-only fixtures for testing. Never modify the original workspace JSONL files when working with fixtures.
