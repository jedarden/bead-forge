# bf-52n: NEEDLE bead_store claim verification

## Task
Verify NEEDLE bead_store uses `bf claim` instead of `br ready + br update` (racy pattern).

## Findings

### Already Implemented
NEEDLE has been updated to use `bf claim` via two mechanisms:

1. **`BrCliBeadStore::claim_auto()`** (lines 795-844)
   - Primary: Calls `run_bf_claim()` → `bf claim --assignee <actor> --json`
   - Fallback: Old `br ready + br claim` only if `bf` unavailable

2. **`BfCliBeadStore`** (lines 899-1268)
   - Dedicated `bf`-only implementation
   - `claim()` delegates to `claim_auto()`
   - `claim_auto()` uses `bf claim` directly with telemetry args

### Remaining Racy Code
`BrCliBeadStore::claim()` (lines 605-646) still uses old `br update` pattern, but:
- Only used when calling `claim()` explicitly
- Normal worker flow uses `claim_auto()` which is atomic

## Conclusion
✅ NEEDLE already uses `bf claim` for atomic bead selection in `claim_auto()`.
The race condition is eliminated for the standard worker claiming flow.
