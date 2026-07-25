# NEEDLE Test Module Inventory

**Generated:** 2026-07-25  
**Total Test Count:** 1,896 tests  
**Purpose:** Comprehensive inventory of all test modules in NEEDLE for test suite execution planning.

---

## Test Type Distribution

### Unit Tests (Module-Based)
- **Location:** `src/*/mod.rs` (inline `#[cfg(test)]` modules)
- **Count:** ~1,600 tests
- **Execution:** `cargo test --lib`

### Integration Tests
- **Location:** `tests/*.rs`
- **Count:** ~27 test files
- **Execution:** `cargo test --test <test_name>`

### Property Tests
- **Location:** `tests/property_tests.rs`
- **Focus:** Concurrent claim exclusivity, mitosis, bead splitting
- **Execution:** Requires longer timeouts

### Regression Tests
- **Location:** `tests/*_regression.rs`, various specific tests
- **Focus:** Historical bug fixes
- **Execution:** Part of integration suite

---

## Module-Based Unit Tests

### Core System (1,498 tests)

| Module | Test Count | Purpose |
|--------|------------|---------|
| `strand` | 297 | Core execution loop, bead processing, waterfall ordering |
| `telemetry` | 120 | OTLP telemetry, event emission, field verification |
| `cli` | 117 | CLI parsing, argument handling, subcommand dispatch |
| `config` | 106 | Configuration loading, validation, routing rules |
| `routing` | 77 | Model routing, adapter resolution, pattern matching |
| `worker` | 72 | Worker lifecycle, claim management, heartbeat |
| `cargo_test` | 65 | Cargo test execution, output capture, compilation detection |
| `dispatch` | 64 | Agent dispatch, timeout handling, output capture |
| `types` | 55 | Core type serialization/deserialization |
| `stats` | 53 | Statistics, p95 latency calculations |
| `health` | 44 | Health checks, liveness, system state |
| `bead_store` | 38 | Bead storage, JSONL parsing, corruption detection |
| `canary` | 33 | Canary deployment, promotion, rollback |
| `prompt` | 29 | Prompt handling, template rendering |
| `validation` | 26 | Input validation, config verification |
| `outcome` | 24 | Test outcome classification, exit code handling |
| `mitosis` | 23 | Bead splitting, child bead creation |
| `cost` | 23 | Cost tracking, token counting |
| `trace` | 20 | Trace generation, output formatting |
| `test_runner` | 20 | Test execution, timeout handling |
| `upgrade` | 18 | Binary upgrade, hot reload detection |
| `claim` | 18 | Atomic claiming, race handling, exclusion sets |
| `skill` | 17 | Skill system, hook dispatch |
| `transcript` | 16 | Transcript generation, event logging |
| `rate_limit` | 16 | Rate limiting, quota enforcement |
| `sanitize` | 15 | Output sanitization, sensitive data filtering |
| `registry` | 15 | Worker registry, peer discovery |
| `claude_md_placement` | 14 | CLAUDE.md placement, workspace detection |
| `test_output` | 11 | Test output parsing, error detection |
| `peer` | 11 | Peer discovery, heartbeat tracking |
| `learning` | 10 | Learning extraction, memory placement |
| `drift` | 9 | Drift detection, consistency checks |
| `decision` | 8 | Decision logic, outcome selection |
| `agent_event` | 6 | Agent event serialization |
| `commit_hook` | 5 | Git commit hooks, trailer injection |
| `supervisor` | 2 | Supervisor socket communication |
| `spawn_path` | 3 | Process spawning, path resolution |

---

## Integration Test Files

### System Integration (9 files)

| File | Purpose | Key Tests |
|------|---------|-----------|
| `integration_tests.rs` | Core integration workflows | End-to-end worker loops, signal handling, cleanup |
| `real_br_integration_tests.rs` | br-compatible integration | Multi-worker claiming, mitosis, mend, explore |
| `process_discovery_integration.rs` | Process tree discovery | Worker detection, reconciliation, cleanup |
| `routing_integration.rs` | Model routing integration | GLM routing, workspace overrides, pattern matching |
| `otlp_integration.rs` | OTLP telemetry | Telemetry write, field verification |
| `config_cli_tests.rs` | Configuration CLI | Config get/set, validation, overrides |
| `heartbeat_validation.rs` | Heartbeat system | Fresh/stale detection, emitter, cleanup |
| `telemetry_field_verification.rs` | Telemetry schema | Field presence, type correctness |

### Regression & Validation (10 files)

