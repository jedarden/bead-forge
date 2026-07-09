# SQLite LIMIT 0 Behavior Test Results

## Test Date
2026-07-02

## SQLite Version
3.46.1

## Findings

**LIMIT 0 returns empty results** - it is explicitly treated as "return at most 0 rows", which means "return no rows at all".

### Test Cases

1. **Basic SELECT with LIMIT 0:**
   ```sql
   SELECT * FROM test LIMIT 0;
   ```
   Result: Empty result set (no output)

2. **Normal LIMIT for comparison:**
   ```sql
   SELECT * FROM test LIMIT 5;
   ```
   Result: Returns 3 rows (all available data)

3. **Aggregate with LIMIT 0:**
   ```sql
   SELECT COUNT(*) FROM test LIMIT 0;
   ```
   Result: Empty result set (COUNT is not even returned)

4. **LIMIT 0 with OFFSET:**
   ```sql
   SELECT * FROM test LIMIT 0 OFFSET 2;
   ```
   Result: Empty result set (OFFSET has no effect when LIMIT is 0)

5. **Simple SELECT with LIMIT 0:**
   ```sql
   SELECT 1 AS test_column LIMIT 0;
   ```
   Result: Empty result set

## Conclusion

LIMIT 0 in SQLite **does not** mean "no limit" or unlimited results. It explicitly means "return zero rows". This is consistent with SQL standard behavior where LIMIT specifies the maximum number of rows to return, and 0 is a valid maximum.

## Implications for bead-forge

When building queries that use LIMIT, using 0 as a default or sentinel value for "no limit" would be incorrect. If we need to represent "no limit", we should:
- Omit the LIMIT clause entirely
- Use a very large number (e.g., `LIMIT -1` is not valid SQLite; use `LIMIT 9223372036854775807` or similar)
- Use a flag to conditionally include LIMIT in the query construction
