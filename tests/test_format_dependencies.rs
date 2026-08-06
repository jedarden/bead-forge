//! Unit tests for format_dependencies_display() function
//!
//! Tests the text formatting of dependency display objects from storage.

use bead_forge::format::text::format_dependencies_display;
use bead_forge::storage::sqlite::DependencyDisplay;

#[test]
fn test_format_dependencies_display_empty() {
    let deps: Vec<DependencyDisplay> = vec![];
    let result = format_dependencies_display(&deps);
    assert_eq!(result, "");
}

#[test]
fn test_format_dependencies_display_single_blocking() {
    let deps = vec![DependencyDisplay {
        dep_type: "blocks".to_string(),
        bead_id: "bf-abc123".to_string(),
        title: "Blocker task".to_string(),
    }];

    let result = format_dependencies_display(&deps);
    assert_eq!(result, "Depends: bf-abc123 (Blocker task) (blocks)");
}

#[test]
fn test_format_dependencies_display_single_non_blocking() {
    let deps = vec![DependencyDisplay {
        dep_type: "related".to_string(),
        bead_id: "bf-def456".to_string(),
        title: "Related task".to_string(),
    }];

    let result = format_dependencies_display(&deps);
    assert_eq!(result, "Depends: bf-def456 (Related task)");
}

#[test]
fn test_format_dependencies_display_multiple_mixed() {
    let deps = vec![
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-blocker1".to_string(),
            title: "First blocker".to_string(),
        },
        DependencyDisplay {
            dep_type: "related".to_string(),
            bead_id: "bf-related1".to_string(),
            title: "Related task".to_string(),
        },
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-blocker2".to_string(),
            title: "Second blocker".to_string(),
        },
    ];

    let result = format_dependencies_display(&deps);
    assert_eq!(
        result,
        "Depends: bf-blocker1 (First blocker) (blocks), bf-related1 (Related task), bf-blocker2 (Second blocker) (blocks)"
    );
}

#[test]
fn test_format_dependencies_display_multiple_all_blocking() {
    let deps = vec![
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-blocker1".to_string(),
            title: "Critical blocker".to_string(),
        },
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-blocker2".to_string(),
            title: "Another blocker".to_string(),
        },
    ];

    let result = format_dependencies_display(&deps);
    assert_eq!(
        result,
        "Depends: bf-blocker1 (Critical blocker) (blocks), bf-blocker2 (Another blocker) (blocks)"
    );
}

#[test]
fn test_format_dependencies_display_special_characters_title() {
    let deps = vec![
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-special1".to_string(),
            title: "Task with \"quotes\" and 'apostrophes'".to_string(),
        },
        DependencyDisplay {
            dep_type: "related".to_string(),
            bead_id: "bf-special2".to_string(),
            title: "Task with <angle> & [brackets] & {braces}".to_string(),
        },
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-special3".to_string(),
            title: "Task with slashes/\\backslashes and pipes|".to_string(),
        },
    ];

    let result = format_dependencies_display(&deps);
    assert_eq!(
        result,
        "Depends: bf-special1 (Task with \"quotes\" and 'apostrophes') (blocks), bf-special2 (Task with <angle> & [brackets] & {braces}), bf-special3 (Task with slashes/\\backslashes and pipes|) (blocks)"
    );
}

#[test]
fn test_format_dependencies_display_unicode_characters() {
    let deps = vec![
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-unicode1".to_string(),
            title: "Tâsk with spëcial çharacters".to_string(),
        },
        DependencyDisplay {
            dep_type: "related".to_string(),
            bead_id: "bf-unicode2".to_string(),
            title: "日本語のタスク".to_string(),
        },
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-unicode3".to_string(),
            title: "Задача с кириллицей".to_string(),
        },
    ];

    let result = format_dependencies_display(&deps);
    assert_eq!(
        result,
        "Depends: bf-unicode1 (Tâsk with spëcial çharacters) (blocks), bf-unicode2 (日本語のタスク), bf-unicode3 (Задача с кириллицей) (blocks)"
    );
}

#[test]
fn test_format_dependencies_display_empty_title() {
    let deps = vec![DependencyDisplay {
        dep_type: "blocks".to_string(),
        bead_id: "bf-empty".to_string(),
        title: "".to_string(),
    }];

    let result = format_dependencies_display(&deps);
    assert_eq!(result, "Depends: bf-empty () (blocks)");
}

#[test]
fn test_format_dependencies_display_parent_type() {
    let deps = vec![DependencyDisplay {
        dep_type: "parent".to_string(),
        bead_id: "bf-parent".to_string(),
        title: "Parent task".to_string(),
    }];

    let result = format_dependencies_display(&deps);
    assert_eq!(result, "Depends: bf-parent (Parent task)");
}

#[test]
fn test_format_dependencies_display_long_title() {
    let deps = vec![DependencyDisplay {
        dep_type: "blocks".to_string(),
        bead_id: "bf-long".to_string(),
        title: "This is a very long title that goes on and on and contains a lot of words but should still be formatted correctly without any truncation or issues whatsoever in the output string".to_string(),
    }];

    let result = format_dependencies_display(&deps);
    assert_eq!(
        result,
        "Depends: bf-long (This is a very long title that goes on and on and contains a lot of words but should still be formatted correctly without any truncation or issues whatsoever in the output string) (blocks)"
    );
}

#[test]
fn test_format_dependencies_display_newlines_and_tabs() {
    let deps = vec![
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-whitespace1".to_string(),
            title: "Task with\nnewlines".to_string(),
        },
        DependencyDisplay {
            dep_type: "related".to_string(),
            bead_id: "bf-whitespace2".to_string(),
            title: "Task with\ttabs".to_string(),
        },
    ];

    let result = format_dependencies_display(&deps);
    assert_eq!(
        result,
        "Depends: bf-whitespace1 (Task with\nnewlines) (blocks), bf-whitespace2 (Task with\ttabs)"
    );
}

#[test]
fn test_format_dependencies_display_order_preserved() {
    let deps = vec![
        DependencyDisplay {
            dep_type: "related".to_string(),
            bead_id: "bf-third".to_string(),
            title: "Third task".to_string(),
        },
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-first".to_string(),
            title: "First task".to_string(),
        },
        DependencyDisplay {
            dep_type: "blocks".to_string(),
            bead_id: "bf-second".to_string(),
            title: "Second task".to_string(),
        },
    ];

    let result = format_dependencies_display(&deps);
    // Verify the original order is preserved
    assert_eq!(
        result,
        "Depends: bf-third (Third task), bf-first (First task) (blocks), bf-second (Second task) (blocks)"
    );
}
