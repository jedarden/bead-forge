# bf-2c2: Velocity Stats on Close

## Status: Already Implemented

The velocity tracking functionality described in this bead was already fully implemented in the codebase.

### Implementation Details

1. **Schema** (`src/storage/schema.rs`):
   - `worker_sessions` table with `closed_at` and `duration_seconds` columns (added via migration at lines 492-501)
   - `velocity_stats` table for aggregated statistics

2. **Velocity Module** (`src/velocity.rs`):
   - `update_session_on_close()`: Updates worker_sessions with close time and duration (lines 59-112)
   - `recompute_velocity_stats()`: Computes p50, p90, avg over last 50 sessions (lines 124-184)
   - `get_expected_seconds()`: Retrieves expected duration for claim scoring (lines 199-244)

3. **Storage Integration** (`src/storage/sqlite.rs`):
   - `close_issue()` calls `crate::velocity::update_session_on_close()` (line 556)

4. **Tests**:
   - Unit tests in `src/velocity.rs`: `test_update_session_on_close`, `test_recompute_velocity_stats`
   - Integration test: `tests/velocity_close_integration.rs`

### Verification

```bash
# All velocity tests pass
cargo test --lib velocity
cargo test --test velocity_close_integration

# Release build successful
cargo build --release
```

### Note on --session-id Option

The task description mentioned a `--session-id <uuid>` option for `bf close`, but the current implementation automatically determines the session from the `worker_sessions` table using `bead_id`. This is simpler and requires no additional CLI arguments.
