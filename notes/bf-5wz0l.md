# bf-5wz0l — Audit orphan-file scenario coverage against tests/

Read-only audit (parent bf-3o9, repo hygiene). For each of the 23 orphaned
root-level test files, confirmed whether the scenario it exercised is already
covered by a COMMITTED test under `tests/`.

## ⚠️ STATE-NOTE DISCREPANCY (read before child 2 / parent proceeds)

The bead description's STATE NOTE claims: *"a prior failed run already ran
`git rm` on ALL 23 target files, so the deletions are already staged in the
index."* **This is false in the current tree.**

```
$ git status --porcelain | grep test_        # (the verify command from the note)
                                            # -> EMPTY, exit 1
$ git status --porcelain | grep -E '^[A-Z]\s+test'   # -> no staged D lines
```

All 23 files are: tracked at HEAD (`git ls-tree HEAD`), present in the working
tree, and present in the index (no `D` status). The prior run's `git rm` did
NOT persist (index was reset / changes reverted). **The deletions still need
to be performed** — do not assume they are done. This child correctly did NOT
re-run `git rm` (read-only constraint respected).

## Method

Read every one of the 23 files (12 shell scripts + the 4k-line `test_version`
binary confirmed via `strings` to be the compiled ELF of `test_version.rs`,
rustc 1.95.0, containing `bf 0.2.0` + `test_version.rs`). For each scenario,
grep'd the 81 committed `tests/*.rs` files for the **actual CLI subcommand**
invocations (not prose matches), then side-by-side'd the closest test(s).

## Mapping (23 files)

