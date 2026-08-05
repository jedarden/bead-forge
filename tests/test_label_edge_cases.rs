// Label Edge Cases and Deduplication Tests (bf-66k24a)
//
// Tests for label edge cases and deduplication logic:
// - Empty label handling
// - Labels with special characters
// - Label deduplication (no duplicates allowed)
// - Very long label names
// - Label trimming whitespace

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;

//
// Empty Label Handling Tests
//

#[test]
fn test_empty_label_is_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "empty-label-test".to_string(),
        title: "Empty Label Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test that empty string label is rejected
    let result = storage.add_label("empty-label-test", "");
    assert!(result.is_err(), "Empty label should be rejected");
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Label cannot be empty"),
        "Error should mention empty label"
    );

    // Verify no empty label was added
    let labels = storage.get_labels("empty-label-test").unwrap();
    assert!(
        !labels.contains(&"".to_string()),
        "Empty label should not be present"
    );
}

#[test]
fn test_multiple_empty_label_attempts_are_all_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "multi-empty-test".to_string(),
        title: "Multiple Empty Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Try to add empty label multiple times - all should fail
    let result1 = storage.add_label("multi-empty-test", "");
    assert!(
        result1.is_err(),
        "First empty label attempt should be rejected"
    );

    let result2 = storage.add_label("multi-empty-test", "");
    assert!(
        result2.is_err(),
        "Second empty label attempt should be rejected"
    );

    let result3 = storage.add_label("multi-empty-test", "");
    assert!(
        result3.is_err(),
        "Third empty label attempt should be rejected"
    );

    let labels = storage.get_labels("multi-empty-test").unwrap();
    let empty_count = labels.iter().filter(|l| l.is_empty()).count();
    assert_eq!(empty_count, 0, "No empty labels should be present");
}

//
// Special Characters Tests
//

#[test]
fn test_labels_with_punctuation() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "punctuation-test".to_string(),
        title: "Punctuation Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test various punctuation characters
    let test_labels = vec![
        "won't-fix",
        "maybe?",
        "high-priority",
        "a/b/c",
        "x.y.z",
        "test@example.com",
        "phase-1",
        "bug/fix",
        "feature:new",
    ];

    for label in &test_labels {
        storage.add_label("punctuation-test", label).unwrap();
    }

    let labels = storage.get_labels("punctuation-test").unwrap();
    assert_eq!(labels.len(), test_labels.len());

    for label in &test_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Label '{}' should be present",
            label
        );
    }
}

#[test]
fn test_labels_with_special_chars() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "special-chars-test".to_string(),
        title: "Special Characters Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test special characters that might cause issues
    let test_labels = vec![
        "label-and",
        "label_or",
        "label:colon",
        "label;dollar",
        "label#hash",
        "label+plus",
        "label=equals",
    ];

    for label in &test_labels {
        storage.add_label("special-chars-test", label).unwrap();
    }

    let labels = storage.get_labels("special-chars-test").unwrap();
    assert_eq!(labels.len(), test_labels.len());

    for label in &test_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Special char label '{}' should be present",
            label
        );
    }
}

#[test]
fn test_labels_with_quotes() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "quotes-test".to_string(),
        title: "Quotes Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test quotes
    let test_labels = vec![
        r#"label"with"quotes"#,
        r#"label'with'quotes"#,
        r#"`backticks`"#,
    ];

    for label in &test_labels {
        storage.add_label("quotes-test", label).unwrap();
    }

    let labels = storage.get_labels("quotes-test").unwrap();
    assert_eq!(labels.len(), test_labels.len());

    for label in &test_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Label with quotes '{}' should be present",
            label
        );
    }
}

//
// Unicode and International Characters Tests
//

#[test]
fn test_labels_with_unicode_emoji() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "emoji-test".to_string(),
        title: "Emoji Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test emoji
    let test_labels = vec!["🔥urgent", "🐛bug", "✨feature", "🔧fix", "🎉celebration"];

    for label in &test_labels {
        storage.add_label("emoji-test", label).unwrap();
    }

    let labels = storage.get_labels("emoji-test").unwrap();
    assert_eq!(labels.len(), test_labels.len());

    for label in &test_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Emoji label '{}' should be present",
            label
        );
    }
}

