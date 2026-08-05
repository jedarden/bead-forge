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

## Practical Usage Examples

The following examples demonstrate real-world usage of multi-label commands in `bf`, including edge cases, error scenarios, and expected outputs.

### Creating Beads with Labels

#### Empty Labels (Default)

```bash
$ bf create --title "Simple task without labels"
Created bead bf-abc123
```

**Result:** Bead created with empty label array `[]`

#### Single Label

```bash
$ bf create --title "Bug fix" --label urgent
Created bead bf-def456

$ bf show bf-def456
ID: bf-def456
Title: Bug fix
Status: open
Labels: urgent
```

**Result:** Bead created with single label `["urgent"]`

#### Multiple Labels

```bash
$ bf create --title "Frontend performance issue" \
  --label P0 \
  --label urgent \
  --label frontend \
  --label performance
Created bead bf-ghi789

$ bf show bf-ghi789
ID: bf-ghi789
Title: Frontend performance issue
Status: open
Priority: 0
Labels: P0, urgent, frontend, performance
```

**Result:** Bead created with 4 labels `["P0", "urgent", "frontend", "performance"]`

#### Labels with Spaces (Quoting)

```bash
$ bf create --title "UX research task" --label "needs research"
Created bead bf-jkl012

$ bf show bf-jkl012 --format json | jq '.labels'
["needs research"]
```

**Result:** Single label with space `"needs research"`

**Common Error (Missing Quotes):**

```bash
$ bf create --title "UX task" --label needs research
Error: The argument '--label <LABEL>' requires a value but no value was supplied
```

**Explanation:** Without quotes, shell word splitting treats "research" as a separate argument.

### Searching with Multi-Label Filters

#### Empty Filter (Search All)

```bash
$ bf search --limit 2
[bf-search1] Database optimization task
[bf-search2] API documentation update
```

**Result:** Returns all beads regardless of labels

#### Single Label Filter

```bash
$ bf search --label urgent --limit 3
[bf-urgent1] Fix authentication bug
[bf-urgent2] Deploy hotfix to production
[bf-urgent3] Security patch for CVE-2025-12345
```

**Result:** Returns beads with "urgent" label

#### Multiple Label Filters (OR Logic)

```bash
$ bf search --label P0 --label urgent --limit 5
[bf-ghi789] Frontend performance issue
[bf-urgent1] Fix authentication bug
[bf-p0-1] Data migration stuck
[bf-p0-2] Memory leak in worker pool
[bf-urgent2] Deploy hotfix to production
```

**Result:** Returns beads with **any** of the specified labels (P0 **OR** urgent)

#### Combined Filters

```bash
$ bf search --status open --label P0 --label frontend --type bug --limit 3
[bf-bug1] Frontend validation error
[bf-bug2] Performance regression
[bf-bug3] Layout break on mobile
```

**Result:** Returns open bugs with P0 **OR** frontend labels

### Label Management Commands

#### Adding Labels (Multiple in Single Command)

```bash
$ bf label add --label P0 --label backend --label database bf-abc123
Added labels to bead bf-abc123: P0, backend, database

$ bf labels bf-abc123
Labels: P0, backend, database
```

**Result:** Three labels added atomically in one transaction

#### Adding Labels with Mixed Short/Long Forms

```bash
$ bf label add -l priority -l urgent --label "needs review" bf-abc123
Added labels to bead bf-abc123: priority, urgent, needs review

$ bf labels bf-abc123 --format json
["P0","backend","database","needs review","priority","urgent"]
```

**Result:** Labels can be added using both `-l` and `--label` forms

#### Removing Labels

```bash
$ bf label remove --label urgent --label backend bf-abc123
Removed labels from bead bf-abc123: urgent, backend

$ bf labels bf-abc123
Labels: P0, database, needs review, priority
```

**Result:** Specified labels removed, others preserved

#### Listing All Labels

```bash
$ bf label list
All unique labels (15):
  P0 (3 beads)
  P1 (7 beads)
  P2 (12 beads)
  backend (8 beads)
  frontend (5 beads)
  urgent (4 beads)
  performance (2 beads)
  database (6 beads)
  security (3 beads)
  documentation (9 beads)
  testing (4 beads)
  priority (2 beads)
  needs review (1 bead)
  migration (3 beads)
  bug (11 beads)
```

**Result:** Shows all labels across all beads with usage counts

### Error Cases and Edge Scenarios

#### Error: No Labels Provided (Required Command)

```bash
$ bf label add bf-abc123
error: The following required arguments were not provided:
  --label <LABEL>

Usage: bf label add --label <LABEL>... <ID>

For more information, try '--help'.
```

