# Formatter Pattern Documentation

## Overview

The formatter system in bead-forge provides a clean abstraction for outputting issues in different formats (text, JSON, toon). This pattern is used in `cmd_list()` and other commands that display issue data.

## The Pattern: How `get_formatter()` is Used

### Source Reference
`src/cli/mod.rs` lines 1087-1089 in `cmd_list()`:

```rust
let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
print!("{}", formatter.format_issues(&issues));
```

### Step-by-Step Breakdown

1. **Parse the format string** (from CLI argument like `--format json`)
   ```rust
   let output_format = OutputFormat::from_str(format).unwrap_or(OutputFormat::Text);
   ```
   - `OutputFormat::from_str()` returns `Option<OutputFormat>`
   - `.unwrap_or(OutputFormat::Text)` provides safe fallback to Text format
   - Valid values: `"text"`, `"json"`, `"toon"` (case-insensitive)

2. **Get the formatter implementation**
   ```rust
   let formatter = get_formatter(output_format);
   ```
   - Returns `Box<dyn Formatter>` - trait object with dynamic dispatch
   - Factory function in `src/format/mod.rs:43-49`
   - Maps enum to concrete formatter:
     - `OutputFormat::Text` → `TextFormatter`
     - `OutputFormat::Json` → `JsonFormatter`
     - `OutputFormat::Toon` → `ToonFormatter`

3. **Format the issues**
   ```rust
   print!("{}", formatter.format_issues(&issues));
   ```
   - Passes reference to issue slice: `&[Issue]`
   - Returns formatted `String`
   - Output format varies by formatter (see below)

## Formatter Trait Interface

Defined in `src/format/mod.rs:11-15`:

```rust
pub trait Formatter {
    fn format_issue(&self, issue: &Issue) -> String;
    fn format_issues(&self, issues: &[Issue]) -> String;
    fn format_error(&self, message: &str) -> String;
}
```

## Issue Struct Fields Required

### Core Fields (Used by All Formatters)

These fields are accessed by every formatter implementation:

| Field | Type | Usage in Formatters |
|-------|------|-------------------|
| `id` | `String` | Primary identifier, always displayed |
| `title` | `String` | Issue title, always displayed |
| `status` | `Status` | Workflow state (open/in_progress/blocked/etc.) |
| `priority` | `Priority` | Integer 0-4 wrapped in newtype |
| `issue_type` | `IssueType` | Category (task/bug/feature/etc.) |

### Optional Fields (Used Conditionally)

| Field | Type | Used By | When Displayed |
|-------|------|---------|----------------|
| `description` | `Option<String>` | TextFormatter, ToonFormatter | If `Some()` |
| `assignee` | `Option<String>` | TextFormatter, ToonFormatter | If `Some()` |
| `labels` | `Vec<String>` | TextFormatter, ToonFormatter | If non-empty |

### Relation Fields (Handled Differently)

| Field | Type | Behavior |
|-------|------|----------|
| `dependencies` | `Vec<Dependency>` | **Stripped** by JsonFormatter for br compatibility |
| `comments` | `Vec<Comment>` | **Stripped** by JsonFormatter for br compatibility |

### Timestamp Fields (For Serialization)

| Field | Type | Usage |
|-------|------|-------|
| `created_at` | `DateTime<Utc>` | Serialized by JsonFormatter, not used by text formatters |
| `updated_at` | `DateTime<Utc>` | Serialized by JsonFormatter, not used by text formatters |

## Per-Formatter Output Examples

### TextFormatter (`src/format/text.rs:29-38`)

**Single line format (used by `format_issues()`):**
```
[bf-abc123] Implement feature - in_progress (P1)
```

**Fields used:** `id`, `title`, `status`, `priority`

### JsonFormatter (`src/format/json.rs:17-29`)

**JSONL format (one JSON object per line):**
```json
{"id":"bf-abc123","title":"Implement feature","status":"in_progress","priority":1,"issue_type":"task","created_at":"2026-07-04T12:00:00Z","updated_at":"2026-07-04T12:00:00Z"}
{"id":"bf-def456","title":"Fix bug","status":"open","priority":0,"issue_type":"bug","created_at":"2026-07-04T11:00:00Z","updated_at":"2026-07-04T11:00:00Z"}
```

**Important:** JsonFormatter strips `dependencies` and `comments` vectors before serialization for br compatibility (lines 11-13, 21-23).

### ToonFormatter (`src/format/toon.rs:30-37`)

**Same as TextFormatter output:**
```
[bf-abc123] Implement feature - in_progress (P1)
```

## Complete Example: Minimal Issue for Formatting

```rust
use crate::model::{Issue, Priority, IssueType, Status};
use chrono::Utc;

// Minimal issue that works with all formatters
let issue = Issue {
    id: "bf-test".to_string(),
    title: "Test bead".to_string(),
    created_at: Utc::now(),
    updated_at: Utc::now(),
    status: Status::Open,
    priority: Priority(2),
    issue_type: IssueType::Task,
    ..Default::default()  // Fills in Option fields as None, Vec fields as empty
};

let issues = vec![issue];

let output_format = OutputFormat::from_str("text").unwrap_or(OutputFormat::Text);
let formatter = get_formatter(output_format);
let output = formatter.format_issues(&issues);
println!("{}", output);
// Output: [bf-test] Test bead - open (P2)
```

## Key Takeaways

1. **Always use `OutputFormat::from_str().unwrap_or(OutputFormat::Text)`** for safe parsing
2. **get_formatter() returns a boxed trait object** - use `&dyn Formatter` for flexibility
3. **Core Issue fields must be populated**: `id`, `title`, `status`, `priority`, `issue_type`, `created_at`, `updated_at`
4. **JsonFormatter strips relations** - dependencies and comments are removed for br compatibility
5. **Text and Toon formatters produce identical single-line output** - the difference is in their `format_issue()` (detailed view vs single line)

## Related Files

- `src/format/mod.rs` - Formatter trait, OutputFormat enum, get_formatter()
- `src/format/text.rs` - TextFormatter implementation
- `src/format/json.rs` - JsonFormatter implementation (JSONL output)
- `src/format/toon.rs` - ToonFormatter implementation
- `src/model.rs` - Issue struct definition (lines 404-544)
- `src/cli/mod.rs` - Usage in cmd_list() (lines 1087-1089)
