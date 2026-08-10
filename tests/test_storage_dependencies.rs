//! Unit tests for storage layer dependency query functions.
//!
//! Tests the `get_dependencies_display()` query function which performs
//! a JOIN between the `dependencies` and `issues` tables to retrieve
//! dependency type, bead ID, and title for all dependencies of a bead.

use bead_forge::model::{DependencyType, Issue};
use bead_forge::storage::Storage;

mod common;
use common::TempWorkspace;

/// Test that a bead with no dependencies returns an empty vector.
#[test]
fn test_get_dependencies_display_empty() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create a bead with no dependencies
    let bead_id = "bf-empty-001";
    let bead = Issue::new(bead_id.to_string(), "Independent bead".to_string(), ".".to_string());
    storage.create_issue(&bead).unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(bead_id).unwrap();

    // Should return empty vector
    assert!(
        result.is_empty(),
        "Bead with no dependencies should return empty vec, got {:?}",
        result
    );
}

/// Test a bead with a single blocking dependency.
#[test]
fn test_get_dependencies_display_single_blocking() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create two beads: blocker and dependent
    let blocker_id = "bf-blocker-001";
    let dependent_id = "bf-dependent-001";

    let blocker = Issue::new(
        blocker_id.to_string(),
        "Blocker task".to_string(),
        ".".to_string(),
    );
    let dependent = Issue::new(
        dependent_id.to_string(),
        "Dependent task".to_string(),
        ".".to_string(),
    );

    storage.create_issue(&blocker).unwrap();
    storage.create_issue(&dependent).unwrap();

    // Add blocking dependency
    storage
        .add_dependency(
            dependent_id,
            blocker_id,
            &DependencyType::Blocks,
            "test",
        )
        .unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(dependent_id).unwrap();

    // Should return exactly one dependency
    assert_eq!(result.len(), 1, "Should have exactly one dependency");

    let dep = &result[0];
    assert_eq!(
        dep.dep_type, "blocks",
        "Dependency type should be 'blocks', got '{}'",
        dep.dep_type
    );
    assert_eq!(
        dep.bead_id, blocker_id,
        "Bead ID should match blocker ID, got '{}'",
        dep.bead_id
    );
    assert_eq!(
        dep.title, "Blocker task",
        "Title should match blocker title, got '{}'",
        dep.title
    );
}

/// Test a bead with multiple dependencies of the same type.
#[test]
fn test_get_dependencies_display_multiple_same_type() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create a bead with three blockers
    let main_id = "bf-main-001";
    let blocker1_id = "bf-blocker-001";
    let blocker2_id = "bf-blocker-002";
    let blocker3_id = "bf-blocker-003";

    let main = Issue::new(main_id.to_string(), "Main task".to_string(), ".".to_string());
    let blocker1 = Issue::new(
        blocker1_id.to_string(),
        "First blocker".to_string(),
        ".".to_string(),
    );
    let blocker2 = Issue::new(
        blocker2_id.to_string(),
        "Second blocker".to_string(),
        ".".to_string(),
    );
    let blocker3 = Issue::new(
        blocker3_id.to_string(),
        "Third blocker".to_string(),
        ".".to_string(),
    );

    storage.create_issue(&main).unwrap();
    storage.create_issue(&blocker1).unwrap();
    storage.create_issue(&blocker2).unwrap();
    storage.create_issue(&blocker3).unwrap();

    // Add all three blocking dependencies
    storage
        .add_dependency(main_id, blocker1_id, &DependencyType::Blocks, "test")
        .unwrap();
    storage
        .add_dependency(main_id, blocker2_id, &DependencyType::Blocks, "test")
        .unwrap();
    storage
        .add_dependency(main_id, blocker3_id, &DependencyType::Blocks, "test")
        .unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(main_id).unwrap();

    // Should return exactly three dependencies
    assert_eq!(result.len(), 3, "Should have exactly three dependencies");

    // Verify each dependency has correct structure
    for dep in &result {
        assert_eq!(
            dep.dep_type, "blocks",
            "All dependencies should be 'blocks' type, got '{}'",
            dep.dep_type
        );
        assert!(
            [blocker1_id, blocker2_id, blocker3_id].contains(&dep.bead_id.as_str()),
            "Bead ID should be one of the blockers, got '{}'",
            dep.bead_id
        );
    }

    // Verify all blocker IDs are present
    let dep_ids: Vec<&str> = result.iter().map(|d| d.bead_id.as_str()).collect();
    assert!(
        dep_ids.contains(&blocker1_id),
        "Should include first blocker ID"
    );
    assert!(
        dep_ids.contains(&blocker2_id),
        "Should include second blocker ID"
    );
    assert!(
        dep_ids.contains(&blocker3_id),
        "Should include third blocker ID"
    );
}

