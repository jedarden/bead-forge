# Bead bf-22u1wj: Add --format option to labels CLI

## Status: Already Implemented

This bead requested adding a `--format` option to the labels CLI command. Upon inspection, this feature is already fully implemented in the codebase.

## Implementation Verification

### Acceptance Criteria Met:

1. ✓ **Add --format argument to labels subcommand**
   - Location: `src/cli/mod.rs:591-592`
   - Code: `#[arg(short, long, default_value_t = LabelsFormat::Text)]`
   - Field: `format: LabelsFormat`

2. ✓ **Format enum supports: text, json**
   - Location: `src/cli/mod.rs:27-33`
   - Enum `LabelsFormat` with variants `Text` and `Json`
   - Derives `ValueEnum` for proper clap integration

3. ✓ **Default value is 'text' for backward compatibility**
   - Location: `src/cli/mod.rs:591`
   - Code: `default_value_t = LabelsFormat::Text`

4. ✓ **Argument is properly parsed and accessible to command handler**
   - Location: `src/cli/mod.rs:1458-1469`
   - Format is converted from `LabelsFormat` enum to string
   - Passed to `cmd_labels` function
   - Used in `cmd_labels` (lines 3092-3137) to conditionally output JSON vs text

## Implementation Details

The implementation includes:
- `LabelsFormat` enum with proper clap `ValueEnum` derive
- `Display` implementation for string representation
- Short flag `-f` and long flag `--format` support
- `--json` alias as convenience option
- Support in both single bead and all beads modes
- Proper JSON output structure for both modes

## Conclusion

All requirements from the original bead have been satisfied. The feature was implemented in a prior commit and is ready for use.
