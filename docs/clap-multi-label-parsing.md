# clap Multi-Value Parsing Patterns in bead-forge

This document explains the clap v4 multi-value argument patterns used throughout bead-forge, including attribute macro examples, common gotchas, and when to use each pattern.

## Overview

bead-forge uses clap v4's derive API for all CLI argument parsing. Multi-value arguments are handled through `Vec<T>` types with various configurations depending on the use case.

## Patterns

### Pattern 1: Optional Repeated Flags (Default Behavior)

**Use case:** Optional flags that can be specified multiple times, collecting all values into a vector.

```rust
#[arg(long)]
label: Vec<String>,
```

**Characteristics:**
- Empty `Vec` when flag is omitted
- One value per flag usage when used
- clap's default `Append` action collects each occurrence
- No `num_args` needed

**Examples:**
```bash
# No labels → empty Vec
bf create --title "Fix bug"

# Single label → Vec["bug"]
bf create --title "Fix bug" --label bug

# Multiple labels → Vec["bug", "urgent", "priority"]
bf create --title "Fix bug" --label bug --label urgent --label priority
```

**Gotcha:** Labels with spaces must be quoted:
```bash
# Correct: "multi word label" is one value
bf create --title "Fix bug" --label "multi word label"

# Incorrect: this would be two flags with two values
bf create --title "Fix bug" --label multi word label
```

**Real example:** `bf create --label` (lines 88-97 in `src/cli/mod.rs`)

---

### Pattern 2: Required Repeated Flags with `num_args = 1..`

**Use case:** Flags that must appear at least once and can be repeated, with validation that at least one value is provided per occurrence.

```rust
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```

**Characteristics:**
- `required = true`: The flag must appear at least once
- `num_args = 1..`: "One or more" values per flag occurrence
- clap validates minimum count at parse time
- Each flag occurrence appends to the Vec

**Examples:**
```bash
# Correct: Each flag has one value
bf label add -l bug -l urgent bf-123

# Correct: Single flag with multiple values
bf label add -l bug urgent priority bf-123

# Error: Flag is required
bf label add bf-123
```

**Gotcha:** `num_args` applies per occurrence, not total:
```bash
# Two args, one flag → ["bug", "urgent"]
bf label add -l bug urgent bf-123

# Two flags, one arg each → ["bug", "urgent"]
bf label add -l bug -l urgent bf-123

# Both produce the same Vec, but the parse tree differs
```

**Real example:** `bf label add -l` (lines 964-973 in `src/cli/mod.rs`)

---

### Pattern 3: Short and Long Flags with Repetition

**Use case:** Filters and options where both short and long forms should support repeated values.

```rust
#[arg(short, long)]
status: Vec<String>,
```

**Characteristics:**
- Both `-s` and `--status` work identically
- clap's `Append` action collects all occurrences
- Values are typically OR-combined in application logic
- No minimum count: zero occurrences yields empty `Vec`

**Examples:**
```bash
# Long form repetition
bf search --status open --status closed

# Short form repetition
bf search -s open -s closed

# Mixed short and long (allowed by clap)
bf search -s open --status blocked

# No filter → empty Vec (searches all statuses)
bf search
```

**Gotcha:** Order matters if the handler is order-sensitive:
```bash
# These produce different Vec order:
bf search -s open -s closed    # Vec["open", "closed"]
bf search -s closed -s open    # Vec["closed", "open"]
```

**Real example:** `bf search --status` (lines 599-607 in `src/cli/mod.rs`)

---

### Pattern 4: Positional Multi-Value Arguments

**Use case:** Commands where all remaining positional arguments should be collected into a vector.

```rust
#[arg(required = true, num_args = 1..)]
text: Vec<String>,
```

**Characteristics:**
- Positional (not a flag) - comes after the command and required args
- `num_args = 1..` collects all remaining arguments
- Application logic typically joins values with spaces
- Useful for free-form text input

**Examples:**
```bash
# Three words → Vec["word1", "word2", "word3"]
bf comments add bf-123 word1 word2 word3

# Mixed quoting → Vec["word1", "word with spaces", "word3"]
bf comments add bf-123 word1 "word with spaces" word3

# Handler joins with spaces: "word1 word with spaces word3"
```

**Gotcha:** Shell word splitting happens before clap sees arguments:
```bash
# Shell splits into three args → Vec["val", "ue"]
--flag val ue

# Shell keeps as one arg → Vec["val ue"]
--flag "val ue"
```

**Real example:** `bf comments add` (lines 1020-1029 in `src/cli/mod.rs`)

---

