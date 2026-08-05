# Clap Multi-Value Parsing Configuration Guide

This document describes the clap v4 multi-value parsing patterns used throughout bead-forge (`bf`), with concrete examples and configuration notes.

## Overview

Clap v4 provides flexible multi-value argument parsing through `Vec<T>` types combined with various attributes like `num_args`, `required`, and value actions. The patterns below are used in bf for handling labels, search filters, workspace paths, and other repeated or collection arguments.

## Pattern 1: Basic Repeated Flag (Optional Multi-Value)

**Purpose:** Allow zero or more values via repeated flag usage  
**Used in:** `bf create --label`, `bf search --status`, `bf search --label`  
**Type:** `Vec<String>`  
**Attributes:** `#[arg(long)]` or `#[arg(short, long)]`

### Clap Configuration
```rust
#[arg(long)]
label: Vec<String>,
```

### Characteristics
- **Default behavior:** clap's `Append` action collects each flag occurrence
- **Optional:** No `required` attribute means the flag can be omitted entirely
- **No minimum count:** Empty `Vec` when flag is not used
- **One value per flag:** Each `--label value` appends one element to the vector

### Usage Examples
```bash
# No labels provided (empty Vec)
bf create --title "Fix bug"

# Single label
bf create --title "Fix bug" --label urgent

# Multiple labels via repeated flags
bf create --title "Fix bug" --label bug --label urgent --label priority

# Labels with spaces require quoting
bf create --title "Fix bug" --label "multi word label"
```

### Implementation Notes
- **No `num_args` needed:** clap's default behavior handles single values per flag
- **Gotcha:** Shell word splitting affects parsing. `--label multi word` is TWO flags (`multi` and `word`), not one label with spaces
- **Storage layer:** The `Vec<String>` is stored as-is in the `Issue.labels` field

---

## Pattern 2: Required Multi-Value Flag

**Purpose:** Require at least one value with optional repetition  
**Used in:** `bf label add --label`, `bf label remove --label`  
**Type:** `Vec<String>`  
**Attributes:** `#[arg(short, long, required = true, num_args = 1..)]`

### Clap Configuration
```rust
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```

### Characteristics
- **`num_args = 1..`**: Requires one or more values per flag occurrence
- **`required = true`**: The flag itself must appear at least once
- **Two-level requirement:** Both the flag presence (`required`) and value count (`num_args`) are enforced
- **Flexible repetition:** Supports both single-flag-multi-value and multi-flag patterns

### Usage Examples
```bash
# Single label (required flag met with one value)
bf label add -l urgent <id>

# Multiple labels via repeated flags (one value each)
bf label add -l bug -l urgent <id>

# Multiple labels via single flag with multiple values
bf label add -l bug urgent <id>

# Mixed patterns also work
bf label add --label bug --label urgent priority <id>
```

### Implementation Notes
- **`num_args` semantics:** `1..` means "one or more" - clap validates at parse time
- **Per-occurrence vs total:** `num_args` applies per flag occurrence, not to the total count
  - `-l bug urgent` (2 args, 1 flag) = `["bug", "urgent"]`
  - `-l bug -l urgent` (2 flags, 1 arg each) = `["bug", "urgent"]`
