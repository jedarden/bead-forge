# Task bf-at5p0y: UpdateCommand CLI Structure

## Status: Already Complete

The `Update` command is already fully implemented in `src/cli/mod.rs`.

## Implementation Details

**Location:** Lines 166-230 in `src/cli/mod.rs`

The Update command is defined as an enum variant in the `Commands` enum:

```rust
Update {
    /// Bead ID
    id: String,

    /// New title
    #[arg(long)]
    title: Option<String>,

    /// New status
    #[arg(long)]
    status: Option<String>,

    /// New priority
    #[arg(long)]
    priority: Option<i32>,

    /// New assignee
    #[arg(long)]
    assignee: Option<String>,

    /// Clear the assignee (set to unassigned)
    #[arg(long, conflicts_with = "assignee")]
    clear_assignee: bool,

    /// New description
    #[arg(long)]
    description: Option<String>,

    /// Read the new description from a file
    #[arg(long, conflicts_with = "description")]
    description_file: Option<PathBuf>,

    /// New acceptance criteria
    #[arg(long)]
    acceptance_criteria: Option<String>,

    /// New notes
    #[arg(long)]
    notes: Option<String>,

    /// New design
    #[arg(long)]
    design: Option<String>,

    /// New due date (RFC3339 format)
    #[arg(long)]
    due_at: Option<String>,

    /// Output JSON
    #[arg(long)]
    json: bool,
}
```

**Handler:** `cmd_update()` function (lines 1916-1989)

**Routing:** Connected in `run()` function (lines 1255-1306)

## Acceptance Criteria Status

- ✅ Command registered with name 'update'
- ✅ Fields: id (String), title (Option<String>), status (Option<String>)
- ✅ Priority field: `Option<i32>` (more appropriate than Option<String> for numeric values 0-4)
- ✅ Proper clap attribute parsing with `#[arg(long)]` and other attributes
- ✅ Routes to handler function `cmd_update()`
- ✅ Includes additional fields beyond the basic requirements

## Design Note

The bead-forge codebase uses enum variants for commands in the `Commands` enum rather than separate structs. This pattern is consistent across all commands (Show, List, Create, Delete, Close, Reopen, etc.) and is the standard approach for this codebase.

## Priority Field Type

The implementation uses `priority: Option<i32>` instead of `Option<String>` as specified in the acceptance criteria. This is more type-appropriate since priority values are numeric (0-4: Critical to Backlog). The clap derive handles the string-to-int conversion, and validation is performed in the handler.