/// Test a bead with dependencies of different types.
#[test]
fn test_get_dependencies_display_mixed_types() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create beads for different dependency types
    let main_id = "bf-main-001";
    let blocker_id = "bf-blocker-001";
    let related_id = "bf-related-001";
    let parent_id = "bf-parent-001";

    let main = Issue::new(main_id.to_string(), "Main task".to_string(), ".".to_string());
    let blocker = Issue::new(
        blocker_id.to_string(),
        "Blocking task".to_string(),
        ".".to_string(),
    );
    let related = Issue::new(
        related_id.to_string(),
        "Related task".to_string(),
        ".".to_string(),
    );
    let parent = Issue::new(parent_id.to_string(), "Parent epic".to_string(), ".".to_string());

    storage.create_issue(&main).unwrap();
    storage.create_issue(&blocker).unwrap();
    storage.create_issue(&related).unwrap();
    storage.create_issue(&parent).unwrap();

    // Add different dependency types
    storage
        .add_dependency(main_id, blocker_id, &DependencyType::Blocks, "test")
        .unwrap();
    storage
        .add_dependency(main_id, related_id, &DependencyType::RelatesTo, "test")
        .unwrap();
    storage
        .add_dependency(main_id, parent_id, &DependencyType::ParentChild, "test")
        .unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(main_id).unwrap();

    // Should return exactly three dependencies
    assert_eq!(result.len(), 3, "Should have exactly three dependencies");

    // Verify dependency types are preserved
    let dep_types: Vec<&str> = result.iter().map(|d| d.dep_type.as_str()).collect();
    assert!(
        dep_types.contains(&"blocks"),
        "Should include 'blocks' type"
    );
    assert!(
        dep_types.contains(&"relates-to"),
        "Should include 'relates-to' type"
    );
    assert!(
        dep_types.contains(&"parent-child"),
        "Should include 'parent-child' type"
    );

    // Verify each dependency has the correct bead ID and title
    for dep in &result {
        match dep.dep_type.as_str() {
            "blocks" => {
                assert_eq!(dep.bead_id, blocker_id);
                assert_eq!(dep.title, "Blocking task");
            }
            "relates-to" => {
                assert_eq!(dep.bead_id, related_id);
                assert_eq!(dep.title, "Related task");
            }
            "parent-child" => {
                assert_eq!(dep.bead_id, parent_id);
                assert_eq!(dep.title, "Parent epic");
            }
            _ => panic!("Unexpected dependency type: {}", dep.dep_type),
        }
    }
}

/// Test that the JOIN query correctly retrieves bead titles.
#[test]
fn test_get_dependencies_display_join_correctness() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create beads with distinctive titles
    let main_id = "bf-main-001";
    let dep_id = "bf-dependency-001";

    let main = Issue::new(
        main_id.to_string(),
        "Main bead".to_string(),
        ".".to_string(),
    );
    let dep = Issue::new(
        dep_id.to_string(),
        "Very Distinctive Title For Dependency".to_string(),
        ".".to_string(),
    );

    storage.create_issue(&main).unwrap();
    storage.create_issue(&dep).unwrap();

    // Add dependency
    storage
        .add_dependency(main_id, dep_id, &DependencyType::Blocks, "test")
        .unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(main_id).unwrap();

    assert_eq!(result.len(), 1);
    let dep_display = &result[0];

    // Verify the JOIN correctly retrieved the title from issues table
    assert_eq!(
        dep_display.title, "Very Distinctive Title For Dependency",
        "JOIN query should correctly retrieve bead title from issues table"
    );
    assert_eq!(dep_display.bead_id, dep_id, "Bead ID should match");
    assert_eq!(dep_display.dep_type, "blocks", "Type should be 'blocks'");
}

