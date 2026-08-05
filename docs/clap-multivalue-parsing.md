# clap CLI Multi-Value Parsing Configuration

This document describes how `bead-forge` (bf) configures clap for multi-value argument parsing, with specific focus on label handling and other repeatable CLI flags.

## Overview

`bead-forge` uses clap v4 for CLI argument parsing. Multi-value arguments are configured using `Vec<String>` field types with clap attributes that control minimum/maximum value counts and parsing behavior.

## Core Configuration Patterns

### Pattern 1: Optional Multi-Value (0 or more)

**Use case:** Arguments that may appear zero or more times (e.g., optional labels, filters).

```rust
#[derive(Parser)]
struct Create {
    /// Labels (optional, repeatable)
    #[arg(long)]
    label: Vec<String>,
}
```

**CLI usage:**
```bash
# No labels
bf create --title "My bead"

# Single label  
bf create --title "My bead" --label P0

# Multiple labels
bf create --title "My bead" --label P0 --label urgent --label frontend
```

**Behavior:**
- Default: empty `Vec<String>` if not provided
- Accepts: 0 or more values
- Parsing: clap automatically collects repeated flags into the Vec

**Used in:**
- `Create.label` - Line 90 in `src/cli/mod.rs`
- `Search.status` - Line 584
- `Search.type_` - Line 588  
- `Search.label` - Line 596

### Pattern 2: Required Multi-Value (1 or more)

**Use case:** Arguments that must appear at least once (e.g., commands that need at least one value).

```rust
#[derive(Subcommand)]
enum LabelCommands {
    /// Add label(s) to a bead
    Add {
        /// Label(s) to add (multiple labels supported)
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,
        
        id: String,
    },
}
```

**CLI usage:**
```bash
# Valid - single label
bf label add -l P0 bf-abc123

# Valid - multiple labels
bf label add -l P0 -l urgent -l frontend bf-abc123

# Invalid - no labels (error: required flag not provided)
bf label add bf-abc123
```

**Behavior:**
- Required: clap errors if flag not provided
- Minimum: `num_args = 1..` requires at least 1 value
- Maximum: No upper bound (accepts N values)
- Validation: clap enforces before main program logic

**Used in:**
- `LabelCommands::Add.label` - Line 933 in `src/cli/mod.rs`
- `LabelCommands::Remove.label` - Line 945
- `CommentsCommands::Add.text` - Line 973

## Key clap Attributes

### `num_args`

Controls the number of values accepted for an argument:

| Attribute | Values Accepted | Use Case |
|-----------|----------------|----------|
| `num_args = 1..` | 1 or more | Required multi-value |
| `num_args = 0..` | 0 or more | Optional multi-value (default for Vec) |
| `num_args = 1..=3` | 1 to 3 | Bounded multi-value |
| `num_args = 0..=1` | 0 or 1 | Optional single value |

**Note:** For `Vec<String>` without `num_args`, clap defaults to `0..` (optional, unlimited).

### `required`

Makes a flag mandatory:

```rust
#[arg(long, required = true)]
label: Vec<String>,
```

**Important:** `required = true` combined with `num_args = 1..` ensures:
1. The flag must be present
2. At least one value must be provided

**Interaction with Vec<String>:**
- `Vec<String>` + `required = true` (without `num_args`) - Flag required, but allows empty value `--flag`
- `Vec<String>` + `required = true` + `num_args = 1..` - Flag required, requires ≥1 value (preferred)

### `short` and `long`

Define short and long flag variants:

```rust
#[arg(short, long)]
label: Vec<String>,
```

**CLI usage:** Both forms work interchangeably:
```bash
bf label add -l P0 -l urgent bf-abc123
bf label add --label P0 --label urgent bf-abc123
```

## Implementation Details

### Storage Layer Integration

Multi-value arguments flow through the CLI layer to storage:

1. **CLI parsing:** clap collects values into `Vec<String>`
2. **Command handler:** Receives `Vec<String>` parameter
3. **Storage layer:** Stores labels as JSON array in `bead_labels` table

**Example from `cmd_create`:**
```rust
fn cmd_create(
    // ... other params
    labels: Vec<String>,  // Collected by clap from repeated --label flags
    // ...
) -> Result<()> {
    // ... 
    issue.labels = labels;  // Direct assignment to Issue struct
    storage.create_issue(&issue)?;
    // ...
}
```

### Search Command Multi-Filtering

The `Search` command uses optional multi-values for OR-combined filtering:

```rust
Search {
    #[arg(short, long)]
    status: Vec<String>,   // Filter by multiple statuses (OR)
    
    #[arg(short, long)]  
    type_: Vec<String>,    // Filter by multiple types (OR)
    
    #[arg(short, long)]
    label: Vec<String>,    // Filter by multiple labels (OR)
}
```

