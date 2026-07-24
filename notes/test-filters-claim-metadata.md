# Test Filter Configuration: Claim & Metadata Tests

This document describes cargo test filters for isolating claim-related and metadata-related tests in bead-forge.

## Overview

The test suite is organized into:
- **Unit tests**: Embedded in source modules (e.g., `src/claim.rs`)
- **Integration tests**: Standalone files in `tests/` directory

## Claim-Related Tests

### Unit Tests in `src/claim.rs`

The `claim.rs` module contains embedded unit tests:

```bash
# Run all claim unit tests
cargo test --lib bead_forge::claim::tests

# Run specific claim test
cargo test --lib test_claim_basic
cargo test --lib test_claim_reclaims_stale
cargo test --lib test_concurrent_claim_no_double_claim
cargo test --lib test_critical_path_bonus_in_claim
cargo test --lib test_get_ready_candidates_respects_limit
```

### Integration Tests

Claim-specific integration test files in `tests/`:

```bash
# All claim integration tests
cargo test --test claim_race
cargo test --test concurrent_claim
cargo test --test claim_fallback
cargo test --test autoflush_batch_claim_delete

# Claim stats integration test
cargo test --test claim_stats
```

### Filter by Test Name Pattern

```bash
# All tests with "claim" in the name
cargo test claim

# All concurrent claim tests
cargo test concurrent_claim

# All claim fallback tests
cargo test claim_fallback
```

## Metadata-Related Tests

### Model Tests

The `model.rs` module contains metadata field tests:

```bash
# All model tests (includes metadata serialization)
cargo test --lib bead_forge::model::tests

# Specific model tests that touch metadata
cargo test --lib test_full_issue_with_all_fields
```

### Label/Metadata Integration Tests

Tests that interact with labels and metadata:

```bash
# Label tests (often test metadata interactions)
cargo test --test test_labels
cargo test --test test_labels_json_format
cargo test --test test_labels_text_format
cargo test --test test_label_comprehensive
cargo test --test comprehensive_label_tests

# Tests that use metadata field
cargo test --test test_show_command
cargo test --test test_update_command
cargo test --test test_close_reopen_integration
```

## Combined Filters

### Run All Claim Tests (Unit + Integration)

```bash
# Method 1: Run by pattern
cargo test claim

# Method 2: Run specific modules
cargo test --lib bead_forge::claim::tests &&
cargo test --test claim_race &&
cargo test --test concurrent_claim &&
cargo test --test claim_fallback &&
cargo test --test autoflush_batch_claim_delete &&
cargo test envelope::claim_stats
```

### Run All Metadata Tests

```bash
# Model unit tests + label integration tests
cargo test --lib bead_forge::model::tests &&
cargo test --test test_labels &&
cargo test --test test_labels_json_format &&
cargo test --test test_labels_text_format
```

## Quick Reference Commands

### Claim Test Suite
```bash
# Quick: All claim tests
cargo test claim

# Thorough: All claim tests with detailed output
cargo test claim -- --nocapture --test-threads=1

# Verbose: Show all test output
cargo test claim -- --exact --show-output
```

### Metadata Test Suite
```bash
# Quick: All metadata/label tests
cargo test --lib bead_forge::model::tests && cargo test --test test_labels

# Thorough: All metadata tests with output
cargo test --lib bead_forge::model::tests -- --nocapture &&
cargo test --test test_labels -- --nocapture
```

### For CI/Isolated Testing
```bash
# Single-threaded (for race condition tests)
cargo test claim -- --test-threads=1

# With logging
RUST_LOG=debug cargo test claim -- --nocapture

# Run only, don't build
cargo test claim --no-run
```

## Test Module Inventory

### Claim Test Modules
| Location | Module | Test Count |
|----------|--------|------------|
| `src/claim.rs` | `bead_forge::claim::tests` | 11 unit tests |
| `tests/claim_race.rs` | `claim_race` | 3 integration tests |
| `tests/concurrent_claim.rs` | `concurrent_claim` | 6 integration tests |
| `tests/claim_fallback.rs` | `claim_fallback` | 9 integration tests |
| `tests/autoflush_batch_claim_delete.rs` | `autoflush_batch_claim_delete` | 2 integration tests |
| `tests/envelope/claim_stats.rs` | `envelope::claim_stats` | Integration tests |

### Metadata Test Modules
| Location | Module | Focus |
|----------|--------|-------|
| `src/model.rs` | `bead_forge::model::tests` | Serialization, metadata field |
| `tests/test_labels.rs` | `test_labels` | Label/metadata CRUD |
| `tests/test_labels_json_format.rs` | `test_labels_json_format` | JSON metadata format |
| `tests/test_labels_text_format.rs` | `test_labels_text_format` | Text metadata format |
| `tests/test_label_comprehensive.rs` | `test_label_comprehensive` | Comprehensive label tests |
| `tests/comprehensive_label_tests.rs` | `comprehensive_label_tests` | Label integration |

## Notes

- Use `--test-threads=1` for concurrent claim tests to avoid actual concurrency
- Some tests use tempfile; ensure sufficient disk space
- Claim fallback tests test workspace selection behavior
- Model tests include metadata field serialization/deserialization
- Label tests interact with the `bead_annotations` table for metadata storage
