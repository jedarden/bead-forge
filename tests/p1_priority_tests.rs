// Unit tests for P1 (High Priority) Priority behavior
// Covers the constant value, Display, serde, FromStr parsing, and ordering

use bead_forge::model::{Issue, Priority};

#[test]
fn test_p1_priority_value() {
    // P1 (HIGH) is the numeric priority 1
    let p1 = Priority::HIGH;
    assert_eq!(p1.0, 1, "P1 (HIGH) should have value 1");
    assert_eq!(p1, Priority(1), "Priority::HIGH should equal Priority(1)");
}

#[test]
fn test_p1_display_format() {
    // Display renders as "P<n>"
    assert_eq!(
        format!("{}", Priority::HIGH),
        "P1",
        "P1 should display as 'P1'"
    );
    assert_eq!(
        Priority::HIGH.to_string(),
        "P1",
        "to_string() should match the Display output"
    );
}

#[test]
fn test_p1_serialization() {
    // Priority is #[serde(transparent)], so it serializes as a bare integer
    let json = serde_json::to_string(&Priority::HIGH).unwrap();
    assert_eq!(json, "1", "Priority::HIGH should serialize to 1");

    // As a field on an Issue it appears as "priority":1
    let mut issue = Issue::new(
        "bf-p1test".to_string(),
        "P1 serialization".to_string(),
        "bead-forge".to_string(),
    );
    issue.priority = Priority::HIGH;

    let issue_json = serde_json::to_string(&issue).unwrap();
    assert!(
        issue_json.contains("\"priority\":1"),
        "Issue JSON should contain \"priority\":1, got: {}",
        issue_json
    );

    // Round-trips back to HIGH
    let decoded: Priority = serde_json::from_str("1").unwrap();
    assert_eq!(decoded, Priority::HIGH, "1 should deserialize to HIGH");
}

#[test]
fn test_p1_parsing() {
    for input in ["P1", "1", "p1", " P1 "] {
        let parsed = input
            .parse::<Priority>()
            .unwrap_or_else(|e| panic!("parsing {:?} should succeed, got: {}", input, e));
        assert_eq!(
            parsed,
            Priority::HIGH,
            "parsing {:?} should give HIGH",
            input
        );
    }

    // Non-numeric and out-of-range values are rejected
    assert!("P9".parse::<Priority>().is_err(), "P9 is out of range");
    assert!("high".parse::<Priority>().is_err(), "'high' is not numeric");
}

#[test]
fn test_p1_ordering() {
    // Lower number == higher priority: P0 < P1 < P2 < P3 < P4
    assert!(Priority::CRITICAL < Priority::HIGH, "P0 < P1");
    assert!(Priority::HIGH < Priority::MEDIUM, "P1 < P2");
    assert!(Priority::MEDIUM < Priority::LOW, "P2 < P3");
    assert!(Priority::LOW < Priority::BACKLOG, "P3 < P4");

    let mut sorted = vec![
        Priority::BACKLOG,
        Priority::HIGH,
        Priority::LOW,
        Priority::CRITICAL,
        Priority::MEDIUM,
    ];
    sorted.sort();
    assert_eq!(
        sorted,
        vec![
            Priority::CRITICAL,
            Priority::HIGH,
            Priority::MEDIUM,
            Priority::LOW,
            Priority::BACKLOG,
        ],
        "sorting should order P0..P4 ascending"
    );
}

#[test]
fn test_p1_vs_other_priorities() {
    let p1 = Priority::HIGH;

    assert_ne!(p1, Priority::CRITICAL, "P1 is not P0");
    assert_ne!(p1, Priority::MEDIUM, "P1 is not P2");
    assert_ne!(p1, Priority::LOW, "P1 is not P3");
    assert_ne!(p1, Priority::BACKLOG, "P1 is not P4");

    assert!(p1 > Priority::CRITICAL, "P1 sorts after P0");
    assert!(p1 < Priority::MEDIUM, "P1 sorts before P2");
    assert!(p1 < Priority::LOW, "P1 sorts before P3");
    assert!(p1 < Priority::BACKLOG, "P1 sorts before P4");

    // P1 is not the default priority (MEDIUM/P2 is)
    assert_ne!(
        p1,
        Priority::default(),
        "P1 should not be the default priority"
    );
}
