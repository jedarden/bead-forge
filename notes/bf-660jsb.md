# NEEDLE Test Infrastructure Survey

Bead: `bf-660jsb`  
Date: 2026-07-24

## Overview

NEEDLE has a comprehensive test infrastructure spanning unit tests, integration tests, end-to-end shell script tests, property-based tests, and performance benchmarks. This document provides a complete survey of all test modules, their structure, dependencies, and execution patterns.

## Test Directory Structure

```
~/NEEDLE/
├── tests/                    # Main integration test directory (30+ files)
│   ├── e2e/                  # End-to-end shell script tests (11 files)
│   ├── fixtures/             # Mock agent scripts and test fixtures
│   └── integration_t/        # Load simulation tests
├── benches/                  # Performance benchmarks (Criterion)
├── src/                      # Source code with embedded unit tests (50+ files)
└── scripts/                  # Test execution scripts
```

## 1. Integration Tests (`~/NEEDLE/tests/`)

### Core Phase Tests

| Test File | Size | Coverage |
|-----------|------|----------|
| `integration_tests.rs` | 84KB | Phase 1: Single worker cycle, 6 outcome paths, exhaustion, graceful shutdown, deterministic ordering |
| `p2_integration_tests.rs` | 65KB | Phase 2: Multi-worker fleet, claiming, crash recovery, mitosis, concurrency |
| `p3_integration_tests.rs` | 32KB | Phase 3: Weave, Unravel, Pulse, validation gates, hot-reload |
| `real_br_integration_tests.rs` | 69KB | Real `br` CLI integration with actual bead store operations |

### Specialized Integration Tests

| Test File | Purpose |
|-----------|---------|
| `routing_integration.rs` | Model-based adapter routing (Claude, GLM, workspace overrides) |
| `cleanup_liveness_regression.rs` | Regression tests for cleanup liveness detection |
| `compilation_error_detection.rs` | Compilation error parsing and classification |
| `otlp_integration.rs` | OpenTelemetry integration (requires Docker/testcontainers) |
| `property_tests.rs` | Property-based tests using proptest |
| `telemetry_field_verification.rs` | Telemetry event field validation |
| `workspace_fixtures.rs` | Workspace test fixtures and helper functions |

### Process Management Tests

| Test File | Purpose |
|-----------|---------|
| `sigterm_heartbeat_cleanup.rs` | Signal handling and graceful shutdown |
| `stop_kills_process_tree.rs` | Process tree termination behavior |
| `process_discovery_integration.rs` | Cross-process discovery mechanisms |
| `heartbeat_validation.rs` | Heartbeat file creation and refresh |

### Validation Tests

| Test File | Purpose |
|-----------|---------|
| `p95_correctness.rs` | P95 calculation algorithm correctness |
| `sanitize_latency_assertion.rs` | Sanitization latency threshold validation |
| `routing_matcher_baseline.rs` | Routing matcher behavior baseline |
| `verify_bf_4390q.rs` | Test output creation verification |
| `verify_deleted_binary_hot_reload.rs` | Binary hot-reload behavior |
| `verify_process_discovery.rs` | Process discovery verification |
| `verify_bash_wrapper_exclusion.rs` | Bash wrapper exclusion logic |

## 2. E2E Shell Script Tests (`~/NEEDLE/tests/e2e/`)

| Script | Purpose |
|--------|---------|
| `single_worker.sh` | Complete bead lifecycle: claim → dispatch → close → exit |
| `multi_worker.sh` | Multi-worker coordination with no duplicate claims |
| `graceful_shutdown.sh` | SIGTERM graceful shutdown behavior |
| `heartbeat_peer_recovery.sh` | Stale worker recovery via heartbeat mechanism |
| `auto_split.sh` | Auto-split at consecutive failure threshold |
| `failure_counter.sh` | Failure counter persistence |
| `failure_counter_persistence.sh` | Counter persistence across restart cycles |
| `forced_mitosis.sh` | Forced mitosis triggering |
| `strand_waterfall.sh` | Strand execution ordering |
| `telemetry_completeness.sh` | Telemetry event completeness validation |
| `additive_launch.sh` | Multi-worker startup coordination |

## 3. Unit Tests in Source Code

Found **1,147+ unit tests** embedded across 50+ source files:

| Module | Test Coverage |
|--------|--------------|
| `src/routing.rs` | Routing logic unit tests |
| `src/cargo_test.rs` | Cargo test execution and output parsing |
| `src/agent_event.rs` | Agent event handling |
| `src/prompt/mod.rs` | Prompt generation |
| `src/bead_store/mod.rs` | Bead store operations |
| `src/canary/mod.rs` | Canary deployment logic |
| `src/claim/mod.rs` | Atomic bead claiming |
| `src/cli/mod.rs` | CLI argument parsing |
| `src/config/mod.rs` | Configuration parsing |
| `src/dispatch/mod.rs` | Agent dispatch logic |
| `src/health/mod.rs` | Health checks and liveness |
| `src/mitosis/mod.rs` | Bead splitting logic |
| `src/outcome/mod.rs` | Outcome classification |
| `src/registry/mod.rs` | Concurrent registry access |
| `src/sanitize/mod.rs` | Trace sanitization |
| Plus 35+ more modules | |

