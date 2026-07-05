// Unit tests for P0 (Critical Priority) Priority enum validation
// Tests core Priority enum behavior for P0 (critical/highest priority)

use bead_forge::model::Priority;
use rusqlite::types::ToSql;

#[test]
fn test_p0_priority_exists() {
    // Test that P0 priority exists in Priority enum
    let p0_critical = Priority::CRITICAL;
    assert_eq!(p0_critical.0, 0, "P0 (CRITICAL) should have value 0");
}

#[test]
fn test_p0_to_string_conversion() {
    // Test that P0 converts to string correctly
    let p0 = Priority::CRITICAL;
    let display_str = format!("{}", p0);
    assert_eq!(display_str, "P0", "P0 should display as 'P0'");
}

#[test]
fn test_p0_from_string_conversion() {
    // Test that P0 can be parsed from string correctly
    let p0_from_p0_str = "P0".parse::<Priority>().unwrap();
    assert_eq!(p0_from_p0_str, Priority::CRITICAL, "Parsing 'P0' should give CRITICAL");

    let p0_from_0_str = "0".parse::<Priority>().unwrap();
    assert_eq!(p0_from_0_str, Priority::CRITICAL, "Parsing '0' should give CRITICAL");

    let p0_from_p0_lowercase = "p0".parse::<Priority>().unwrap();
    assert_eq!(p0_from_p0_lowercase, Priority::CRITICAL, "Parsing 'p0' should give CRITICAL (case insensitive)");

    let p0_from_p0_uppercase = "P0".parse::<Priority>().unwrap();
    assert_eq!(p0_from_p0_uppercase, Priority::CRITICAL, "Parsing 'P0' should give CRITICAL");

    // Test with whitespace
    let p0_from_whitespace = "  P0  ".parse::<Priority>().unwrap();
    assert_eq!(p0_from_whitespace, Priority::CRITICAL, "Parsing '  P0  ' should give CRITICAL (whitespace trimmed)");
}

#[test]
fn test_p0_is_highest_priority() {
    // Test that P0 compares correctly as highest priority (lowest numeric value)
    let p0 = Priority::CRITICAL;
    let p1 = Priority::HIGH;
    let p2 = Priority::MEDIUM;
    let p3 = Priority::LOW;
    let p4 = Priority::BACKLOG;

    // P0 should be less than all other priorities (lower value = higher priority)
    assert!(p0 < p1, "P0 should be less than P1 (higher priority)");
    assert!(p0 < p2, "P0 should be less than P2 (higher priority)");
    assert!(p0 < p3, "P0 should be less than P3 (higher priority)");
    assert!(p0 < p4, "P0 should be less than P4 (higher priority)");

    // P0 should not be greater than any other priority
    assert!(!(p0 > p1), "P0 should not be greater than P1");
    assert!(!(p0 > p2), "P0 should not be greater than P2");
    assert!(!(p0 > p3), "P0 should not be greater than P3");
    assert!(!(p0 > p4), "P0 should not be greater than P4");
}

#[test]
fn test_p0_priority_const_definition() {
    // Test that Priority::CRITICAL is properly defined as P0
    assert_eq!(Priority::CRITICAL, Priority(0), "CRITICAL should equal Priority(0)");
    assert_eq!(Priority::CRITICAL.0, 0, "CRITICAL value should be 0");
}

#[test]
fn test_all_priority_constants() {
    // Test all priority constants are properly defined
    assert_eq!(Priority::CRITICAL.0, 0, "CRITICAL should be 0");
    assert_eq!(Priority::HIGH.0, 1, "HIGH should be 1");
    assert_eq!(Priority::MEDIUM.0, 2, "MEDIUM should be 2");
    assert_eq!(Priority::LOW.0, 3, "LOW should be 3");
    assert_eq!(Priority::BACKLOG.0, 4, "BACKLOG should be 4");
}

#[test]
fn test_priority_ordering() {
    // Test complete priority ordering: P0 < P1 < P2 < P3 < P4
    assert!(Priority::CRITICAL < Priority::HIGH, "P0 < P1");
    assert!(Priority::HIGH < Priority::MEDIUM, "P1 < P2");
    assert!(Priority::MEDIUM < Priority::LOW, "P2 < P3");
    assert!(Priority::LOW < Priority::BACKLOG, "P3 < P4");

    // Test transitive property: P0 < P4
    assert!(Priority::CRITICAL < Priority::BACKLOG, "P0 < P4");
}

#[test]
fn test_p0_equality() {
    // Test P0 equality comparisons
    let p0a = Priority::CRITICAL;
    let p0b = Priority(0);

    assert_eq!(p0a, p0b, "CRITICAL should equal Priority(0)");
    assert_eq!(p0a, Priority::CRITICAL, "CRITICAL should equal itself");
    assert_eq!(p0b, Priority::CRITICAL, "Priority(0) should equal CRITICAL");
}

#[test]
fn test_p0_inequality() {
    // Test P0 inequality with other priorities
    let p0 = Priority::CRITICAL;

    assert_ne!(p0, Priority::HIGH, "P0 should not equal P1");
    assert_ne!(p0, Priority::MEDIUM, "P0 should not equal P2");
    assert_ne!(p0, Priority::LOW, "P0 should not equal P3");
    assert_ne!(p0, Priority::BACKLOG, "P0 should not equal P4");
}