/// Test querying dependencies for a non-existent bead.
#[test]
fn test_get_dependencies_display_nonexistent_bead() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Query dependencies for a bead that doesn't exist
    let result = storage.get_dependencies_display("bf-nonexistent-001").unwrap();

    // Should return empty vector (no rows match the WHERE clause)
    assert!(
        result.is_empty(),
        "Non-existent bead should return empty vec, got {:?}",
        result
    );
}

/// Test that dependencies are queried based on issue_id, not depends_on_id.
///
/// This verifies the WHERE clause correctly filters by the parent bead
/// (the one that has dependencies), not the beads being depended upon.
#[test]
fn test_get_dependencies_display_direction_correctness() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create three beads: A -> B (A depends on B)
    let bead_a = "bf-bead-a";
    let bead_b = "bf-bead-b";

    let a = Issue::new(bead_a.to_string(), "Bead A".to_string(), ".".to_string());
    let b = Issue::new(bead_b.to_string(), "Bead B".to_string(), ".".to_string());

    storage.create_issue(&a).unwrap();
    storage.create_issue(&b).unwrap();

    // A depends on B (B blocks A)
    storage
        .add_dependency(bead_a, bead_b, &DependencyType::Blocks, "test")
        .unwrap();

    // Query dependencies for bead A (should return B)
    let deps_a = storage.get_dependencies_display(bead_a).unwrap();
    assert_eq!(
        deps_a.len(),
        1,
        "Bead A should have one dependency (bead B)"
    );
    assert_eq!(deps_a[0].bead_id, bead_b, "Dependency should be bead B");

    // Query dependencies for bead B (should return empty)
    let deps_b = storage.get_dependencies_display(bead_b).unwrap();
    assert!(
        deps_b.is_empty(),
        "Bead B should have no dependencies (it is depended upon, not depending)"
    );
}

/// Test with a bead that both blocks and is blocked (middle of a chain).
///
/// This tests the case where a bead has dependencies (blocked_by) and also
/// appears as a blocker for other beads (blocks). The query should only
/// return what the bead depends on, not what depends on it.
#[test]
fn test_get_dependencies_display_middle_of_chain() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create chain: A -> B -> C (B depends on A, C depends on B)
    let bead_a = "bf-bead-a";
    let bead_b = "bf-bead-b";
    let bead_c = "bf-bead-c";

    let a = Issue::new(bead_a.to_string(), "Bead A".to_string(), ".".to_string());
    let b = Issue::new(bead_b.to_string(), "Bead B".to_string(), ".".to_string());
    let c = Issue::new(bead_c.to_string(), "Bead C".to_string(), ".".to_string());

    storage.create_issue(&a).unwrap();
    storage.create_issue(&b).unwrap();
    storage.create_issue(&c).unwrap();

    // Create dependencies: B depends on A, C depends on B
    storage
        .add_dependency(bead_b, bead_a, &DependencyType::Blocks, "test")
        .unwrap();
    storage
        .add_dependency(bead_c, bead_b, &DependencyType::Blocks, "test")
        .unwrap();

    // Query dependencies for bead B (middle of chain)
    let deps_b = storage.get_dependencies_display(bead_b).unwrap();

    // Should only return what B depends on (A), not what depends on B (C)
    assert_eq!(
        deps_b.len(),
        1,
        "Middle bead should only return its dependencies, not dependents"
    );
    assert_eq!(
        deps_b[0].bead_id, bead_a,
        "Middle bead B should only show dependency on A"
    );
    assert_ne!(deps_b[0].bead_id, bead_c, "Should NOT include C (depends on B)");
}

