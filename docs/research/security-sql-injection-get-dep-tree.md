# Security Vulnerability: SQL Injection in `get_dep_tree`

**Discovery Date:** 2026-08-10  
**Severity:** CRITICAL  
**Status:** Documented, fix pending  
**Bead:** bf-2sdtf4  

## Summary

A critical SQL injection vulnerability exists in the `get_dep_tree` function in `src/storage/sqlite.rs`. The vulnerability allows an attacker to execute arbitrary SQL queries by passing malicious input through the `root_id` parameter.

## Vulnerability Details

### Location
- **File:** `/home/coding/bead-forge/src/storage/sqlite.rs`
- **Function:** `get_dep_tree` (lines 1888-1998)
- **Vulnerable Code:** Lines 1934 and 1947

### Root Cause

The `root_id` parameter is directly interpolated into the SQL query string without proper parameterization or sanitization:

```rust
pub fn get_dep_tree(
    &self,
    root_id: &str,  // ← User-controlled input
    direction: &str,
    max_depth: usize,
) -> Result<Vec<DepTreeNode>> {
    // ...
    let sql = format!(
        "WITH RECURSIVE dep_tree AS (
            SELECT
                {id_col} as id,
                i.title,
                i.status,
                i.priority,
                0 as depth,
                d.type as dep_type,
                '{root_id}' || ',' || {id_col} as path  // ← VULNERABLE: Direct string interpolation
                // ...
        )
        // ...
    );
}
```

### Attack Vectors

An attacker can pass malicious input through `root_id` to:

1. **Information Disclosure:** Extract sensitive data from the database
2. **Data Modification:** Modify or delete database records
3. **Authentication Bypass:** Bypass access controls
4. **Denial of Service:** Crash the database or server

### Example Attack

If `root_id` is set to:
```
bf-123' UNION SELECT id, title, status, priority, 0, type, 'X' FROM issues WHERE '1'='1
```

The resulting SQL query would be:
```sql
WITH RECURSIVE dep_tree AS (
    SELECT
        d.depends_on_id as id,
        i.title,
        i.status,
        i.priority,
        0 as depth,
        d.type as dep_type,
        'bf-123' UNION SELECT id, title, status, priority, 0, type, 'X' FROM issues WHERE '1'='1' || ',' || d.depends_on_id as path
        -- ...
)
```

This would leak all issues in the database, bypassing the intended dependency tree query.

### Impact

- **Confidentiality:** HIGH - All bead data can be extracted
- **Integrity:** HIGH - Database can be modified or corrupted
- **Availability:** MEDIUM - Can cause denial of service

## Fix Requirements

### Immediate Fix (Priority: P0)

The fix must:

1. **Use Parameterized Queries:** Bind `root_id` as a parameter instead of string interpolation
2. **Validate Input:** Validate that `root_id` is a valid bead ID format
3. **Add Tests:** Add security tests to prevent regression

### Suggested Implementation

```rust
pub fn get_dep_tree(
    &self,
    root_id: &str,
    direction: &str,
    max_depth: usize,
) -> Result<Vec<DepTreeNode>> {
    let conn = self.conn.lock().unwrap();

    // Validate root_id format before using in query
    if !is_valid_bead_id(root_id) {
        return Err(BeadForgeError::validation(format!(
            "Invalid bead ID format: {}", root_id
        )));
    }

    // Build recursive CTE based on direction
    let (anchor_join, recursive_join, id_col, dep_col) = match direction {
        "up" => {
            (
                "d.depends_on_id = ?1",  // ← Use parameter placeholder
                "d.depends_on_id = rec.id",
                "d.issue_id",
                "d.depends_on_id",
            )
        }
        _ => {
            (
                "d.issue_id = ?1",  // ← Use parameter placeholder
                "d.issue_id = rec.id",
                "d.depends_on_id",
                "d.issue_id",
            )
        }
    };

    let depth_limit = if max_depth == 0 {
        String::new()
    } else {
        format!("AND rec.depth < {}", max_depth)  // ← Safe: max_depth is usize (controlled type)
    };

    let sql = format!(
        "WITH RECURSIVE dep_tree AS (
            SELECT
                {id_col} as id,
                i.title,
                i.status,
                i.priority,
                0 as depth,
                d.type as dep_type,
                ?2 || ',' || {id_col} as path  // ← Use parameter placeholder for root_id
            FROM dependencies d
            INNER JOIN issues i ON i.id = {id_col}
            WHERE {anchor_join}

            UNION ALL

            SELECT
                {id_col} as id,
                i.title,
                i.status,
                i.priority,
                rec.depth + 1 as depth,
                d.type as dep_type,
                rec.path || ',' || {id_col} as path
            FROM dependencies d
            INNER JOIN issues i ON i.id = {id_col}
            INNER JOIN dep_tree rec ON {recursive_join}
            WHERE rec.path NOT LIKE '%' || {id_col} || '%'
            {depth_limit}
        )
        SELECT id, title, status, priority, depth, dep_type, path
        FROM dep_tree
        ORDER BY depth, id"
    );

    let mut stmt = conn.prepare(&sql)?;
    
    // ← Bind root_id as parameter
    let mut rows = stmt.query(params![root_id, root_id])?;
    
    // ... rest of function
}
```

## Affected Code Paths

1. **CLI Command:** Any CLI command that calls `get_dep_tree` (likely `bf dep-tree` or similar)
2. **API Endpoints:** Any HTTP API that exposes dependency tree functionality
3. **Needle Fleet:** Any NEEDLE worker that uses dependency tree queries

## Testing Strategy

### Unit Tests Required

1. **SQL Injection Test:** Verify malicious input is rejected or properly escaped
2. **Validation Test:** Verify invalid bead IDs are rejected
3. **Regression Test:** Verify normal functionality still works after fix

### Example Test

```rust
#[test]
fn test_get_dep_tree_sql_injection() {
    let storage = setup_storage();
    
    // Test various SQL injection payloads
    let malicious_inputs = vec![
        "bf-123' OR '1'='1",
        "bf-123'; DROP TABLE dependencies; --",
        "bf-123' UNION SELECT * FROM issues WHERE '1'='1",
        "' OR '1'='1",
    ];
    
    for payload in malicious_inputs {
        let result = storage.get_dep_tree(payload, "down", 10);
        assert!(result.is_err() || matches!(result, Ok(ref tree) if tree.is_empty()));
    }
}
```

## Timeline

- **2026-08-10:** Vulnerability discovered and documented
- **Pending:** Fix implementation
- **Pending:** Security review of fix
- **Pending:** Release with fix

## References

- OWASP SQL Injection: https://owasp.org/www-community/attacks/SQL_Injection
- CWE-89: SQL Injection
- Related Beads: bf-1b2428 (P0 New Bug - triager: Critical security issue)

## Additional Notes

This vulnerability was discovered during investigation of bead bf-2sdtf4. The vulnerability exists in the current codebase and is exploitable if:
1. An attacker can control the `root_id` parameter
2. The application has database access
3. The `get_dep_tree` function is called with user-provided input

The fix should be implemented as a P0 security patch and released as soon as possible.
