# Verification: clap CLI Multi-Label Parsing

## Summary
Verified that the bead-forge CLI properly supports multi-label parsing via clap's Vec<String> type.

## Implementation Details

### Create Command Configuration (src/cli/mod.rs:67-95)

The `Create` command defines the label field as:
```rust
/// Labels
#[arg(long)]
label: Vec<String>,
```

**How it works:**
- clap automatically interprets `Vec<String>` fields as repeatable arguments
- No additional attributes (like `num_args`) are required - clap's default behavior handles multiple values
- Users can pass multiple labels by repeating the `--label` flag:
  ```bash
  bf create --title "My Bead" --label phase-1 --label priority --label backend
  ```

### Command Handler Wiring (src/cli/mod.rs:1548-1558)

The `cmd_create` function signature:
```rust
fn cmd_create(
    ...
    labels: Vec<String>,
    ...
) -> Result<()>
```

The labels are passed directly from clap parsing to the handler, then assigned to the issue:
```rust
issue.labels = labels;
```

### Documentation

The Create command's docstring (src/cli/mod.rs:62-66) already documents multi-label usage:
```rust
/// Create a new bead
///
/// Generates a unique short ID and prints it. Type defaults to "task" and
/// priority to 2 (Normal); 0 is Critical, 4 is Backlog. Pass --label
/// repeatedly to attach multiple labels.
```

## Test Coverage

Multiple tests verify multi-label parsing works correctly:

1. **tests/comprehensive_label_cli.rs:test_create_with_duplicate_labels**
   - Tests creating beads with multiple `--label` flags including duplicates
   - Example: `--label urgent --label urgent --label backend --label urgent --label backend`
   - Verifies labels are deduplicated to unique values: `["urgent", "backend"]`

2. **tests/test_epic_with_labels_cli.rs**
   - Tests multi-label creation for epics: `--label common --label epic1`

3. **tests/comprehensive_label_cli.rs** (lines 21-25)
   - Tests overlapping multi-label creation across multiple beads

## Comparison with Label Subcommands

For comparison, the `LabelCommands::Add` subcommand explicitly specifies `num_args(1..)`:
```rust
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```

This is functionally equivalent to the Create command's `Vec<String>` with default clap behavior - both support multiple values. The explicit `num_args(1..)` in LabelCommands::Add is for API clarity, not functional necessity.

## Conclusion

✅ **VERIFIED**: The Create command's label field (`label: Vec<String>`) with `#[arg(long)]` attribute properly supports multi-label parsing via clap's default behavior for Vec types.

✅ **DOCUMENTED**: Command docstring already instructs users to "Pass --label repeatedly to attach multiple labels."

✅ **TESTED**: Multiple tests verify the functionality works correctly, including duplicate handling.

**No changes needed** - the implementation is correct and complete.
