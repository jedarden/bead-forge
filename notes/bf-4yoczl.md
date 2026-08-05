# bf-4yoczl: format_issue assignee parameter verification

## Summary
✅ **VERIFIED**: The formatter.format_issue function correctly accepts and processes the assignee parameter across all formatters.

## Investigation Details

### 1. Issue Model (src/model.rs:469-470)
The `Issue` struct contains an `assignee` field:
```rust
/// Assigned user.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

### 2. Formatter Trait (src/format/mod.rs:112)
The `format_issue` method signature:
```rust
fn format_issue(&self, issue: &Issue) -> String;
```
This takes a reference to an `Issue`, which includes the `assignee` field.

### 3. Implementation by Formatter Type

#### JSON Formatter (src/format/json.rs)
- **Location**: `issue_to_value` function (line 27-37) calls `ensure_display_fields`
- **Key Function** (line 39-43):
  ```rust
  fn ensure_display_fields(map: &mut Map<String, Value>) {
      map.entry("assignee").or_insert(Value::Null);
      map.entry("labels").or_insert_with(|| Value::Array(vec![]));
  }
  ```
- **Behavior**: Guarantees the `assignee` key is **always present** in JSON output (as `null` when unset)
- **Tests**: `assignee_null_when_unset`, `assignee_and_labels_populated_when_present`

#### Text Formatter (src/format/text.rs:20-22)
- **Implementation**:
  ```rust
  if let Some(assignee) = &issue.assignee {
      s.push_str(&format!("Assignee: {}\n", assignee));
  }
  ```
- **Behavior**: Conditionally includes assignee field when present

#### Toon Formatter (src/format/toon.rs:21-23)
- **Implementation**:
  ```rust
  if let Some(assignee) = &issue.assignee {
      parts.push(format!("Assignee: {}", assignee));
  }
  ```
- **Behavior**: Conditionally includes assignee field when present

## Gap Analysis
**No gaps found**. All formatter implementations correctly handle the assignee parameter:
- ✅ Assignee parameter exists in Issue model
- ✅ format_issue accepts Issue reference (includes assignee)
- ✅ All formatters process assignee through format chain
- ✅ JSON formatter ensures assignee is always present (for CLI consumers)
- ✅ Text/Toon formatters conditionally display assignee
- ✅ Comprehensive test coverage exists

## Additional Notes
The JSON formatter's `ensure_display_fields` function is particularly noteworthy - it ensures downstream consumers can distinguish between "field not set" vs "field absent", which is important for JSON parsing consistency.