**Explanation:** `bf label add` requires at least one label due to `required = true, num_args = 1..`

#### Error: Bead Not Found

```bash
$ bf label add --label urgent bf-nonexistent
Error: Bead not found: bf-nonexistent
```

**Result:** Clear error message for non-existent bead ID

#### Error: Invalid Label Format (Empty Label)

```bash
$ bf label add --label "" bf-abc123
Error: Label cannot be empty
```

**Result:** Validation rejects empty label strings

#### Error: Too Many Labels (Practical Limit)

```bash
# Creating a bead with 50 labels (unusual but allowed)
$ bf create --title "Over-labeled task" \
  $(for i in {1..50}; do echo "--label label$i"; done)
Created bead bf-overflow123

Warning: Bead has 50 labels (high)
```

**Result:** Succeeds but warns about excessive labeling

### Integration Examples

#### Combining with Priority Filters

```bash
$ bf search --label P0 --priority-max 1 --limit 5
[bf-crit1] Database corruption
[bf-crit2] Service unavailable
[bf-crit3] Data loss bug
[bf-crit4] Security vulnerability
[bf-crit5] Performance regression
```

**Result:** High-priority beads with P0 label

#### Combining with Assignee Filters

```bash
$ bf search --label backend --assignee worker-7
[bf-w7-1] API endpoint optimization
[bf-w7-2] Database query improvement
[bf-w7-3] Cache invalidation fix
```

**Result:** Backend-labeled beads assigned to specific worker

#### Combining with Status Filters

```bash
$ bf search --label urgent --status in_progress --limit 3
[bf-prog1] Hotfix for login bug
[bf-prog2] Memory leak resolution
[bf-prog3] Deployment automation fix
```

**Result:** Urgent beads currently being worked on

### Batch Operations with Labels

#### Creating Beads with Labels via Batch

```bash
$ bf batch --json '[
  {"op": "create", "title": "Auth fix", "type": "bug", "priority": 0, "labels": ["urgent", "security"]},
  {"op": "create", "title": "UI polish", "type": "task", "priority": 2, "labels": ["frontend"]},
  {"op": "label_add", "id": "@0", "labels": ["backend"]}
]'
[
  {"op": 0, "status": "ok", "id": "bf-new1", "message": "Created bead bf-new1"},
  {"op": 1, "status": "ok", "id": "bf-new2", "message": "Created bead bf-new2"},
  {"op": 2, "status": "ok", "message": "ok: backend added to bf-new1"}
]
```

**Result:** Atomic batch with label operations using placeholder references

### Performance Considerations

#### Label Filtering Performance

```bash
# Fast search with label filter (indexed)
$ time bf search --label P0 --limit 10
real 0m0.045s

# Slower search without label filter (full scan)
$ time bf search --limit 10
real 0m0.128s
```

**Tip:** Label filters are optimized in the SQLite schema with indexes on `bead_labels.label`

#### Bulk Label Operations

```bash
# Efficient: Multiple labels in one command
$ bf label add -l a -l b -l c -l d -l e bf-abc123
# Single transaction, ~5ms

# Less efficient: Multiple commands
$ for label in a b c d e; do
  bf label add -l $label bf-abc123
done
# 5 transactions, ~25ms (5x slower)
```

**Tip:** Use single commands with multiple flags for better performance

### Cross-Reference to CLI Commands

The following commands support multi-label parsing as documented in the [CLI reference](README.md#commands):

| Command | Multi-Label Support | Documentation Reference |
|---------|-------------------|------------------------|
| `bf create --label` | Optional multi-value | [§ Commands: bf create](README.md#commands) |
| `bf search --label` | Optional multi-value (OR logic) | [§ Commands: bf search](README.md#commands) |
| `bf label add --label` | Required multi-value | [§ Commands: bf label add](README.md#commands) |
| `bf label remove --label` | Required multi-value | [§ Commands: bf label remove](README.md#commands) |
| `bf batch` (label_add op) | Array of labels in JSON | [§ Batch Operations: label_add](README.md#batch-operation-json-schema) |

## References

- clap v4 documentation: https://docs.rs/clap/latest/clap/
- Attribute macro reference: https://docs.rs/clap/latest/clap/_derive/index.html
- bead-forge source: `src/cli/mod.rs` (search for "CLAP MULTI-VALUE PATTERN")
- [CLI command reference](README.md#commands) — full command documentation
- [Batch operations schema](../batch-json-schema.md) — JSON-based multi-label operations
