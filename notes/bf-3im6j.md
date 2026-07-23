# Batch Operation Flow Audit (bf-3im6j)

## Overview

**File:** `src/batch.rs`
**Total Operation Types:** 8
**Transaction Strategy:** Single `BEGIN IMMEDIATE` transaction wrapping all operations
**Flush Strategy:** Single auto-flush after successful commit

---

## 1. Batch Operation Types

| Op Type | Enum Variant | Function | Description |
|---------|--------------|----------|-------------|
| 1 | `BatchOp::Create` | `execute_create()` | Creates a new bead with title, type, priority, description, assignee, labels |
| 2 | `BatchOp::Update` | `execute_update()` | Updates existing bead fields (title, description, design, acceptance_criteria, notes, status, priority, assignee, owner, issue_type) |
| 3 | `BatchOp::DepAddBlocker` | `execute_dep_add_blocker()` | Adds blocking dependency (id depends on blocker) |
| 4 | `BatchOp::DepRemove` | `execute_dep_remove()` | Removes dependency between beads |
| 5 | `BatchOp::LabelAdd` | `execute_label_add()` | Adds labels to a bead |
| 6 | `BatchOp::LabelRemove` | `execute_label_remove()` | Removes labels from a bead |
| 7 | `BatchOp::Comment` | `execute_comment()` | Adds a comment to a bead |
| 8 | `BatchOp::Close` | `execute_close()` | Closes a bead with optional reason |

---

## 2. Transaction Boundary

### Single Transaction Entry Point
**Location:** `execute_batch()` function, line 191-438

```rust
pub fn execute_batch(
    storage: &Storage,
    ops: Vec<BatchOp>,
    workspace_dir: &std::path::Path,
    no_auto_flush: bool,
) -> Result<Vec<BatchResult>>
```

### Transaction Wrapper
**Line 201:** `storage.with_immediate_transaction(|tx| {...})`

- **Transaction Type:** `BEGIN IMMEDIATE` (via `with_immediate_transaction()`)
- **Scope:** All batch operations (lines 202-418)
- **Commit Condition:** All operations succeed without error
- **Rollback Condition:** Any operation fails (fail-fast at line 411-413)

### Transaction Flow

1. **Begin Transaction:** `with_immediate_transaction()` acquires reserved lock immediately
2. **Initialize:** 
   - `results: Vec<BatchResult>` - accumulates operation results
   - `created_ids: Vec<String>` - tracks bead IDs for placeholder resolution (@0, @1, etc.)
3. **Execute Operations Loop (lines 205-416):**
   - Iterates through `ops` vector
   - Each operation dispatches to its `execute_*` function
   - Results accumulated in `results` vector
   - **Fail-fast:** If any operation returns error status, returns `Err(...)` immediately (line 412)
4. **Commit:** If all operations succeed, returns `Ok(results)` (line 418)
5. **Post-Commit Flush (lines 421-435):**

```rust
// Single auto-flush after successful transaction commit
let flush_outcome = autoflush::after_mutation_with_config(
    workspace_dir,
    &config,
    no_auto_flush,
);

// Surface flush failures as warnings (non-fatal)
if let Some(warning) = flush_outcome.warning() {
    eprintln!("warning: {}", warning);
}
```

---

## 3. Flush Points

### Primary Flush Point
**Location:** Lines 426-435 (after transaction commit)

```rust
let flush_outcome = autoflush::after_mutation_with_config(
    workspace_dir,
    &config,
    no_auto_flush,
);
```

**Characteristics:**
- **Timing:** AFTER successful transaction commit
- **Scope:** Exports ALL beads marked dirty during the transaction
- **Failure Mode:** Non-fatal (warnings only, batch still succeeds)
- **Idempotence:** If auto-flush fails, dirty marks persist; next mutation or `bf sync --flush-only` retries

### Dirty Marking Within Transaction

**Helper Function:** `mark_dirty_tx()` (lines 446-452)

```rust
fn mark_dirty_tx(tx: &Connection, id: &str) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO dirty_issues (issue_id, marked_at) VALUES (?1, ?2)",
        rusqlite::params![id, Utc::now().to_rfc3339()],
    )?;
    Ok(())
}
```

**Calling Pattern:** Each `execute_*` function calls `mark_dirty_tx()` for affected beads:
- `execute_create()`: Marks newly created bead (line 578)
- `execute_dep_add_blocker()`: Marks both id and blocker (lines 664-665)
- `execute_close()`: Marks closed bead and cascaded dependents (lines 710, 762)
- `execute_update()`: Marks updated bead (line 874)
- `execute_dep_remove()`: Marks both id and depends_on (lines 933-934)
- `execute_label_add()`: Marks bead (line 978)
- `execute_label_remove()`: Marks bead (line 1006)
- `execute_comment()`: Marks bead (line 1035)

