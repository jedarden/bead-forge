# P0 Level 3 Placeholder Closure Plan

## Executive Summary

Bead bf-22ttng ("P0 Level 3") has been closed and reopened 7 times due to NEEDLE's shipped-work verification gate. This plan documents the root cause and provides the correct closure procedure.

## Problem Statement

bf-22ttng is a **placeholder bead** following the pattern of P0 Level 0, 1, and 2 beads. It has:
- No actual work required
- No implementation needed
- Dependency bf-kfdiwh already closed
- Status: `blocked` with label `verification-failed`

### Why It Failed Verification 7 Times

The bead failed NEEDLE's shipped-work verification because:

1. **No substantial commit was made** - No code changes were pushed (as expected for a placeholder)
2. **No bead notes were updated** - No agent ran `bf update --notes` to explain why no work was needed

NEEDLE's shipped-work gate (located in `/home/coding/NEEDLE/src/validation/shipped_work.rs`) requires **EITHER**:
- A substantial commit pushed (touches files outside `notes/`/`.beads/`) **OR**
- The bead's `notes` field changed during dispatch

The verification gate is documented in the code comments:

> Gated by `worker.enforce_shipped_work` (default `true`). Runs only when an
> agent has already closed the bead. Passes if either:
> - a commit was made since dispatch started that touches at least one file
>   outside `notes/`/`.beads/`, and that commit has been pushed; or
> - the bead's own `notes` field changed during this dispatch — i.e. the agent
>   ran `bf update --notes` to record why no code change was needed.

## Verification Gate Technical Details

### Location
`/home/coding/NEEDLE/src/validation/shipped_work.rs` - function `verify_shipped_work()`

### What Gets Checked

1. **Commit Check:**
   - Compares current HEAD SHA against pre-dispatch snapshot
   - If different, checks if commit touches "substantial" paths
   - Paths that **DON'T count as substantial**: `notes/`, `.beads/`, `.needle-predispatch-sha`
   - Verifies commit has been pushed to upstream (`@{u}`)

2. **Notes Field Fallback:**
   - Compares hash of bead's `notes` field before and after dispatch
   - If notes changed during dispatch, verification passes
   - This is the correct path for placeholder beads

### Why Notes-Only Commits Are Rejected

From the code comments:

> Deliberately does NOT accept a commit touching only `notes/`/`.beads/` as
> sufficient on its own: a prior incident (see docs/notes on ARMOR's
> commit-storm) showed a worker stuck retrying an uncompletable bead will
> happily satisfy a bare "must have a commit" rule by committing a trivial
> "still blocked" doc file every cycle, each one triggering paired CI
> version-bump commits.

## Correct Closure Procedure

To properly close bf-22ttng as a documented placeholder:

```bash
# Step 1: Update bead notes explaining why no work is needed
bf update bf-22ttng --notes "P0 Level 3 placeholder test bead - no actual work required. Follows pattern of P0 Level 0, 1, 2. Dependency bf-kfdiwh is closed. This is a test placeholder with no implementation requirements."

# Step 2: Close the bead
bf close bf-22ttng --reason "Documented placeholder P0 Level 3 test bead - no actual work required. Notes field updated to satisfy verification gate."
```

**Important**: The `bf update --notes` call **must** happen before `bf close`. This ensures the notes field hash changes during dispatch, satisfying condition 2 of the verification gate.

## Why This Works

The shipped-work gate's `evaluate()` function checks the notes fallback last:

```rust
// Fallback: the agent recorded an explanation on the bead itself.
match snapshot.notes_hash.as_deref() {
    Some(pre_hash) => {
        if hash_notes(post_notes) != pre_hash {
            return Ok(GateResult::Pass);
        }
    }
    None => {
        // Notes were unreadable at dispatch, so there is nothing to diff
        // against. Accept a non-empty note rather than failing a bead on a
        // comparison the gate could not make.
        if !post_notes.trim().is_empty() {
            return Ok(GateResult::Pass);
        }
    }
}
```

When you run `bf update --notes` before closing:
- Pre-dispatch snapshot has `notes_hash` of the old notes value
- Post-closure check has different `notes_hash` because notes changed
- Gate passes without requiring a commit

## Verification Test Cases

From the test suite in `shipped_work.rs`:

### ✅ Passing Tests (What We're Emulating)

- `notes_changed_during_dispatch_passes_without_a_commit`: Shows that changing notes without a commit passes
- `snapshot_without_readable_notes_accepts_a_non_empty_note`: Shows non-empty notes pass when snapshot lacks readable notes

### ❌ Failing Tests (What Happened Before)

- `closing_without_shipping_or_noting_anything_fails`: Shows closing with unchanged notes and no commit fails
- `notes_only_commit_is_treated_as_trivial`: Shows committing only to `notes/` doesn't satisfy the gate

## General Pattern for Placeholder Beads

For any future placeholder beads (no implementation work required):

```bash
# Before closing
bf update <bead-id> --notes "Explanation of why no work is needed"

# Then close
bf close <bead-id> --reason "Summary of what was done (or not done)"
```

**DO NOT**:
- Create `notes/<bead-id>.md` files (they don't help verification)
- Commit only to `notes/` or `.beads/` (they're treated as trivial paths)
- Close without updating the bead's notes field first

**DO**:
- Use `bf update --notes` to record why no code change was needed
- Keep the explanation concise but clear
- Reference dependencies if relevant

## Current State

As of 2026-08-13:
- **bf-22ttng status**: `blocked`
- **Labels**: `deferred`, `umbrella`, `verification-failed`
- **Notes field**: Already contains: "P0 Level 3 placeholder test bead - no actual work required. Follows pattern of P0 Level 0, 1, 2. Dependency bf-kfdiwh is closed. Note: working directory has uncommitted changes from OTHER beads, not from this placeholder."
- **Issue**: The notes field was populated AFTER closure attempts, not before

The notes field already has good content, but because it wasn't updated via `bf update --notes` BEFORE the closure attempts, the verification gate didn't see it as a change during dispatch.

## Action Required

Run this command sequence:

```bash
bf update bf-22ttng --notes "P0 Level 3 placeholder test bead - no actual work required. Follows pattern of P0 Level 0, 1, 2. Dependency bf-kfdiwh is closed. This is a documented test placeholder with no implementation requirements. Verification investigation complete - shipped-work gate requires either substantial commit OR notes field change during dispatch. This update provides the required notes field change."

bf close bf-22ttng --reason "Documented placeholder P0 Level 3 test bead - no actual work required. Notes field updated to satisfy shipped-work verification gate condition 2 (notes field change during dispatch). See docs/p0-level3-placeholder-closure-plan.md for full investigation and closure procedure."
```

## References

- **NEEDLE shipped-work gate**: `/home/coding/NEEDLE/src/validation/shipped_work.rs`
- **Verification test suite**: Lines 188-427 in `shipped_work.rs`
- **Bead investigation comment**: bf-22ttng comment ID 1786086925793329
- **Related beads**: bf-1h6uyf (investigation), bf-3rgn3r (add content), bf-19x7v4 (fix and close)
