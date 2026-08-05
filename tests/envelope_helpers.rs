//! Reusable helper functions for validating JSON envelope structure across all commands.
//!
//! This module provides generic test helpers for validating the envelope structure
//! used by `bf --json` output. All helpers work with `serde_json::Value` to enable
//! flexible validation across different command outputs.
//!
//! ## Envelope Structure
//!
//! All `bf` commands that support JSON output emit a stable envelope:
//!
//! ```json
//! {
//!   "version": 1,
//!   "kind": "<command>",
//!   "data": <command-specific data>,
//!   "warning": "<optional warning message>"
//! }
//! ```
//!
//! ## Usage
//!
//! ```rust
//! use crate::envelope_helpers::*;
//!
//! let json_output = run_command("--json");
//! let envelope: serde_json::Value = parse_envelope(&json_output);
//!
//! // Validate basic envelope structure
//! validate_envelope_structure(&envelope, "list");
//!
//! // Validate metadata fields
//! validate_metadata_fields(&envelope, "list", 1);
//!
//! // Validate data is an array (for list-like commands)
//! assert!(data_is_array(&envelope["data"]));
//! ```
//!
//! ## Command-Specific Data Shapes
//!
//! | Command | `data` shape | Empty result |
//! |---------|-------------|--------------|
//! | `create` | `{"id": "bf-xxx"}` | N/A (always succeeds) |
//! | `list` | `[{...}, {...}]` | `[]` |
//! | `ready` | `[{...}, {...}]` | `[]` |
//! | `show` | `{...}` | error (not found) |
//! | `claim` | `{...}` | `{}` (no bead available) |
//! | `update` | `{id: "..."}` | N/A |
//! | `close` | `{id: "..."}` | N/A |
//! | `reopen` | `{id: "..."}` | N/A |
//! | `delete` | `{id: "..."}` | N/A |
//! | `stats` | `{total: ..., ...}` | N/A |
//! | `velocity` | `[{...}, {...}]` | `[]` |
//! | `search` | `[{...}, {...}]` | `[]` |
//! | `recent` | `[{...}, {...}]` | `[]` |
//! | `batch` | `[{op: ..., result: ...}]` | `[]` |
//!
//! Bead: bf-607ow

use serde_json::Value;

// ============================================================================
// Core Envelope Validation
// ============================================================================

/// Validate the basic envelope structure has all required fields.
///
/// Verifies that the envelope is a JSON object with:
/// - `version` field (must equal 1)
/// - `kind` field (must match expected_kind)
/// - `data` field (must be present)
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
/// * `expected_kind` - Expected command identifier (e.g., "list", "ready", "show")
///
/// # Panics
///
/// Panics with descriptive message if any validation fails.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "list",
///     "data": [{"id": "bf-1"}]
/// });
/// validate_envelope_structure(&envelope, "list"); // passes
/// ```
pub fn validate_envelope_structure(envelope: &Value, expected_kind: &str) {
    // Verify envelope is an object
    assert!(
        envelope.is_object(),
        "Envelope must be a JSON object, got: {}",
        get_type_name(envelope)
    );

    // Verify version field is present and equals 1
    let version = envelope.get("version").and_then(|v| v.as_u64());
    assert_eq!(
        version,
        Some(1),
        "Envelope 'version' field must be 1, got: {:?}",
        version
    );

    // Verify kind field matches expected
    let kind = envelope.get("kind").and_then(|k| k.as_str());
    assert_eq!(
        kind,
        Some(expected_kind),
        "Envelope 'kind' field must be '{}', got: {:?}",
        expected_kind,
        kind
    );

    // Verify data field is present
    assert!(
        envelope.get("data").is_some(),
        "Envelope must have a 'data' field"
    );
}