**CLI usage:**
```bash
# Find beads that are P0 OR urgent, bug OR task, with frontend OR backend label
bf search --status open --label P0 --label urgent --type bug --type task
```

**Implementation in `cmd_search`:**
```rust
let statuses: Vec<Status> = status
    .iter()
    .filter_map(|s| Status::from_str(s).ok())
    .collect();

let issues = storage.search_issues(
    query.as_deref(),
    &statuses,        // OR combined filtering
    &types,
    assignee.as_deref(),
    &label,           // OR combined filtering
    // ...
)?;
```

## Common Patterns and Gotchas

### Pattern: Empty String vs. Missing Flag

**Problem:** Distinguishing between `--label ""` and no `--label` flag.

**Solution:** clap treats both as empty Vec for optional multi-value. Validation happens at handler level:

```rust
// In cmd_create
let title_trimmed = title.trim();
if title_trimmed.is_empty() {
    return Err(anyhow!("Title cannot be empty or only whitespace"));
}
```

### Pattern: Multi-value with Other Flags

Multi-value arguments interact normally with other flags:

```rust
Create {
    #[arg(long)]
    title: String,
    
    #[arg(long)]
    label: Vec<String>,
    
    #[arg(long)]
    priority: i32,
}
```

**CLI usage:** Order doesn't matter
```bash
bf create --label P0 --title "My bead" --priority 0 --label urgent
```

### Pattern: Subcommand-specific Multi-values

Subcommands can have their own multi-value configuration:

```rust
#[derive(Subcommand)]
enum LabelCommands {
    Add {
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,
        id: String,
    },
    Remove {
        #[arg(short, long, required = true, num_args = 1..)]
        label: Vec<String>,  
        id: String,
    },
}
```

**CLI usage:**
```bash
bf label add -l P0 -l urgent bf-abc123
bf label remove -l old-label bf-abc123
```

## Testing

Multi-value parsing is tested in:

- `tests/test_cli_create_label_parsing.rs` - Basic create command label parsing
- `tests/test_p0_multilabel_cli.rs` - Integration tests for multi-label workflows
- `tests/comprehensive_label_cli.rs` - Comprehensive label CLI tests

**Example test pattern:**
```rust
#[test]
fn test_create_multiple_labels_parsing() {
    let args = vec![
        "bf", "create",
        "--label", "P0",
        "--label", "urgent", 
        "--label", "frontend",
        "--title", "Multi-label bead",
    ];
    
    let cli = Cli::parse_from(args);
    
    match cli.command {
        Commands::Create { label, .. } => {
            assert_eq!(label.len(), 3);
            assert!(label.contains(&"P0".to_string()));
        }
        _ => panic!("Expected Create command"),
    }
}
```

## Reference Implementation Locations

**Primary CLI definitions:** `src/cli/mod.rs`

- Line 90: `Create.label` - Optional multi-value
- Line 584: `Search.status` - Optional multi-value  
- Line 588: `Search.type_` - Optional multi-value
- Line 596: `Search.label` - Optional multi-value
- Line 933: `LabelCommands::Add.label` - Required multi-value with `num_args = 1..`
- Line 945: `LabelCommands::Remove.label` - Required multi-value with `num_args = 1..`
- Line 973: `CommentsCommands::Add.text` - Required multi-value with `num_args = 1..`

**Handler implementations:**
- `cmd_create` (Line 1552) - Processes `Create.label`
- `cmd_search` (Line 3081) - Processes search filter Vecs
- `cmd_label` - Processes label add/remove

## Migration Notes

When converting from single-value to multi-value parsing:

1. **Change field type:** `String` → `Vec<String>`
2. **Add `num_args`** if minimum required: `#[arg(num_args = 1..)]`
3. **Update handler logic:** Iterate over Vec instead of single value
4. **Update storage layer:** Expect array instead of scalar
5. **Add tests:** Cover 0, 1, and N value cases

**Example migration:**
```rust
// Before
#[arg(long)]
label: String,

// After  
#[arg(long)]
label: Vec<String>,
```

## clap Version Compatibility

These patterns are tested with clap v4. Earlier versions (clap v3, v2) have different attribute syntax:
- clap v2: `multiple = true` instead of `Vec<String>` type
- clap v3: Similar to v4 but some attribute differences
- clap v4: Current configuration (documented here)

**bead-forge uses:** clap v4 (see `Cargo.toml`)

## Summary

- **Optional multi-value:** `Vec<String>` without attributes (defaults to 0+)
- **Required multi-value:** `Vec<String>` with `num_args = 1..` and `required = true`
- **CLI interaction:** Users repeat flags: `-l a -l b` or space-separated values in some configs
- **Storage:** Vecs flow through to storage as JSON arrays
- **Validation:** clap handles count validation, handlers validate content

This configuration provides a clean, type-safe way to handle repeatable CLI flags in bead-forge.