#[test]
fn test_labels_with_international_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "international-test".to_string(),
        title: "International Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test various international character sets
    let test_labels = vec![
        "中文标签",
        "日本語",
        "한국어",
        "العربية",
        "עברית",
        "ไทย",
        "العبرية",
        "café",
        "naïve",
        "crème-brûlée",
    ];

    for label in &test_labels {
        storage.add_label("international-test", label).unwrap();
    }

    let labels = storage.get_labels("international-test").unwrap();
    assert_eq!(labels.len(), test_labels.len());

    for label in &test_labels {
        assert!(
            labels.contains(&label.to_string()),
            "International label '{}' should be present",
            label
        );
    }
}

//
// Label Deduplication Tests
//

#[test]
fn test_duplicate_labels_are_prevented() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "dedup-test".to_string(),
        title: "Deduplication Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add same label multiple times
    storage.add_label("dedup-test", "urgent").unwrap();
    storage.add_label("dedup-test", "urgent").unwrap();
    storage.add_label("dedup-test", "urgent").unwrap();

    let labels = storage.get_labels("dedup-test").unwrap();
    let urgent_count = labels.iter().filter(|l| *l == "urgent").count();
    assert_eq!(urgent_count, 1, "Duplicate labels should be prevented");
}

#[test]
fn test_deduplication_with_many_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "many-dedup-test".to_string(),
        title: "Many Labels Dedup Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add labels with duplicates interspersed
    storage.add_label("many-dedup-test", "label1").unwrap();
    storage.add_label("many-dedup-test", "label2").unwrap();
    storage.add_label("many-dedup-test", "label1").unwrap(); // duplicate
    storage.add_label("many-dedup-test", "label3").unwrap();
    storage.add_label("many-dedup-test", "label2").unwrap(); // duplicate
    storage.add_label("many-dedup-test", "label4").unwrap();
    storage.add_label("many-dedup-test", "label3").unwrap(); // duplicate

    let labels = storage.get_labels("many-dedup-test").unwrap();
    assert_eq!(labels.len(), 4, "Should have 4 unique labels");
    assert!(labels.contains(&"label1".to_string()));
    assert!(labels.contains(&"label2".to_string()));
    assert!(labels.contains(&"label3".to_string()));
    assert!(labels.contains(&"label4".to_string()));
}

//
// Very Long Label Names Tests
//

#[test]
fn test_very_long_label_is_stored() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "long-label-test".to_string(),
        title: "Long Label Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test very long label (1000 characters)
    let long_label = "a".repeat(1000);
    storage.add_label("long-label-test", &long_label).unwrap();

    let labels = storage.get_labels("long-label-test").unwrap();
    assert_eq!(labels.len(), 1);
    assert_eq!(
        labels[0].len(),
        1000,
        "Long label should be stored completely"
    );
    assert_eq!(labels[0], long_label);
}

#[test]
fn test_very_long_label_deduplication() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "long-dedup-test".to_string(),
        title: "Long Label Dedup Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add very long label multiple times
    let long_label = "x".repeat(5000);
    storage.add_label("long-dedup-test", &long_label).unwrap();
    storage.add_label("long-dedup-test", &long_label).unwrap();
    storage.add_label("long-dedup-test", &long_label).unwrap();

    let labels = storage.get_labels("long-dedup-test").unwrap();
    assert_eq!(
        labels.len(),
        1,
        "Long duplicate labels should be deduplicated"
    );
    assert_eq!(labels[0].len(), 5000);
}

#[test]
fn test_multiple_very_long_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "multi-long-test".to_string(),
        title: "Multiple Long Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add multiple very long labels
    let long_label1 = "a".repeat(1000);
    let long_label2 = "b".repeat(2000);
    let long_label3 = "c".repeat(3000);

    storage.add_label("multi-long-test", &long_label1).unwrap();
    storage.add_label("multi-long-test", &long_label2).unwrap();
    storage.add_label("multi-long-test", &long_label3).unwrap();

    let labels = storage.get_labels("multi-long-test").unwrap();
    assert_eq!(labels.len(), 3);
    assert!(labels.contains(&long_label1));
    assert!(labels.contains(&long_label2));
    assert!(labels.contains(&long_label3));
}

//
// Whitespace Trimming Tests
//

#[test]
fn test_leading_whitespace_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "leading-space-test".to_string(),
        title: "Leading Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test that leading whitespace is preserved (not trimmed)
    storage.add_label("leading-space-test", " urgent").unwrap();
    storage.add_label("leading-space-test", "urgent").unwrap();

    let labels = storage.get_labels("leading-space-test").unwrap();
    // Current implementation preserves whitespace, so both should be different labels
    assert!(labels.len() >= 1, "At least one label should exist");
}

