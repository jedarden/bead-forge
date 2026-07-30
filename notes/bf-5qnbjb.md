# NEEDLE Test Suite Inventory

**Task:** Inventory NEEDLE test suite and categorize tests  
**Date:** 2026-07-24  
**Scope:** ~/NEEDLE complete test structure

## Overview

The NEEDLE project has a comprehensive test infrastructure with **104+ test modules** across multiple categories:

- **27 standalone integration/property test files** in `tests/`
- **15 example/benchmark test files** in `examples/`
- **60+ source files** with embedded `#[cfg(test)]` unit test modules
- **12 end-to-end shell script tests** in `tests/e2e/`
- **Multiple test fixtures and mock implementations**

## Test Categories

### 1. Integration Tests (by Phase)

#### Phase 1 Integration Tests (`tests/integration_tests.rs`)
**Purpose:** Single worker lifecycle and outcome handling  
**Test Categories:**
- End-to-end single worker cycle
- All 6 outcome paths (success, failure, timeout, agent_not_found, interrupted, crash)
- Exhaustion (empty queue → Pluck returns NoWork → Knot fires → EXHAUSTED)
- Graceful shutdown (shutdown flag during various states)
- Deterministic ordering (property test)
- Cross-workspace mend: two-workspace zombie scenario

**Infrastructure:** Uses `IntegrationMockStore` for controlled bead states

#### Phase 2 Integration Tests (`tests/p2_integration_tests.rs`)
**Purpose:** Multi-worker fleet coordination  
**Test Categories:**
1. Multi-worker claiming — N workers, M beads, each claimed exactly once
2. Crashed worker bead released by peer monitoring
3. Mend strand cleans stale claims and orphaned locks
4. Provider/model concurrency limits enforced
5. Mitosis splits multi-task beads correctly
6. Mitosis dedup — duplicate split creates zero new children
7. Concurrent claiming — flock serializes multiple claimers
8. Registry concurrent access — no corruption
9. Heartbeat liveness — emitter writes and stop cleans up
10. Strand waterfall ordering with Mend
11. Explore discovers work in other workspaces (real br)
12. Mitosis splits multi-task bead, creates children
13. Duplicate mitosis on same parent: zero new children
14. Two workers mitosis on same parent: flock serializes

#### Phase 3 Integration Tests (`tests/p3_integration_tests.rs`)
**Purpose:** Advanced features end-to-end  
**Test Categories:**
- Weave: gap analysis and bead creation from documentation
- Unravel: alternatives for HUMAN-blocked beads
- Pulse: codebase health scans
- Validation gates: pre-closure verification
- Hook sink: telemetry dispatch to external commands
- Release channels: canary promote/reject/rollback
- Hot-reload: binary hash comparison and channel switching

### 2. Real br CLI Integration Tests (`tests/real_br_integration_tests.rs`)
**Purpose:** Integration tests using actual `br` binary  
**Test Categories:**
1. Multi-worker claiming — N workers, M beads, each claimed exactly once
2. Crashed worker bead released by peer monitoring
3. Explore strand discovers work in other workspaces
4. Mend strand cleans stale claims and orphaned locks
5. Mitosis splits multi-task beads correctly
6. Duplicate mitosis on same parent creates zero new children
7. Concurrent mitosis on same parent: flock serializes
8. Database corruption — corrupt SQLite, verify auto-repair from JSONL

**Infrastructure:** Uses isolated temporary workspaces, parallel-safe

### 3. Property-Based Tests (`tests/property_tests.rs`)
**Purpose:** Verify core design invariants using randomized inputs  
**Framework:** `proptest`  
**Test Categories:**
- Bead status transition properties
- Priority ordering invariants
- Dependency DAG consistency
- ID format validation

### 4. Routing Tests

#### `tests/routing_integration.rs`
**Purpose:** Model-based adapter routing end-to-end  
**Test Categories:**
1. Anthropic Claude models → claude-print (subscription billing)
2. GLM models → claude-code-glm-4.7 (default adapter)
3. Workspace override of global routing rules
4. Missing adapter failure (strict mode)

#### `tests/routing_matcher_baseline.rs`
**Purpose:** Routing matcher baseline tests

### 5. Telemetry Tests

#### `tests/otlp_integration.rs`
**Purpose:** OpenTelemetry protocol integration  
**Test Categories:** OTLP event export and validation

#### `tests/telemetry_field_verification.rs`
**Purpose:** Telemetry event field verification  
**Test Categories:** Field completeness and format validation

