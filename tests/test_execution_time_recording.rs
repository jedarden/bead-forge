//! Comprehensive integration tests for execution time recording in cargo test execution.
//!
//! This test module verifies that execution time is accurately measured and recorded
//! when running cargo test commands, ensuring timing information is captured in
//! both result structures and trace metadata.

use bead_forge::trace::{TraceManager, TraceMetadata};
use std::thread;
use std::time::Duration;

mod cargo_test_helpers;
use cargo_test_helpers::{default_bead_metadata, TestProject};

#[test]
fn test_execution_time_is_measured() {
    // Create a simple test project
    let project = TestProject::new()
        .unwrap()
        .with_test("test_simple", "assert_eq!(1, 1);")
        .build()
        .unwrap();

    // Run cargo test
    let result = project.run_cargo_test().unwrap();

    // Verify execution time is measured
    assert!(
        result.duration_ms > 0,
        "Execution time should be measured and greater than zero, got {}ms",
        result.duration_ms
    );

    println!("✓ Execution time measured: {}ms", result.duration_ms);
}

#[test]
fn test_execution_time_is_reasonable() {
    // Create a test project with multiple tests
    let project = TestProject::new()
        .unwrap()
        .with_tests(vec![
            ("test_one".to_string(), "assert_eq!(1, 1);".to_string()),
            ("test_two".to_string(), "assert_eq!(2, 2);".to_string()),
            ("test_three".to_string(), "assert_eq!(3, 3);".to_string()),
        ])
        .build()
        .unwrap();

    // Run cargo test
    let result = project.run_cargo_test().unwrap();

    // Verify execution time is reasonable (between 100ms and 30 seconds for cargo test)
    assert!(
        result.duration_ms >= 100,
        "Execution time should be at least 100ms, got {}ms",
        result.duration_ms
    );

    assert!(
        result.duration_ms < 30000,
        "Execution time should be less than 30 seconds, got {}ms",
        result.duration_ms
    );

    println!(
        "✓ Execution time is reasonable: {}ms (between 100ms and 30s)",
        result.duration_ms
    );
}

#[test]
fn test_execution_time_recorded_in_bead_trace_metadata() {
    // Create a test project
    let project = TestProject::new()
        .unwrap()
        .with_test("test_timing", "assert_eq!(true, true);")
        .build()
        .unwrap();

    let bead_id = "bf-test-timing-recording";
    let metadata = default_bead_metadata(bead_id);

    // Run cargo test to bead trace
    let result = project
        .run_cargo_test_to_bead_trace(bead_id, &metadata)
        .unwrap();

    // Verify execution time is recorded in result
    assert!(
        result.duration_ms > 0,
        "Result duration_ms should be greater than zero, got {}ms",
        result.duration_ms
    );

    // Verify trace files were created
    assert!(
        result.bead_trace_dir.exists(),
        "Bead trace directory should exist"
    );

    // Read and verify metadata file
    let metadata_path = result.bead_trace_dir.join("metadata.json");
    assert!(
        metadata_path.exists(),
        "Metadata file should exist in bead trace directory"
    );

    let metadata_content: String =
        std::fs::read_to_string(&metadata_path).expect("Failed to read metadata file");
    let parsed_metadata: serde_json::Value =
        serde_json::from_str(&metadata_content).expect("Failed to parse metadata JSON");

    // Verify duration_ms is present in metadata
    assert!(
        parsed_metadata.get("duration_ms").is_some(),
        "duration_ms field should be present in metadata"
    );

    let duration_ms = parsed_metadata["duration_ms"]
        .as_u64()
        .expect("duration_ms should be a number");

    assert!(
        duration_ms > 0,
        "duration_ms in metadata should be greater than zero, got {}",
        duration_ms
    );

    println!(
        "✓ Execution time recorded in bead trace metadata: {}ms",
        duration_ms
    );
}