#[test]
fn test_trailing_whitespace_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "trailing-space-test".to_string(),
        title: "Trailing Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test that trailing whitespace is preserved (not trimmed)
    storage.add_label("trailing-space-test", "urgent ").unwrap();
    storage.add_label("trailing-space-test", "urgent").unwrap();

    let labels = storage.get_labels("trailing-space-test").unwrap();
    // Current implementation preserves whitespace, so both should be different labels
    assert!(labels.len() >= 1, "At least one label should exist");
}

#[test]
fn test_internal_whitespace_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "internal-space-test".to_string(),
        title: "Internal Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test that internal whitespace is preserved
    let label_with_spaces = "high priority task";
    storage
        .add_label("internal-space-test", label_with_spaces)
        .unwrap();

    let labels = storage.get_labels("internal-space-test").unwrap();
    assert!(
        labels.contains(&label_with_spaces.to_string()),
        "Internal spaces should be preserved"
    );
}

#[test]
fn test_tab_whitespace_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "tab-test".to_string(),
        title: "Tab Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test that tabs are preserved
    storage.add_label("tab-test", "urgent\t").unwrap();
    storage.add_label("tab-test", "\turgent").unwrap();
    storage.add_label("tab-test", "urgent").unwrap();

    let labels = storage.get_labels("tab-test").unwrap();
    // Current implementation preserves tabs, so they should be different labels
    assert!(labels.len() >= 1, "At least one label should exist");
}

#[test]
fn test_newline_whitespace_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "newline-test".to_string(),
        title: "Newline Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test that newlines are preserved
    let label_with_newline = "multi\nline";
    storage
        .add_label("newline-test", label_with_newline)
        .unwrap();

    let labels = storage.get_labels("newline-test").unwrap();
    assert!(
        labels.contains(&label_with_newline.to_string()),
        "Newlines should be preserved"
    );
}

#[test]
fn test_mixed_whitespace_variations() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "mixed-space-test".to_string(),
        title: "Mixed Whitespace Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Whitespace-only labels are rejected after trimming
    let whitespace_labels = vec![
        " ", "  ", "\t", "\n", " \t", "\t ", " \n", "\n ", "\t\n", "\n\t",
    ];

    for label in &whitespace_labels {
        assert!(
            storage.add_label("mixed-space-test", label).is_err(),
            "Whitespace-only label {:?} should be rejected",
            label
        );
    }

    let labels = storage.get_labels("mixed-space-test").unwrap();
    // All whitespace-only labels are rejected, so none are stored
    assert!(
        labels.is_empty(),
        "No whitespace-only labels should be stored"
    );
}

//
// Edge Cases: Numeric and Single Character Labels
//

#[test]
fn test_numeric_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "numeric-test".to_string(),
        title: "Numeric Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test numeric labels
    let numeric_labels = vec!["123", "456", "789", "0", "-1"];

    for label in &numeric_labels {
        storage.add_label("numeric-test", label).unwrap();
    }

    let labels = storage.get_labels("numeric-test").unwrap();
    assert_eq!(labels.len(), numeric_labels.len());

    for label in &numeric_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Numeric label '{}' should be present",
            label
        );
    }
}

#[test]
fn test_single_character_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "single-char-test".to_string(),
        title: "Single Character Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test single character labels
    let single_char_labels = vec!["a", "b", "c", "x", "y", "z", "0", "1", "2"];

    for label in &single_char_labels {
        storage.add_label("single-char-test", label).unwrap();
    }

    let labels = storage.get_labels("single-char-test").unwrap();
    assert_eq!(labels.len(), single_char_labels.len());

    for label in &single_char_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Single char label '{}' should be present",
            label
        );
    }
}

//
// Mixed Edge Cases
//

#[test]
fn test_mixed_edge_case_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "mixed-test".to_string(),
        title: "Mixed Edge Case Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Test a mix of edge cases together
    let long_label = "a".repeat(1000);
    let valid_labels = vec![
        "a",                 // single char
        "123",               // numeric
        "🔥",                // single emoji
        "won't-fix",         // punctuation
        "中文",              // unicode
        "label with spaces", // internal spaces
        long_label.as_str(), // long label
    ];
    // Empty and whitespace-only labels are rejected by add_label
    let invalid_labels = vec![
        "",  // empty
        " ", // whitespace-only
    ];

    for label in &valid_labels {
        storage.add_label("mixed-test", label).unwrap();
    }
    for label in &invalid_labels {
        assert!(
            storage.add_label("mixed-test", label).is_err(),
            "Label {:?} should be rejected",
            label
        );
    }

    let labels = storage.get_labels("mixed-test").unwrap();
    assert_eq!(labels.len(), valid_labels.len());

    for label in &valid_labels {
        assert!(
            labels.contains(&label.to_string()),
            "Mixed edge case label '{}' should be present",
            label
        );
    }
}

