# bf-3cfn: --session-id flag for velocity attribution

## Finding

The feature described in this bead is **already fully implemented**.

## Evidence

1. **CLI flag exists** (`src/cli/mod.rs:143-145`):
   ```rust
   /// Session ID (from bf claim output) for precise velocity attribution
   #[arg(long)]
   session_id: Option<String>,
   ```

2. **Passed to storage** (`src/cli/mod.rs:1115`):
   ```rust
   storage.close_issue(id, reason, "cli", session_id.as_deref())?;
   ```

3. **Used in close logic** (`src/storage/sqlite.rs:606-622`):
   ```rust
   pub fn close_issue(&self, id: &str, reason: &str, actor: &str, session_id: Option<&str>) -> Result<()> {
       // ...
       crate::velocity::update_session_on_close(tx, id, now, session_id)?;
   ```

4. **Velocity update handles session_id** (`src/velocity.rs:60-161`):
   - Finds session by session_id if provided
   - Falls back to bead_id lookup if session_id is None
   - Updates closed_at and duration_seconds
   - Recomputes velocity_stats

5. **Claim outputs session_id** (`src/cli/mod.rs:1322, 1329, 1335, 1365, 1378, 1407, 1414, 1420`):
   ```rust
   "session_id": session_id  // JSON format
   println!("{} (session_id: {})", bead_id, sid);  // Text format
   ```

## Verification

```bash
$ ./target/release/bf close --help
--session-id <SESSION_ID>  Session ID (from bf claim output) for precise velocity attribution
```

## Schema

The `worker_sessions` table has `session_id` with a unique index (`src/storage/schema.rs:278-284`):
```sql
session_id TEXT NOT NULL DEFAULT (lower(hex(randomblob(16)))),
```

Migration 4 in `apply_migrations()` adds `session_id` to existing databases.

## Conclusion

The bead description appears to have been written before this feature was implemented.
All functionality specified in plan §4B.6 is present and working.
