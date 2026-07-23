# Audit: Read-Only Commands for JSONL Write Paths

**Task:** Audit every read-only and diagnostic command to identify if any call into autoflush, export_jsonl, export_jsonl_dirty, or sync_to_jsonl.

**Date:** 2026-07-23

## Commands Audited

1. list
2. show
3. ready
4. critical-path
5. doctor (including --json and --repair on healthy workspace)
6. sync --status (does not exist as a command)
7. labels
8. comments list
9. velocity
10. commit_check

## Summary

**CLEAN:** 8 commands have no write paths
**VIOLATIONS FOUND:** 2 violations
**EXPECTED WRITES:** 4 commands (not violations - they are write operations by design)

## Detailed Findings

### ✅ CLEAN: No Write Paths

1. **bf list** (lines 1551-1660 in cli/mod.rs)
   - Uses: `storage.list_issues()`, `list_all_with_archives()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

2. **bf show** (lines 1662-1749 in cli/mod.rs)
   - Uses: `storage.get_issue()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

3. **bf ready** (lines 1860-1924 in cli/mod.rs)
   - Uses: `get_ready_candidates()`, `storage.get_issue()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

4. **bf critical-path** (lines 3373-3414 in cli/mod.rs)
   - Uses: `compute_epic_critical_path()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

5. **bf labels** (lines 2785-2798 in cli/mod.rs)
   - Uses: `storage.get_labels()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

6. **bf velocity** (lines 3160-3193 in cli/mod.rs)
   - Uses: `crate::velocity::get_velocity_stats()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

7. **bf commit-check** (lines 2388-2399 in cli/mod.rs)
   - Uses: `scan_staged_beads()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

8. **bf comments list** (lines 2812-2824 in cli/mod.rs)
   - Uses: `storage.list_comments()`
   - No autoflush/export_jsonl calls
   - Verdict: CLEAN

### ❌ VIOLATIONS: Unexpected Write Paths

9. **bf doctor --repair --flush-first on healthy workspace**
   - Location: doctor.rs lines 1007-1023
   - Issue: When workspace is healthy (no rebuild needed), code returns early at line 1003, but line 1011 still calls `storage.sync_to_jsonl(&jsonl_path, false)` if `opts.flush_first` is true
   - Code snippet:
     ```rust
     if !needs_rebuild || !still_needs_rebuild {
         report.healthy = true;
         // ...
         if opts.flush_first && post_local.unflushed_count > 0 && db_path.exists() {
             // The user explicitly asked to checkpoint; flushing is a safe, non-rebuild
             // operation, so honor it even though no rebuild was needed.
             if let Ok(storage) = Storage::open(&db_path) {
                 if let Ok(flushed) = storage.sync_to_jsonl(&jsonl_path, false) {  // ← WRITE HERE
                     report.messages.push(format!("Flushed {} unflushed bead(s) to JSONL", flushed));
                 }
             }
         }
         return Ok(report);
     }
     ```
   - Impact: A read-only diagnostic command (`--repair` on healthy workspace that returns early) performs a write to JSONL
   - Verdict: **VIOLATION** - read-only operation should not write

### ✅ EXPECTED WRITES (Not violations - write operations by design)

10. **bf sync** (all variants)
    - Uses: `sync::flush()`, `sync::import()`, `sync::sync()`
    - Location: sync.rs - flush(), flush_dirty(), flush_after_delete()
    - These are write operations by design
    - Verdict: EXPECTED (not a violation)

11. **bf comments add**
    - Uses: `autoflush_after_mutation()` at line 2809 in cli/mod.rs
    - This is a write operation (adds a comment)
    - Verdict: EXPECTED (not a violation)

12. **bf doctor --repair** (actual rebuild path)
    - Uses: `storage.sync_to_jsonl()` at lines 649, 1033 in doctor.rs
    - These are in the repair rebuild path, which is a write operation by design
    - Verdict: EXPECTED (not a violation)

13. **bf doctor --reclaim-stale**
    - Uses: Database UPDATE operations at lines 1146-1157 in doctor.rs
    - This is a write operation (reclaims stale beads)
    - Verdict: EXPECTED (not a violation)

## Analysis

### The Violation

The violation is subtle: `bf doctor --repair --flush-first` on a healthy workspace.

The user expectation is:
- `bf doctor` (no flags) → read-only health check ✅
- `bf doctor --repair` → write operation (may rebuild) ✅
- `bf doctor --repair --flush-first` → write operation with explicit flush intent ✅

However, the bug occurs when:
- Workspace is healthy
- User runs `bf doctor --repair --flush-first`
- The function returns early (line 1003) because no rebuild is needed
- BUT before returning, it still flushes to JSONL (line 1011) if `--flush-first` is set

This means a "repair" command that repairs nothing still writes to disk. The comment says "The user explicitly asked to checkpoint; flushing is a safe, non-rebuild operation, so honor it even though no rebuild was needed."

While this behavior might be intentional (checkpoint on request), it blurs the line between read-only diagnostics and write operations. A true read-only audit would flag this as a violation.

### Root Cause

The `--flush-first` flag was designed to protect unflushed beads before a destructive rebuild. However, it's being used even when no rebuild happens, which makes it function like `bf sync --flush-only` but only in the context of a repair command.

## Recommendation

**FILE A NEW ISSUE BEAD** to address this violation:

The bead should fix one or both of:

1. **Remove flush from healthy-workspace early-return** - Don't flush when no rebuild is needed, even if `--flush-first` is set
2. **Clarify the contract** - Update documentation to explicitly state that `--flush-first` may write even on healthy workspaces

## Commands Not Audited

- `status` - Does not exist as a command in bead-forge
- `sync --status` - Does not exist as a flag (sync has `--flush-only` and `--import-only`)

## Conclusion

The audit found **one genuine violation** where a read-only diagnostic command performs a write operation:

**`bf doctor --repair --flush-first` on a healthy workspace writes to JSONL even though no repair is performed.**

This is subtle because the `--flush-first` flag implies write intent, but the overall operation is presented as a "repair" that doesn't actually repair anything. The early-return path should be truly read-only.