### Pattern 5: `Vec<PathBuf>` for Path Collections

**Use case:** Collecting multiple file or directory paths with validation.

```rust
#[arg(long)]
workspace_paths: Vec<PathBuf>,
```

**Characteristics:**
- clap parses each argument as a `PathBuf`
- clap validates that values are valid paths
- All arguments after the flag are collected until next flag
- Optionally can use `num_args(1..)` for explicit "at least one" requirement

**Examples:**
```bash
# Multiple workspace paths
bf claim --any --workspace-paths /path1 /path2 /path3

# Single path
bf claim --any --workspace-paths /path1

# No paths → empty Vec (uses auto-discovery in this case)
bf claim --any
```

**Gotcha:** Invalid paths fail at parse time:
```bash
# Error: clap cannot parse this as a PathBuf
bf claim --any --workspace-paths ""
```

**Real example:** `bf claim --workspace-paths` (lines 320-328 in `src/cli/mod.rs`)

---

## Attribute Macro vs Builder Pattern

All patterns in bead-forge use the **attribute macro** (derive) pattern:

```rust
#[derive(Parser)]
struct Cli {
    #[arg(short, long, num_args = 1..)]
    labels: Vec<String>,
}
```

The equivalent **builder pattern** would be:

```rust
let cmd = Command::new("bf")
    .arg(
        Arg::new("labels")
            .short('l')
            .long("label")
            .num_args(1..)
            .action(ArgAction::Append)
            .value_parser(value_parser!(String))
    );
```

bead-forge uses the attribute macro exclusively for consistency and type safety.

---

## Common Gotchas and Troubleshooting

### 1. Empty Vec vs. Missing Flag

A flag that's not present yields an **empty Vec**, not `None`:

```rust
#[arg(long)]
labels: Vec<String>,

// --help says: optional flag
// Behavior: absent → [], present with value → ["value"]
```

If you need `None` vs. `Some(Vec)`, use `Option<Vec<T>>`:

```rust
#[arg(long)]
labels: Option<Vec<String>>,

// Behavior: absent → None, present with value → Some(["value"])
```

### 2. `num_args` Applies Per Occurrence

The `num_args` setting validates per flag occurrence, not total:

```rust
#[arg(short, long, num_args = 1..)]
label: Vec<String>,

// These are ALL valid:
-l bug              // One occurrence, one value
-l bug urgent       // One occurrence, two values
-l bug -l urgent    // Two occurrences, one value each
```

### 3. `required = true` is Independent of `num_args`

```rust
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,

// required = true: The flag must appear at least once
// num_args = 1..: Each occurrence must have at least one value
```

### 4. Shell Word Splitting vs. clap Parsing

Remember that the shell processes quotes before clap sees arguments:

```bash
# Shell splits → clap sees: ["val", "ue"] → two values in Vec
--flag val ue

# Shell keeps together → clap sees: ["val ue"] → one value in Vec
--flag "val ue"
```

### 5. Order of Values

For repeated flags, **order is preserved**:

```bash
-s open -s closed    # Vec["open", "closed"]
-s closed -s open    # Vec["closed", "open"]
```

If your application logic is order-sensitive, document this behavior.

---

## Testing Multi-Value Arguments

When testing clap commands with multi-value arguments:

```bash
# Test empty Vec (flag absent)
bf create --title "Test"

# Test single value
bf create --title "Test" --label bug

# Test multiple values
bf create --title "Test" --label bug --label urgent

# Test spaces in values (must quote)
bf create --title "Test" --label "multi word"

# Test short forms
bf search -s open -s closed

# Test mixed short/long
bf search -s open --status blocked
```

---

## Choosing the Right Pattern

| Requirement | Pattern |
|-------------|---------|
| Optional, can repeat | `Vec<T>` with `#[arg(long)]` |
| Required, can repeat | `Vec<T>` with `#[arg(long, required = true, num_args = 1..)]` |
| Optional, single value only | `Option<T>` with `#[arg(long)]` |
| Required, single value only | `T` with `#[arg(long, required = true)]` |
| Collect all remaining args | `Vec<T>` with `#[arg(required = true, num_args = 1..)]` (positional) |
| Short and long forms | `Vec<T>` with `#[arg(short, long)]` |
| Path validation | `Vec<PathBuf>` with `#[arg(long)]` |

---

## References

- clap v4 documentation: https://docs.rs/clap/latest/clap/
- Attribute macro reference: https://docs.rs/clap/latest/clap/_derive/index.html
- bead-forge source: `src/cli/mod.rs` (search for "CLAP MULTI-VALUE PATTERN")
