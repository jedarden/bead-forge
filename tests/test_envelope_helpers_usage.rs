//! Example usage of envelope_helpers module.
//!
//! This test demonstrates how to use the reusable envelope validation
//! helpers across different commands.
//!
//! Bead: bf-607ow

mod envelope_helpers;
use envelope_helpers::*;

#[test]
fn example_validate_list_envelope() {
    // Simulate list --json envelope output
    let json_output = r#"{
        "version": 1,
        "kind": "list",
        "data": [
            {"id": "bf-1", "title": "First task", "status": "open"},
            {"id": "bf-2", "title": "Second task", "status": "open"}
        ]
    }"#;

    // Parse the envelope
    let envelope = parse_envelope(json_output);

    // Validate basic envelope structure
    validate_envelope_structure(&envelope, "list");

    // Validate metadata fields
    validate_metadata_fields(&envelope, "list", 1);

    // Verify no warning present
    validate_no_warning(&envelope);

    // Extract and validate data field
    let data = get_data(&envelope);

    // Verify data is an array (for list-like commands)
    assert!(data_is_array(data));
    assert_data_is_array(data, Some("list data"));
    assert_data_array_length(data, 2, Some("list data"));
}

#[test]
fn example_validate_show_envelope() {
    // Simulate show --json envelope output
    let json_output = r#"{
        "version": 1,
        "kind": "show",
        "data": {
            "id": "bf-1",
            "title": "Test Bead",
            "status": "open",
            "priority": 2,
            "type": "task"
        }
    }"#;

    let envelope = parse_envelope(json_output);

    // Validate envelope structure
    validate_envelope_structure(&envelope, "show");

    // Extract and validate data field
    let data = get_data(&envelope);

    // Verify data is an object (for single-object commands)
    assert!(data_is_object(data));
    assert_data_is_object(data, Some("show data"));

    // Verify specific fields exist
    assert_data_object_has_key(data, "id", Some("show data"));
    assert_data_object_has_key(data, "title", Some("show data"));
}

#[test]
fn example_validate_empty_list_envelope() {
    // Simulate list --json on empty workspace
    let json_output = r#"{
        "version": 1,
        "kind": "list",
        "data": []
    }"#;

    let envelope = parse_envelope(json_output);

    validate_envelope_structure(&envelope, "list");

    let data = get_data(&envelope);

    // Verify data is an empty array
    assert_data_array_empty(data, Some("empty list data"));
}

#[test]
fn example_validate_claim_envelope_with_warning() {
    // Simulate claim --json with auto-flush warning
    let json_output = r#"{
        "version": 1,
        "kind": "claim",
        "data": {
            "bead_id": "bf-1",
            "assignee": "test-worker",
            "reclaimed": 0
        },
        "warning": "auto-flush failed: write error"
    }"#;

    let envelope = parse_envelope(json_output);

    validate_envelope_structure(&envelope, "claim");

    // Verify warning is present and contains expected text
    validate_warning_present(&envelope, "auto-flush failed");

    let data = get_data(&envelope);
    assert_data_is_object(data, Some("claim data"));

    // Verify claim-specific fields
    assert_data_object_has_key(data, "bead_id", Some("claim data"));
    assert_data_object_has_key(data, "assignee", Some("claim data"));
}

#[test]
fn example_validate_stats_envelope() {
    // Simulate stats --json envelope output
    let json_output = r#"{
        "version": 1,
        "kind": "stats",
        "data": {
            "total": 42,
            "open": 20,
            "in_progress": 15,
            "closed": 7
        }
    }"#;

    let envelope = parse_envelope(json_output);

    validate_envelope_structure(&envelope, "stats");

    let data = get_data(&envelope);
    assert_data_is_object(data, Some("stats data"));

    // Verify stats-specific fields
    assert_data_object_has_key(data, "total", Some("stats data"));
    assert_data_object_has_key(data, "open", Some("stats data"));
}

#[test]
fn example_use_extractor_helpers() {
    let envelope = serde_json::json!({
        "version": 1,
        "kind": "ready",
        "data": [{"id": "bf-1", "priority": 2}]
    });

    // Use extractor helpers
    let kind = get_kind(&envelope);
    assert_eq!(kind, "ready");

    let version = get_version(&envelope);
    assert_eq!(version, 1);

    let data = get_data(&envelope);
    assert!(data_is_array(data));
}

#[test]
fn example_custom_validation_with_helpers() {
    // Custom validation scenario: verify list items have required fields
    let json_output = r#"{
        "version": 1,
        "kind": "search",
        "data": [
            {"id": "bf-1", "title": "First result"},
            {"id": "bf-2", "title": "Second result"}
        ]
    }"#;

    let envelope = parse_envelope(json_output);
    validate_envelope_structure(&envelope, "search");

    let data = get_data(&envelope);
    assert_data_is_array(data, Some("search results"));

    // Custom validation: each item must have id and title
    if let Some(items) = data.as_array() {
        for (i, item) in items.iter().enumerate() {
            assert!(
                item.get("id").is_some(),
                "Search result {} must have 'id' field",
                i
            );
            assert!(
                item.get("title").is_some(),
                "Search result {} must have 'title' field",
                i
            );
        }
    }
}
