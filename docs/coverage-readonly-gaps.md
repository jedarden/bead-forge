# Coverage Gaps Analysis: Readonly Commands

**Generated:** 2026-07-23  
**Baseline Report:** `.tarpaulin/html/`  
**Threshold:** < 80% line coverage

## Summary

Out of 11 readonly commands analyzed, **3 have inadequate coverage** (< 80% line coverage):

| Command | Coverage | Status |
|---------|----------|--------|
| `critical-path` | 94.70% | ✅ Good |
| `doctor` | 89.77% | ✅ Good |
| `sync --status` | 88.41% | ✅ Good |
| `list` | 65.41%* | ⚠️ **Low** |
| `show` | 65.41%* | ⚠️ **Low** |
| `ready` | 65.41%* | ⚠️ **Low** |
| `labels` | 65.41%* | ⚠️ **Low** |
| `comments list` | 65.41%* | ⚠️ **Low** |
| `velocity` | 77.40% | ⚠️ **Low** |
| `commit_check` | 70.53% | ⚠️ **Low** |
| `status` | N/A | ❌ Does not exist |

*These commands share the same source file (`cli/mod.rs`) which has 65.41% line coverage.

---

## Commands with < 80% Coverage

### 1. `bf list` - 65.41%

**Source:** `src/cli/mod.rs` (cmd_list, ~109 lines)

**Coverage:** 65.41% line coverage in cli/mod.rs

**Tested:** ✅ Yes (`tests/readonly_commands.rs::test_list_variants`)

**Uncovered/Under-covered Areas:**

From baseline coverage analysis, the cli/mod.rs file has significant coverage gaps. While basic `list` functionality is tested, the following areas likely lack coverage:

1. **Annotation filtering** (`--annotation key=value`): Lines 1563-1572
   ```rust
   let annotation_filter = match annotation {
       Some(ref ann) => {
           let parts: Vec<&str> = ann.splitn(2, '=').collect();
           if parts.len() != 2 {
               return Err(anyhow!("Invalid annotation format. Use key=value"));
           }
           Some((parts[0].to_string(), parts[1].to_string()))
       }
       None => None,
   };
   ```
   - Error path for invalid format may not be tested
   - The annotation filter application in `--all` mode (lines 1615-1617)

2. **Archive mode with filters** (`--all` with filters): Lines 1599-1624
   - Multiple filter combinations when `--all` flag is used
   - Truncation logic (lines 1619-1623)
   - Filter application: status, type, assignee, priority, annotation (lines 1600-1623)

3. **Envelope output format:** Lines 1630-1651
   ```rust
   match output_format {
       OutputFormat::Json => {
           let jsonl = formatter.format_issues(&issues);
           if envelope {
               // Wrap in envelope with kind="list"
               let data = if jsonl.is_empty() { "[]".to_string() } else { /* ... */ };
               println!("{}", formatter.format_with_envelope("list", &data));
           } else { /* ... */ }
       }
       _ => { /* ... */ }
   }
   ```
   - The envelope wrapping logic for JSON output
   - Empty result handling in envelope mode

4. **Limit 0 (unlimited) behavior:** Lines 1594-1595
   ```rust
   filter.limit = limit.and_then(|l| if l == 0 { None } else { Some(l) });
   ```

---

### 2. `bf show` - 65.41%

**Source:** `src/cli/mod.rs` (cmd_show, ~87 lines)

**Coverage:** 65.41% line coverage in cli/mod.rs

**Tested:** ✅ Yes (`tests/readonly_commands.rs::test_show_variants`)

**Uncovered/Under-covered Areas:**

1. **Archive fallback:** Lines 1666-1672
   ```rust
   let issue = match storage.get_issue(id)? {
       Some(i) => i,
       None => {
           find_bead_in_archives(beads_dir, id)?
               .ok_or_else(|| anyhow!("Bead not found: {}", id))?
       }
   };
   ```
   - The archive fallback path when bead not found in database
   - Error case when bead not found in archives either

2. **JSON envelope output:** Lines 1676-1693
   ```rust
   if envelope {
       println!("{}", formatter.format_with_envelope("show", &json_str));
   } else {
       println!("[{}]", json_str);
   }
   ```
   - Envelope wrapping vs raw array output

3. **Toon format:** Lines 1695-1718
   ```rust
   "toon" => {
       println!("ID: {}", issue.id);
       println!("Title: {}", issue.title);
       // ... more fields
       if !issue.dependencies.is_empty() {
           println!("Dependencies:");
           for dep in &issue.dependencies {
               println!("  -> {} ({})", dep.depends_on_id, dep.dep_type);
           }
       }
   }
   ```
   - The toon format output for show command
   - Dependency printing in toon format

4. **Non-toon text format with dependencies:** Lines 1738-1743
   - Similar dependency printing logic in default text format

---

### 3. `bf ready` - 65.41%

**Source:** `src/cli/mod.rs` (cmd_ready, ~65 lines)

**Coverage:** 65.41% line coverage in cli/mod.rs

**Tested:** ✅ Yes (`tests/readonly_commands.rs::test_ready_variants`)