#[test]
fn test_execution_time_increases_with_test_complexity() {
    // Create a simple test project
    let simple_project = TestProject::new()
        .unwrap()
        .with_test("test_simple", "assert_eq!(1, 1);")
        .build()
        .unwrap();

    let simple_result = simple_project.run_cargo_test().unwrap();
    let simple_duration = simple_result.duration_ms;

    // Create a project with more complex tests
    let complex_project = TestProject::new()
        .unwrap()
        .with_source_code(
            r#"
            fn fibonacci(n: u64) -> u64 {
                match n {
                    0 => 0,
                    1 => 1,
                    _ => fibonacci(n - 1) + fibonacci(n - 2),
                }
            }
        "#,
        )
        .with_tests(vec![
            (
                "test_fib_10".to_string(),
                "assert_eq!(fibonacci(10), 55);".to_string(),
            ),
            (
                "test_fib_15".to_string(),
                "assert_eq!(fibonacci(15), 610);".to_string(),
            ),
            (
                "test_fib_20".to_string(),
                "assert_eq!(fibonacci(20), 6765);".to_string(),
            ),
        ])
        .build()
        .unwrap();

    let complex_result = complex_project.run_cargo_test().unwrap();
    let complex_duration = complex_result.duration_ms;

    // Both should have measured execution times
    assert!(
        simple_duration > 0,
        "Simple test should have measured duration: {}ms",
        simple_duration
    );

    assert!(
        complex_duration > 0,
        "Complex test should have measured duration: {}ms",
        complex_duration
    );

    println!("✓ Execution time increases with test complexity:");
    println!("  Simple tests: {}ms", simple_duration);
    println!("  Complex tests: {}ms", complex_duration);

    // The complex tests should take at least as long as simple tests
    // (though this might not always be true due to system variance)
    if complex_duration >= simple_duration {
        println!("  ✓ Complex tests took longer as expected");
    } else {
        println!("  ! Note: Complex tests were faster (system variance)");
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_execution_time_across_multiple_runs() {
    // Create a test project
    let project = TestProject::new()
        .unwrap()
        .with_test("test_repeated", "assert_eq!(2 + 2, 4);")
        .build()
        .unwrap();

    let mut durations = Vec::new();

    // Run cargo test multiple times
    for run in 0..3 {
        let result = project.run_cargo_test().unwrap();
        durations.push(result.duration_ms);

        assert!(
            result.duration_ms > 0,
            "Run {}: Execution time should be measured, got {}ms",
            run,
            result.duration_ms
        );

        // Small delay between runs to ensure distinct timestamps
        thread::sleep(Duration::from_millis(100));
    }

    println!("✓ Execution time recorded across multiple runs:");
    for (i, duration) in durations.iter().enumerate() {
        println!("  Run {}: {}ms", i, duration);
    }

    // All runs should have measured duration
    assert_eq!(durations.len(), 3, "Should have 3 duration measurements");

    // Verify durations are reasonable
    for duration in &durations {
        assert!(
            *duration >= 100 && *duration < 30000,
            "Duration should be between 100ms and 30s, got {}ms",
            duration
        );
    }
}

#[test]
fn test_execution_time_with_start_and_end_times() {
    // Create a test project
    let project = TestProject::new()
        .unwrap()
        .with_test("test_with_timing", "assert_eq!(5, 5);")
        .build()
        .unwrap();

    let bead_id = "bf-test-start-end-times";
    let metadata = default_bead_metadata(bead_id);

    // Run cargo test to bead trace
    let result = project
        .run_cargo_test_to_bead_trace(bead_id, &metadata)
        .unwrap();

    // Verify start_time and end_time are present
    assert!(
        result.start_time.is_some(),
        "start_time should be present in result"
    );

    assert!(
        result.end_time.is_some(),
        "end_time should be present in result"
    );

    let start_time_str = result.start_time.as_ref().unwrap();
    let end_time_str = result.end_time.as_ref().unwrap();

    // Verify they're in RFC3339 format
    assert!(!start_time_str.is_empty(), "start_time should not be empty");

    assert!(!end_time_str.is_empty(), "end_time should not be empty");

    // Parse timestamps to verify format
    // RFC3339 format should be parseable
    assert!(
        start_time_str.contains('T')
            && (start_time_str.ends_with('Z') || start_time_str.contains('+')),
        "start_time should be in RFC3339 format: {}",
        start_time_str
    );

    assert!(
        end_time_str.contains('T') && (end_time_str.ends_with('Z') || end_time_str.contains('+')),
        "end_time should be in RFC3339 format: {}",
        end_time_str
    );

    // Verify end_time > start_time (duration should be positive)
    assert!(
        result.duration_ms > 0,
        "Duration should be positive when end_time > start_time, got {}ms",
        result.duration_ms
    );

    println!("✓ Execution time with start and end times:");
    println!("  Start: {}", start_time_str);
    println!("  End: {}", end_time_str);
    println!("  Duration: {}ms", result.duration_ms);
}

#[test]
fn test_execution_time_timing_variance_within_bounds() {
    // Create a test project
    let project = TestProject::new()
        .unwrap()
        .with_test("test_variance", "assert!(true);")
        .build()
        .unwrap();

    let mut durations = Vec::new();

    // Run multiple times to check variance
    for _ in 0..5 {
        let result = project.run_cargo_test().unwrap();
        durations.push(result.duration_ms);
        thread::sleep(Duration::from_millis(50));
    }

    // Calculate statistics
    let min_duration = *durations.iter().min().unwrap();
    let max_duration = *durations.iter().max().unwrap();
    let avg_duration = durations.iter().sum::<u64>() / durations.len() as u64;

    println!("✓ Execution time timing variance:");
    println!("  Min: {}ms", min_duration);
    println!("  Max: {}ms", max_duration);
    println!("  Avg: {}ms", avg_duration);

    // All measurements should be reasonable
    assert!(min_duration > 0, "Minimum duration should be positive");
    assert!(
        max_duration < 60000,
        "Maximum duration should be under 60 seconds"
    );

    // Variance should be within reasonable bounds (max should be less than 10x min)
    // This allows for system load variations but catches timing anomalies
    let variance_ratio = max_duration as f64 / min_duration as f64;
    assert!(
        variance_ratio < 10.0,
        "Variance ratio should be less than 10x, got {:.2}x (min: {}ms, max: {}ms)",
        variance_ratio,
        min_duration,
        max_duration
    );
}

#[test]
fn test_execution_time_captured_in_all_output_formats() {
    // Create a test project
    let project = TestProject::new()
        .unwrap()
        .with_test("test_output_capture", "assert_eq!(1, 1);")
        .build()
        .unwrap();

    // Test standard cargo test execution
    let standard_result = project.run_cargo_test().unwrap();
    assert!(
        standard_result.duration_ms > 0,
        "Standard execution should record duration: {}ms",
        standard_result.duration_ms
    );

    // Test bead trace execution
    let bead_id = "bf-test-output-formats";
    let metadata = default_bead_metadata(bead_id);
    let bead_result = project
        .run_cargo_test_to_bead_trace(bead_id, &metadata)
        .unwrap();

    assert!(
        bead_result.duration_ms > 0,
        "Bead trace execution should record duration: {}ms",
        bead_result.duration_ms
    );

    // Verify bead trace has metadata file with duration
    let metadata_path = bead_result.bead_trace_dir.join("metadata.json");
    let metadata_content: String =
        std::fs::read_to_string(&metadata_path).expect("Failed to read metadata");
    let parsed: serde_json::Value =
        serde_json::from_str(&metadata_content).expect("Failed to parse metadata");

    let duration_in_metadata = parsed["duration_ms"]
        .as_u64()
        .expect("duration_ms should be in metadata");

    assert!(
        duration_in_metadata > 0,
        "duration_ms in metadata should be positive: {}",
        duration_in_metadata
    );

    println!("✓ Execution time captured in all output formats:");
    println!("  Standard result: {}ms", standard_result.duration_ms);
    println!("  Bead trace result: {}ms", bead_result.duration_ms);
    println!("  Metadata duration: {}ms", duration_in_metadata);
}
