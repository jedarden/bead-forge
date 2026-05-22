# Stale Bead Reclamation Test Results

## Tests Verified

### Unit Tests (src/)

1. **`doctor::tests::test_reclaim_stale`** - Tests `reclaim_stale()` function
   - Creates a stale in_progress bead with updated_at 60 minutes ago
   - Reclaims with 30 minute TTL
   - Verifies bead is reset to open status with no assignee

2. **`claim::tests::test_claim_reclaims_stale`** - Tests automatic reclamation during claim
   - Creates stale in_progress bead and open bead
   - Performs claim operation
   - Verifies stale bead is reclaimed and claimed
   - Verifies `reclaimed` count in ClaimResult

### Integration Tests (tests/)

3. **`claim_race::test_concurrent_stale_reclamation`** - Tests concurrent reclamation
   - Creates 5 beads, sets 2 to stale in_progress
   - 10 workers claim simultaneously
   - Verifies all 5 beads are claimed (2 stale + 3 open)
   - Verifies stale beads were properly reclaimed

## CLI Commands Tested

- `bf doctor --reclaim-stale --ttl <N>` - Manual stale bead reclamation
- `bf claim` - Automatic stale reclamation as part of claiming

## All Tests Pass

```
running 6 claim tests ... ok
running 4 doctor tests ... ok
running 12 claim_race tests ... ok
```

## Implementation Details

Stale bead reclamation works by:
1. Inside `BEGIN IMMEDIATE` transaction
2. `UPDATE issues SET status='open', assignee=NULL WHERE status='in_progress' AND updated_at < now - claim_ttl`
3. Default claim_ttl is 30 minutes (configurable via `claim_ttl_minutes` in config.yaml)
