# bead bf-4axlz: JSON formatter integration tests

## Task
Fix integration tests for JSON formatter verification

## Verification
All 10 integration tests in `tests/json_formatter_verification.rs` pass:

1. `issue_array_commands_share_formatter` - ✓ Byte-identical JSON across list/ready/search
2. `issue_arrays_are_jsonl_with_stable_fields` - ✓ JSONL format verified
3. `search_jsonl_is_consistent_with_list` - ✓ Search matches list output
4. `claim_emits_single_object` - ✓ Single JSON object (not Issue)
5. `claim_dry_run_emits_preview_object` - ✓ Preview fields present
6. `claim_empty_emits_empty_object` - ✓ Empty object `{}` when no beads
7. `stats_emits_single_object_with_optional_breakdowns` - ✓ Object with optional nested breakdowns
8. `velocity_emits_json_array` - ✓ JSON array output
9. `empty_result_behavior_is_as_documented` - ✓ Asymmetry documented
10. `json_alias_matches_format_flag` - ✓ --json equals --format json

Tests already passing - no fixes needed. Build is clean with no errors.
