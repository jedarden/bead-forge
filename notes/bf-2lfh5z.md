# Expected Batch Operations JSON Format

Extracted from `tests/test_p0_multilabel_cli.rs` lines 370-419.

## Batch Input Format

**Structure:** JSON Array
**Purpose:** Input to `bf batch --stdin` command

```json
[
  {
    "op": "create",
    "title": "P0 batch test 1",
    "type": "task",
    "priority": 0,
    "labels": ["critical", "batch"]
  },
  {
    "op": "create",
    "title": "P0 batch test 2",
    "type": "bug",
    "priority": 0,
    "labels": ["urgent", "batch"]
  },
  {
    "op": "create",
    "title": "P0 batch test 3",
    "type": "feature",
    "priority": 0,
    "labels": ["critical", "hotfix"]
  }
]
```

### Input Field Specifications

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `op` | string | Yes | Operation type (e.g., "create") |
| `title` | string | Yes | Bead title |
| `type` | string | Yes | Issue type (e.g., "task", "bug", "feature") |
| `priority` | number | Yes | Priority level (0 for P0) |
| `labels` | array of strings | No | Labels to attach to the bead |

## List Output Format (with --envelope flag)

**Structure:** JSON Object with envelope wrapper
**Purpose:** Output from `bf list --priority 0 --json --envelope`

```json
{
  "data": [
    {
      "priority": 0,
      "labels": ["critical", "batch"],
      // ... other bead fields
    },
    {
      "priority": 0,
      "labels": ["urgent", "batch"],
      // ... other bead fields
    },
    {
      "priority": 0,
      "labels": ["critical", "hotfix"],
      // ... other bead fields
    }
  ]
}
```

### Envelope Structure

- **Top-level:** Object with `data` key
- **`data` field:** Array of bead objects
- **Bead object fields (verified in test):**
  - `priority`: number
  - `labels`: array of strings

## Test Validation Pattern

The test validates batch operations by:

1. Creating beads via `bf batch --stdin` with JSON array input
2. Listing beads with `bf list --priority 0 --json --envelope`
3. Extracting the `data` array from the envelope
4. Verifying:
   - Correct number of beads created (array length)
   - Each bead has `priority` field with expected value
   - Each bead has `labels` array containing expected labels