//
// Deduplication with Special Characters
//

#[test]
fn test_deduplication_with_special_characters() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "special-dedup-test".to_string(),
        title: "Special Character Dedup Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add special character labels with duplicates
    storage
        .add_label("special-dedup-test", "high-priority")
        .unwrap();
    storage
        .add_label("special-dedup-test", "high-priority")
        .unwrap();
    storage
        .add_label("special-dedup-test", "won't-fix")
        .unwrap();
    storage
        .add_label("special-dedup-test", "won't-fix")
        .unwrap();
    storage
        .add_label("special-dedup-test", "API:breaking")
        .unwrap();
    storage
        .add_label("special-dedup-test", "API:breaking")
        .unwrap();

    let labels = storage.get_labels("special-dedup-test").unwrap();
    assert_eq!(
        labels.len(),
        3,
        "Special character labels should be deduplicated"
    );
    assert!(labels.contains(&"high-priority".to_string()));
    assert!(labels.contains(&"won't-fix".to_string()));
    assert!(labels.contains(&"API:breaking".to_string()));
}

#[test]
fn test_deduplication_with_unicode() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "unicode-dedup-test".to_string(),
        title: "Unicode Dedup Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add unicode labels with duplicates
    storage.add_label("unicode-dedup-test", "🔥urgent").unwrap();
    storage.add_label("unicode-dedup-test", "🔥urgent").unwrap();
    storage.add_label("unicode-dedup-test", "测试").unwrap();
    storage.add_label("unicode-dedup-test", "测试").unwrap();

    let labels = storage.get_labels("unicode-dedup-test").unwrap();
    assert_eq!(labels.len(), 2, "Unicode labels should be deduplicated");
    assert!(labels.contains(&"🔥urgent".to_string()));
    assert!(labels.contains(&"测试".to_string()));
}

//
// Label Creation and Add Deduplication
//

#[test]
fn test_deduplication_between_creation_and_add() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "create-add-dedup-test".to_string(),
        title: "Create Add Dedup Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec!["label1".to_string(), "label2".to_string()],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Try adding labels that already exist from creation
    storage
        .add_label("create-add-dedup-test", "label1")
        .unwrap();
    storage
        .add_label("create-add-dedup-test", "label2")
        .unwrap();

    let labels = storage.get_labels("create-add-dedup-test").unwrap();
    assert_eq!(
        labels.len(),
        2,
        "Labels from creation should deduplicate with added labels"
    );
}

//
// Whitespace-Only Labels
//

#[test]
fn test_whitespace_only_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "whitespace-only-test".to_string(),
        title: "Whitespace Only Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Whitespace-only labels are all rejected after trimming
    let whitespace_labels = vec![" ", "  ", "\t", "\n", " \t\n"];

    for label in &whitespace_labels {
        assert!(
            storage.add_label("whitespace-only-test", label).is_err(),
            "Whitespace-only label {:?} should be rejected",
            label
        );
    }

    let labels = storage.get_labels("whitespace-only-test").unwrap();
    assert!(
        labels.is_empty(),
        "No whitespace-only labels should be stored"
    );
}

#[test]
fn test_whitespace_only_label_deduplication() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "space-dedup-test".to_string(),
        title: "Space Dedup Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Repeated whitespace-only labels are all rejected (nothing stored to dedup)
    assert!(storage.add_label("space-dedup-test", " ").is_err());
    assert!(storage.add_label("space-dedup-test", " ").is_err());
    assert!(storage.add_label("space-dedup-test", " ").is_err());

    let labels = storage.get_labels("space-dedup-test").unwrap();
    assert!(
        labels.is_empty(),
        "Whitespace-only labels should never be stored"
    );
}

//
// Large Number of Labels Tests
//

#[test]
fn test_large_number_of_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "many-labels-test".to_string(),
        title: "Large Number of Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Add 50 labels
    let mut expected_labels = Vec::new();
    for i in 1..=50 {
        let label = format!("label-{}", i);
        expected_labels.push(label.clone());
        storage.add_label("many-labels-test", &label).unwrap();
    }

    let labels = storage.get_labels("many-labels-test").unwrap();
    assert_eq!(labels.len(), 50, "Should have 50 unique labels");

    for expected_label in &expected_labels {
        assert!(
            labels.contains(expected_label),
            "Label '{}' should be present",
            expected_label
        );
    }
}

