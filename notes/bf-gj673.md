# bf-gj673 — Fix assignee-clearing gap (umbrella)

Umbrella tracking bead for the assignee-clearing gap (docs/plan/plan.md §3.4).
Closes when both children are closed and the previously-ignored assignee tests
are restored and green.

## Verification (2026-07-22)

Both child beads closed:
- **bf-4mj7l** — removed CLI-level assignee rejection; `validate_assignee` →
  `normalize_assignee` (pure `Option<&str>` → `Option<String>` trim/collapse-empty
  helper). `cmd_create` normalizes empty/whitespace → None; `cmd_update` relies on
  the storage-layer trim-and-NULL mapping.
- **bf-2uhsk** — `cmd_reopen` now builds `IssueChanges { assignee: Some(String::new()), .. }`
  so reopen explicitly clears a stale assignee (storage maps empty → NULL).

Confirmed in tree:
- `src/cli/mod.rs:13` imports `normalize_assignee`
- `src/cli/mod.rs:1342` `issue.assignee = normalize_assignee(assignee.as_deref())`
- `src/cli/mod.rs:1613` reopen sets `assignee: Some(String::new())`
- `src/validation.rs:38` defines `normalize_assignee`; no `validate_assignee` remains
- `tests/test_assignee_validation.rs` — no `#[ignore]` / "aspirational" markers left

Test results:
- `cargo build` — clean (exit 0)
- `cargo test --test test_assignee_validation` — 12 passed, 0 ignored
- `cargo test --test test_close_reopen_integration reopen` — 5 passed
  (incl. `test_reopen_clears_assignee`, `test_reopen_without_assignee_is_noop`)

Acceptance criteria met; no additional code changes required.
