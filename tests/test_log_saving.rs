//! Integration test for log file saving functionality
//
// TEMPORARILY DISABLED: The imported functions don't exist in module_test.rs yet
// This test file is waiting for implementation of run_and_save_module_test and save_module_test_log

// use bead_forge::module_test::{run_and_save_module_test, save_module_test_log};
// use std::fs;
// use std::path::Path;

// TEMPORARILY DISABLED: The imported functions don't exist in module_test.rs yet
//
// #[test]
// fn test_save_module_log_creates_file() {
//     // Create a sample output to test the log saving
//     let sample_output = std::process::Output {
//         status: std::process::Command::new("true")
//             .status()
//             .expect("failed to get status"),
//         stdout: b"Test stdout output\nLine 2\n".to_vec(),
//         stderr: b"Test stderr output\nLine 2\n".to_vec(),
//     };
//
//     let result = save_module_test_log("test_save_log", &sample_output);
//
//     assert!(result.is_ok(), "save_module_log should succeed");
//
//     let log_path = result.unwrap();
//     assert!(log_path.exists(), "Log file should exist");
//     assert!(log_path.ends_with("test_save_log-raw.log"));
//
//     // Verify log file contents
//     let content = fs::read_to_string(&log_path)
//         .expect("Should be able to read log file");
//
//     assert!(content.contains("test_save_log"), "Should contain module name");
//     assert!(content.contains("Exit Code:"), "Should contain exit code");
//     assert!(content.contains("SUCCESS"), "Should indicate success");
//     assert!(content.contains("Timestamp:"), "Should contain timestamp");
//     assert!(content.contains("STDOUT"), "Should contain stdout marker");
//     assert!(content.contains("STDERR"), "Should contain stderr marker");
//     assert!(content.contains("Test stdout output"), "Should contain stdout content");
//     assert!(content.contains("Test stderr output"), "Should contain stderr content");
// }
//
// #[test]
// fn test_save_module_log_preserves_all_output() {
//     // Test that all output is preserved without truncation
//     let large_stdout = "Line ".repeat(1000);
//     let large_stderr = "Error ".repeat(1000);
//
//     let sample_output = std::process::Output {
//         status: std::process::Command::new("true")
//             .status()
//             .expect("failed to get status"),
//         stdout: large_stdout.as_bytes().to_vec(),
//         stderr: large_stderr.as_bytes().to_vec(),
//     };
//
//     let log_path = save_module_test_log("test_large_output", &sample_output)
//         .expect("save_module_log should succeed");
//
//     let content = fs::read_to_string(&log_path)
//         .expect("Should be able to read log file");
//
//     assert!(content.contains(&large_stdout), "Should preserve all stdout");
//     assert!(content.contains(&large_stderr), "Should preserve all stderr");
// }
//
// #[test]
// fn test_save_module_log_includes_exit_code() {
//     let sample_output = std::process::Output {
//         status: std::process::Command::new("false")
//             .status()
//             .expect("failed to get status"),
//         stdout: b"Some output\n".to_vec(),
//         stderr: b"Some error\n".to_vec(),
//     };
//
//     let log_path = save_module_test_log("test_exit_code", &sample_output)
//         .expect("save_module_log should succeed");
//
//     let content = fs::read_to_string(&log_path)
//         .expect("Should be able to read log file");
//
//     assert!(content.contains("Exit Code:"), "Should contain exit code marker");
//     // The exit code should be non-zero for false command
//     assert!(content.contains("FAILED"), "Should indicate failure");
// }
//
// #[test]
// fn test_save_module_log_creates_directory_if_needed() {
//     // This test verifies that ensure_trace_dir creates the directory
//     let sample_output = std::process::Output {
//         status: std::process::Command::new("true")
//             .status()
//             .expect("failed to get status"),
//         stdout: b"Test\n".to_vec(),
//         stderr: b"".to_vec(),
//     };
//
//     let result = save_module_test_log("test_dir_creation", &sample_output);
//
//     assert!(result.is_ok(), "save_module_log should succeed");
//
//     let log_path = result.unwrap();
//     let traces_dir = Path::new(".beads/traces/bf-4kzs6h-remaining");
//
//     assert!(traces_dir.exists(), "Trace directory should be created");
//     assert!(traces_dir.is_dir(), "Trace path should be a directory");
//
//     // Clean up
//     let _ = fs::remove_file(&log_path);
// }
//
// #[test]
// fn test_run_and_save_module_test_integration() {
//     // Test the full integration: run test and save log
//     // Use a simple module that should exist
//     let result = run_and_save_module_test("model", 30);
//
//     // We don't care if the test passes or fails, just that we capture it
//     match result {
//         Ok((output, log_path)) => {
//             assert!(log_path.exists(), "Log file should exist");
//             assert!(log_path.ends_with("model-raw.log"));
//
//             let content = fs::read_to_string(&log_path)
//                 .expect("Should be able to read log file");
//
//             assert!(content.contains("model"), "Should contain module name");
//             assert!(content.contains("Exit Code:"), "Should contain exit code");
//             assert!(content.contains("STDOUT"), "Should contain stdout marker");
//             assert!(content.contains("STDERR"), "Should contain stderr marker");
//         }
//         Err(bead_forge::module_test::TestError::Timeout { .. }) => {
//             // Timeout is acceptable - just means the test took too long
//             println!("Test timed out - acceptable for integration test");
//         }
//         Err(e) => {
//             panic!("Unexpected error: {}", e);
//         }
//     }
// }
