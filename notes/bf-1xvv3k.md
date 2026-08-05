# Verification: clap CLI Multi-Label Parsing (bf-1xvv3k)

## Date
2026-08-05

## Verification Summary
✅ **PASS** - The Create command properly supports multi-label parsing using clap v4's default behavior for `Vec<String>` fields.

## Create Command Label Field Configuration

### Field Definition (src/cli/mod.rs:97)
```rust
#[arg(long)]
label: Vec<String>,
```

### clap Attributes Analysis
- **Type**: `Vec<String>` - Enables repeated flag usage
- **Attribute**: `#[arg(long)]` - Basic flag declaration without `num_args`
- **clap v4 behavior**: Default `Append` action collects each `--label` occurrence into the vector

### Documentation (lines 88-95)
```rust
/// Labels
///
/// CLAP MULTI-VALUE PATTERN:
/// - `Vec<String>` enables repeated flag usage: `--label bug --label enhancement`
/// - clap v4's default Append action collects each occurrence into the vector
/// - Usage: `bf create --title "Fix bug" --label bug --label urgent --label priority`
/// - No `num_args` needed: empty Vec when flag is omitted, one value per flag when used
/// - Gotcha: labels with spaces must be quoted: `--label "multi word label"`
```

## Handler Wiring Verification

### Command Pattern Match (lines 1207-1225)
```rust
Commands::Create {
    title,
    type_,
    priority,
    description,
    assignee,
    label,        // ← Extracted from command
    json,
} => cmd_create(
    &beads_dir,
    title,
    type_,
    priority,
    description,
    assignee,
    label,        // ← Passed to handler
    json,
    no_auto_flush,
),
```

### cmd_create Function Signature (lines 1607-1616)
```rust
fn cmd_create(
    beads_dir: &PathBuf,
    title: String,
    type_: String,
    priority: i32,
    description: Option<String>,
    assignee: Option<String>,
    labels: Vec<String>,  // ← Received as labels (plural)
    json: bool,
    no_auto_flush: bool,
) -> Result<()>
```

### Assignment to Issue (line 1648)
```rust
issue.labels = labels;  // ← Assigned to Issue struct
```

## Comparison with Other Multi-Value Fields

### Similar Pattern in Create Command
- Create's `label` field uses minimal `#[arg(long)]` configuration
- Optional (no `required = true`): creating beads without labels is valid
- No `num_args` needed for repeated flag pattern

### Stricter Pattern in LabelCommands::Add (lines 961-977)
```rust
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```
- **Why different?** Add command requires at least one label
- `num_args = 1..` enforces "one or more" values
- `required = true` forces flag presence
- `short` enables `-l` shorthand

### Search Command Labels (line 626)
```rust
#[arg(short, long)]
label: Vec<String>,
```
- Same minimal pattern as Create
- Multiple labels OR'd together in search logic

## Usage Examples

### Valid Usage Patterns
```bash
# Single label
bf create --title "Fix bug" --label bug

# Multiple labels (repeated flag)
bf create --title "Fix bug" --label bug --label urgent --label priority

# No labels (empty Vec)
bf create --title "Fix bug"

# Multi-word label (quoted)
bf create --title "Fix bug" --label "high priority"
```

### Invalid Usage
```bash
# This passes each label as a separate word, not as repeated flags
bf create --title "Fix bug" --label bug urgent priority
# Only "bug" is captured; "urgent" and "priority" are parsed as positional args
```

## clap v4 Mechanics

### Default Action for Vec<String>
- clap's `Append` action is automatically used for `Vec<T>` fields
- Each occurrence of `--label <value>` appends to the vector
- No explicit `num_args` needed for the repeated flag pattern
- Empty Vec when flag is not provided

### When to Use num_args
- `num_args = 1..` - Require at least one value (enforces presence)
- `num_args(0..)` - Explicitly allow zero or more (already default for Vec)
- `num_args(N)` - Exact count (rare for repeated flags)

## Conclusion

The Create command's label field is correctly configured for multi-label parsing:
1. ✅ Field defined as `Vec<String>` at line 97
2. ✅ clap attribute `#[arg(long)]` enables repeated flag usage
3. ✅ Proper documentation in comment block (lines 88-95)
4. ✅ Correctly wired to cmd_create handler (lines 1213, 1222)
5. ✅ Assigned to Issue.labels at line 1648

**No adjustments needed** - the current configuration follows clap v4 best practices for optional repeated flags.

## Related Commands with Similar Patterns
- `Search.status` / `Search.type_` / `Search.label` - Repeated filter flags
- `Claim.workspace_paths` - Multiple path arguments
- `LabelCommands::Add.label` - Stricter variant with required/enforced
- `LabelCommands::Remove.label` - Same strict pattern
- `CommentsCommands.Add.text` - Positional multi-value (not flag-based)
