# Batch JSON Schema Documentation

The `bf batch` command executes multiple operations atomically under a single `BEGIN IMMEDIATE` transaction. All operations succeed or all fail — no partial state.

## Format

Operations are provided as a JSON array of objects, each with an `op` field identifying the operation type:

```bash
bf batch --json '[
  {"op": "...", ...},
  {"op": "...", ...}
]'
```

Or from a file:

```bash
bf batch --file operations.json
```

## Operation Types

### 1. `create` - Create a new bead

```json
{
  "op": "create",
  "title": "Fix authentication bug",
  "type": "bug",
  "priority": 0,
  "description": "Users cannot log in when tokens expire",
  "assignee": "worker-1",
  "labels": ["urgent", "auth"]
}
```

**Fields:**
- `op` (required): `"create"`
- `title` (required): Bead title (1-500 chars)
- `type` (optional): Issue type, default `"task"` (options: `task`, `bug`, `feature`, `epic`, `chore`, `docs`, `question`)
- `priority` (optional): Priority 0-4, default `2` (0=Critical, 4=Backlog)
- `description` (optional): Detailed description
- `assignee` (optional): Assigned user/worker
- `labels` (optional): Array of label strings

**Result:** Returns the generated bead ID in the `id` field.

---

### 2. `update` - Update an existing bead

```json
{
  "op": "update",
  "id": "bf-abc123",
  "status": "in_progress",
  "priority": 0,
  "assignee": "worker-2",
  "title": "Updated title"
}
```

**Fields:**
- `op` (required): `"update"`
- `id` (required): Bead ID to update
- `title` (optional): New title
- `description` (optional): New description
- `design` (optional): Design notes
- `acceptance_criteria` (optional): Acceptance criteria
- `notes` (optional): Additional notes
- `status` (optional): New status (options: `open`, `in_progress`, `blocked`, `deferred`, `draft`, `closed`, `tombstone`, `pinned`)
- `priority` (optional): New priority 0-4
- `assignee` (optional): New assignee (empty string clears the field)
- `owner` (optional): New owner
- `issue_type` (optional): New issue type

All fields are optional — only specified fields are updated.

---

### 3. `dep_add_blocker` - Add a blocking dependency

```json
{
  "op": "dep_add_blocker",
  "id": "bf-child",
  "blocker": "bf-parent"
}
```

**Fields:**
- `op` (required): `"dep_add_blocker"`
- `id` (required): The bead being blocked (must close after blocker closes)
- `blocker` (required): The bead that blocks (must close before id can close)

**Aliases:** For backward compatibility, `parent` and `child` field names are accepted as aliases for `blocker` and `id` respectively.

**Effect:** Creates a dependency where `id` depends on `blocker` (blocker blocks id). Both beads are marked dirty for export.

**Validation:** Rejects circular dependencies and duplicate dependencies.

---

### 4. `dep_remove` - Remove a dependency

```json
{
  "op": "dep_remove",
  "id": "bf-child",
  "depends_on": "bf-parent"
}
```

**Fields:**
- `op` (required): `"dep_remove"`
- `id` (required): The bead that has the dependency
- `depends_on` (required): The bead that is being depended on (to remove the dependency from)

**Effect:** Removes the dependency relationship. Both endpoints are marked dirty. The blocked_issues_cache and critical_path_cache are rebuilt.

**Validation:** Returns error if the dependency does not exist or either bead is missing.

---

### 5. `label_add` - Add labels to a bead

```json
{
  "op": "label_add",
  "id": "bf-abc123",
  "labels": ["urgent", "bug", "security"]
}
```

**Fields:**
- `op` (required): `"label_add"`
- `id` (required): Bead ID
- `labels` (required): Array of label strings to add

**Effect:** Adds labels to the bead. Duplicate labels are ignored. The bead is marked dirty for export.

---

### 6. `label_remove` - Remove labels from a bead

```json
{
  "op": "label_remove",
  "id": "bf-abc123",
  "labels": ["urgent", "bug"]
}
```

**Fields:**
- `op` (required): `"label_remove"`
- `id` (required): Bead ID
- `labels` (required): Array of label strings to remove

**Effect:** Removes the specified labels from the bead. The bead is marked dirty for export.

---

### 7. `comment` - Add a comment to a bead

```json
{
  "op": "comment",
  "id": "bf-abc123",
  "author": "worker-1",
  "text": "Found edge case in token refresh logic"
}
```

**Fields:**
- `op` (required): `"comment"`
- `id` (required): Bead ID
- `author` (optional): Comment author, default `"batch"`
- `text` (required): Comment text

**Effect:** Adds a comment to the bead. The bead is marked dirty for export.

---

### 8. `close` - Close a bead

```json
{
  "op": "close",
  "id": "bf-abc123",
  "reason": "Completed: Fixed auth token refresh"
}
```

**Fields:**
- `op` (required): `"close"`
- `id` (required): Bead ID to close
- `reason` (optional): Close reason, default `"Completed"`

**Effect:**
- Sets bead status to `closed` with timestamp and reason
- Triggers blocked→open cascade for dependent beads
- Rebuilds blocked_issues_cache and critical_path_cache
- Marks the closed bead and any cascade-affected beads as dirty

**Cascade behavior:** When a bead is closed, any dependents that were `blocked` and have no remaining blockers transition to `open` automatically. This is the same behavior as the CLI `bf close` command.

---

## Placeholder References

Operations can reference beads created earlier in the same batch using placeholder references:

- `@0` - References the first bead created in this batch
- `@1` - References the second bead created
- `@N` - References the (N+1)th bead created

This is critical for operations like **mitosis** (splitting a parent bead into children):

