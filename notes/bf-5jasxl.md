# Storage Layer NULL Assignee Handling (bf-5jasxl)

## Summary

**No code changes required** — the storage layer already properly handles `Option::None` for the assignee field.

## Verification

### 1. INSERT Operation (src/storage/sqlite.rs:368)

```rust
&issue.assignee,  // &Option<String> passed directly
```

rusqlite's `ToSql` trait automatically converts `Option::None` → SQL `NULL`. This requires no special handling.

### 2. UPDATE Operation (src/storage/sqlite.rs:583-591)

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

This code intentionally converts empty strings to NULL (to prevent empty assignee from hiding beads during claiming), which is correct behavior.

## Conclusion

The existing code already handles NULL assignee values correctly via:
1. Automatic rusqlite `Option::None` → `NULL` conversion in INSERT/SELECT operations
2. Intentional empty string → NULL conversion in UPDATE operations

No changes to `src/storage/sqlite.rs` were needed.
