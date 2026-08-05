# Batch 1 Test Execution Results

**Executed:** 2026-08-05  
**Bead ID:** bf-3zzl0s  
**Total Modules:** 72  
**Command:** `cargo test <module>` (no output capture flags)

## Summary Statistics

- **Total:** 72 modules
- **Passed:** 38 modules (52.8%)
- **Failed:** 34 modules (47.2%)
- **Crashed/Timeout:** 0 modules

## Passed Modules (38)

envelope_coverage, envelope_helpers, envelope_integration_tests, epic7_p0_priority_labels_verification, epic_children_status, epic_cli, epic_cli_label_creation, epic_cli_label_display, epic_cli_label_mutate, epic_cli_label_sort_filter, epic_cli_labels, epic_complex_labels, epic_comprehensive, epic_default_priority, epic_description, epic_json_format, epic_label_edge_cases, epic_labels_priority_comprehensive, epic_p0_labels, epic_p1_integration, epic_type_basic, epic_with_labels, feature_default_priority, fleet_concurrency, integration_trace_stderr_timing, kill_worker_preserves_beads, label_integration_test, limit_zero, list_command_tests, migrate_git_reconstruction, p0_epic_creation, p0_epic_labels, p1_epic_creation, p1_priority_tests, p2_epic_creation, priority_p0_validation, readonly_commands, readonly_coverage_gaps

## Failed Modules (34)

autoflush_batch_claim_delete, autoflush_comprehensive_mutations, autoflush_diagnostics_and_rotation, autoflush_failure_contract, autoflush_mutation, autoflush_readonly, autoflush_wiring, batch_atomic, batch_cascade_and_rotation, batch_mitosis, batch_transaction_tests, bf_520v_json_format, br_isolation, bug_default_priority, claim_fallback, claim_race, claim_stress, close_reopen, close_reopen_cycle, comments_cli, comprehensive_label_cli, comprehensive_label_tests, concurrent_claim, count_command, description_update_test_infrastructure, dirty_tracking, doctor_reconcile, doctor_repair_unflushed, doctor_safety_stack, duplicate_label_test, label_list, label_removal_test, label_storage, label_tests

## Detailed Results Location

All per-module logs and detailed summaries are stored in:
`.beads/traces/bf-4kzs6h-remaining/`

Files:
- `passed.txt` - List of 38 passed modules
- `failed.txt` - List of 34 failed modules  
- `crashed.txt` - List of crashed/timeout modules (empty)
- `batch1-summary.md` - Detailed analysis
- `<module>.log` - Per-module execution logs

## Observations

1. **Module naming:** Test modules use file names; `cargo test <module>` searches for matching test functions, causing mismatches.

2. **Cargo-remote:** All tests detected uncommitted changes and used local execution under cgroup limits (CPUQuota=200%, MemoryMax=6G).

3. **No crashes:** All 72 modules completed without crashes or timeouts.

## Next Steps

Remaining ~71 modules await execution in Batch 2. Consider adjusting test approach for better discovery.