/// Validate that all required metadata fields are present and correct.
///
/// Verifies:
/// - `version` field is present and equals expected_version
/// - `kind` field is present and equals expected_kind
///
/// This is a more targeted metadata check than `validate_envelope_structure`,
/// allowing custom version values for testing compatibility scenarios.
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
/// * `expected_kind` - Expected command identifier
/// * `expected_version` - Expected version number (typically 1)
///
/// # Panics
///
/// Panics with descriptive message if any validation fails.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "stats",
///     "data": {"total": 42}
/// });
/// validate_metadata_fields(&envelope, "stats", 1); // passes
/// ```
pub fn validate_metadata_fields(envelope: &Value, expected_kind: &str, expected_version: u64) {
    // Verify version field
    let version = envelope.get("version").and_then(|v| v.as_u64());
    assert_eq!(
        version,
        Some(expected_version),
        "Metadata 'version' must be {}, got: {:?}",
        expected_version,
        version
    );

    // Verify kind field
    let kind = envelope.get("kind").and_then(|k| k.as_str());
    assert_eq!(
        kind,
        Some(expected_kind),
        "Metadata 'kind' must be '{}', got: {:?}",
        expected_kind,
        kind
    );
}

/// Validate that the warning field is present and contains expected text.
///
/// The `warning` field is optional and only present when auto-flush or
/// other operations produce warnings. This helper verifies a warning
/// is present when expected.
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
/// * `expected_warning` - Expected warning text (can be partial match)
///
/// # Panics
///
/// Panics if warning field is missing or does not contain expected text.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "create",
///     "data": {"id": "bf-1"},
///     "warning": "auto-flush failed: write error"
/// });
/// validate_warning_present(&envelope, "auto-flush failed"); // passes
/// ```
pub fn validate_warning_present(envelope: &Value, expected_warning: &str) {
    let warning = envelope.get("warning").and_then(|w| w.as_str());
    assert!(
        warning.is_some(),
        "Envelope must have a 'warning' field when warning is expected"
    );
    assert!(
        warning.unwrap().contains(expected_warning),
        "Warning must contain '{}', got: '{}'",
        expected_warning,
        warning.unwrap()
    );
}

/// Validate that the warning field is absent (no warning condition).
///
/// Use this to verify that an operation completed without warnings.
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
///
/// # Panics
///
/// Panics if warning field is present.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "list",
///     "data": []
/// });
/// validate_no_warning(&envelope); // passes (no warning field)
/// ```
pub fn validate_no_warning(envelope: &Value) {
    let warning = envelope.get("warning");
    assert!(
        warning.is_none() || warning.unwrap().is_null(),
        "Envelope must not have a 'warning' field, got: {:?}",
        warning
    );
}

// ============================================================================
// Envelope Parsing and Extraction
// ============================================================================

/// Parse a JSON string into an envelope.
///
/// Returns the parsed `serde_json::Value` or panics with a descriptive
/// error message if parsing fails.
///
/// # Arguments
///
/// * `json_str` - JSON string output from a `bf` command
///
/// # Returns
///
/// Parsed envelope as `serde_json::Value`
///
/// # Panics
///
/// Panics if the string is not valid JSON.
///
/// # Example
///
/// ```rust
/// let json_output = r#"{"version":1,"kind":"list","data":[]}"#;
/// let envelope = parse_envelope(json_output);
/// assert_eq!(envelope["kind"], "list");
/// ```
pub fn parse_envelope(json_str: &str) -> Value {
    serde_json::from_str(json_str).unwrap_or_else(|e| {
        panic!(
            "Failed to parse envelope as valid JSON: {}\nInput: {}",
            e, json_str
        )
    })
}

/// Extract the data field from an envelope.
///
/// Returns a reference to the `data` field for further validation.
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
///
/// # Returns
///
/// Reference to the `data` field
///
/// # Panics
///
/// Panics if the data field is missing.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "show",
///     "data": {"id": "bf-1", "title": "Test"}
/// });
/// let data = get_data(&envelope);
/// assert_eq!(data["id"], "bf-1");
/// ```
pub fn get_data(envelope: &Value) -> &Value {
    envelope
        .get("data")
        .unwrap_or_else(|| panic!("Envelope must have a 'data' field"))
}