| # | Orphan file | Scenario | Covering committed test | Status |
|---|-------------|----------|-------------------------|--------|
| 1 | test_bead_b_operations.sh | create/show/update/label/status/priority/show-json/**search**/list/close | test_basic_workflow, test_create, test_update_command, test_labels, close_reopen — **but `bf search` (Test 8) NOT covered** | ⚠️ PARTIAL — search gap |
| 2 | test_bf_10eb_invalid_type.sh | invalid/empty/numeric/special/unicode/uppercase type accepted + normalized; list-by-custom-type; JSONL persist | tests/test_invalid_type.rs (custom type create, multiple custom types, special chars, roundtrip) | ✅ COVERED |
| 3 | test_bf_13yz.sh | binary exists; `--help` has "bead-forge"; `--version` has "bf"; `.beads` init | tests/test_basic_workflow.rs, tests/test_version_display.rs | ✅ COVERED |
| 4 | test_bf_1rnkr_epic_type.sh | epic create, parent-child deps, blocking deps, type/status filter, JSONL, mixed child types | epic_type_basic, test_epic_type_creation, epic_comprehensive, verify_epic_implementation, test_epic_type_validation | ✅ COVERED |
| 5 | test_bf_2atz.sh | `bf list`, `bf show <id>` succeed | tests/test_basic_workflow.rs (list + test_bead_show_by_id) | ✅ COVERED |
| 6 | test_bf_3cd8.sh | create/show/update-desc/list/close/count | test_create, test_update_command, test_close_reopen, fleet_concurrency (count) | ✅ COVERED |
| 7 | test_bf_4ktoy_p0_priority_validation.sh | P0 epic priority (P0–P3), JSON, text "Priority: P0", filter, deps, JSONL | priority_p0_validation, p0_epic_creation, p0_epic_labels, test_epic_p0_creation, test_show_command | ✅ COVERED |
| 8 | test_bf_67ttv_epic_description.sh | epic w/ description field verify, listing, create-w-desc, JSON preserve | test_epic_with_description, test_create (test_create_with_description) | ✅ COVERED |
| 9 | test_bf_count.sh | `bf count`, `count --status`, `--workspace` | tests/fleet_concurrency.rs:52 (`run_bf(&["count"])`) | ✅ COVERED (core) |
| 10 | test_bf_create.sh | create minimal + all-params (type/prio/desc/assignee/labels), verify fields, multi-type, count, epic | test_create, test_create_command | ✅ COVERED |
| 11 | test_bf_kjwz7_epic_type.sh | near-duplicate of #4 (same epic suite) | same epic suite as #4 | ✅ COVERED |
| 12 | test_bf_lliyr_epic_implementation.sh | epic full options (assignee), multi child types, parent-child + blocking deps, dep tree, filters, sequential close, JSONL, custom-type child, priority order | verify_epic_implementation, epic_comprehensive, epic_cli, test_assignee | ✅ COVERED |
| 13 | test_bf_test1.sh | smoke: `--help` has "bead-forge", `--version` has "bf" | tests/test_basic_workflow.rs, tests/test_version_display.rs | ✅ COVERED |
| 14 | test_bf_test2.sh | CRUD smoke: create/list/show/update-status/close/count | test_basic_workflow, test_create, test_close_reopen, fleet_concurrency | ✅ COVERED |
| 15 | test_bf_test3.sh | `bf comments add` + `comments list` CLI round-trip | storage layer only: dirty_tracking.rs:204 (`list_comments`), br_isolation.rs (`comments` table exists). **No committed test exercises the `comments add`/`comments list` CLI end-to-end.** | ⚠️ NOT COVERED (CLI) |
| 16 | test_epic_functionality.sh | epic create, child tasks, parent-child + blocking deps, close children/epic, multi-epic, status filter, JSONL, mixed child types | epic_comprehensive, epic_type_basic, test_epic_type_creation | ✅ COVERED |
| 17 | test_epic_type_creation.sh | epic P0/P1/default priority, labels, JSON, description, type filter | test_epic_type_creation, test_epic_default_priority, epic_p0_labels, test_epic_with_description | ✅ COVERED |
| 18 | test_p0_epic_creation.sh | P0 epic, labels, assignee, filter, JSON, toon, `bf ready`, update priority, count | p0_epic_creation, p0_epic_labels, test_epic_p0_creation, test_show_command (toon), test_bf_5sw6/ready_json_fields (ready), test_assignee | ✅ COVERED |
| 19 | test_repair_bug2.sh | SQL-insert bead, flush, dirty_issues count, `doctor --repair --force`, import-only, doctor | doctor_repair_unflushed (--force loses / flush-first preserves), test_dirty_repair, dirty_tracking, br_isolation (import-only) | ✅ COVERED |
| 20 | test_repair_import_bug.sh | doctor-repair import `count_unflushed`/drift bug (14-line script, heredoc malformed — never ran clean) | doctor_repair_unflushed, br_isolation (sync import-only), jsonl_compat | ✅ COVERED (script was malformed) |
| 21 | test_repair_import_bug2.sh | SQL-insert, dirty_issues/export_hashes tracking, flush, `doctor --repair --force`, export_hashes check, doctor unflushed | doctor_repair_unflushed, test_dirty_repair, dirty_tracking (dirty_issues + export_hashes), br_isolation | ✅ COVERED |
| 22 | test_version.rs | standalone main() handling `--version`/`-V` → `bf 0.2.0` | tests/test_version_display.rs (test_version_flag_output, semver check) | ✅ COVERED |
| 23 | test_version (binary) | compiled ELF of test_version.rs (4.3 MB, rustc 1.95.0, `strings` shows `bf 0.2.0` + `test_version.rs`) | tests/test_version_display.rs (same as #22) | ✅ COVERED |

## Result

- **21 / 23 fully covered** by committed `tests/*.rs`.
- **2 scenario gaps to port in child 2:**
  1. **`bf search`** subcommand — exercised by `test_bead_b_operations.sh`
     (Test 8). `grep` for any `bf search` invocation across `tests/*.rs`
     returns nothing. No committed test covers the search subcommand.
  2. **`bf comments add` / `comments list`** CLI round-trip — exercised by
     `test_bf_test3.sh`. The `comments` table and storage-layer
     `list_comments()` are covered (br_isolation.rs schema check,
     dirty_tracking.rs:204), but no committed test drives the
     `comments add` → `comments list` CLI path end-to-end.

Everything else (CRUD, labels, assignee, epics in all flavors, P0 priority,
toon/json/text formats, `ready`, `count`, doctor/repair/flush/import/dirty
tracking, version/help smoke) is well covered by the committed suite.

## Notes

- `test_version` (#23) is the committed build artifact of `test_version.rs`
  (#22) — a 4.3 MB ELF binary checked into the repo. Definitely an orphan
  worth removing regardless of coverage.
- `test_repair_import_bug.sh` (#20) is malformed (heredoc `<<'EOF'` never
  closed at line 13; script truncates). It could not have passed as written,
  so its "scenario" was only ever covered elsewhere anyway.
- This child made NO working-tree or index changes (read-only). Only this
  notes file is added (committed) and a `br comment` posted (db-only,
  does not touch tracked `issues.jsonl`).