---

## 4. Operation Execution Paths

### Operation Dispatch (Lines 206-408)

Each `BatchOp` variant matches to its executor:

```rust
for (idx, op) in ops.iter().enumerate() {
    let result = match op {
        BatchOp::Create { ... } => { /* dispatch to execute_create */ }
        BatchOp::Update { ... } => { /* dispatch to execute_update */ }
        BatchOp::DepAddBlocker { id, blocker } => { /* dispatch to execute_dep_add_blocker */ }
        BatchOp::DepRemove { id, depends_on } => { /* dispatch to execute_dep_remove */ }
        BatchOp::LabelAdd { id, labels } => { /* dispatch to execute_label_add */ }
        BatchOp::LabelRemove { id, labels } => { /* dispatch to execute_label_remove */ }
        BatchOp::Comment { id, author, text } => { /* dispatch to execute_comment */ }
        BatchOp::Close { id, reason } => { /* dispatch to execute_close */ }
    };
    
    // Fail-fast on error
    if result.status == "error" {
        return Err(anyhow!("{}", result.error.unwrap_or_default()));
    }
    
    results.push(result);
}
```

### Reference Resolution (Lines 243, 278, 310, 333, 352, 371, 390)

Placeholder references (`@0`, `@1`, etc.) are resolved via `resolve_reference()` (lines 455-465):

```rust
fn resolve_reference(reference: &str, created_ids: &[String]) -> String {
    if let Some(rest) = reference.strip_prefix('@') {
        if let Ok(idx) = rest.parse::<usize>() {
            if idx < created_ids.len() {
                return created_ids[idx].clone();
            }
        }
    }
    reference.to_string()
}
```

### Execution Path Summary

| Op | Line Range | Reference Resolution | Dirty Marks | Special Handling |
|-----|-----------|---------------------|-------------|------------------|
| create | 207-241 | N/A (creates new) | 1 bead (new) | ID collision retry (5 attempts) |
| update | 265-308 | Yes | 1 bead | Invalidates critical path cache on status change |
| dep_add_blocker | 242-264 | Yes (id, blocker) | 2 beads (both endpoints) | Validates: existence, duplicate, cycle |
| dep_remove | 309-331 | Yes (id, depends_on) | 2 beads (both endpoints) | Rebuilds blocked_issues_cache, invalidates critical path |
| label_add | 332-350 | Yes | 1 bead | INSERT OR IGNORE (duplicate-safe) |
| label_remove | 351-369 | Yes | 1 bead | Direct DELETE |
| comment | 370-388 | Yes | 1 bead | Timestamp-based comment ID |
| close | 389-408 | Yes | 1+N beads | Cascade: transitions dependents blocked→open, rebuilds caches |

---

## 5. Special Operations

### Mitosis (Lines 1066-1137)

**Functions:** `mitosis()`, `mitosis_ex()`

**Purpose:** Split a parent bead into N child beads atomically

**Operations Generated:**
1. N × `BatchOp::Create` (one per child)
2. N × `BatchOp::DepAddBlocker` (each child blocks parent)
3. 1 × `BatchOp::Close` (closes parent)

**Example Flow:**
```rust
let ops = mitosis("bf-parent", vec![
    ("Child 1".to_string(), "task".to_string(), 2),
    ("Child 2".to_string(), "bug".to_string(), 0),
], Some("Split".to_string()))?;

// ops contains:
// [Create(Child1), Create(Child2), DepAddBlocker(parent,@0), DepAddBlocker(parent,@1), Close(parent)]
```

---

## 6. Validation

### Pre-Parse Validation (Lines 165-189)

`validate_op_fields()` checks for unknown fields before parsing:

```rust
fn validate_op_fields(value: &serde_json::Value) -> Result<()> {
    let obj = value.as_object()?;
    let op_name = obj.get("op").and_then(|v| v.as_str())?;
    let allowed = get_allowed_fields(op_name);
    
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(anyhow!("Unknown field '{}' in operation '{}'", key, op_name));
        }
    }
    Ok(())
}
```