- **Gotcha:** Order matters if the handler is order-sensitive (bf's handlers are not order-sensitive)
- **Validation:** clap rejects the command immediately if `--label` is missing or has zero values

---

## Pattern 3: Positional Multi-Value

**Purpose:** Collect all remaining positional arguments into a vector  
**Used in:** `bf comments add` (comment text)  
**Type:** `Vec<String>`  
**Attributes:** `#[arg(required = true, num_args = 1..)]`

### Clap Configuration
```rust
#[arg(required = true, num_args = 1..)]
text: Vec<String>,
```

### Characteristics
- **Positional (not a flag):** Values are parsed from command-line position, not after `--flag`
- **Greedy collection:** Consumes all remaining arguments until the next flag or end of input
- **Join in handler:** Values are typically joined with spaces in the command handler
- **Shell word splitting:** Applies before clap sees arguments

### Usage Examples
```bash
# Single word (Vec has one element)
bf comments add <id> fix

# Multiple words (each word becomes separate Vec element)
bf comments add <id> fix the bug

# Quoted multi-word (single Vec element with spaces)
bf comments add <id> "fix the bug"

# Mixed quoted and unquoted (three elements: "fix", "the bug", "now")
bf comments add <id> fix "the bug" now
```

### Implementation Notes
- **Handler processing:** In `cmd_comments_add`, the vector is joined: `text.join(" ")`
- **Quote handling:** Shell strips quotes before clap sees them, so `"the bug"` arrives as one element `the bug`
- **Gotcha:** `--flag "val ue"` is one arg (after shell processing), but positional args undergo the same shell processing
- **Use case:** Ideal for free-form text where users shouldn't need to quote every word

---

## Pattern 4: PathBuf Multi-Value (Flag-Based)

**Purpose:** Collect multiple file/directory paths via repeated flags  
**Used in:** `bf claim --workspace-paths`  
**Type:** `Vec<PathBuf>`  
**Attributes:** `#[arg(long)]`

### Clap Configuration
```rust
#[arg(long)]
workspace_paths: Vec<PathBuf>,
```

### Characteristics
- **Path validation:** clap validates each value as a valid path string
- **Same repetition pattern:** Works like Pattern 1 (basic repeated flag)
- **Type conversion:** clap converts `String` to `PathBuf` automatically
- **Optional:** No `required` or `num_args` means paths are optional

### Usage Examples
```bash
# No workspace paths (uses auto-discovery)
bf claim --any --assignee worker-1

# Single workspace path
bf claim --any --workspace-paths /path/to/ws --assignee worker-1

# Multiple workspace paths
bf claim --any --workspace-paths /path1 /path2 /path3 --assignee worker-1

# Paths with spaces require quoting
bf claim --any --workspace-paths "/path/with spaces" --assignee worker-1
```

### Implementation Notes
- **Type safety:** `PathBuf` provides type-safe path operations in handlers
- **Validation deferred:** Path existence is validated in the handler, not by clap
- **Gotcha:** clap only validates the path string format, not whether the path exists
- **Alternative:** Could use `num_args(1..)` for explicit "at least one path" requirement

---

## Pattern 5: Short & Long Flag Aliases

**Purpose:** Provide both short and long flag forms for the same multi-value argument  
**Used in:** `bf label add -l/--label`, `bf search -s/--status`, `bf search -t/--type`  
**Type:** `Vec<String>`  
**Attributes:** `#[arg(short, long)]` or with additional constraints

### Clap Configuration
```rust
// Optional variant (Pattern 1)
#[arg(short, long)]
status: Vec<String>,

// Required variant (Pattern 2)  
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```

### Usage Examples
```bash
# Long form
bf search --status open --status closed

# Short form
bf search -s open -s closed

# Mixed forms
bf search -s open --status closed

# With values
bf label add -l bug -l urgent <id>
bf label add --label bug --label urgent <id>
```

### Implementation Notes
- **No difference in behavior:** Short and long forms are functionally identical
- **User preference:** Users choose based on context (interactive vs scripts)
- **Documentation:** Both forms appear in `bf --help` output
- **Consistency:** bf uses consistent short letters: `-s` (status), `-t` (type), `-l` (label)

---

## Attribute Combinations Reference

### Basic Optional Multi-Value
```rust
#[arg(long)]                        // Optional, no repetition constraints
label: Vec<String>,                // Result: empty Vec or [value1, value2, ...]
```

### Required Multi-Value
```rust
#[arg(long, required = true)]       // Flag must appear, but needs num_args for values
```
⚠️ **Incomplete** - Use with `num_args` below

### Required with Minimum Values
```rust
#[arg(long, required = true, num_args = 1..)]
label: Vec<String>,                // Flag required, 1+ values per occurrence
```

### Optional with Range Constraints
```rust
#[arg(long, num_args = 1..)]       // Optional flag, but 1+ values if present
label: Vec<String>,                // Use rare: "if provided, provide at least one"
```

### Exact Count
```rust
#[arg(long, num_args = 2)]          // Exactly 2 values per flag occurrence
label: Vec<String>,                // Pattern: --label val1 val2 (unusual in bf)
```

### Positional Multi-Value
```rust
#[arg(required = true, num_args = 1..)]
text: Vec<String>,                 // Consumes all remaining positional args
```

---

## Common Gotchas and Solutions

### Gotcha 1: Spaces in Values
**Problem:** `--label multi word` is parsed as two separate labels (`multi`, `word`), not one.  
**Solution:** Quote the value: `--label "multi word"`  
**Root cause:** Shell word splitting happens before clap sees the arguments.

### Gotcha 2: Empty Values
**Problem:** `--label ""` produces an empty string element in the `Vec`.  
**Solution:** Handler validation or storage-layer filtering (bf doesn't filter these).  
**Note:** bf treats empty labels as valid (no validation layer rejects them).

### Gotcha 3: `num_args` Scope
**Problem:** `num_args = 1..` applies per flag occurrence, not to the total count.  
**Example:** With `#[arg(long, num_args = 1..)]`:
- `--label a b` (1 flag, 2 values) ✅ Valid
- `--label a --label b` (2 flags, 1 value each) ✅ Valid  
- Both produce `["a", "b"]`

### Gotcha 4: Flag vs Positional Mixing
**Problem:** Positional multi-values consume all remaining args, including flags.  
**Example:** `bf comments add <id> text --flag value` → `--flag` and `value` become part of the text.  
**Solution:** In bf, positional multi-values always come last in the command signature.

### Gotcha 5: Order Sensitivity
**Problem:** Some handlers care about flag order; most don't.  
**bf behavior:** Label and search filters are order-insensitive (joined/combined in handler).  
**Check:** Handler code for `.extend()`, `.append()`, or `join(" ")` patterns.

---

## Handler Implementation Patterns

### Pattern 1: Direct Assignment
```rust
Commands::Create { label, .. } => cmd_create(..., label, ...)
```
The `Vec<String>` is passed directly to the handler without modification.

### Pattern 2: Extension/Combination
```rust
let labels = ["base_label".to_string()];
labels.extend(user_labels); // Append user-provided labels
```
Used when there are default labels combined with user labels.

### Pattern 3: Joining
```rust
let text = text_vec.join(" "); // "word1 word2 word3"
```
Used in `bf comments add` to convert positional `Vec<String>` to a single string.

### Pattern 4: Iteration
```rust
for label in labels {
    storage.add_label(issue_id, label)?;
}
```
Used in `bf label add` to store each label individually in the database.

---

## Testing Multi-Value Arguments

### Unit Test Pattern
```rust
#[test]
fn test_multiple_labels() {
    let cli = Cli::parse_from(&[
        "bf", "create",
        "--title", "Test",
        "--label", "bug",
        "--label", "urgent",
    ]);
    assert_eq!(cli.command.unwrap().labels(), vec!["bug", "urgent"]);
}
```

### Integration Test Pattern
```bash
# Test basic multi-value
bf create --title "Test" --label a --label b
bf show $ID | grep "Labels: a, b"

# Test required multi-value (should fail without --label)
bf label add <id>  # Should error: "required flag not provided"

# Test positional multi-value
bf comments add <id> word1 "word 2" word3
bf show <id> | grep "word1 word 2 word3"
```

---

## Clap Version Notes

- **Version:** clap v4.x (as specified in `Cargo.toml`)
- **Default action:** `Append` for `Vec<T>` types
- **Alternative actions:** `Set` (replaces previous value), `Append` (default for Vec)
- **Documentation:** https://docs.rs/clap/latest/clap/

---

## Migration from Older Patterns

### From clap v3 to v4
- **No breaking changes** for the patterns used in bf
- `Vec<T>` behavior is consistent across v3 and v4
- `num_args` syntax: `min_values(1)` in v3 → `num_args = 1..` in v4

### From manual splitting to clap multi-value
**Before (manual):**
```rust
#[arg(long)]
labels: String, // Comma-separated: "bug,urgent,priority"

// Handler:
let labels: Vec<String> = labels.split(',').map(|s| s.to_string()).collect();
```

**After (clap native):**
```rust
#[arg(long)]
label: Vec<String>, // Repeated flags: --label bug --label urgent

// Handler: No splitting needed, use Vec directly
```

---

## References in bead-forge Codebase

| Location | Pattern | Command | Description |
|----------|---------|---------|-------------|
| `src/cli/mod.rs:90-97` | Pattern 1 | `bf create --label` | Basic optional multi-value |
| `src/cli/mod.rs:599-607` | Pattern 1 | `bf search --status` | Short/long optional multi-value |
| `src/cli/mod.rs:611-619` | Pattern 1 | `bf search --type` | Short/long optional multi-value |
| `src/cli/mod.rs:964-973` | Pattern 2 | `bf label add -l` | Required multi-value flag |
| `src/cli/mod.rs:985-992` | Pattern 2 | `bf label remove -l` | Required multi-value flag |
| `src/cli/mod.rs:1020-1029` | Pattern 3 | `bf comments add` | Positional multi-value |
| `src/cli/mod.rs:320-328` | Pattern 4 | `bf claim --workspace-paths` | PathBuf multi-value |

---

## Summary

Bead-forge uses four primary clap multi-value patterns:

1. **Basic Repeated Flag** (optional, `Vec<String>`): Used for labels, search filters
2. **Required Multi-Value Flag** (required + `num_args = 1..`): Used for label add/remove
3. **Positional Multi-Value** (positional, `num_args = 1..`): Used for comment text
4. **PathBuf Multi-Value** (`Vec<PathBuf>`): Used for workspace paths

All patterns rely on clap v4's default `Append` action for `Vec<T>` types, with optional constraints via `num_args` and `required` attributes. The patterns are well-documented inline in `src/cli/mod.rs` with usage examples and gotchas for each application.