/// Extract the kind field from an envelope.
///
/// Returns the command identifier string.
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
///
/// # Returns
///
/// The command kind (e.g., "list", "show", "claim")
///
/// # Panics
///
/// Panics if the kind field is missing or not a string.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "stats",
///     "data": {"total": 42}
/// });
/// let kind = get_kind(&envelope);
/// assert_eq!(kind, "stats");
/// ```
pub fn get_kind(envelope: &Value) -> &str {
    envelope
        .get("kind")
        .and_then(|k| k.as_str())
        .unwrap_or_else(|| panic!("Envelope must have a 'kind' field that is a string"))
}

/// Extract the version field from an envelope.
///
/// Returns the version number.
///
/// # Arguments
///
/// * `envelope` - Parsed JSON envelope as `serde_json::Value`
///
/// # Returns
///
/// The envelope version number (typically 1)
///
/// # Panics
///
/// Panics if the version field is missing or not a number.
///
/// # Example
///
/// ```rust
/// let envelope = serde_json::json!({
///     "version": 1,
///     "kind": "ready",
///     "data": []
/// });
/// let version = get_version(&envelope);
/// assert_eq!(version, 1);
/// ```
pub fn get_version(envelope: &Value) -> u64 {
    envelope
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or_else(|| panic!("Envelope must have a 'version' field that is a number"))
}

// ============================================================================
// Data Type Validation Helpers
// ============================================================================

/// Check if the data field contains an array.
///
/// Use this for list-like commands (list, ready, search, recent, velocity, batch).
///
/// # Arguments
///
/// * `data` - The data field from an envelope
///
/// # Returns
///
/// `true` if data is an array, `false` otherwise
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!([{"id": "bf-1"}, {"id": "bf-2"}]);
/// assert!(data_is_array(&data));
/// ```
pub fn data_is_array(data: &Value) -> bool {
    data.is_array()
}

/// Check if the data field contains an object.
///
/// Use this for single-object commands (show, create, claim, update, close, stats).
///
/// # Arguments
///
/// * `data` - The data field from an envelope
///
/// # Returns
///
/// `true` if data is an object, `false` otherwise
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!({"id": "bf-1", "title": "Test"});
/// assert!(data_is_object(&data));
/// ```
pub fn data_is_object(data: &Value) -> bool {
    data.is_object()
}

/// Check if the data field contains a null value.
///
/// Some commands may emit null for error or empty states.
///
/// # Arguments
///
/// * `data` - The data field from an envelope
///
/// # Returns
///
/// `true` if data is null, `false` otherwise
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!(null);
/// assert!(data_is_null(&data));
/// ```
pub fn data_is_null(data: &Value) -> bool {
    data.is_null()
}

/// Check if the data field contains a string value.
///
/// # Arguments
///
/// * `data` - The data field from an envelope
///
/// # Returns
///
/// `true` if data is a string, `false` otherwise
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!("output text");
/// assert!(data_is_string(&data));
/// ```
pub fn data_is_string(data: &Value) -> bool {
    data.is_string()
}

/// Check if the data field contains a numeric value.
///
/// # Arguments
///
/// * `data` - The data field from an envelope
///
/// # Returns
///
/// `true` if data is a number, `false` otherwise
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!(42);
/// assert!(data_is_number(&data));
/// ```
pub fn data_is_number(data: &Value) -> bool {
    data.is_number()
}

/// Check if the data field contains a boolean value.
///
/// # Arguments
///
/// * `data` - The data field from an envelope
///
/// # Returns
///
/// `true` if data is a boolean, `false` otherwise
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!(true);
/// assert!(data_is_boolean(&data));
/// ```
pub fn data_is_boolean(data: &Value) -> bool {
    data.is_boolean()
}

