# Rusqlite NULL Handling for Option<String> Fields (bf-48ypxh)

## Summary

**rusqlite automatically converts Rust `Option::None` to SQL `NULL`** via its `ToSql` trait implementation. No special handling is required for the assignee field — it "just works."

## How rusqlite Handles Option<T>

rusqlite provides a blanket `ToSql` implementation for `Option<T>` where `T: ToSql`:

- `Option::Some(value)` → converts to the appropriate SQL value via `T::to_sql()`
- `Option::None` → converts to SQL `NULL`

This is automatic and requires no special code.

## Evidence from Codebase

### 1. Model Definition (src/model.rs:470)

```rust
/// Assigned user.
#[serde(default, skip_serializing_if = "Option::is_none")]
pub assignee: Option<String>,
```

The assignee field is `Option<String>`, matching other optional fields like `owner`, `description`, etc.

### 2. Reading from Database (src/storage/sqlite.rs:1075)

```rust
assignee: row.get(10)?,  // Direct get returns Option<String>
```

When reading, `row.get()` automatically converts SQL NULL to `Option::None` and SQL text to `Option::Some(value)`.

### 3. Writing to Database (src/storage/sqlite.rs:360)

```rust
params![
    // ...
    &issue.assignee,  // &Option<String> passed directly
    // ...
]
```

When writing, `&Option<String>` is passed to `params![]`. rusqlite's `ToSql` implementation handles the conversion automatically.

### 4. Explicit NULL Handling in Updates (src/storage/sqlite.rs:575-584)

```rust
if let Some(ref assignee) = changes.assignee {
    if assignee.trim().is_empty() {
        // Clearing stores NULL, never an empty string
        updates.push("assignee = NULL");
    } else {
        updates.push("assignee = ?");
        params.push(Box::new(assignee.clone()));
    }
}
```

This code explicitly assigns NULL when clearing assignee (empty string), but this is intentional behavior to ensure empty strings are never stored — they'd read back as "assigned" and hide beads from claiming.

## Consistent Pattern Across All Optional Fields

The same automatic NULL handling applies to all `Option<String>` fields:

- `assignee: Option<String>` (line 470)
- `owner: Option<String>` (line 474)
- `description: Option<String>` (line 442)
- `design: Option<String>` (line 446)
- `acceptance_criteria: Option<String>` (line 450)
- `notes: Option<String>` (line 454)
- `created_by: Option<String>` (line 485)
- `close_reason: Option<String>` (line 496)
- `closed_by_session: Option<String>` (line 500)
- `external_ref: Option<String>` (line 512)
- `source_system: Option<String>` (line 516)
- `source_repo: Option<String>` (line 520)
- `deleted_by: Option<String>` (line 526)
- `delete_reason: Option<String>` (line 528)

All of these use the same automatic `Option<T>` ↔ NULL conversion.

## Conclusion

**No code changes are needed** for assignee NULL handling. rusqlite's built-in `ToSql` implementation for `Option<T>` automatically handles the `Option::None` → NULL conversion. The existing code already works correctly.

The only explicit NULL assignment (in `update_issue`) is intentional: it ensures empty strings are converted to NULL rather than stored, which is application-specific logic for bead claiming behavior, not a rusqlite limitation.