**Uncovered/Under-covered Areas:**

1. **Envelope output format:** Lines 1869-1884
   ```rust
   if envelope {
       let data = if jsonl.is_empty() { "[]".to_string() } else { jsonl };
       println!("{}", formatter.format_with_envelope("ready", &data));
   } else {
       if jsonl.is_empty() {
           println!("[]");
       } else {
           println!("{}", jsonl);
       }
   }
   ```
   - Empty vs non-empty JSONL handling
   - Envelope vs non-envelope output

2. **Toon format:** Lines 1894-1906
   ```rust
   "toon" => {
       for candidate in candidates {
           println!(
               "{}",
               crate::format::toon::format_ready_bead(/* ... */)
           );
       }
   }
   ```
   - Toon format for ready output

3. **Empty candidates handling:**
   - Both JSON and toon format when no ready beads available

---

### 4. `bf labels` - 65.41%

**Source:** `src/cli/mod.rs` (cmd_labels, ~8 lines handler + LabelCommands dispatch)

**Coverage:** 65.41% line coverage in cli/mod.rs

**Tested:** ✅ Yes (`tests/readonly_commands.rs::test_labels_variants`)

**Uncovered/Under-covered Areas:**

The `cmd_labels` handler is minimal (~8 lines at line 1317), but coverage gaps exist in:

1. **JSON format handling:** The `cmd_labels` handler may not have JSON format coverage

2. **Error paths:**
   - Invalid bead ID
   - Database errors during label queries

---

### 5. `bf comments list` - 65.41%

**Source:** `src/cli/mod.rs` (cmd_comments → CommentsCommands::List)

**Coverage:** 65.41% line coverage in cli/mod.rs

**Tested:** ✅ Yes (`tests/readonly_commands.rs::test_comments_list`)

**Uncovered/Under-covered Areas:**

1. **CommentsCommand dispatch logic:** Lines 1270, 924-946
   ```rust
   Comments(CommentsCommands)
   // ...
   Comments(CommentsCommands) => cmd_comments(&beads_dir, comments, no_auto_flush),
   ```

2. **cmd_comments handler:** Need to read the full implementation to identify gaps
   - Error handling paths
   - Format variations

---

### 6. `bf velocity` - 77.40%

**Source:** `src/velocity.rs` (354 lines total)

**Coverage:** 77.40% line coverage (274/354 lines)

**Tested:** ✅ Yes (`tests/readonly_commands.rs::test_velocity_variants`)

**Uncovered/Under-covered Areas:**

From `src/velocity.rs`, approximately 80 lines are uncovered. Key gaps include:

1. **Error handling in `parse_datetime`:** Lines 26-46
   - Empty string rejection (line 30-32) may not be fully tested
   - SQLite native format with T separator (line 38)

2. **`update_session_on_close` error paths:** Lines 88-161
   ```rust
   let session = tx.query_row(/* ... */).optional()?;
   let (claimed_at_str, model, harness, issue_type) = match session {
       None => return Ok(false), // May lack coverage
       Some(s) => s,
   };
   
   let claimed_at = match parse_datetime(&claimed_at_str) {
       Ok(dt) => dt,
       Err(_) => return Ok(false), // Parse error fallback
   };
   ```
   - Session not found case (line 122)
   - Parse failure fallback (line 134)

3. **`get_expected_seconds` fallback chain:** Lines 252-297
   ```rust
   // Try exact match first
   let result: Option<i64> = tx.query_row(/* ... exact match */)?;
   
   if let Some(seconds) = result { /* ... */ }
   
   // Fallback: model + issue_type (any harness)
   let result: Option<i64> = tx.query_row(/* ... */)?;
   
   if let Some(seconds) = result { /* ... */ }
   
   // Fallback: issue_type only (any model/harness)
   let result: Option<i64> = tx.query_row(/* ... */)?;
   ```
   - The two fallback queries may not be tested (lines 273-295)

4. **`get_velocity_stats` dynamic query building:** Lines 343-391
   ```rust
   if let Some(model) = model_filter {
       query.push_str(&format!(" AND model = ?{}", param_idx));
       params.push(model.to_string());
       param_idx += 1;
   }
   
   if let Some(harness) = harness_filter {
       query.push_str(&format!(" AND harness = ?{}", param_idx));
       params.push(harness.to_string());
       param_idx += 1;
   }
   ```
   - Dynamic query construction with both filters (lines 356-366)

5. **Empty result handling in `get_all_velocity_stats`:** Lines 308-332
   - Empty stats list iteration

---

### 7. `bf commit-check` - 70.53%

**Source:** `src/commit_check.rs` (190 lines total)

**Coverage:** 70.53% line coverage (134/190 lines)

**Tested:** ❌ **Disabled** (`tests/readonly_commands.rs` line 228)
```rust
// NOTE: test_commit_check disabled - cmd_commit_check calls process::exit(0) which hangs tests
//test_readonly_command_with_exit!(test_commit_check, ["commit-check"], "bf commit-check");
```

**Uncovered/Under-covered Areas:**

From `src/commit_check.rs`, approximately 56 lines are uncovered. Key gaps include:

