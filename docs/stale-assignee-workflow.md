# Stale Assignee Clearing Workflow

## Problem Statement

When NEEDLE workers crash or abandon beads, they leave stale assignees that prevent the beads from being discovered by new workers. A bead with a non-empty `assignee` field is excluded from the ready/claim list, effectively making it invisible to the fleet.

## Acceptance Criteria

✅ **All acceptance criteria verified:**

1. ✅ Create a test bead with assignee 'dead-worker-X' and status 'open'
2. ✅ Verify that NEEDLE explore strand would exclude it (assignee is non-empty)
3. ✅ Use 'bf update --clear-assignee' to clear the assignee
4. ✅ Verify the bead is now discoverable (assignee is NULL)
5. ✅ Document the workflow for fixing stale assignees fleet-wide

## End-to-End Workflow

### 1. Identify Stale Assignees

Find beads that have been stuck in the same assignee for too long:

```bash
# List all beads with assignees (sorted by assignee)
bf stats --by-assignee

# Find beads assigned to a specific worker
bf list --assignee "dead-worker-X"

# Show detailed information about a specific bead
bf show <bead-id>
```

### 2. Clear the Stale Assignee

Use either of these equivalent commands:

```bash
# Method 1: Using --clear-assignee flag (recommended for discoverability)
bf update <bead-id> --clear-assignee

# Method 2: Using empty string (also works but less discoverable)
bf update <bead-id> --assignee ""
```

The `--clear-assignee` flag is preferred because:
- It's more self-documenting
- It's easier to discover in help text
- It clearly expresses the intent of clearing the field

### 3. Verify the Fix

Confirm the bead is now discoverable:

```bash
# Check that assignee is now NULL
bf show <bead-id> --format json --envelope | jq '.data.assignee'
# Should output: null

# Verify the bead appears in the ready list
bf ready --format json | jq '.[] | select(.id == "<bead-id>")'
# Should show the bead

# Test that it can be claimed
bf claim --assignee "test-worker" --dry-run
# Should include the bead in candidate list
```

## Fleet-Wide Stale Assignee Cleanup

### Manual Cleanup for Specific Workers

When you know a specific worker is dead or abandoned:

```bash
# Find all beads assigned to the dead worker
bf list --assignee "dead-worker-X" --json | jq -r '.[].id' | while read bead_id; do
    echo "Clearing assignee for $bead_id"
    bf update "$bead_id" --clear-assignee
done
```

### Batch Cleanup Using bf batch

For multiple beads, use the atomic batch operation:

```json
[
  {
    "op": "create",
    "title": "Clear stale assignees for dead-worker-X",
    "type": "task",
    "priority": 2
  },
  {
    "op": "dep_add_blocker",
    "id": "@0",
    "blocker": "bf-clear-stale-assignees-cleanup"
  }
]
```

Then use the script approach or manual clearing for the affected beads.

### Automated Reconciliation

Use `bf doctor --reconcile` to fix multiple stale assignees at once:

```bash
# Reconcile fixes:
# - Beads stuck at 'blocked' with all blockers closed (reopens them)
# - Empty-string assignees (normalizes to NULL)
bf doctor --reconcile
```

### Reclaiming Stale Claims

For beads stuck in `in_progress` status with stale claims:

```bash
# Reclaim beads that have been in_progress longer than the claim TTL
bf doctor --reclaim-stale [--ttl <minutes>]

# Example: Reclaim beads stuck for more than 2 hours
bf doctor --reclaim-stale --ttl 120
```

## Prevention Strategies

### 1. Monitoring Stale Assignees

Set up monitoring to detect stale assignees:

```bash
# Find beads assigned but not updated recently
bf recent --assignee "worker-name" --time-period 24h
```

### 2. Worker Health Checks

Ensure workers have proper health checks and cleanup on exit:

```bash
# Worker shutdown sequence should:
# 1. Stop claiming new work
# 2. Clear assignees on any in-progress beads
# 3. Exit cleanly
```

### 3. Claim TTL Configuration

Configure appropriate claim TTL in `.beads/config.yaml`:

```yaml
claim_ttl_minutes: 30  # Adjust based on your workflow
```

## Testing

The workflow is validated by the end-to-end test in:
`tests/stale_assignee_clearing_workflow.rs`

Run the test to verify the workflow:

```bash
cargo test stale_assignee_clearing_workflow
```

## API Usage

For programmatic access (e.g., from NEEDLE or other tools):

```rust
use bead_forge::model::IssueChanges;

// Clear assignee programmatically
let changes = IssueChanges {
    assignee: Some(String::new()), // Empty string = clear to NULL
    ..Default::default()
};
storage.update_issue(bead_id, &changes)?;
```

## Related Commands

- `bf claim` - Claims a bead atomically (prevents race conditions)
- `bf reopen` - Reopens closed beads and clears stale assignees
- `bf doctor --reclaim-stale` - Reclaims stuck in_progress beads
- `bf doctor --reconcile` - Fixes various data inconsistencies
- `bf ready` - Lists discoverable (unassigned) beads

## Technical Details

### How Assignee Clearing Works

1. CLI Level: `--clear-assignee` flag is converted to `assignee: Some(String::new())`
2. Storage Level: Empty string is mapped to `NULL` in the database
3. Query Level: Beads with `NULL` assignee are included in ready/claim lists
4. Event Level: An `AssigneeChanged` event is recorded for audit trail

### Database Representation

```sql
-- Before clearing
UPDATE issues SET assignee = 'dead-worker-X' WHERE id = 'bf-abc123';

-- After clearing
UPDATE issues SET assignee = NULL WHERE id = 'bf-abc123';
```

### NEEDLE Integration

NEEDLE's explore strand uses queries like:

```sql
SELECT id, title, priority FROM issues 
WHERE status = 'open' 
AND assignee IS NULL  -- Only unassigned beads
AND id NOT IN (SELECT blocked FROM dependencies WHERE blocker IN (...))
ORDER BY priority, created_at;
```

Beads with non-NULL assignees are automatically excluded.

## Troubleshooting

### Bead Still Not Discoverable After Clearing

1. Verify the assignee is actually NULL:
   ```bash
   bf show <id> --format json --envelope | jq '.data.assignee'
   ```

2. Check if the bead has blocking dependencies:
   ```bash
   bf dep tree <id>
   ```

3. Verify the bead status is 'open':
   ```bash
   bf show <id> | grep Status
   ```

### Command Fails with "Conflicts" Error

Don't use both `--clear-assignee` and `--assignee` together:
```bash
# ❌ Wrong - will fail
bf update <id> --clear-assignee --assignee "worker"

# ✅ Correct - use one or the other
bf update <id> --clear-assignee
bf update <id> --assignee "worker"
```

### Batch Clearing Fails Partway Through

Use atomic batch operations for safety:
```bash
# Create a batch JSON file with all clear operations
echo '[{"op":"clear_assignee","id":"bf-1"},{"op":"clear_assignee","id":"bf-2"}]' | bf batch --stdin
```

## See Also

- [bf CLI Reference](README.md#commands)
- [NEEDLE Integration](research/br-compatibility.md#needle-integration)
- [Claim Concurrency](README.md#claiming--concurrency)
- [Doctor Commands](README.md#maintenance--config)
