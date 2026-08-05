# Worker Timeout Root Cause Analysis (bf-56b5ig)

## Summary

Diagnosed why bead bf-1w0xhs repeatedly timed out after 10 minutes during worker dispatch.

## Root Cause Identified

### The Problem
- Bead bf-1w0xhs had **empty content** - no description, no acceptance criteria, just the title "P0 Infrastructure fix"
- The NEEDLE watchdog's `bead_timeout: 600` setting (in `/home/coding/.needle/config.yaml:135`) was killing processes after exactly 600 seconds
- Exit code 124 is the standard `timeout` command exit code

### Why It Happened
1. **Empty bead inference overhead**: The agent had to figure out what to do with an empty bead
2. **Complex split operation**: The agent decided to split the bead into 4 child beads with dependency chains
3. **Multi-step process**: Analyzing → creating beads → setting up dependencies → adding labels
4. **Slow model**: GLM-4.7 is slower than other models, causing operations to exceed 600 seconds

### Evidence from .beads/events.jsonl
```json
{"bead":"bf-1w0xhs","duration_ms":600309,"event":"timeout","exit_code":124,"outcome":"timeout"}
{"bead":"bf-1w0xhs","duration_ms":600334,"event":"timeout","exit_code":124,"outcome":"timeout"}
{"bead":"bf-1w0xhs","duration_ms":192101,"event":"complete","exit_code":0,"outcome":"success"}
```

## Classification

**This is a NEEDLE watchdog configuration issue**, not:
- ❌ A bead content issue (though empty beads contribute)
- ❌ A dispatch problem
- ❌ A specific adapter bug

The 600-second timeout is too aggressive for complex operations involving:
- Empty beads requiring agent inference
- Slower models like GLM-4.7
- Multi-command operations (splits, dependencies, labels)

## Recommended Fix

Increase `watchdog.bead_timeout` in `/home/coding/.needle/config.yaml` from 600 to 1800 seconds (30 minutes).

This provides:
- ✅ Enough time for complex operations to complete
- ✅ Still protects against truly stuck processes
- ✅ Accommodates slower models and inference-heavy tasks

## Worker/Adapter Combination

**Failing combination:**
- Worker: `juliet`
- Adapter: `claude-code-glm-4.7`
- Model: `glm-4.7`

## Files Modified

- `.beads/issues.jsonl` - Comment added to parent bead bf-1w0xhs
- `notes/bf-56b5ig.md` - This file