#### `tests/heartbeat_validation.rs`
**Purpose:** Heartbeat validation tests

#### `tests/test_telemetry_write.rs` & `tests/test_telemetry_write_debug.rs`
**Purpose:** Telemetry write functionality tests

### 6. Regression Tests

#### `tests/cleanup_liveness_regression.rs`
**Purpose:** Prevent recurrence of cleanup/liveness bugs  
**Size:** 31,838 bytes (significant regression test suite)

#### `tests/compilation_error_detection.rs`
**Purpose:** Compilation error detection regression tests

#### `tests/stop_kills_process_tree.rs`
**Purpose:** Process tree cleanup regression tests

#### `tests/sigterm_heartbeat_cleanup.rs`
**Purpose:** Signal handling and heartbeat cleanup regression

### 7. Specialized Integration Tests

#### `tests/process_discovery_integration.rs`
**Purpose:** Process discovery across workspaces

#### `tests/config_cli_tests.rs`
**Purpose:** CLI configuration command tests

#### `tests/p95_correctness.rs`
**Purpose:** P95 latency metrics correctness

#### `tests/sanitize_latency_assertion.rs`
**Purpose:** Latency sanitization assertions

### 8. Verification Tests

#### `tests/verify_bash_wrapper_exclusion.rs`
**Purpose:** Verify bash wrapper exclusion behavior

#### `tests/verify_bf_4390q.rs`
**Purpose:** Verify specific bf-4390q behavior

#### `tests/verify_deleted_binary_hot_reload.rs`
**Purpose:** Hot-reload with deleted binaries

#### `tests/verify_process_discovery.rs`
**Purpose:** Process discovery verification

### 9. Test Infrastructure

#### `tests/workspace_fixtures.rs` (22,968 bytes)
**Purpose:** Test fixtures and mock helpers for workspace management

#### `tests/tmux_fixture.rs` (17,940 bytes)
**Purpose:** Tmux session test fixtures for multi-terminal testing

## Example/Benchmark Tests (`examples/`)

**Purpose:** Performance benchmarking and concurrency debugging

### P95 Performance Tests
- `test_p95_simple.rs` - Basic P95 benchmark
- `test_p95_simple_manual.rs` - Manual P95 testing
- `test_p95_output.rs` - P95 output validation
- `test_benchmark_p95.rs` - Comprehensive P95 benchmarking
- `validate_p95_values.rs` - P95 value validation
- `verify_p95_reporting.rs` - P95 reporting verification
- `verify_all_latency_metrics.rs` - Latency metrics completeness

### Concurrency Debugging
- `test_bf_list_concurrency.rs` - bf list concurrency testing
- `test_bf_write_concurrency.rs` - bf write concurrency testing
- `test_bf_list_error_output.rs` - Error output under concurrency
- `test_actual_bf_error_format.rs` - Actual error format validation

### Other Tests
- `test_trace_check.rs` - Trace validation
- `test_compilation_error.rs` - Compilation error detection
- `test_4390q_debug.rs` - Debug for issue 4390q
- `extract_p95_from_criterion.rs` - P95 extraction from Criterion benchmarks

## End-to-End Shell Tests (`tests/e2e/`)

**Purpose:** Full lifecycle testing using real binaries  

### Test Scripts
1. `telemetry_completeness.sh` - Telemetry event completeness validation
2. `graceful_shutdown.sh` - Graceful shutdown behavior
3. `failure_counter_persistence.sh` - Failure counter persistence across restarts
4. `failure_counter.sh` - Failure counter functionality
5. `forced_mitosis.sh` - Forced mitosis behavior
6. `single_worker.sh` - Single worker lifecycle
7. `heartbeat_peer_recovery.sh` - Peer recovery via heartbeats
8. `additive_launch.sh` - Additive worker launch
9. `auto_split.sh` - Automatic bead splitting
10. `strand_waterfall.sh` - Strand execution ordering
11. `multi_worker.sh` - Multi-worker coordination

### Infrastructure
- `lib/workspace.sh` - E2E test library with shared helpers

## Unit Tests (Embedded in Source)

**45+ modules** with `#[cfg(test)]` sections containing component-level tests:

