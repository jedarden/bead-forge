//! Test infrastructure and helpers for JSON output testing

pub mod json_output;

// Re-export the main helpers for convenience
pub use json_output::{
    test_workspace,
    bf_binary,
    bf_command,
    json_validation,
    fixtures,
    envelope,
    capture,
};