```json
[
  {"op": "create", "title": "Child 1", "type": "task"},
  {"op": "create", "title": "Child 2", "type": "task"},
  {"op": "dep_add_blocker", "id": "bf-parent", "blocker": "@0"},
  {"op": "dep_add_blocker", "id": "bf-parent", "blocker": "@1"},
  {"op": "close", "id": "bf-parent", "reason": "Split into children"}
]
```

Placeholder references are resolved at execution time. If a placeholder is out of bounds or references a non-existent bead, the reference is passed through as-is (e.g., `@5` remains `@5`).

---

## Output Format

Returns a JSON array of results, one per operation:

```json
[
  {"op": 0, "status": "ok", "id": "bf-new123", "message": "Created bead bf-new123"},
  {"op": 1, "status": "ok", "message": "Added labels to bf-abc123"},
  {"op": 2, "status": "ok", "message": "Closed bead-bf-xyz789"}
]
```

**Fields:**
- `op`: Zero-based index of the operation in the input array
- `status`: `"ok"` or `"error"`
- `id`: (for create operations only) The generated bead ID
- `message`: Success or error message
- `error`: (only on error) Error details

---

## Atomicity and Transaction Guarantees

All operations execute inside a single `BEGIN IMMEDIATE` transaction:

1. **All-or-nothing:** If any operation fails, the entire transaction rolls back. No partial state is committed.
2. **Crash safety:** If the process crashes mid-batch, SQLite rolls back automatically. No orphaned state.
3. **Single auto-flush:** With Phase 7.1's `sync.auto_flush` (default on), dirty beads are exported to JSONL once at transaction end, not per-operation. This minimizes write amplification.
4. **Blocked→open cascade:** The batch `close` operation includes the same blocked→open cascade as the CLI `bf close` command (see commit 519449b tests). Dependent beads automatically transition from `blocked` to `open` when their last blocker closes.

---

## Validation

The batch command validates operation structure before execution:

1. **Field validation:** Each operation type has a whitelist of allowed fields. Unknown fields are rejected with a clear error listing allowed fields.
2. **Reference validation:** Bead IDs are checked for existence before mutation.
3. **Dependency validation:** Circular and duplicate dependencies are rejected.
4. **Idempotence:** Operations like `close` are idempotent — closing an already-closed bead succeeds with no error.

---

## CLI-Style Input (One Operation Per Line)

For simple cases, operations can be provided as one operation per line (stdin or file):

```bash
echo 'create --title "Child 1" --type task
create --title "Child 2" --type bug
dep add-blocker bf-parent @0
dep add-blocker bf-parent @1
close bf-parent "Split into children"' | bf batch
```

Supported CLI-style operations:
- `create --title "X" --type Y --priority Z`
- `update <id> --status X --priority Y --assignee Z`
- `dep add-blocker <id> <blocker>`
- `dep remove <id> <depends_on>`
- `label add <id> <label1> <label2> ...`
- `label remove <id> <label1> <label2> ...`
- `comment <id> <text>`
- `close <id> <reason>`

---

## Use Cases

### Mitosis (Split a bead into children)

```json
[
  {"op": "create", "title": "Implement auth UI", "type": "task", "priority": 2},
  {"op": "create", "title": "Implement auth API", "type": "task", "priority": 2},
  {"op": "create", "title": "Add auth tests", "type": "chore", "priority": 3},
  {"op": "dep_add_blocker", "id": "bf-auth-feature", "blocker": "@0"},
  {"op": "dep_add_blocker", "id": "bf-auth-feature", "blocker": "@1"},
  {"op": "dep_add_blocker", "id": "bf-auth-feature", "blocker": "@2"},
  {"op": "close", "id": "bf-auth-feature", "reason": "Split into implementation tasks"}
]
```

### Bulk label management

```json
[
  {"op": "label_add", "id": "bf-abc123", "labels": ["priority-0", "security"]},
  {"op": "label_add", "id": "bf-def456", "labels": ["priority-0", "security"]},
  {"op": "label_remove", "id": "bf-ghi789", "labels": ["backlog"]}
]
```

### Status updates with comments

```json
[
  {"op": "update", "id": "bf-abc123", "status": "in_progress", "assignee": "worker-1"},
  {"op": "comment", "id": "bf-abc123", "author": "worker-1", "text": "Started work on authentication flow"},
  {"op": "label_add", "id": "bf-abc123", "labels": ["in-progress"]}
]
```

### Dependency restructuring

```json
[
  {"op": "dep_remove", "id": "bf-feature", "depends_on": "bf-old-blocker"},
  {"op": "dep_add_blocker", "id": "bf-feature", "blocker": "bf-new-blocker"},
  {"op": "comment", "id": "bf-feature", "text": "Replaced dependency after restructure"}
]
```

---

## Error Handling

On error, the transaction rolls back and the command returns a non-zero exit code. The error result includes:

```json
{"op": 2, "status": "error", "error": "Bead not found: bf-missing", "message": null}
```

Common errors:
- **Unknown field:** Typo in field name, with list of allowed fields
- **Bead not found:** Referenced bead ID does not exist
- **Circular dependency:** Dependency would create a cycle
- **Dependency already exists:** Attempting to add a duplicate dependency
- **Dependency does not exist:** Attempting to remove a non-existent dependency
- **Invalid status/type:** Invalid status or issue_type value

---

## Mitosis Helper Functions

The `mitosis()` and `mitosis_ex()` functions construct mitosis batch operations programmatically:

```rust
let ops = mitosis("bf-parent", vec![
    ("Child 1".to_string(), "task".to_string(), 2),
    ("Child 2".to_string(), "bug".to_string(), 0),
], Some("Split into children".to_string()))?;
```

This is used internally by NEEDLE and other tools for automated bead splitting.
