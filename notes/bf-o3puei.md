# NULL Assignee Persistence Verification (bf-o3puei)

## Summary

The storage layer **already correctly handles NULL assignee persistence**. No code changes were needed - rusqlite's `ToSql` and `FromSql` traits automatically handle `Option<String>` → NULL conversion.

## Implementation Details

### 1. Storage (CREATE) - src/storage/sqlite.rs:422

```rust
&issue.assignee,  // Option<String> passed directly to SQL
```

**How it works:**
- `issue.assignee` is `Option<String>`
- rusqlite's `ToSql` trait implementation for `Option<T>` automatically converts:
  - `None` → SQL `NULL`
  - `Some(value)` → SQL string value

### 2. Storage (READ) - src/storage/sqlite.rs:1137

```rust
assignee: row.get(10)?,  // Reads from SQLite column 10
```

**How it works:**
- rusqlite's `FromSql` trait automatically converts:
  - SQL `NULL` → `None`
  - SQL string value → `Some(String)`

### 3. Update (CLEAR) - src/storage/sqlite.rs:637-646

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

**How it works:**
- When clearing assignee via `IssueChanges { assignee: Some(String::new()) }`
- The empty string is detected and explicitly stored as NULL
- This prevents empty strings from breaking unassigned bead claiming

## Verification

### Existing Test Coverage

**tests/test_assignee.rs:test_clear_bead_assignee** (lines 165-190)

This test already validates:
1. Create bead with assignee "charlie"
2. Clear assignee using `IssueChanges { assignee: Some(String::new()) }`
3. Verify `bead.assignee.is_none()` returns true

The test **passes**, confirming correct NULL persistence.

### Database-Level Verification

The implementation ensures:

1. **Create with None → NULL in database**
   ```sql
   INSERT INTO issues (..., assignee, ...) VALUES (..., NULL, ...)
   ```

2. **Read NULL → None in Rust**
   ```rust
   let assignee: Option<String> = row.get(10)?;
   // NULL becomes None
   ```

3. **Clear assignee → NULL, not empty string**
   ```sql
   UPDATE issues SET assignee = NULL WHERE id = ?
   ```

4. **Query unassigned beads works correctly**
   ```sql
   SELECT * FROM issues WHERE assignee IS NULL
   ```

## Why This Works

rusqlite provides built-in support for `Option<T>`:

```rust
// rusqlite's ToSql for Option<T>
impl<T: ToSql> ToSql for Option<T> {
    fn to_sql(&self) -> Result<ToSqlOutput<'_>> {
        match self {
            None => Ok(ToSqlOutput::Null),        // ← NULL in SQL
            Some(val) => val.to_sql(),             // ← value in SQL
        }
    }
}

// rusqlite's FromSql for Option<T>
impl<T: FromSql> FromSql for Option<T> {
    fn column_result(value: ValueRef<'_>) -> Result<Self> {
        match value {
            ValueRef::Null => Ok(None),           // ← None from NULL
            other => T::column_result(other).map(Some), // ← Some(value)
        }
    }
}
```

## No Breaking Changes

The existing implementation has no breaking changes:
- Setting assignee: `IssueChanges { assignee: Some("alice".to_string()) }`
- Clearing assignee: `IssueChanges { assignee: Some(String::new()) }`
- Reading unassigned: `bead.assignee.is_none()` returns true

All existing tests pass (9 tests in test_assignee.rs).

## Conclusion

✅ **Task Complete** - The storage layer already correctly persists NULL assignee values using rusqlite's built-in Option<T> support. No code changes required.