/// Test that titles with special characters are handled correctly.
#[test]
fn test_get_dependencies_display_special_characters() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create beads with special characters in titles
    let main_id = "bf-main-001";
    let dep_id = "bf-dep-001";

    let main = Issue::new(main_id.to_string(), "Main task".to_string(), ".".to_string());
    let dep = Issue::new(
        dep_id.to_string(),
        "Task with 'quotes' and \"double quotes\" and <special>".to_string(),
        ".".to_string(),
    );

    storage.create_issue(&main).unwrap();
    storage.create_issue(&dep).unwrap();

    // Add dependency
    storage
        .add_dependency(main_id, dep_id, &DependencyType::Blocks, "test")
        .unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(main_id).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].title,
        "Task with 'quotes' and \"double quotes\" and <special>"
    );
}

/// Test with long titles (within database limit) to ensure no truncation occurs.
#[test]
fn test_get_dependencies_display_long_title() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create bead with a long title (but under the 500 character limit)
    let main_id = "bf-main-001";
    let dep_id = "bf-dep-001";

    // Create a 400-character title (well under the 500-char limit)
    let long_title = "This is a moderately long title that is still under the database limit of 500 characters and should be stored and retrieved without truncation. This text repeats to reach approximately four hundred characters. "
        .repeat(2)
        .chars()
        .take(400)
        .collect::<String>();

    let main = Issue::new(main_id.to_string(), "Main task".to_string(), ".".to_string());
    let dep = Issue::new(dep_id.to_string(), long_title.clone(), ".".to_string());

    storage.create_issue(&main).unwrap();
    storage.create_issue(&dep).unwrap();

    // Add dependency
    storage
        .add_dependency(main_id, dep_id, &DependencyType::Blocks, "test")
        .unwrap();

    // Query dependencies display
    let result = storage.get_dependencies_display(main_id).unwrap();

    assert_eq!(result.len(), 1);
    assert_eq!(
        result[0].title, long_title,
        "Long titles should not be truncated"
    );
}

/// Test that all DependencyType variants are correctly stringified.
#[test]
fn test_get_dependencies_display_all_dependency_types() {
    let ws = TempWorkspace::new().unwrap();
    let storage = ws.storage().unwrap();

    // Create a main bead
    let main_id = "bf-main-001";
    let main = Issue::new(main_id.to_string(), "Main task".to_string(), ".".to_string());
    storage.create_issue(&main).unwrap();

    // Test each dependency type
    let dep_types = vec![
        ("blocks", DependencyType::Blocks),
        ("parent-child", DependencyType::ParentChild),
        ("conditional-blocks", DependencyType::ConditionalBlocks),
        ("waits-for", DependencyType::WaitsFor),
        ("relates-to", DependencyType::RelatesTo),
        ("related", DependencyType::Related),
        ("duplicates", DependencyType::Duplicates),
        ("supersedes", DependencyType::Supersedes),
        ("caused-by", DependencyType::CausedBy),
        ("discovered-from", DependencyType::DiscoveredFrom),
        ("replies-to", DependencyType::RepliesTo),
    ];

    for (expected_type_str, dep_type) in dep_types {
        let dep_id = format!("bf-dep-{}", expected_type_str.replace("-", "_"));
        let dep = Issue::new(
            dep_id.clone(),
            format!("Dependency for {}", expected_type_str),
            ".".to_string(),
        );
        storage.create_issue(&dep).unwrap();

        storage
            .add_dependency(main_id, &dep_id, &dep_type, "test")
            .unwrap();

        // Query and verify the type string
        let result = storage.get_dependencies_display(main_id).unwrap();
        let found = result.iter().find(|d| d.bead_id == dep_id);

        assert!(
            found.is_some(),
            "Should find dependency with type {}",
            expected_type_str
        );
        assert_eq!(
            found.unwrap().dep_type, expected_type_str,
            "DependencyType variant should stringify correctly"
        );
    }
}
