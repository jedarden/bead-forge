# bf-3o9 — Repo hygiene: remove orphaned root-level test scripts & stray binary

Closure note for `bf-3o9`. This is the re-dispatch that verified and closed the bead;
the mechanical removal had already landed across three earlier bf-3o9 commits.

## What was removed (all 22 listed files + the throwaway binary's source)

All 23 files the task named are gone from **both** git tracking and the working tree
(verified: `git ls-files` finds none in the repo root; `ls test_*.sh test_version*`
returns nothing). They were removed in:

- `8dc4a79` — bulk removal of the 21 `test_bf_*.sh` / `test_bead_*` / `test_epic_*` /
  `test_p0_*` / `test_repair_*` scripts + `test_version` (4.2 MB compiled binary) +
  `test_version.rs`.
- `6937e4c` — the two remaining orphaned scripts `test_epic_labels.sh`,
  `test_labels_verify.sh` (not in the original 22-list but same residue class).
- `9fa1e19` — `.gitignore` recurrence guard (see below).

### `test_version` was NOT a release artifact

`test_version` was a throwaway debug build with its `test_version.rs` source checked in
beside it. The real `bf` release binary is `bf-linux-x86_64`, produced by the Argo
`bead-forge-build` WorkflowTemplate in `declarative-config` (not this repo root). Safe to
remove — nothing referenced it (no `grep` hit in `scripts/ deploy/ systemd/`, Cargo.toml,
or CI).

## Scenario coverage — verified, not assumed

Side-by-side audit done under child bead `bf-4081u7` (`notes/bf-4081u7.md`): each reader
opened the cited `tests/*.rs` and confirmed the scenario is genuinely exercised (test fn
names + assertion quotes), not inferred from filenames. All 6 categories map to >=1 real
test:

| Removed-script category | Covered by (tests/*.rs) |
|---|---|
| Epic type / description / implementation | `epic_cli.rs`, `epic_comprehensive.rs`, `epic_type_basic.rs`, `test_epic_type_creation.rs`, `test_epic_type_validation.rs` |
| P0 priority validation | `p0_epic_creation.rs`, `priority_p0_validation.rs`, `epic_p0_labels.rs`, `test_epic_p0_creation.rs` |
| Invalid type | `test_invalid_type.rs` |
| Repair / import round-trip | `doctor_repair_unflushed.rs`, `doctor_safety_stack.rs`, `migrate_git_reconstruction.rs`, `test_jsonl.rs`, `test_jsonl_roundtrip.rs` |
| Version display | `test_version_display.rs` (strictly stronger than the deleted throwaway) |
| Create / count / basic ops | `test_create.rs`, `test_create_command.rs`, `count_command.rs`, `test_basic_workflow.rs` |
| Labels (the two extra scripts) | `test_labels*.rs`, `comprehensive_label_tests.rs` |

No category was genuinely uncovered → **no port was required**.

## Recurrence guard

`.gitignore` now root-anchors (leading `/`) four patterns so they match ONLY files
directly in the repo root — legit tests under `tests/` and `src/**/tests/*.rs` stay tracked:

```
/test_*.sh
/test_*.py
/test_*.rs
/test_version
```

This pairs with the `root-ad-hoc-files` rule already in `repo_hygiene.sh` (defense in depth).

## State at closure

- Working tree: only this `notes/bf-3o9.md` added by this dispatch. The unrelated WIP in
  `src/batch.rs`, `src/cli/mod.rs`, `tests/test_json_edge_cases.rs`, etc. is other agents'
  work and was **not** touched (single-path commit, shared workspace).
- Blocker deps `bf-1ig29`, `bf-9ho17r`: both `closed`.
- All bf-3o9 commits pushed to `origin/needle/bf-5wku` (local == origin before this note).
