# bf-3y9: Critical Path Cache Auto-Invalidation - Verification

## Summary

The critical_path_cache auto-invalidation was already fully implemented. This note verifies that all required invalidation points are in place.

## Required Scenarios (from plan)

1. **dep_add_blocker** → Cache recompute
2. **dep_remove_blocker** → Cache recompute
3. **Status transitions** → Cache recompute
4. **Close/reopen** → Cache recompute

## Implementation Verification

All invalidation happens via `invalidate_and_recompute_cache()` which:
1. Deletes all entries from `critical_path_cache`
2. Recomputes critical paths for all beads
3. Runs within the same BEGIN IMMEDIATE transaction

### Call Sites

| Function | Line | Scenario |
|----------|------|----------|
| `Storage::add_dependency()` | 777 | Adding a blocker dependency |
| `Storage::remove_dependency()` | 786 | Removing a blocker dependency |
| `Storage::update_issue()` | 454 | Status changes |
| `Storage::close_issue()` | 558 | Closing a bead |
| `claim::claim()` | 259, 354 | Claim operations (open → in_progress) |
| `Storage::create_issue()` | 316 | New bead creation (may add dependencies) |
| `Storage::update_issue_from_json()` | 534 | JSONL import (may change deps/status) |

## Conclusion

Task bf-3y9 requirements are already satisfied. No code changes needed.
