# SQLite LIMIT 0 Behavior Test (Bead bf-66ub)

## Test Date
2026-07-02

## Question
What happens when SQLite receives a query with `LIMIT 0`?

## Test Results

### Test Queries
```sql
CREATE TABLE test (id INTEGER, name TEXT);
INSERT INTO test VALUES (1, 'a'), (2, 'b'), (3, 'c');

-- Test 1: LIMIT 0
SELECT * FROM test LIMIT 0;
-- Result: (empty - 0 rows)

-- Test 2: LIMIT -1
SELECT * FROM test LIMIT -1;
-- Result: 1|a, 2|b, 3|c (all rows)

-- Test 3: LIMIT 1
SELECT * FROM test LIMIT 1;
-- Result: 1|a (1 row)
```

### Summary
- `LIMIT 0` → **0 rows** (empty result set)
- `LIMIT -1` → **unlimited** (all rows)
- `LIMIT N` (N > 0) → exactly N rows

## Official SQLite Documentation

From [SQLite SELECT Documentation](https://sqlite.org/lang_select.html), Section 5 "The LIMIT clause":

> "If the LIMIT expression evaluates to a **negative value**, then there is **no upper bound** on the number of rows returned. **Otherwise**, the SELECT returns the first N rows of its result set only, where N is the value that the LIMIT expression evaluates to."

**Interpretation:**
- Negative LIMIT → unlimited
- Zero or positive LIMIT → returns exactly N rows (0 returns 0 rows)

## Historical Context

In older versions of SQLite (pre-2003), `LIMIT 0` was treated as "unlimited" — a "unixism" where 0 meant "disable the limit." This behavior differed from MySQL, PostgreSQL, and MSSQL Server, where `LIMIT 0` returns empty results.

The SQLite community discussed this in a [2003 mailing list thread](https://sqlite-users.sqlite.narkive.com/lHdLuW6d/select-from-table-limit-0) and eventually aligned with the SQL standard convention.

## Comparison with Other Databases

| Database | LIMIT 0 Behavior | Unlimited Sentinel |
|----------|------------------|-------------------|
| SQLite (modern) | 0 rows | -1 |
| MySQL | 0 rows | -1 |
| PostgreSQL | 0 rows | ALL (keyword) |
| MSSQL Server | TOP 0 returns 0 rows | No LIMIT clause |

## Conclusion

**Modern SQLite follows standard SQL behavior:**
- `LIMIT 0` returns an empty result set (0 rows)
- `LIMIT -1` (or any negative value) returns unlimited rows
- This is consistent with MySQL and other major SQL databases

**For bead-forge implementation:** If constructing LIMIT clauses dynamically, be aware that:
- Use `LIMIT 0` to get no results (e.g., for column metadata only)
- Use `LIMIT -1` or omit LIMIT entirely for unlimited results
- Use `LIMIT N` (N > 0) to get exactly N rows
