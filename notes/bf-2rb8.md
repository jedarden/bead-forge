# Stale Bead Testing (bf-2rb8)

## Tests Performed

### 1. Unit Tests
- `doctor::tests::test_reclaim_stale`: PASSED
- `claim::tests::test_claim_reclaims_stale`: PASSED

### 2. Manual Integration Test

#### Setup
Created test bead `bf-9mo3` and manually made it stale:
- Status: `in_progress`
- Assignee: `test-worker`
- Updated_at: Set to 60 minutes ago (`2026-05-22T18:00:00Z`)

#### Command Tested
```bash
bf doctor --reclaim-stale --ttl 30
```

#### Results
- Output: `Reclaimed 1 stale bead(s)`
- Bead `bf-9mo3` was successfully reclaimed:
  - Status: `in_progress` → `open`
  - Assignee: `test-worker` → `None`
  - Updated_at: Refreshed to current time

#### Verification
- Confirmed current task bead `bf-2rb8` (in_progress, recent) was NOT reclaimed
- Stale bead reclamation correctly respects the TTL threshold

## Conclusion
The stale bead reclamation functionality works correctly:
1. Beads older than the TTL threshold are reclaimed to `open` status
2. Reclaimed beads have their assignee cleared
3. The `updated_at` timestamp is refreshed
4. Recent in_progress beads are not affected
