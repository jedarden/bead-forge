# Stdout/Stderr Capture Implementation - bf-33evj2

## Acceptance Criteria ✓ - All Met

1. **Capture stdout from cargo test process**
   - Implemented in `src/trace.rs:750` via `String::from_utf8_lossy(&output.stdout)`
   - Available in `BeadTestResult.stdout` field

2. **Capture stderr from cargo test process**
   - Implemented in `src/trace.rs:751` via `String::from_utf8_lossy(&output.stderr)`
   - Available in `BeadTestResult.stderr` field

3. **Streams captured separately (not interleaved)**
   - Stdout and stderr are captured independently using `Command::output()`
   - Separate String buffers prevent interleaving
   - Verification shows clean separation

4. **Captured output available as in-memory buffer**
   - `BeadTestResult` struct contains `stdout: String` and `stderr: String` fields
   - Data remains in memory for processing before trace persistence

5. **Works with existing cargo test execution**
   - `TraceManager::run_cargo_test_to_bead_trace()` integrates capture with cargo test
   - `TraceManager::run_cargo_test_to_bead_trace_with_args()` supports custom arguments
   - Generic capture available via `subprocess::execute_command()`

## Implementation Locations

### Trace Module (`src/trace.rs`)

**Primary Function:** `run_cargo_test_to_bead_trace()` (lines 719-781)

```rust
pub fn run_cargo_test_to_bead_trace(
    &self,
    workspace_dir: &Path,
    bead_id: &str,
    metadata: &TraceMetadata,
) -> Result<BeadTestResult>
```

**Process:**
1. Execute `cargo test` with `.output()` to capture both streams (line 736)
2. Extract stdout: `String::from_utf8_lossy(&output.stdout).to_string()` (line 750)
3. Extract stderr: `String::from_utf8_lossy(&output.stderr).to_string()` (line 751)
4. Store in `BeadTestResult` as in-memory strings (lines 878-897)
5. Write to trace files for persistence via `write_bead_trace_to_path()` (line 770)

**Supporting Function:** `run_cargo_test_to_bead_trace_with_args()` (lines 799-862)
- Supports custom cargo test arguments
- Same capture mechanism

### Subprocess Module (`src/subprocess.rs`)

**Primary Function:** `execute_command()` (lines 167-239)

```rust
pub fn execute_command(
    command: &str,
    args: &[&str],
    config: SubprocessConfig,
) -> Result<SubprocessResult>
```

**Process:**
1. Configure stdout/stderr to `Stdio::piped()` for capture (lines 180-189)
2. Execute with `.output()` to capture both streams (line 203)
3. Return `SubprocessResult` with separate stdout/stderr fields (lines 217-227)
4. Configurable capture via `SubprocessConfig` (lines 64-73)

**Supporting Functions:**
- `execute_command_to_trace()` - Writes capture to trace files (lines 273-342)
- `execute_command_streaming()` - Line-by-line streaming capture (lines 372-492)

## Data Structures

### BeadTestResult (`src/trace.rs:881-897`)

```rust
pub struct BeadTestResult {
    pub exit_code: i32,
    pub start_time: Option<String>,
    pub end_time: Option<String>,
    pub duration_ms: u64,
    pub bead_trace_dir: PathBuf,
    pub stdout: String,      // ← In-memory stdout buffer
    pub stderr: String,      // ← In-memory stderr buffer
}
```

### SubprocessResult (`src/subprocess.rs:77-131`)

```rust
pub struct SubprocessResult {
    pub exit_code: i32,
    pub success: bool,
    pub stdout: String,      // ← In-memory stdout buffer
    pub stderr: String,      // ← In-memory stderr buffer
    pub duration_ms: u64,
    pub timed_out: bool,
    pub command: String,
    pub args: Vec<String>,
}
```

## Verification

### Test Results
All capture tests pass:
- `test_run_cargo_test_to_bead_trace` - Full integration test ✓
- `test_run_cargo_test_to_bead_trace_with_failure` - Error handling ✓
- `test_run_cargo_test_to_bead_trace_with_args` - Custom arguments ✓
- `test_multiple_runs_create_distinct_traces` - Multiple executions ✓
- `test_execute_command` - Generic subprocess capture ✓
- `test_execute_command_with_stderr` - Separate stderr capture ✓

### Manual Verification
```bash
# Capture verification shows proper separation:
✓ Stdout captured: 'stdout message'
✓ Stderr captured: 'stderr message'
✓ Streams captured separately:
  stdout lines: ["out1", "out2"]
  stderr lines: ["err1", "err2"]
✓ Output available as in-memory buffers
✓ Integration: trace.rs::run_cargo_test_to_bead_trace()
✓ Integration: subprocess.rs::execute_command()
```

## Key Implementation Details

1. **Separate Capture:** Using `Command::output()` automatically captures stdout and stderr into separate `Vec<u8>` buffers
2. **UTF-8 Handling:** `String::from_utf8_lossy()` handles potential encoding issues gracefully
3. **Memory Efficiency:** Data stays in memory until written to trace files
4. **Error Resilience:** Capture works regardless of exit code (success or failure)
5. **Configurability:** `SubprocessConfig` allows disabling capture for inherit-to-parent behavior

## Integration Points

- **Trace Persistence:** Captured output flows to `write_bead_trace_to_path()` for file storage
- **NEEDLE Integration:** `run_cargo_test_to_bead_trace()` called by needle agents for test execution
- **Generic Execution:** `execute_command()` available for any subprocess capture needs

## Dependencies

- Uses `std::process::Command` for subprocess execution
- Requires `chrono` for timestamp metadata
- Requires `serde_json` for metadata serialization
- Requires `anyhow` for error handling

## Status: COMPLETE ✓

All acceptance criteria met and verified. Implementation is production-ready and integrated with existing cargo test execution.