**Allowed Fields per Operation (Lines 125-158):**
- `create`: op, title, type, priority, description, assignee, labels
- `update`: op, id, title, description, design, acceptance_criteria, notes, status, priority, assignee, owner, issue_type
- `dep_add_blocker`: op, id, blocker, parent, child (parent/child are legacy aliases)
- `dep_remove`: op, id, depends_on
- `label_add`: op, id, labels
- `label_remove`: op, id, labels
- `comment`: op, id, author, text
- `close`: op, id, reason

### Runtime Validation

Each `execute_*` function validates:
- **Bead existence:** Checks `SELECT EXISTS(...) FROM issues WHERE id = ?`
- **Duplicate checks:** For dependencies, labels
- **Cycle detection:** For dependencies (reverse dependency check)
- **Idempotence:** Close operation skips if already closed (line 691-694)

---

## 7. Error Handling

### Fail-Fast Strategy (Lines 410-413)

```rust
// Fail fast on error
if result.status == "error" {
    return Err(anyhow!("{}", result.error.unwrap_or_default()));
}
```

**Behavior:**
- Transaction is rolled back immediately
- No dirty marks persist (rollback clears `dirty_issues` table)
- Auto-flush is skipped (only runs on successful commit)

### Error Result Format (Lines 113-122)

```rust
pub struct BatchResult {
    pub op: usize,           // Operation index in batch
    pub status: String,      // "ok" or "error"
    pub id: Option<String>,  // Created bead ID (create only)
    pub error: Option<String>, // Error message (error only)
    pub message: Option<String>, // Success message (ok only)
}
```

---

## 8. Cache Invalidation

### Critical Path Cache

Invalidated and recomputed by:
- `execute_close()` (lines 780-781)
- `execute_update()` when status changes (lines 878-879)
- `execute_dep_remove()` (lines 949-950)

```rust
crate::critical_path::invalidate_cache(tx)?;
crate::critical_path::compute_all_critical_paths(tx)?;
```

### Blocked Issues Cache

Rebuilt by:
- `execute_close()` (lines 767-777)
- `execute_dep_remove()` (lines 937-947)

```rust
tx.execute("DELETE FROM blocked_issues_cache", [])?;
tx.execute(
    "INSERT INTO blocked_issues_cache (issue_id, blocked_by, blocked_at)
     SELECT d.issue_id, '[' || GROUP_CONCAT('\"' || d.depends_on_id || '\"') || ']' AS blocked_by, ?1
     FROM dependencies d
     INNER JOIN issues i ON i.id = d.depends_on_id
     WHERE d.type IN ('blocks', 'parent-child', 'conditional-blocks', 'waits-for')
     AND i.status NOT IN ('closed', 'tombstone', 'done', 'completed')
     GROUP BY d.issue_id",
    rusqlite::params![Utc::now().to_rfc3339()],
)?;
```

---

## 9. JSON Schema

All operations use internally tagged enum serialization:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum BatchOp {
    #[serde(rename = "create")]
    Create { ... },
    #[serde(rename = "update")]
    Update { ... },
    // ... etc
}
```

**Example JSON:**
```json
{"op": "create", "title": "Fix bug", "type": "bug", "priority": 0, "description": "...", "assignee": "worker-1", "labels": ["urgent"]}
{"op": "update", "id": "bf-123", "status": "in_progress", "priority": 0}
{"op": "dep_add_blocker", "id": "bf-child", "blocker": "bf-parent"}
{"op": "close", "id": "bf-456", "reason": "Completed"}
```

---

## 10. Test Coverage

Test modules (lines 1403-2976) verify:
- Placeholder reference resolution (`@0`, `@1`)
- Serde alias compatibility (parent/child → blocker/id)
- Field validation (unknown field rejection)
- Dependency direction parity with CLI
- Cycle and duplicate detection
- Mitosis end-to-end functionality
- Mixed-operation atomicity
- Rollback on failure
- Transaction sharing (`with_immediate_transaction`)
- Dirty marking within transaction
- Single auto-flush after commit
- Fail-fast with no dirty marks on partial failure

---

## Summary

**Transaction Boundary:** Single `BEGIN IMMEDIATE` transaction wraps all operations (line 201)
**Flush Point:** Single auto-flush after successful commit (lines 426-435)
**Dirty Marks:** Set within transaction via `mark_dirty_tx()`, cleared by flush
**Failure Mode:** Fail-fast with automatic rollback, no dirty marks persist
**Operation Count:** 8 operation types (create, update, dep_add_blocker, dep_remove, label_add, label_remove, comment, close)
**Special Features:** Mitosis (parent splitting), placeholder references (@N), cache invalidation, cascade status transitions
