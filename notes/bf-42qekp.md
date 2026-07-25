# NEEDLE Test Infrastructure Survey for bead-forge Trace Capture

## Task: bf-42qekp
Survey current test infrastructure and trace capture requirements

## 1. Test Modules in ~/NEEDLE

### Core Test Infrastructure Files
- `src/test_runner.rs` - Basic test runner for executing `cargo test` commands
- `src/test_output.rs` - Test output capture utilities (`.test_outputs/` directory management)
- `src/cargo_test.rs` - Advanced cargo test execution with compilation error parsing
- `src/trace/mod.rs` - Full trace capture system for bead execution

### Test Files in ~/NEEDLE/tests/
```
cleanup_liveness_regression.rs
compilation_error_detection.rs
config_cli_tests.rs
heartbeat_validation.rs
integration_tests.rs
load_simulation_example.rs
mod.rs
otlp_integration.rs
p2_integration_tests.rs
p3_integration_tests.rs
p95_correctness.rs
process_discovery_integration.rs
property_tests.rs
real_br_integration_tests.rs
routing_integration.rs
sanitize_latency_assertion.rs
sigterm_heartbeat_cleanup.rs
stop_kills_process_tree.rs
telemetry_field_verification.rs
test_telemetry_write.rs
test_telemetry_write_debug.rs
tmux_fixture.rs
verify_bf_4390q.rs
verify_bash_wrapper_exclusion.rs
verify_deleted_binary_hot_reload.rs
verify_process_discovery.rs
workspace_fixtures.rs
```

## 2. Current Test Execution Method (cargo test invocation pattern)

### TestRunner (src/test_runner.rs)
```rust
pub struct TestRunner {
    workspace: PathBuf,
    timeout_secs: u64,        // Default: 300s
    extra_args: Vec<String>,
}
```
- Basic `cargo test` execution with `stdout(Stdio::piped())` and `stderr(Stdio::piped())`
- Returns `TestResult` with captured stdout/stderr, exit code, duration
- Supports timeout protection via `execute_with_timeout()`
- Example invocation: `runner.run_tests(&["--lib", "--test", "integration_tests"])`

### CargoTest (src/cargo_test.rs)
```rust
pub struct CargoTest {
    workspace: PathBuf,
    args: TestArgs,
    timeout_secs: u64,        // Default: 600s
}
```
- Advanced test execution with configurable `TestArgs` (target, filter, test_names, flags)
- Comprehensive `TestOutcome` with compilation error detection and parsing
- Methods:
  - `run()` - Basic execution
  - `run_with_output_files(test_name)` - Creates `.test_outputs/<test-name>/` files
  - `run_with_bead_trace(bead_id)` - **Integrated with TraceCapture**

## 3. Current stdout/stderr Capture Implementation

### TestOutput Module (`.test_outputs/` directory)
**Directory Structure:**
```
.test_outputs/
└── <test-name>/
    ├── stdout.txt      # Raw stdout from test execution
    ├── stderr.txt      # Raw stderr from test execution
    └── combined.txt    # Combined stdout + stderr with interleaving
```

**Key Methods:**
```rust
pub struct TestOutput {
    output_dir: PathBuf,   // .test_outputs/<test-name>
    enabled: bool,
}
// Methods: write_stdout(), write_stderr(), write_combined()
```

### TraceCapture Module (`.beads/traces/<bead-id>/` directory)
**Directory Structure:**
```
.beads/traces/<bead-id>/
├── trace.jsonl              # Structured trace events (one JSON object per line)
├── stdout.txt               # Raw stdout from agent/test process
├── stderr.txt               # Raw stderr from agent/test process
├── test-output.txt          # Processed test output (for test runs)
├── metadata.json             # Timing, tokens, cost, template version
├── test_metrics.json        # Test execution metrics (exit code, duration, output sizes)
└── compilation_errors.json  # Detailed compilation error information
```

**Key Methods:**
```rust
pub struct TraceCapture {
    trace_dir: PathBuf,      // .beads/traces/<bead-id>
    enabled: bool,
    sanitizer: Option<Arc<Sanitizer>>,
}
// Methods: write_stdout(), write_stderr(), write_test_output(), 
//         write_metadata(), write_test_metrics(), write_compilation_errors()
```

### Integration Pattern
```rust
// From src/cargo_test.rs
pub fn run_with_bead_trace(&self, bead_id: &str) -> Result<TestOutcome> {
    let outcome = self.run()?;
    
    if let Some(trace) = TraceCapture::new(&bead_id, &self.workspace) {
        trace.write_stdout(&outcome.stdout)?;
        trace.write_stderr(&outcome.stderr)?;
        trace.write_test_metrics(&outcome.to_metrics(test_name))?;
        
        if !outcome.compilation_errors.is_empty() {
            trace.write_compilation_errors(&outcome.compilation_errors)?;
        }
        
        // Write metadata
        trace.write_metadata(&TraceMetadata {
            bead_id: bead_id.into(),
            exit_code: outcome.exit_code.unwrap_or(-1),
            outcome: if outcome.success() { "success" } else { "failure" },
            duration_ms: outcome.duration.as_millis() as u64,
            captured_at: Utc::now(),
            // ... additional fields
        })?;
    }
    
    Ok(outcome)
}
```

## 4. Gaps in Trace Capture for Test Execution

### Existing Coverage
✅ **Already Implemented in NEEDLE:**
- Full stdout/stderr capture during cargo test execution
- Test metrics (exit code, duration, output sizes) in JSON format
- Compilation error detection and structured storage (error code, variant, file location)
- Metadata tracking (bead_id, agent, provider, model, outcome, duration)
- Multiple output targets (`.test_outputs/` for test files, `.beads/traces/` for bead execution)

### Potential Gaps for bead-forge
❓ **Questions to Verify:**
1. **Test Execution Context**: Does bead-forge need its own test execution or does it run through NEEDLE's test infrastructure?
2. **Trace Granularity**: Does bead-forge need per-test traces (individual test functions) or is suite-level capture sufficient?
3. **CI Integration**: Is there specific trace capture needed for iad-ci Argo Workflow execution beyond the existing TraceCapture?
4. **Binary Build Traces**: Does bead-forge need separate trace capture for `cargo build --release` vs `cargo test`?

❌ **Missing from Current Implementation:**
- No per-test-function trace capture (only suite-level)
- No structured test result parsing (test names, individual pass/fail status)
- No coverage report integration
- No benchmark result capture (criterion benches)

## Summary

NEEDLE has comprehensive trace capture infrastructure that's already integrated with cargo test execution through `CargoTest::run_with_bead_trace()`. The system captures:

**Captured:**
- ✅ Full stdout/stderr streams from cargo test
- ✅ Test metrics (exit code, duration, output sizes)
- ✅ Compilation errors with structured metadata
- ✅ Bead execution metadata (agent, model, timing)

**Not Captured:**
- ❌ Individual test function results (pass/fail per test)
- ❌ Test coverage data
- ❌ Benchmark metrics

For bead-forge's purposes, the existing TraceCapture infrastructure appears sufficient if bead-forge tests are executed via NEEDLE's CargoTest integration. The main gap may be in structured test result parsing if bead-forge needs per-test-function granularity.

## Verification Needed

1. Confirm how bead-forge tests are executed (NEEDLE integration vs standalone)
2. Determine if per-test-function trace capture is required
3. Identify any CI/Argo Workflow specific trace requirements
