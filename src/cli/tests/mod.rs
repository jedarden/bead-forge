//! Test infrastructure and helpers for JSON output testing

pub mod json_output;
pub mod show_json_tests;
pub mod list_ready_recent_json_tests;
pub mod search_json_tests;
pub mod edge_case_json_tests;
pub mod error_json_tests;
pub mod json_schema_validation;

// Re-export the main helpers for convenience
pub use json_output::{
    test_workspace,
    bf_binary,
    bf_command,
    bf_command_with_workspace,
    json_validation,
    format_detection,
    fixtures,
    envelope,
    capture,
};
