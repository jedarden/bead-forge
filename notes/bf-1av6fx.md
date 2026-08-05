# Multi-Label Parsing Implementation Inventory

**Bead:** bf-1av6fx
**Date:** 2026-08-05

## Summary

Inventory of all CLI commands that use multi-label parsing in bead-forge. The implementation uses clap's `Vec<String>` type with various attribute patterns to support multiple values.

---

## Commands Using Multi-Label Parsing

### 1. Create Command (`bf create`)

**Location:** `src/cli/mod.rs:88-90`

**Pattern:**
```rust
/// Labels
#[arg(long)]
label: Vec<String>,
```

**Usage:**
```bash
bf create --title "My bead" --label bug --label urgent --label frontend
```

**Characteristics:**
- Optional field (no `required = true`)
- Long flag only: `--label`
- Repeatable flag
- No explicit `num_args` (defaults to clap's default behavior)
- Type: `Vec<String>`

---

### 2. Search Command (`bf search`)

**Location:** `src/cli/mod.rs:594-596`

**Pattern:**
```rust
/// Filter by label
#[arg(short, long)]
label: Vec<String>,
```

**Usage:**
```bash
bf search --label bug --label urgent
bf search -l bug -l urgent
```

**Characteristics:**
- Optional field
- Both short and long flags: `-l` / `--label`
- Repeatable flag
- No explicit `num_args` (defaults to clap's default behavior)
- Type: `Vec<String>`

---

### 3. Label Add Command (`bf label add`)

**Location:** `src/cli/mod.rs:932-934`

**Pattern:**
```rust
/// Label(s) to add (multiple labels supported)
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```

**Usage:**
```bash
bf label add bf-abc123 -l bug -l urgent
bf label add bf-abc123 --label bug --label urgent
```

**Characteristics:**
- **Required field** (`required = true`)
- Both short and long flags: `-l` / `--label`
- **Explicit `num_args = 1..`** (requires at least 1 value)
- Repeatable flag
- Type: `Vec<String>`

---

### 4. Label Remove Command (`bf label remove`)

**Location:** `src/cli/mod.rs:944-946`

**Pattern:**
```rust
/// Label(s) to remove (multiple labels supported)
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```

**Usage:**
```bash
bf label remove bf-abc123 -l bug
bf label remove bf-abc123 --label bug --label urgent
```

**Characteristics:**
- **Required field** (`required = true`)
- Both short and long flags: `-l` / `--label`
- **Explicit `num_args = 1..`** (requires at least 1 value)
- Repeatable flag
- Type: `Vec<String>`

---

## Other Multi-Value String Arguments (Similar Patterns)

For comparison, other fields use similar multi-value patterns:

### Search Command - Status Filter
```rust
/// Filter by status
#[arg(short, long)]
status: Vec<String>,
```
**Usage:** `bf search -s open -s blocked`

### Search Command - Type Filter
```rust
/// Filter by type
#[arg(short, long)]
type_: Vec<String>,
```
**Usage:** `bf search -t bug -t feature`

### Comments Add Command - Text
```rust
/// Comment text
#[arg(required = true, num_args = 1..)]
text: Vec<String>,
```
**Usage:** `bf comments add bf-abc123 This is a comment`

---

## Clap Attribute Patterns Identified

### Pattern 1: Simple Optional Multi-Value
```rust
#[arg(long)]
label: Vec<String>,
```
- Used in: `Create` command
- Characteristics: Optional, long form only

### Pattern 2: Optional Multi-Value with Short/Long
```rust
#[arg(short, long)]
label: Vec<String>,
```
- Used in: `Search` command (label, status, type_)
- Characteristics: Optional, both forms, no explicit num_args

### Pattern 3: Required Multi-Value with Explicit Minimum
```rust
#[arg(short, long, required = true, num_args = 1..)]
label: Vec<String>,
```
- Used in: `LabelCommands::Add`, `LabelCommands::Remove`, `CommentsCommands::Add`
- Characteristics: Required, both forms, explicit `num_args = 1..` (at least one)

---

## clap Configuration Analysis

### Value Parser
- **No explicit `value_parser` found** for label arguments
- clap automatically uses `std::str::FromStr` for `Vec<String>` when no parser is specified
- Default behavior: splits each `--label value` into separate vector elements

### `num_args` Semantics
- **Not specified** in Create/Search: accepts 0 or more values
- **`num_args = 1..`** in Label add/remove: requires at least 1 value
- clap's `num_args` syntax:
  - `1..` means "one or more"
  - Absence means use clap's default (typically 0 or more for `Vec<T>`)

### `required` vs `num_args` Interaction
- `required = true` alone: the argument itself must be provided at least once
- `num_args = 1..`: ensures at least one value is provided
- **Combined** (`required = true, num_args = 1..`): argument must be present and have ≥1 value

---

## Commands Supporting Multiple Labels

| Command | Flag | Required | Short | Pattern | Example |
|--------|------|----------|-------|---------|---------|
| `bf create` | `--label` | No | No | `Vec<String>` | `--label bug --label urgent` |
| `bf search` | `-l` / `--label` | No | Yes | `Vec<String>` | `-l bug -l urgent` |
| `bf label add` | `-l` / `--label` | Yes | Yes | `Vec<String>, num_args=1..` | `-l bug -l urgent` |
| `bf label remove` | `-l` / `--label` | Yes | Yes | `Vec<String>, num_args=1..` | `-l bug` |

---

## Data Flow

Labels flow through the system as follows:

1. **CLI Parsing**: clap collects repeated `--label` flags into `Vec<String>`
2. **Command Handler**: Receives `Vec<String>` parameter
3. **Storage Layer**: Labels are stored in `bead_labels` table (one row per label)
4. **Output**: Labels are displayed comma-separated or as JSON arrays

---

## Observations

1. **No explicit `value_parser`**: All label arguments rely on clap's default `Vec<String>` parsing
2. **No `num_args` on optional fields**: Create and Search commands don't specify `num_args`, relying on clap's default 0-or-more behavior
3. **Consistent naming**: All use `label: Vec<String>` (not `labels`)
4. **Short flag inconsistency**: Create uses `--label` only; Search and Label subcommands use both `-l` and `--label`
5. **Required patterns**: Only Label subcommands use `required = true` + `num_args = 1..`

---

## Related Files

- `src/cli/mod.rs` - CLI command definitions
- `src/model.rs` - `Issue` struct with `labels: Vec<String>` field
- `src/storage/schema.rs` - `bead_labels` table DDL