## 4. Test Infrastructure Source Modules

| Module | Purpose |
|--------|---------|
| `src/test_runner.rs` | Test execution framework with output capture |
| `src/test_output.rs` | Test output file management (`.test_outputs/` directory) |
| `src/cargo_test.rs` | Cargo test command execution and result parsing |
| `src/telemetry/test_utils.rs` | In-memory telemetry sink for testing |

## 5. Performance Benchmarks (`~/NEEDLE/benches/`)

### `sanitize.rs` (26KB)
- **Framework**: Criterion (`cargo bench`)
- **Test Sizes**: 10KB, 100KB, 1MB traces
- **Metrics**: P95 and P99 latency reporting
- **Validation**: Aho-Corasick pre-filter effectiveness
- **Configuration**: `criterion.toml` (10 samples, 3s warmup, 5s measurement)

## 6. Test Dependencies

### Cargo.toml Dev Dependencies
```toml
[dev-dependencies]
tokio-test = "0.4"        # Async test utilities
tempfile = "3"            # Temporary directory creation
proptest = "1"            # Property-based testing
filetime = "0.2"          # File time manipulation
criterion = "0.5"         # Benchmarking framework
```

### Integration Testing Features
```toml
[features]
integration = [
    "otlp",              # OpenTelemetry support
    "testcontainers",    # Docker container management
]
```

## 7. Test Execution Infrastructure

### Test Runner Scripts

**`~/NEEDLE/scripts/run-tests-with-capture.sh`** (Main test runner)
- Captures all cargo test output to timestamped trace files
- Stores output in `.beads/traces/cargo-test-YYYYMMDD-HHMMSS.log`
- Creates symlink to latest: `.beads/traces/cargo-test-latest.log`
- Generates test summaries and warning counts
- Supports passing cargo test arguments

**`~/NEEDLE/tests/validate_heartbeat.sh`**
- Heartbeat validation script

### Test Fixtures (`~/NEEDLE/tests/fixtures/`)

Mock agent scripts for testing different agent outcomes:
- `mock-agent-crash.sh` - Agent crash simulation
- `mock-agent-failure.sh` - Agent failure simulation
- `mock-agent-interrupted.sh` - Agent interruption simulation
- `mock-agent-not-found.sh` - Agent not found simulation
- `mock-agent-success.sh` - Agent success simulation
- `mock-agent-timeout.sh` - Agent timeout simulation

## 8. Test Execution Patterns

### Running All Tests
```bash
cd ~/NEEDLE
cargo test
```

### Running Specific Test Categories
```bash
# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# Specific test
cargo test test_name

# With features
cargo test --features integration

# Benchmarks
cargo bench
```

### Running E2E Tests
```bash
cd ~/NEEDLE/tests/e2e
./single_worker.sh
./multi_worker.sh
```

### Running with Output Capture
```bash
cd ~/NEEDLE
./scripts/run-tests-with-capture.sh
./scripts/run-tests-with-capture.sh --lib
./scripts/run-tests-with-capture.sh test::specific_test
```

## 9. Known Issues and Problematic Tests

### Identified Issues

1. **P7.1a bug** (`cleanup_liveness_regression.rs`):
   - Issue: `pane_pid` (shell wrapper) ≠ actual needle process PID
   - The buggy behavior is documented in regression tests

2. **Flaky OTLP assertion** (`otlp_integration.rs`):
   - Note: "This assertion might be flaky if no logs were emitted inside bead.lifecycle"
   - Depends on log emission timing

## 10. Test Coverage Summary

| Category | Count | Type |
|----------|-------|------|
| Integration test files | 30+ | Rust integration tests |
| E2E shell scripts | 11 | Bash end-to-end tests |
| Unit test modules | 50+ | Embedded unit tests |
| Total unit tests | 1,147+ | #[test] functions |
| Property tests | Multiple | proptest-based |
| Benchmarks | 1 | Criterion-based |

## 11. CI/CD Integration

### Docker-based CI Environment
- `ci/Dockerfile.ci` - Container for CI environment
- `ci/Dockerfile.ci-deps` - Dependencies for CI

### Testcontainers Support
- Integration tests can spawn Docker containers via testcontainers
- Used for OTLP integration tests with OpenTelemetry collector

## Recommendations for Bead-Forge

Based on this survey, consider:

1. **Test Structure**: Follow NEEDLE's separation of unit vs integration vs e2e tests
2. **Mock Infrastructure**: Implement mock fixtures similar to NEEDLE's `mock-agent-*.sh` scripts
3. **Property-Based Tests**: Add proptest for invariant verification (like NEEDLE's `property_tests.rs`)
4. **Test Execution**: Implement test output capture similar to `run-tests-with-capture.sh`
5. **Performance Benchmarks**: Add Criterion benchmarks for critical paths
6. **Integration Tests**: Create real br CLI integration tests (like NEEDLE's `real_br_integration_tests.rs`)

## Conclusion

NEEDLE has a mature, comprehensive test infrastructure with excellent separation of concerns. The combination of unit tests, integration tests, e2e shell scripts, property-based tests, and benchmarks provides strong confidence in code correctness. Bead-forge can leverage many of these patterns and approaches.