/// Assert that the data field is an array.
///
/// Panics with descriptive message if data is not an array.
///
/// # Arguments
///
/// * `data` - The data field from an envelope
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an array.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!([1, 2, 3]);
/// assert_data_is_array(&data, Some("list output"));
/// ```
pub fn assert_data_is_array(data: &Value, context: Option<&str>) {
    let ctx = context.unwrap_or("data");
    assert!(
        data.is_array(),
        "{} must be an array, got: {}",
        ctx,
        get_type_name(data)
    );
}

/// Assert that the data field is an object.
///
/// Panics with descriptive message if data is not an object.
///
/// # Arguments
///
/// * `data` - The data field from an envelope
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an object.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!({"id": "bf-1"});
/// assert_data_is_object(&data, Some("show output"));
/// ```
pub fn assert_data_is_object(data: &Value, context: Option<&str>) {
    let ctx = context.unwrap_or("data");
    assert!(
        data.is_object(),
        "{} must be an object, got: {}",
        ctx,
        get_type_name(data)
    );
}

// ============================================================================
// Array Data Helpers
// ============================================================================

/// Get the length of an array data field.
///
/// Returns the number of elements in the array, or 0 if data is not an array.
///
/// # Arguments
///
/// * `data` - The data field from an envelope (expected to be an array)
///
/// # Returns
///
/// Number of elements in the array
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!([{"id": "bf-1"}, {"id": "bf-2"}]);
/// assert_eq!(data_array_length(&data), 2);
/// ```
pub fn data_array_length(data: &Value) -> usize {
    data.as_array().map(|arr| arr.len()).unwrap_or(0)
}

/// Assert that an array data field has the expected length.
///
/// # Arguments
///
/// * `data` - The data field from an envelope (expected to be an array)
/// * `expected_len` - Expected number of elements
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an array or length doesn't match.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!([1, 2, 3]);
/// assert_data_array_length(&data, 3, Some("list output"));
/// ```
pub fn assert_data_array_length(data: &Value, expected_len: usize, context: Option<&str>) {
    let ctx = context.unwrap_or("data array");
    assert!(data.is_array(), "{} must be an array to check length", ctx);
    let actual_len = data.as_array().unwrap().len();
    assert_eq!(
        actual_len, expected_len,
        "{} must have {} elements, got {}",
        ctx, expected_len, actual_len
    );
}

/// Assert that an array data field is empty.
///
/// Use this to verify empty result sets for list-like commands.
///
/// # Arguments
///
/// * `data` - The data field from an envelope (expected to be an array)
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an array or is not empty.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!([]);
/// assert_data_array_empty(&data, Some("empty list"));
/// ```
pub fn assert_data_array_empty(data: &Value, context: Option<&str>) {
    let ctx = context.unwrap_or("data array");
    assert!(
        data.is_array(),
        "{} must be an array to check emptiness",
        ctx
    );
    assert!(data.as_array().unwrap().is_empty(), "{} must be empty", ctx);
}

/// Assert that an array data field is non-empty.
///
/// Use this to verify that list-like commands returned results.
///
/// # Arguments
///
/// * `data` - The data field from an envelope (expected to be an array)
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an array or is empty.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!([{"id": "bf-1"}]);
/// assert_data_array_non_empty(&data, Some("ready list"));
/// ```
pub fn assert_data_array_non_empty(data: &Value, context: Option<&str>) {
    let ctx = context.unwrap_or("data array");
    assert!(
        data.is_array(),
        "{} must be an array to check emptiness",
        ctx
    );
    assert!(
        !data.as_array().unwrap().is_empty(),
        "{} must be non-empty",
        ctx
    );
}

// ============================================================================
// Object Data Helpers
// ============================================================================

