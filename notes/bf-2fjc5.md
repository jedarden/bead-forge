# bf-2fjc5: Fix text format envelope test failures

## Summary

Text format envelope tests were already passing. The tests were implemented and fixed in prior beads:

- `0ef48cb` (bf-17vd5): Add text format envelope tests
- `3f4a14a` (bf-16y4t): Fix non-JSON envelope test failures (text format is non-JSON)

## Verification

```bash
cargo test --test envelope_integration_tests envelope::text_format
# test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 34 filtered out; finished in 0.47s
```

### Tests Verified

All 19 text format envelope tests pass:

**Stats Command (5 tests):**
- stats_envelope_outputs_plain_text
- stats_envelope_output_matches_no_envelope
- stats_envelope_text_structure
- stats_envelope_empty_workspace
- stats_envelope_multiple_beads

**Claim Command (5 tests):**
- claim_envelope_outputs_plain_text
- claim_envelope_output_matches_no_envelope
- claim_envelope_empty_workspace
- claim_envelope_outputs_bead_id
- claim_envelope_structure_consistency

**List Command (4 tests):**
- list_envelope_outputs_plain_text
- list_envelope_output_matches_no_envelope
- list_envelope_empty_workspace
- list_envelope_shows_bead_info

**Ready Command (2 tests):**
- ready_envelope_outputs_plain_text
- ready_envelope_output_matches_no_envelope

**Show Command (3 tests):**
- show_envelope_outputs_plain_text
- show_envelope_output_matches_no_envelope
- show_envelope_shows_detailed_info

## Acceptance Criteria

✓ cargo test envelope::text_format passes
✓ All text format assertions succeed
✓ No panics or errors in text format tests
✓ Tests exist (added in prior bead bf-17vd5)
