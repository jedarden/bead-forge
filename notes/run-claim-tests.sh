#!/bin/bash
# Test filter runner for claim and metadata tests
# Usage: ./notes/run-claim-tests.sh [claim|metadata|all|help]

set -e

MODE="${1:-help}"

case "$MODE" in
  claim)
    echo "=== Running Claim Tests ==="
    cargo test claim
    ;;

  claim-thorough)
    echo "=== Running Claim Tests (Single-threaded, Verbose) ==="
    cargo test claim -- --test-threads=1 --nocapture
    ;;

  claim-unit)
    echo "=== Running Claim Unit Tests ==="
    cargo test --lib bead_forge::claim::tests
    ;;

  claim-integration)
    echo "=== Running Claim Integration Tests ==="
    cargo test --test claim_race &&
    cargo test --test concurrent_claim &&
    cargo test --test claim_fallback &&
    cargo test --test autoflush_batch_claim_delete &&
    cargo test envelope::claim_stats
    ;;

  metadata)
    echo "=== Running Metadata Tests ==="
    cargo test --lib bead_forge::model::tests &&
    cargo test --test test_labels &&
    cargo test --test test_labels_json_format &&
    cargo test --test test_labels_text_format
    ;;

  metadata-verbose)
    echo "=== Running Metadata Tests (Verbose) ==="
    cargo test --lib bead_forge::model::tests -- --nocapture &&
    cargo test --test test_labels -- --nocapture &&
    cargo test --test test_labels_json_format -- --nocapture &&
    cargo test --test test_labels_text_format -- --nocapture
    ;;

  all)
    echo "=== Running All Claim & Metadata Tests ==="
    cargo test claim &&
    cargo test --lib bead_forge::model::tests &&
    cargo test --test test_labels
    ;;

  list)
    echo "=== Available Claim/Metadata Tests ==="
    echo "Claim unit tests:"
    cargo test --lib bead_forge::claim::tests -- --list
    echo ""
    echo "Claim integration tests:"
    for test_file in claim_race concurrent_claim claim_fallback autoflush_batch_claim_delete; do
      echo "  - $test_file"
    done
    ;;

  help|--help|-h)
    cat <<EOF
Test Filter Runner for bead-forge claim/metadata tests

Usage: ./notes/run-claim-tests.sh [MODE]

Modes:
  claim              Run all claim tests (unit + integration)
  claim-thorough     Run claim tests single-threaded with verbose output
  claim-unit         Run only claim unit tests (src/claim.rs)
  claim-integration  Run only claim integration tests (tests/*.rs)
  metadata           Run all metadata/label tests
  metadata-verbose   Run metadata tests with verbose output
  all                Run all claim and metadata tests
  list               List available test modules
  help               Show this message

Examples:
  ./notes/run-claim-tests.sh claim
  ./notes/run-claim-tests.sh claim-thorough
  ./notes/run-claim-tests.sh metadata
EOF
    ;;

  *)
    echo "Unknown mode: $MODE"
    echo "Run './notes/run-claim-tests.sh help' for usage"
    exit 1
    ;;
esac
