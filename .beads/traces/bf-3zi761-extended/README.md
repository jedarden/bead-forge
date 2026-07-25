# Extended Test Execution Infrastructure

## Directory Structure

This directory contains organized test execution logs and output for the bf-3zi761 extended test batch.

```
bf-3zi761-extended/
├── README.md                        # This file
├── test_modules.txt                 # Complete test module inventory and selection
├── results.txt                     # Test execution results summary
├── SUMMARY.md                      # Detailed test results and findings
│
├── phase1_sanity/                  # Phase 1: Sanity and Smoke Tests (15 modules)
├── phase2_unit_core/               # Phase 2: Unit Test Modules - Core Library (29 modules)
├── phase3_data_integrity/          # Phase 3: Data Integrity and Migration (8 modules)
├── phase4_batch_concurrent/       # Phase 4: Batch and Concurrent Operations (8 modules)
├── phase5_error_recovery/          # Phase 5: Error Recovery and Diagnostics (7 modules)
├── phase6_label_tests/             # Phase 6: Extended Label Tests (20 modules)
├── phase7_performance/             # Phase 7: Performance and Timing (5 modules)
├── phase8_format_validation/       # Phase 8: Format and Output Validation (6 modules)
└── phase9_config_infra/            # Phase 9: Configuration and Infrastructure (8 modules)
```

## Execution Format

### Per-Module Logging

Each test module generates three types of output files:

1. **`<module_name>.stdout`** - Standard output from test execution
2. **`<module_name>.stderr`** - Standard error from test execution
3. **`<module_name>.log`** - Combined log output with timestamps

### File Naming Convention

- Unit tests: `cargo test --lib <module_name>` → `<module_name>.log`
- Integration tests: `cargo test --test <test_name>` → `<test_name>.log`
- Multi-output tests: Separate `.stdout` and `.stderr` files

### Test Module Categories

#### Phase 1: Sanity and Smoke Tests (15 modules)
Quick validation tests for basic functionality. These run first and provide early feedback.

- `test_version_display` - Verify version output
- `test_bf_2l7_help_flag` - Help flag validation
- `test_bf_52is_smoke` - Basic smoke test
- `common` - Common utilities test
- And 11 more basic functionality tests

#### Phase 2: Unit Test Modules - Core Library (29 modules)
Core library unit tests covering individual components.

- `model`, `id`, `config`, `secrets` - Core data structures
- `jsonl`, `json`, `envelope` - Serialization
- `autoflush`, `batch`, `bead_store` - Storage operations
- And 20 more unit test modules

#### Phase 3: Data Integrity and Migration (8 modules)
Tests for data persistence, migration, and integrity verification.

- `jsonl_compat` - JSONL compatibility
- `test_jsonl` - JSONL import/export
- `migrate_git_reconstruction` - Git migration
- And 5 more data integrity tests

#### Phase 4: Batch and Concurrent Operations (8 modules)
Tests for atomic batch operations and concurrent access patterns.

- `batch_atomic` - Atomic batch operations
- `concurrent_claim` - Concurrent claiming
- `claim_race` - Race condition handling
- And 5 more concurrency tests

#### Phase 5: Error Recovery and Diagnostics (7 modules)
Tests for error handling, recovery, and diagnostic tools.

- `doctor_repair_unflushed` - Doctor repair functionality
- `autoflush_failure_contract` - Failure handling
- And 5 more error recovery tests

#### Phase 6: Extended Label Tests (20 modules)
Comprehensive label functionality testing.

- `test_label_comprehensive` - Label features
- `epic_label_edge_cases` - Epic label edge cases
- And 18 more label tests

#### Phase 7: Performance and Timing (5 modules)
Performance characteristics and timing validation.

- `timing` - Timing utilities
- `velocity` - Velocity calculation
- And 3 more performance tests

#### Phase 8: Format and Output Validation (6 modules)
Output format consistency and validation.

- `format` - Format utilities
- `json_formatter_verification` - JSON output validation
- And 4 more format tests

#### Phase 9: Configuration and Infrastructure (8 modules)
Configuration and infrastructure validation.

- `config` - Configuration handling
- `secrets` - Secret management
- And 6 more infrastructure tests

## Execution Instructions

### Running All Tests

```bash
# From workspace root
cd /home/coding/bead-forge

# Execute extended test batch
cargo test --lib --test '*' 2>&1 | tee .beads/traces/bf-3zi761-extended/full_execution.log
```

### Running Individual Phases

```bash
# Phase 1: Sanity tests
cargo test --test test_version_display --test test_bf_2l7_help_flag --test test_bf_52is_smoke

# Phase 2: Unit tests
cargo test --lib

# Specific integration test
cargo test --test <test_name>
```

### Capturing Module Output

Each module's output is captured to its respective log file:

```bash
# Unit test output
cargo test --lib <module_name> > .beads/traces/bf-3zi761-extended/phase2_unit_core/<module_name>.log 2>&1

# Integration test output  
cargo test --test <test_name> > .beads/traces/bf-3zi761-extended/phase1_sanity/<test_name>.log 2>&1
```

## Write Permissions

All directories have `755` permissions (drwxr-xr-x):
- Owner: read/write/execute
- Group: read/execute
- Others: read/execute

Log files are created with `644` permissions (rw-r--r--):
- Owner: read/write
- Group: read
- Others: read

## Verification

To verify write permissions for cargo test output:

```bash
# Test write permissions
touch .beads/traces/bf-3zi761-extended/phase1_sanity/test_write.tmp && rm -f .beads/traces/bf-3zi761-extended/phase1_sanity/test_write.tmp && echo "Write permissions OK"

# Verify directory structure
ls -R .beads/traces/bf-3zi761-extended/
```

## Results Summary

Test results are aggregated in:
- `results.txt` - Quick summary
- `SUMMARY.md` - Detailed findings

## Notes

- Total modules in extended batch: 78 (50.3% of total 155 modules)
- Estimated execution time: 35-50 minutes
- Unit tests can run in parallel
- Integration tests may require sequential execution to avoid database conflicts
- Failed tests should be re-run individually with verbose output

## Maintenance

This infrastructure is maintained as part of bead bf-3zi761. For updates or questions, refer to the bead documentation.