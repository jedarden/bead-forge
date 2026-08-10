# Test Infrastructure

This directory contains the integration test infrastructure for bead-forge.

## Structure

```
tests/
├── common.rs              # Common test utilities and fixtures
├── fixtures/              # Test fixture files
├── README.md             # This file
└── *.rs                  # Integration test files
```

## Common Test Utilities (common.rs)

The `common.rs` module provides comprehensive test infrastructure:

### TempWorkspace

A harness for creating isolated test workspaces with automatic cleanup:

```rust
let ws = TempWorkspace::new().unwrap();
ws.create_bead("bf-test", "Test bead").unwrap();
// Automatically cleaned up when ws is dropped
```

### Key Methods

- `TempWorkspace::new()` - Create a fresh workspace
- `TempWorkspace::with_jsonl(content)` - Initialize with JSONL data
- `TempWorkspace::from_fixture(name)` - Load from fixtures directory
- `workspace.create_bead(id, title)` - Create a test bead
- `workspace.create_issue(issue)` - Create with full Issue struct
- `workspace.get_bead(id)` - Retrieve a bead by ID
- `workspace.list_beads()` - Get all beads
- `workspace.storage()` - Access storage backend

### JSONL Fixtures

Helper functions for creating JSONL test data:

- `sample_bead_jsonl(id, title)` - Basic bead
- `sample_closed_bead_jsonl(id, title, reason)` - Closed bead
- `sample_bead_with_deps_jsonl(id, title, deps)` - With dependencies
- `sample_bead_with_labels_jsonl(id, title, labels)` - With labels

### P0 Epic Test Infrastructure

Comprehensive support for P0 epic testing:

- `sample_p0_epic_jsonl(id, title)` - Minimal P0 epic
- `sample_p0_epic_with_description_jsonl(id, title, description)` - With description
- `sample_p0_epic_with_labels_jsonl(id, title, labels)` - With labels
- `sample_p0_epic_full_jsonl(id, title, description, assignee, labels)` - Complete
- `P0EpicBuilder` - Builder pattern for P0 epic fixtures
- `assert_p0_epic(issue, context)` - Assert P0 epic properties
- `seed_p0_epics(workspace, count)` - Create multiple P0 epics
- `count_p0_epics(workspace)` - Count P0 epics in workspace

## Running Tests

### Run all tests:
```bash
cargo test
```

### Run specific test file:
```bash
cargo test --test test_jsonl
```

### Run with output:
```bash
cargo test -- --nocapture
```

### Run specific test:
```bash
cargo test test_temp_workspace_creation
```

## Benchmarks

Benchmarks use Criterion and are in the `benches/` directory:

### Run all benchmarks:
```bash
cargo bench
```

### Run specific benchmark:
```bash
cargo bench --bench basic_benchmark
```

Benchmark results are saved to `target/criterion/`.

## Test Fixtures

Place JSONL fixtures in `tests/fixtures/` for use in tests:

```rust
let ws = TempWorkspace::from_fixture("my-snapshot.jsonl").unwrap();
ws.import_jsonl().unwrap();
```

## Test Organization

Tests are organized by functionality:

- `test_jsonl.rs` - JSONL import/export tests
- `test_basic_workflow.rs` - Core workflow tests  
- `test_create*.rs` - Bead creation tests
- `test_label*.rs` - Label functionality tests
- `test_epic*.rs` - Epic-specific tests
- `blocker_dependency_basics.rs` - Dependency blocking tests
- `test_blocking_bead.rs` - Blocking bead tests

## Dependencies

Test dependencies in Cargo.toml:

```toml
[dev-dependencies]
tempfile = "3"          # Temporary directories for test isolation
chrono = { version = "0.4", features = ["serde"] }
rusqlite = { version = "0.31", features = ["bundled"] }
criterion = "0.5"       # Benchmarking
```

## Writing New Tests

1. Use `TempWorkspace` for isolated test environments
2. Use `common::` helpers for JSONL fixtures
3. Clean up is automatic via TempDir
4. Follow the existing test patterns

Example test:
```rust
#[test]
fn test_my_feature() {
    let ws = TempWorkspace::new().unwrap();
    
    // Arrange
    ws.create_bead("bf-test", "Test").unwrap();
    
    // Act
    let bead = ws.get_bead("bf-test").unwrap().unwrap();
    
    // Assert
    assert_eq!(bead.id, "bf-test");
}
```

## Test Configuration

The test framework uses standard Rust testing with these enhancements:

- **Isolation**: Each test gets a fresh workspace
- **Fixtures**: Reusable JSONL test data
- **Cleanup**: Automatic via TempDir
- **Helpers**: Common operations in common.rs
- **P0 Support**: Special infrastructure for P0 epic testing
