# bf-b8s5ha: Batch Operations Test Run Results

## Task
Run `test_p0_batch_operations_with_labels` to completion and capture the actual CLI output.

## Result
**Test did not compile.** Compilation failed with type errors before any tests could run.

## Key Compilation Errors

The build failed with 13 compilation errors, all related to type mismatches between `anyhow::Error` and `BeadForgeError`:

### 1. src/storage/sqlite.rs:1118 - import_jsonl type mismatch
```
error[E0308]: mismatched types
expected `Result<_, Error>`, found `Result<ImportResult, BeadForgeError>`
```
**Location:** `import_jsonl(jsonl_path, |issue| { ... })` call
**Issue:** Returns `Result<ImportResult, BeadForgeError>` but caller expects `Result<_, anyhow::Error>`

### 2. src/storage/sqlite.rs:1144-1145 - Storage methods type mismatches
```
|| self.list_dirty_issues().map_err(|e| anyhow::anyhow!(e)),
|| self.clear_dirty().map_err(|e| anyhow::anyhow!(e)),
```
**Issue:** Closures expect `Result<T, anyhow::Error>` but methods return `Result<T, BeadForgeError>`

### 3. src/storage/sqlite.rs:1149 - list_all_issues type mismatch
```
export_jsonl(jsonl_path, || self.list_all_issues().map_err(|e| anyhow::anyhow!(e)))?;
```
**Issue:** Same pattern - returns wrong error type

### 4. src/sync.rs:120 - clear_dirty type mismatch
```
|| storage.clear_dirty(),
```
**Issue:** Returns `Result<(), BeadForgeError>` but expects `Result<(), Error>`

### 5. src/sync.rs:216 - import_jsonl type mismatch
```
import_jsonl(&jsonl_path, |issue| { ... })
```
**Issue:** Same error as #1

### 6. src/doctor.rs (multiple lines) - export_jsonl type mismatches
Lines 1467, 1547, 1592, 1645, 1687, 1787 all have:
```
export_jsonl(&jsonl_path, || storage.list_all_issues()).unwrap();
```
**Issue:** `list_all_issues()` returns `Result<Vec<Issue>, BeadForgeError>` but `export_jsonl` expects `Result<Vec<Issue>, anyhow::Error>`

### 7. src/jsonl.rs:966 - Test error construction
```
Err::<UpsertResult, anyhow::Error>(anyhow::anyhow!("Database error"))
```
**Issue:** Creates `anyhow::Error` but should create `BeadForgeError`

## Root Cause
Recent changes to error handling created an impedance mismatch between:
- Storage methods that return `Result<T, BeadForgeError>`
- Functions/closures expecting `Result<T, anyhow::Error>` (or vice versa)

## Next Steps
This is purely a data collection step as per the bead instructions. No fixes attempted yet.
