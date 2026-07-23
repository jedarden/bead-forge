# bf-4p1sr: Label test bead

Date: 2026-07-23

## What was done

Replaced placeholder labels on bead bf-4p1sr with appropriate functional labels.

## Changes

**Before:** `["test1", "test2"]`
**After:** `["label", "test"]`

## Rationale

This bead is titled "Label test bead" and serves as a test bead for the labeling functionality. The new labels better reflect its purpose:
- `test`: Indicates this is a test bead (consistent with other test beads in the system)
- `label`: Indicates this specifically tests label functionality

## Verification

```bash
br show bf-4p1sr --json | jq -r '.[0].labels'
# Output: ["label", "test"]
```

Labels successfully applied using `br label add/remove` commands.