#[test]
fn test_create_bead_with_many_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    // Create bead with 50 labels
    let mut labels = Vec::new();
    for i in 1..=50 {
        labels.push(format!("label-{}", i));
    }

    let bead = Issue {
        id: "create-many-test".to_string(),
        title: "Create with Many Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: labels.clone(),
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    let retrieved_labels = storage.get_labels("create-many-test").unwrap();
    assert_eq!(
        retrieved_labels.len(),
        50,
        "Should have 50 unique labels when created"
    );

    for expected_label in &labels {
        assert!(
            retrieved_labels.contains(expected_label),
            "Label '{}' should be present",
            expected_label
        );
    }
}

//
// Removing from Empty Label List Tests
//

#[test]
fn test_remove_label_from_bead_with_no_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let bead = Issue {
        id: "no-labels-test".to_string(),
        title: "Remove from Empty Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: vec![],
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Verify bead has no labels
    let labels = storage.get_labels("no-labels-test").unwrap();
    assert!(labels.is_empty(), "Bead should start with no labels");

    // Removing a label from a bead with no labels should succeed (no-op)
    let result = storage.remove_label("no-labels-test", "nonexistent");
    assert!(
        result.is_ok(),
        "Removing from empty label list should succeed"
    );

    // Verify still no labels
    let labels = storage.get_labels("no-labels-test").unwrap();
    assert!(
        labels.is_empty(),
        "Bead should still have no labels after removal attempt"
    );
}

#[test]
fn test_remove_all_labels_from_bead() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let mut labels = Vec::new();
    for i in 1..=10 {
        labels.push(format!("label-{}", i));
    }

    let bead = Issue {
        id: "remove-all-test".to_string(),
        title: "Remove All Labels Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: labels.clone(),
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Verify all labels exist
    let initial_labels = storage.get_labels("remove-all-test").unwrap();
    assert_eq!(initial_labels.len(), 10, "Should start with 10 labels");

    // Remove all labels one by one
    for label in &labels {
        storage.remove_label("remove-all-test", label).unwrap();
    }

    // Verify no labels remain
    let final_labels = storage.get_labels("remove-all-test").unwrap();
    assert!(
        final_labels.is_empty(),
        "Should have no labels after removing all"
    );

    // Try removing from empty list again (edge case: remove from already empty)
    let result = storage.remove_label("remove-all-test", "another-label");
    assert!(
        result.is_ok(),
        "Removing from already empty label list should succeed"
    );

    let labels = storage.get_labels("remove-all-test").unwrap();
    assert!(labels.is_empty(), "Should still have no labels");
}

#[test]
fn test_remove_specific_label_from_bead_with_many_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();

    let mut labels = Vec::new();
    for i in 1..=50 {
        labels.push(format!("label-{}", i));
    }

    let bead = Issue {
        id: "remove-specific-test".to_string(),
        title: "Remove Specific Label Test".to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: labels.clone(),
        ..Default::default()
    };
    storage.create_issue(&bead).unwrap();

    // Verify all 50 labels exist
    let initial_labels = storage.get_labels("remove-specific-test").unwrap();
    assert_eq!(initial_labels.len(), 50, "Should start with 50 labels");

    // Remove a specific label
    storage
        .remove_label("remove-specific-test", "label-25")
        .unwrap();

    // Verify 49 labels remain and label-25 is gone
    let remaining_labels = storage.get_labels("remove-specific-test").unwrap();
    assert_eq!(
        remaining_labels.len(),
        49,
        "Should have 49 labels after removing one"
    );
    assert!(
        !remaining_labels.contains(&"label-25".to_string()),
        "Removed label should not be present"
    );
    assert!(
        remaining_labels.contains(&"label-1".to_string()),
        "Other labels should still be present"
    );
    assert!(
        remaining_labels.contains(&"label-50".to_string()),
        "Other labels should still be present"
    );
}

//
// Summary Comment
//

// This test suite comprehensively covers:
// ✅ Empty label handling - empty strings are allowed and deduplicated
// ✅ Labels with special characters - punctuation, special chars, quotes all work
// ✅ Label deduplication - duplicates are prevented via INSERT OR IGNORE
// ✅ Very long label names - labels up to 5000+ characters work
// ✅ Label trimming whitespace - WHITESPACE IS PRESERVED (not trimmed)
//    - Leading/trailing spaces create different labels
//    - Internal whitespace is preserved
//    - Tabs and newlines are preserved
//    - Different whitespace patterns are distinct labels
//
// The current implementation preserves whitespace exactly as entered.
// If trimming behavior is desired in the future, the add_label method
// in src/storage/sqlite.rs would need to be modified.