1. **No-commit repository handling:** Lines 32-56
   ```rust
   if !output.status.success() {
       let stderr = String::from_utf8_lossy(&output.stderr);
       if stderr.contains("does not have any commits yet") {
           // Repo has no commits yet - check if .beads/ files exist in index
           let ls_output = Command::new("git")
               .args(["ls-files", "--cached", ".beads/"])
               .output()?;
           // ...
       }
   }
   ```
   - The entire "no commits yet" fallback path (lines 33-55)

2. **`parse_diff_and_scan` edge cases:** Lines 71-139
   - Complex diff parsing logic with multiple branches
   - File path extraction (lines 84-94)
   - Hunk header parsing (lines 96-107)
   - Added line scanning (lines 108-122)
   - Context line tracking (lines 123-128)

3. **`scan_staged_files` (newly added files):** Lines 141-179
   ```rust
   fn scan_staged_files(scanner, files, beads_dir) -> Result<ScanResult> {
       for file in files {
           let output = Command::new("git")
               .args(["show", ":0", file])
               .output()?;
           // ...
       }
   }
   ```
   - Entire function for scanning files in repos without commits (lines 142-179)

4. **`format_scan_results` with multiple secrets:** Lines 182-203
   - Loop formatting multiple secret matches
   - May not have test with >1 secret found

5. **Test disabled due to `process::exit` issue:**
   - While the source code exists, it cannot be tested in integration tests due to the `process::exit(0)` call at line 2391
   - The command is designed for git pre-commit hooks where exit codes matter
   - Needs refactoring to return `Result<()>` instead of calling exit

---

## Commands with ≥ 80% Coverage

### ✅ `bf critical-path` - 94.70%

**Source:** `src/critical_path.rs` (396 lines total, 375 covered)

**Coverage:** 94.70% line coverage

**Status:** **Good coverage** - only ~21 lines uncovered

---

### ✅ `bf doctor` - 89.77%

**Source:** `src/doctor.rs` (1280 lines total, 1149 covered)

**Coverage:** 89.77% line coverage

**Status:** **Good coverage** - ~131 lines uncovered

---

### ✅ `bf sync --status` - 88.41%

**Source:** `src/sync.rs` (345 lines total, 305 covered)

**Coverage:** 88.41% line coverage

**Note:** `bf sync` does not have a `--status` flag. The readonly test is disabled:
```rust
// NOTE: test_sync_status disabled - bf sync does not have a --status option
//test_readonly_command!(test_sync_status, ["sync", "--status"], "bf sync --status");
```

**Status:** **Good coverage** - the `sync` command itself is well covered

---

## Commands That Do Not Exist

### ❌ `bf status`

The `status` command does not exist in bead-forge. The readonly test is disabled:
```rust
// NOTE: test_status_variants disabled - bf status command does not exist
```

If this command is needed, it would need to be implemented first.

---

## Recommendations

### High Priority

1. **Fix `bf commit-check` testing (70.53% coverage)**
   - Refactor `cmd_commit_check` to return `Result<()>` instead of calling `process::exit`
   - Create a separate wrapper for the pre-commit hook use case that calls exit
   - Enable integration tests in `readonly_commands.rs`

2. **Improve `bf velocity` coverage (77.40% → 80%+)**
   - Add tests for error fallback paths in `update_session_on_close`
   - Test the three-level fallback chain in `get_expected_seconds`
   - Add tests for dynamic query building with both model and harness filters

3. **Address cli/mod.rs coverage gaps (65.41% → 80%+)**
   - Add annotation filtering tests (error case and valid case)
   - Test `--all` mode with all filter combinations
   - Cover envelope output format for all readonly commands
   - Test archive fallback for `show` command
   - Add toon format tests for `show`, `ready`, and other commands

### Medium Priority

4. **Cover `parse_diff_and_scan` edge cases in commit_check**
   - Test diff parsing for files without commits
   - Test `scan_staged_files` function
   - Add tests with multiple secrets found

5. **Test error paths across all commands**
   - Invalid bead IDs
   - Database connection failures
   - Malformed input data

### Low Priority

6. **Implement and test `bf status` command** if needed
   - This would be a new feature, not a coverage gap fix

---

## Testing Infrastructure Notes

1. **Readonly command tests** are well-structured in `tests/readonly_commands.rs`
   - Uses macros to generate parametric tests
   - Tests JSONL immutability (file content unchanged after command)
   - Supports multiple variants per test

2. **Coverage data source:** `.tarpaulin/html/` (llvm-cov based)
   - Generated via: `cargo llvm-cov --html`
   - Provides line-by-line coverage data

3. **Known test limitation:**
   - Commands using `process::exit` cannot be tested directly
   - Requires refactoring to separate exit logic from business logic

---

## Next Steps

1. ✅ Document completed (this file)
2. ⏳ Fix `commit_check` exit code issue
3. ⏳ Add velocity coverage for fallback paths
4. ⏳ Improve cli/mod.rs coverage for filter combinations
5. ⏳ Add envelope and toon format tests
6. ⏳ Re-run coverage report after fixes
