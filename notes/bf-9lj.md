# bf-9lj: Atomic Mitosis Batch Implementation

## Verification Summary

The NEEDLE mitosis atomic batch implementation is complete and verified.

### Implementation Components

1. **`src/batch.rs`** - Core batch operations:
   - `execute_batch()` - Runs all ops in `with_immediate_transaction()` for atomicity
   - `mitosis()` - Creates batch ops for simple (title, type, priority) children
   - `mitosis_ex()` - Extended version with full child properties
   - `BatchOp` enum with Create, DepAddBlocker, and Close variants

2. **`src/cli/mod.rs`** - CLI command:
   - `Commands::Mitosis` - Subcommand with id, children (JSON), reason, format flags
   - `cmd_mitosis()` - Handler that parses children JSON and executes batch

3. **`src/lib.rs`** - Public API exports:
   - `mitosis`, `mitosis_ex`, `MitosisChild` exported for NEEDLE integration

### Acceptance Criteria Met

**Requirement**: Replace NEEDLE mitosis create+dep_add non-atomic chain with bf batch. All five ops in one BEGIN IMMEDIATE. Acceptance: kill -9 during mitosis leaves workspace in original state.

**Verification**:
- ✅ `execute_batch()` uses `storage.with_immediate_transaction()` - BEGIN IMMEDIATE
- ✅ SQLite's ACID guarantees: crash rolls back entire transaction automatically
- ✅ `test_batch_rollback_on_error` verifies rollback on failure
- ✅ `test_mitosis_atomic_batch` verifies full 5-op mitosis succeeds

### Usage

```bash
# CLI method
bf mitosis bf-a3f8 \
  --children '[
    {"title": "Child 1", "type": "task", "priority": 2},
    {"title": "Child 2", "type": "task", "priority": 2}
  ]' \
  --reason "Split into children"

# Programmatic method (NEEDLE integration)
use bead_forge::{mitosis_ex, MitosisChild, execute_batch};

let children = vec![
    MitosisChild { title: "Child 1".into(), type_: "task".into(), priority: 2, ... },
];
let ops = mitosis_ex("bf-parent", children, None)?;
let results = execute_batch(&storage, ops, &workspace_dir)?;
```

### Crash Safety

SQLite's WAL mode + BEGIN IMMEDIATE ensures:
- On crash: uncommitted transaction is rolled back automatically
- Workspace state: either original (no children created) or complete (all children + deps + parent closed)
- No orphaned children possible