#[test]
fn test_p0_display_variations() {
    // Test various string representations of P0
    let p0 = Priority::CRITICAL;

    assert_eq!(format!("{}", p0), "P0");
    assert_eq!(format!("{:?}", p0), "Priority(0)");
}

#[test]
fn test_p0_invalid_string_parsing() {
    // Test that invalid strings return proper errors
    let invalid_inputs = vec!["P5", "P-1", "invalid", "PX", "P 0", "P0extra"];

    for input in invalid_inputs {
        let result = input.parse::<Priority>();
        assert!(result.is_err(), "Parsing '{}' should fail", input);
        if let Err(e) = result {
            assert!(e.contains("Invalid priority"), "Error should mention 'Invalid priority'");
        }
    }
}

#[test]
fn test_p0_serialization() {
    // Test that P0 serializes correctly to JSON
    let p0 = Priority::CRITICAL;
    let json = serde_json::to_string(&p0).unwrap();
    assert_eq!(json, "0", "P0 should serialize as JSON number 0");
}

#[test]
fn test_p0_deserialization() {
    // Test that P0 deserializes correctly from JSON
    // Priority uses #[serde(transparent)] so it only deserializes as integer, not string
    let p0: Priority = serde_json::from_str("0").unwrap();
    assert_eq!(p0, Priority::CRITICAL, "JSON 0 should deserialize to CRITICAL");
}

#[test]
fn test_p0_clone_and_copy() {
    // Test that P0 can be cloned and copied
    let p0 = Priority::CRITICAL;
    let p0_clone = p0;
    let p0_copy = p0;

    assert_eq!(p0_clone, Priority::CRITICAL);
    assert_eq!(p0_copy, Priority::CRITICAL);
    assert_eq!(p0_clone, p0_copy);
}

#[test]
fn test_p0_debug_formatting() {
    // Test debug formatting for P0
    let p0 = Priority::CRITICAL;
    let debug_str = format!("{:?}", p0);
    assert_eq!(debug_str, "Priority(0)", "Debug format should show Priority(0)");
}

#[test]
fn test_priority_default_is_not_p0() {
    // Test that default priority is MEDIUM (P2), not P0
    let default_priority = Priority::default();
    assert_eq!(default_priority, Priority::MEDIUM, "Default priority should be MEDIUM");
    assert_ne!(default_priority, Priority::CRITICAL, "Default priority should not be CRITICAL");
    assert_eq!(default_priority.0, 2, "Default priority value should be 2 (MEDIUM)");
}

#[test]
fn test_p0_range_validation() {
    // Test that valid priority range is 0-4 (inclusive)
    let valid_priorities = vec![0, 1, 2, 3, 4];

    for value in valid_priorities {
        let result = value.to_string().parse::<Priority>();
        assert!(result.is_ok(), "Priority {} should be valid", value);
        assert_eq!(result.unwrap().0, value);
    }

    // Test invalid values outside range
    let invalid_values = vec![-1, 5, 10, 100];

    for value in invalid_values {
        let result = value.to_string().parse::<Priority>();
        assert!(result.is_err(), "Priority {} should be invalid", value);
    }
}

#[test]
fn test_p0_ord_trait_implementation() {
    // Test that Ord trait is correctly implemented for P0
    use std::cmp::Ordering;

    let p0 = Priority::CRITICAL;
    let p1 = Priority::HIGH;

    assert_eq!(p0.cmp(&p1), Ordering::Less, "P0 should be Less than P1");
    assert_eq!(p1.cmp(&p0), Ordering::Greater, "P1 should be Greater than P0");
    assert_eq!(p0.cmp(&p0), Ordering::Equal, "P0 should be Equal to itself");
}

#[test]
fn test_p0_partial_ord_trait_implementation() {
    // Test that PartialOrd trait is correctly implemented for P0
    let p0 = Priority::CRITICAL;
    let p1 = Priority::HIGH;

    assert!(p0 < p1, "P0 should be less than P1");
    assert!(p1 > p0, "P1 should be greater than P0");
    assert!(p0 <= p1, "P0 should be less than or equal to P1");
    assert!(p1 >= p0, "P1 should be greater than or equal to P0");
    assert!(p0 <= p0, "P0 should be less than or equal to itself");
    assert!(p0 >= p0, "P0 should be greater than or equal to itself");
}

#[test]
fn test_p0_with_rusqlite_compatibility() {
    // Test that P0 works with rusqlite ToSql and FromSql traits
    // This is a compile-time test - the trait bounds are already implemented
    let p0 = Priority::CRITICAL;

    // Test that ToSql is implemented (this would fail at compile time if not)
    let _to_sql_result = p0.to_sql();
    assert!(_to_sql_result.is_ok(), "P0 should be convertible to SQL");

    // Test that Priority can be constructed from i32
    let p0_from_i32 = Priority(0);
    assert_eq!(p0_from_i32, Priority::CRITICAL);
}