### Key Modules with Unit Tests
- `src/routing.rs` - Routing logic tests
- `src/config/mod.rs` - Configuration validation tests
- `src/bead_store/mod.rs` - Bead store implementation tests
- `src/worker/mod.rs` - Worker behavior tests
- `src/dispatch/mod.rs` - Dispatcher tests
- `src/claim/mod.rs` - Claimer tests
- `src/cli/mod.rs` - CLI command tests
- `src/agent_event.rs` - Agent event handling tests
- `src/canary/mod.rs` - Canary deployment tests
- `src/cost/mod.rs` - Cost calculation tests
- `src/decision/mod.rs` - Decision logic tests
- `src/drift/mod.rs` - Drift detection tests
- `src/health/mod.rs` - Health monitoring tests
- `src/learning/mod.rs` - Learning module tests
- `src/mitosis/mod.rs` - Mitosis logic tests
- `src/outcome/mod.rs` - Outcome handling tests
- `src/peer/mod.rs` - Peer monitoring tests
- And 25+ additional modules

## Test Fixtures and Mocks

### Mock Implementations
- **IntegrationMockStore** - Tracks operations, returns configurable beads
- **MemorySink** - In-memory telemetry collection for testing

### Agent Behavior Mocks (`tests/fixtures/`)
- `mock-agent-crash.sh` - Simulates agent crashes
- `mock-agent-failure.sh` - Simulates task failures
- `mock-agent-timeout.sh` - Simulates timeouts
- `mock-agent-success.sh` - Simulates successful completion
- `mock-agent-interrupted.sh` - Simulates interruptions
- `mock-agent-not-found.sh` - Simulates agent not found errors
- `test-echo.yaml` - Test configuration fixture

### Integration Test Infrastructure
- Isolated temporary workspaces for parallel safety
- Controlled bead states (dead/alive, excluded/assigned/claimable)
- Real binary integration (br CLI, needle)

## Velocity-Aware Scoring

**Note:** The velocity-aware scoring functionality from **bf-2x26cl** was about **claim metadata flags**, not a separate "velocity-aware scoring test" module.

### Implementation Location
The velocity-aware scoring is implemented in core modules (not separate test files):

1. **`src/bead_store/mod.rs`**
   - `BrCliBeadStore.model` - Model name for velocity-aware claim scoring
   - `BrCliBeadStore.harness` - Harness name for velocity-aware claim scoring
   - `BrCliBeadStore.harness_version` - Harness version for velocity-aware claim scoring
   - References plan §4B.6 for velocity-adjusted scoring

2. **`src/worker/mod.rs`**
   - Passes velocity-scoring metadata in claim operations
   - Ensures remote-workspace claims carry same metadata as home-workspace claims

3. **`src/dispatch/mod.rs`**
   - Contains model/harness/harness_version fields for routing decisions

### Purpose
Routes beads to the model/harness combo that closes each issue_type fastest based on historical performance data. The system tracks `worker_sessions` and `velocity_stats` to compute `velocity_adjusted_score`.

### Testing
Velocity-aware scoring is tested as part of:
- Integration tests for claiming behavior
- Routing integration tests
- Real br CLI integration tests

## Test Dependencies and Setup Requirements

### Required Tools
- **br CLI** - Bead management CLI (tested in real_br_integration_tests.rs)
- **needle binary** - NEEDLE binary for E2E tests
- **tmux** - Terminal multiplexer for tmux_fixture.rs tests
- **bash** - Shell for E2E test scripts

### Rust Dependencies
- `proptest` - Property-based testing framework
- `tempfile` - Temporary directory management
- `chrono` - Time handling for tests
- `anyhow` - Error handling in tests
- `async_trait` - Async trait bindings
- `which` - Binary discovery for br CLI

### Test Database
Tests use isolated temporary workspaces with their own `.beads/` directories. No external database required.

### Parallel Safety
All integration tests use unique workspace paths per test for parallel execution safety.

### Environment Setup
No special environment variables required. Tests discover the br CLI via PATH or `~/.local/bin/br`.

## Summary

The NEEDLE test suite is comprehensive and well-organized:

- **Integration tests** organized by implementation phase (P1, P2, P3)
- **Property tests** verify core invariants
- **E2E shell tests** validate full lifecycles
- **Unit tests** embedded in 45+ source modules
- **Regression tests** prevent recurrence of known bugs
- **Performance tests** benchmark P95 latencies
- **Infrastructure** provides mocks, fixtures, and helpers

**Total Test Count:** 104+ test modules/files  
**Test Frameworks:** Rust built-in, proptest, shell scripts  
**Key Focus Areas:** Multi-worker coordination, telemetry completeness, regression prevention, performance benchmarking
