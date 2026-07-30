# Claim and Concurrency Test Results - bf-4ml2m

## Execution Date
2026-07-23

## Tests Executed

### 1. claim_fallback.rs (24 tests)
Verified claim fallback behavior across multiple scenarios:
- Empty workspace handling
- Primary workspace exhaustion
- Multiple workspace fallback scenarios
- Pinned beads respect during fallback
- Dependency handling with fallback
- Velocity stats fallback to 1800s when empty
- CLI integration with exhausted workspace

**Result: 24 passed**

### 2. claim_race.rs (24 tests)
Verified race condition handling under concurrency:
- Empty workspace concurrent claims
- Priority preservation during concurrent access
- Dependency handling in concurrent scenarios
- Ephemeral beads in concurrent claims
- Pinned beads with concurrent access
- Stale reclamation under concurrency
- High frequency claim attempts
- Rapid claim/release cycles
- Thundering herd with 20 workers (no duplicates)

**Result: 24 passed**

### 3. concurrent_claim.rs (4 tests)
Verified concurrent claim behavior:
- Empty workspace concurrent claims
- Priority ordering under concurrency
- No duplicate claims
- Stale reclamation in concurrent scenarios

**Result: 4 passed**

### 4. fleet_concurrency.rs (3 tests)
Verified fleet-level concurrency:
- Concurrent creates with no silent bead loss
- Beads survive flush and reimport operations
- Concurrent claims with no double claim scenarios

**Result: 3 passed**

### 5. kill_worker_preserves_beads.rs (7 tests)
Verified worker kill scenarios and auto-flush protection:
- Default autoflush makes beads visible immediately
- Doctor repair force on healthy workspace preserves dirty beads
- Doctor repair on unflushed-only is a safe no-op
- Flush failure surfaces warnings in human output
- Flush failure surfaces warnings in JSON output
- Doctor repair with flush-first preserves dirty beads
- Killed worker between mutation and flush loses nothing

**Result: 7 passed**

## Summary
**Total: 62 tests passed, 0 failed**

All claim and concurrency integration tests verified successfully. The test suite confirms:
- Robust claim fallback behavior across multiple workspaces
- Safe race condition handling under high concurrency
- Proper concurrent claim behavior with priority preservation
- Fleet-level operations maintain data integrity
- Worker kill scenarios are protected by auto-flush