/// Assert that an object data field contains a specific key.
///
/// # Arguments
///
/// * `data` - The data field from an envelope (expected to be an object)
/// * `key` - Expected key name
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an object or key is missing.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!({"id": "bf-1", "title": "Test"});
/// assert_data_object_has_key(&data, "id", Some("show output"));
/// ```
pub fn assert_data_object_has_key(data: &Value, key: &str, context: Option<&str>) {
    let ctx = context.unwrap_or("data object");
    assert!(
        data.is_object(),
        "{} must be an object to check for key '{}'",
        ctx,
        key
    );
    assert!(
        data.get(key).is_some(),
        "{} must contain key '{}', keys present: {:?}",
        ctx,
        key,
        data.as_object().unwrap().keys().collect::<Vec<_>>()
    );
}

/// Assert that an object data field has a specific key-value pair.
///
/// # Arguments
///
/// * `data` - The data field from an envelope (expected to be an object)
/// * `key` - Expected key name
/// * `expected_value` - Expected value (as JSON)
/// * `context` - Optional context string for error messages
///
/// # Panics
///
/// Panics if data is not an object, key is missing, or value doesn't match.
///
/// # Example
///
/// ```rust
/// let data = serde_json::json!({"id": "bf-1", "status": "open"});
/// assert_data_object_has_value(&data, "status", &serde_json::json!("open"), None);
/// ```
pub fn assert_data_object_has_value(
    data: &Value,
    key: &str,
    expected_value: &Value,
    context: Option<&str>,
) {
    let ctx = context.unwrap_or("data object");
    assert_data_object_has_key(data, key, Some(ctx));

    let actual_value = data.get(key).unwrap();
    assert_eq!(
        actual_value, expected_value,
        "{} field '{}' must be {:?}, got {:?}",
        ctx, key, expected_value, actual_value
    );
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get a human-readable type name for a JSON value.
///
/// Used in error messages to describe what type was received.
///
/// # Arguments
///
/// * `value` - A JSON value
///
/// # Returns
///
/// String describing the type ("object", "array", "string", etc.)
fn get_type_name(value: &Value) -> &'static str {
    if value.is_object() {
        "object"
    } else if value.is_array() {
        "array"
    } else if value.is_string() {
        "string"
    } else if value.is_number() {
        "number"
    } else if value.is_boolean() {
        "boolean"
    } else if value.is_null() {
        "null"
    } else {
        "unknown"
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_envelope_structure() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "list",
            "data": []
        });
        validate_envelope_structure(&envelope, "list"); // should not panic
    }

    #[test]
    fn test_validate_envelope_structure_wrong_kind() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "list",
            "data": []
        });
        let result = std::panic::catch_unwind(|| {
            validate_envelope_structure(&envelope, "show");
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_metadata_fields() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "stats",
            "data": {"total": 42}
        });
        validate_metadata_fields(&envelope, "stats", 1); // should not panic
    }

    #[test]
    fn test_parse_envelope() {
        let json_str = r#"{"version":1,"kind":"list","data":[]}"#;
        let envelope = parse_envelope(json_str);
        assert_eq!(envelope["kind"], "list");
    }

    #[test]
    fn test_parse_envelope_invalid_json() {
        let result = std::panic::catch_unwind(|| {
            parse_envelope("not json");
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_get_data() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "show",
            "data": {"id": "bf-1"}
        });
        let data = get_data(&envelope);
        assert_eq!(data["id"], "bf-1");
    }

    #[test]
    fn test_get_kind() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "claim",
            "data": {"bead_id": "bf-1"}
        });
        assert_eq!(get_kind(&envelope), "claim");
    }

    #[test]
    fn test_get_version() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "ready",
            "data": []
        });
        assert_eq!(get_version(&envelope), 1);
    }

    #[test]
    fn test_data_is_array() {
        let data = serde_json::json!([1, 2, 3]);
        assert!(data_is_array(&data));
        assert!(!data_is_object(&data));
    }

    #[test]
    fn test_data_is_object() {
        let data = serde_json::json!({"id": "bf-1"});
        assert!(data_is_object(&data));
        assert!(!data_is_array(&data));
    }

    #[test]
    fn test_data_is_null() {
        let data = serde_json::json!(null);
        assert!(data_is_null(&data));
        assert!(!data_is_object(&data));
    }

    #[test]
    fn test_data_is_string() {
        let data = serde_json::json!("test");
        assert!(data_is_string(&data));
        assert!(!data_is_array(&data));
    }

    #[test]
    fn test_data_is_number() {
        let data = serde_json::json!(42);
        assert!(data_is_number(&data));
        assert!(!data_is_string(&data));
    }

    #[test]
    fn test_data_is_boolean() {
        let data = serde_json::json!(true);
        assert!(data_is_boolean(&data));
        assert!(!data_is_number(&data));
    }

    #[test]
    fn test_assert_data_is_array() {
        let data = serde_json::json!([1, 2, 3]);
        assert_data_is_array(&data, None); // should not panic

        let obj = serde_json::json!({"key": "value"});
        let result = std::panic::catch_unwind(|| {
            assert_data_is_array(&obj, None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_data_is_object() {
        let data = serde_json::json!({"id": "bf-1"});
        assert_data_is_object(&data, None); // should not panic

        let arr = serde_json::json!([]);
        let result = std::panic::catch_unwind(|| {
            assert_data_is_object(&arr, None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_data_array_length() {
        let data = serde_json::json!([1, 2, 3, 4, 5]);
        assert_eq!(data_array_length(&data), 5);

        let empty = serde_json::json!([]);
        assert_eq!(data_array_length(&empty), 0);
    }

    #[test]
    fn test_assert_data_array_length() {
        let data = serde_json::json!([1, 2, 3]);
        assert_data_array_length(&data, 3, None); // should not panic

        let result = std::panic::catch_unwind(|| {
            assert_data_array_length(&data, 5, None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_data_array_empty() {
        let data = serde_json::json!([]);
        assert_data_array_empty(&data, None); // should not panic

        let non_empty = serde_json::json!([1]);
        let result = std::panic::catch_unwind(|| {
            assert_data_array_empty(&non_empty, None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_data_array_non_empty() {
        let data = serde_json::json!([1]);
        assert_data_array_non_empty(&data, None); // should not panic

        let empty = serde_json::json!([]);
        let result = std::panic::catch_unwind(|| {
            assert_data_array_non_empty(&empty, None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_data_object_has_key() {
        let data = serde_json::json!({"id": "bf-1", "title": "Test"});
        assert_data_object_has_key(&data, "id", None); // should not panic
        assert_data_object_has_key(&data, "title", None); // should not panic

        let result = std::panic::catch_unwind(|| {
            assert_data_object_has_key(&data, "missing", None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_assert_data_object_has_value() {
        let data = serde_json::json!({"id": "bf-1", "status": "open"});
        assert_data_object_has_value(&data, "status", &serde_json::json!("open"), None); // should not panic

        let result = std::panic::catch_unwind(|| {
            assert_data_object_has_value(&data, "status", &serde_json::json!("closed"), None);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_warning_present() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "create",
            "data": {"id": "bf-1"},
            "warning": "auto-flush failed: write error"
        });
        validate_warning_present(&envelope, "auto-flush failed"); // should not panic
    }

    #[test]
    fn test_validate_warning_present_missing() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "list",
            "data": []
        });
        let result = std::panic::catch_unwind(|| {
            validate_warning_present(&envelope, "some warning");
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_no_warning() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "list",
            "data": []
        });
        validate_no_warning(&envelope); // should not panic
    }

    #[test]
    fn test_validate_no_warning_with_warning() {
        let envelope = serde_json::json!({
            "version": 1,
            "kind": "create",
            "data": {"id": "bf-1"},
            "warning": "something failed"
        });
        let result = std::panic::catch_unwind(|| {
            validate_no_warning(&envelope);
        });
        assert!(result.is_err());
    }
}
