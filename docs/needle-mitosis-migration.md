# NEEDLE Mitosis Migration Guide

## The Problem (NEEDLE's current non-atomic implementation)

NEEDLE's `bead_store/mod.rs` mitosis uses separate `br` CLI calls:

```bash
# Step 1: Create child 1
br create --title "child 1" --type task --priority 2

# Step 2: Create child 2
br create --title "child 2" --type task --priority 2

# Step 3: Add dependency (child 1 blocks parent)
br dep add child1 parent

# Step 4: Add dependency (child 2 blocks parent)
br dep add child2 parent

# Step 5: Close parent
br close parent --reason "Split into children"
```

**Race condition**: If NEEDLE crashes between steps, you get orphaned children or a parent that never closes.

## The Solution (bf batch - all atomic)

### Method 1: Dedicated `bf mitosis` command (recommended)

```bash
bf mitosis bf-a3f8 \
  --children '[
    {"title": "Implement login handler", "type": "task", "priority": 2},
    {"title": "Add session tests", "type": "task", "priority": 2}
  ]' \
  --reason "Split into children"
```

### Method 2: `bf batch` with JSON

```bash
bf batch --json '[
  {"op": "create", "title": "child 1", "type": "task", "priority": 2},
  {"op": "create", "title": "child 2", "type": "task", "priority": 2},
  {"op": "dep_add_blocker", "parent": "@0", "child": "bf-a3f8"},
  {"op": "dep_add_blocker", "parent": "@1", "child": "bf-a3f8"},
  {"op": "close", "id": "bf-a3f8", "reason": "Split into children"}
]'
```

## Crash Safety Guarantee

All operations run inside a single SQLite `BEGIN IMMEDIATE` transaction:

1. Transaction begins (`BEGIN IMMEDIATE`)
2. Child 1 is created
3. Child 2 is created
4. Dependencies are added
5. Parent is closed
6. Transaction commits (`COMMIT`)

If the process crashes at any point before commit, SQLite automatically rolls back the entire transaction. **Kill -9 during mitosis leaves the workspace in its original state.**

## Migration: One-line change in NEEDLE

Replace the mitosis function in NEEDLE's `bead_store/mod.rs`:

```rust
// OLD (non-atomic, 5 separate br calls)
pub fn mitosis(parent_id: &str, children: Vec<Child>) -> Result<Vec<String>> {
    let mut child_ids = Vec::new();
    for child in &children {
        let id = br_create(&child.title, &child.type_, child.priority)?;
        child_ids.push(id);
    }
    for child_id in &child_ids {
        br_dep_add(child_id, parent_id)?;
    }
    br_close(parent_id, "Split into children")?;
    Ok(child_ids)
}

// NEW (atomic, single bf batch call)
pub fn mitosis(parent_id: &str, children: Vec<Child>) -> Result<Vec<String>> {
    let children_json = serde_json::to_string(&children)?;
    let output = Command::new("bf")
        .args(["mitosis", parent_id, "--children", &children_json])
        .output()?;
    let results: Vec<BatchResult> = serde_json::from_slice(&output.stdout)?;
    Ok(results.into_iter().filter_map(|r| r.id).collect())
}
```

## Verification

Test the crash safety:

```bash
# Setup
cd /tmp
mkdir test-mitosis && cd test-mitosis
bf init
bf create --title "parent" --type task
PARENT=$(bf list --json | jq -r '.[0].id')

# Run mitosis in background, kill it mid-transaction
bf mitosis $PARENT --children '[{"title":"child1"}]' &
sleep 0.001  # Tiny delay to let transaction start
kill -9 $!

# Verify: parent should still be open, no child exists
bf show $PARENT --json | jq '.status'  # Still "open"
bf list --json | jq 'length'           # Still 1 bead (parent only)
```

## References

- Implementation: `src/batch.rs` (mitosis, mitosis_ex, execute_batch)
- CLI command: `src/cli/mod.rs` (cmd_mitosis, cmd_batch)
- Tests: `tests/batch_mitosis.rs`
- Crash safety: `src/storage/sqlite.rs` (with_immediate_transaction)