| File | Purpose | Regression Coverage |
|------|---------|---------------------|
| `cleanup_liveness_regression.rs` | Cleanup doesn't remove live sessions | P71A tmux session split, tmux cleanup |
| `compilation_error_detection.rs` | Compilation error parsing | Borrow checker, import errors, could_not_compile |
| `stop_kills_process_tree.rs` | Process tree cleanup | SIGTERM, graceful shutdown |
| `sigterm_heartbeat_cleanup.rs` | Signal handling | SIGTERM removes heartbeat |
| `sanitize_latency_assertion.rs` | Latency sanitization | Threshold enforcement |
| `verify_bash_wrapper_exclusion.rs` | Process discovery filters | Bash wrapper processes excluded |
| `verify_deleted_binary_hot_reload.rs` | Binary hot reload | Deleted binary detection, force reload |
| `verify_process_discovery.rs` | Process discovery accuracy | All processes discovered |
| `verify_bf_4390q.rs` | Specific issue verification | BF-4390Q test case |

### Property-Based Tests (1 file)

| File | Purpose |
|------|---------|
| `property_tests.rs` | Concurrent claim exclusivity (proptest), mitosis correctness, bead splitting |

### Correctness & Performance (5 files)

| File | Purpose |
|------|---------|
| `p95_correctness.rs` | p95 latency calculation correctness |
| `p95_correctness.rs` | Edge cases, outliers, unsorted input |
| `routing_matcher_baseline.rs` | Routing pattern matching baseline |
| `test_telemetry_write.rs` | Telemetry write correctness |
| `test_telemetry_write_debug.rs` | Debug telemetry output |

### Fixtures & Support (4 files)

| File | Purpose |
|------|---------|
| `tmux_fixture.rs` | tmux session fixture for tests |
| `workspace_fixtures.rs` | Workspace test fixtures |
| `needle_transform_claude.rs` | Claude transformation tests |
| `integration_t/mod.rs` | Integration test utilities |
| `integration_t/load_simulation_example.rs` | Load simulation example |

---

## Test Categorization Summary

### By Purpose

1. **Unit Tests** (~1,600 tests)
   - Module-specific logic
   - Fast execution (ms to seconds)
   - No external dependencies

2. **Integration Tests** (~200 tests)
   - Cross-module workflows
   - Process tree management
   - Signal handling
   - Slower execution (seconds to minutes)

3. **Property Tests** (~50 tests)
   - Concurrent correctness
   - Invariant verification
   - Can run longer (minutes)

4. **Regression Tests** (~100 tests)
   - Historical bug fixes
   - Edge case coverage
   - Prevent reintroduction

### By Execution Time

- **Fast (< 1s each):** Config parsing, CLI parsing, type serialization
- **Medium (1-10s each):** Worker lifecycle, claiming, dispatch
- **Slow (> 10s each):** Property tests, integration tests, process discovery

### By Dependencies

- **No deps:** Most unit tests
- **tmux:** Some integration tests (via fixture)
- **Filesystem:** Bead store, config loading, trace writing
- **Network:** OTLP telemetry (some tests)
- **Subprocess:** Cargo test execution, process tree

---

## Recommended Test Execution Strategy

### Quick Smoke Test
```bash
cargo test --lib strand::tests::worker_boot_rejects_invalid_config
cargo test --lib config::tests::default_config_is_valid
cargo test --lib cli::tests::cli_parses_run_defaults
```

### Module-Specific Testing
```bash
# Test specific module
cargo test --lib <module>::tests

# Example: test config module
cargo test --lib config::tests
```

### Integration Testing
```bash
# Run all integration tests
cargo test --test integration_tests

# Specific integration test
cargo test --test real_br_integration_tests
```

### Full Suite
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# All tests (unit + integration)
cargo test
```

---

## Key Test Modules for bead-forge Development

Since this inventory is for bead-forge development, these NEEDLE tests are most relevant:

1. **bead_store (38 tests)** - Bead storage, JSONL parsing, corruption detection
2. **cargo_test (65 tests)** - Cargo test execution, output capture
3. **config (106 tests)** - Configuration system, validation
4. **cli (117 tests)** - CLI parsing, subcommand handling
5. **real_br_integration_tests** - br-compatible integration scenarios

---

## Notes

- Total count: 1,896 tests
- Largest module: `strand` (297 tests) - core execution loop
- Integration tests are in `tests/` directory
- Unit tests are inline in `src/*/mod.rs`
- Property tests use proptest for concurrent correctness
- Regression tests prevent reintroduction of fixed bugs
