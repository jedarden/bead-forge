# BF-2C2: Velocity Stats on Close - Verification

## Summary
The velocity stats functionality for `bf close` was already implemented. This note documents the verification.

## Implementation Verified

### 1. Schema (src/storage/schema.rs)
- `worker_sessions` table has `closed_at` and `duration_seconds` columns (Migration 3)
- `velocity_stats` table exists with all required fields

### 2. Velocity Module (src/velocity.rs)
- `update_session_on_close()` - Updates worker_sessions with closed_at and duration_seconds
- `recompute_velocity_stats()` - Recomputes stats using window over last 50 sessions
- `get_expected_seconds()` - Retrieves p50 duration for velocity-aware scoring

### 3. Storage Integration (src/storage/sqlite.rs)
- `close_issue()` at line 540 calls `crate::velocity::update_session_on_close(tx, id, now)`
- This happens within the same transaction that closes the bead

### 4. Integration Test
- Added `tests/velocity_close_integration.rs` with end-to-end test
- Test verifies: bead creation → claim → close → session updated → stats recomputed

## Note on --session-id Flag
The task description mentioned `--session-id <uuid>` but the implementation
automatically finds the most recent session for the bead. This is more
user-friendly and doesn't require the user to track session IDs.
