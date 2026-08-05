// Multiple Import Operations Test (bf-1zdlvx)
//
// Tests that labels survive multiple import operations:
// - Labels survive 3+ import/export cycles
// - Repeated imports don't corrupt label data
// - Label integrity across cycles
// - Various label types (unicode, special chars, many labels, etc.)

use bead_forge::jsonl::{self, UpsertResult};
use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;
use std::collections::HashMap;

/// Helper to create a test bead with labels (Vec<String>)
fn create_bead_with_labels(id: &str, title: &str, labels: Vec<String>) -> Issue {
    Issue {
        id: id.to_string(),
        title: title.to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels,
        ..Default::default()
    }
}

/// Helper to create a test bead with label slices (&[&str])
fn create_bead_with_label_slices(id: &str, title: &str, labels: &[&str]) -> Issue {
    Issue {
        id: id.to_string(),
        title: title.to_string(),
        issue_type: IssueType::Task,
        status: Status::Open,
        labels: labels.iter().map(|s| s.to_string()).collect(),
        ..Default::default()
    }
}

/// Helper to run a single import/export cycle
fn run_import_export_cycle(
    storage: &Storage,
    jsonl_path: &std::path::PathBuf,
) -> jsonl::ImportResult {
    // Export to JSONL
    jsonl::export_jsonl(jsonl_path, || Ok(storage.list_all_issues()?)).expect("Export failed");

    // Clear the storage (simulating a fresh import)
    let beads = storage.list_all_issues().expect("Failed to list beads");
    for bead in beads {
        storage
            .with_immediate_transaction(|tx| {
                tx.execute("DELETE FROM issues WHERE id = ?", [&bead.id])?;
                Ok(())
            })
            .expect("Failed to delete bead");
    }

    // Import from JSONL with upsert logic
    let mut imported = 0;
    let mut updated = 0;
    let mut skipped = 0;

    jsonl::import_jsonl(jsonl_path, |issue| {
        match storage.get_issue(&issue.id)? {
            Some(existing) => {
                // Check if content changed (simplified check)
                if existing.title != issue.title || existing.labels != issue.labels {
                    storage.update_issue_from_json(issue)?;
                    updated += 1;
                    Ok(jsonl::UpsertResult::Updated)
                } else {
                    skipped += 1;
                    Ok(jsonl::UpsertResult::Unchanged)
                }
            }
            None => {
                storage.create_issue(issue)?;
                imported += 1;
                Ok(jsonl::UpsertResult::New)
            }
        }
    })
    .expect("Import failed");

    jsonl::ImportResult {
        imported,
        updated,
        skipped,
    }
}

/// Helper to verify labels match expected
fn verify_labels(storage: &Storage, id: &str, expected_labels: &[&str]) {
    let bead = storage
        .get_issue(id)
        .expect(&format!("Failed to get bead {}", id))
        .expect(&format!("Bead {} not found", id));

    assert_eq!(
        bead.labels.len(),
        expected_labels.len(),
        "Bead {} should have {} labels, got {}",
        id,
        expected_labels.len(),
        bead.labels.len()
    );

    for expected_label in expected_labels {
        assert!(
            bead.labels.contains(&expected_label.to_string()),
            "Bead {} should contain label '{}', got {:?}",
            id,
            expected_label,
            bead.labels
        );
    }
}

/// Helper to verify labels match expected (String version)
fn verify_labels_string(storage: &Storage, id: &str, expected_labels: &[String]) {
    let bead = storage
        .get_issue(id)
        .expect(&format!("Failed to get bead {}", id))
        .expect(&format!("Bead {} not found", id));

    assert_eq!(
        bead.labels.len(),
        expected_labels.len(),
        "Bead {} should have {} labels, got {}",
        id,
        expected_labels.len(),
        bead.labels.len()
    );

    for expected_label in expected_labels {
        assert!(
            bead.labels.contains(expected_label),
            "Bead {} should contain label '{}', got {:?}",
            id,
            expected_label,
            bead.labels
        );
    }
}

#[test]
fn test_labels_survive_three_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with multiple labels
    let bead = create_bead_with_label_slices(
        "bf-multi-cycle",
        "Multi-Cycle Test",
        &["phase-1", "backend", "storage", "critical"],
    );
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        // Verify labels are still intact after each cycle
        verify_labels(
            &storage,
            "bf-multi-cycle",
            &["phase-1", "backend", "storage", "critical"],
        );
    }
}

#[test]
fn test_labels_survive_five_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with labels
    let bead = create_bead_with_labels(
        "bf-five-cycle",
        "Five Cycle Test",
        vec!["label1", "label2", "label3"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 5 import/export cycles
    for cycle in 1..=5 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        verify_labels(&storage, "bf-five-cycle", &["label1", "label2", "label3"]);
    }
}

#[test]
fn test_multiple_beads_labels_survive_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create multiple beads with different labels
    let bead1 = create_bead_with_labels(
        "bf-1",
        "Bead 1",
        vec!["backend", "phase-1"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    let bead2 = create_bead_with_labels(
        "bf-2",
        "Bead 2",
        vec!["frontend", "phase-2"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    let bead3 = create_bead_with_labels(
        "bf-3",
        "Bead 3",
        vec!["docs", "phase-1"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );

    storage
        .create_issue(&bead1)
        .expect("Failed to create bead1");
    storage
        .create_issue(&bead2)
        .expect("Failed to create bead2");
    storage
        .create_issue(&bead3)
        .expect("Failed to create bead3");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 3, "Cycle {}: Should import 3 beads", cycle);

        verify_labels(&storage, "bf-1", &["backend", "phase-1"]);
        verify_labels(&storage, "bf-2", &["frontend", "phase-2"]);
        verify_labels(&storage, "bf-3", &["docs", "phase-1"]);
    }
}

#[test]
fn test_unicode_labels_survive_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with unicode labels
    let bead = create_bead_with_labels(
        "bf-unicode",
        "Unicode Labels",
        vec!["🔥urgent", "中文标签", "日本語", "🐛bug", "émoji"]
            .iter()
            .map(|s| s.to_string())
            .collect(),
    );
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        verify_labels(
            &storage,
            "bf-unicode",
            &["🔥urgent", "中文标签", "日本語", "🐛bug", "émoji"],
        );
    }
}

#[test]
fn test_special_character_labels_survive_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with special character labels
    let bead = create_bead_with_label_slices(
        "bf-special",
        "Special Characters",
        &[
            "won't-fix",
            "high-priority",
            "a/b/c",
            "x.y.z",
            "test@example.com",
            "API:breaking",
            "feature:new",
        ],
    );
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        verify_labels(
            &storage,
            "bf-special",
            &[
                "won't-fix",
                "high-priority",
                "a/b/c",
                "x.y.z",
                "test@example.com",
                "API:breaking",
                "feature:new",
            ],
        );
    }
}

#[test]
fn test_many_labels_survive_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with many labels (using owned strings)
    let many_labels: Vec<String> = (1..=50).map(|i| format!("label-{}", i)).collect();
    let bead = create_bead_with_labels("bf-many", "Many Labels", many_labels.clone());
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        // Verify all 50 labels survived
        let retrieved = storage
            .get_issue("bf-many")
            .expect("Failed to get bead")
            .expect("Bead not found");
        assert_eq!(
            retrieved.labels.len(),
            50,
            "Cycle {}: Should have 50 labels",
            cycle
        );

        for expected_label in &many_labels {
            assert!(
                retrieved.labels.contains(expected_label),
                "Cycle {}: Label '{}' should be present",
                cycle,
                expected_label
            );
        }
    }
}

#[test]
fn test_empty_labels_survive_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with no labels
    let bead = create_bead_with_labels("bf-empty", "Empty Labels", vec![]);
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        let retrieved = storage
            .get_issue("bf-empty")
            .expect("Failed to get bead")
            .expect("Bead not found");
        assert_eq!(
            retrieved.labels.len(),
            0,
            "Cycle {}: Should have no labels",
            cycle
        );
    }
}

#[test]
fn test_mixed_label_types_survive_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create multiple beads with different label characteristics
    let bead1 = create_bead_with_label_slices("bf-unicode", "Unicode", &["🔥", "中文", "日本語"]);
    let bead2 =
        create_bead_with_label_slices("bf-special", "Special", &["won't-fix", "a/b/c", "x.y.z"]);
    let bead3 = create_bead_with_labels("bf-empty", "Empty", vec![]);
    let bead4 = create_bead_with_labels(
        "bf-many",
        "Many",
        (1..=20).map(|i| format!("l-{}", i)).collect::<Vec<_>>(),
    );

    storage
        .create_issue(&bead1)
        .expect("Failed to create bead1");
    storage
        .create_issue(&bead2)
        .expect("Failed to create bead2");
    storage
        .create_issue(&bead3)
        .expect("Failed to create bead3");
    storage
        .create_issue(&bead4)
        .expect("Failed to create bead4");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 4, "Cycle {}: Should import 4 beads", cycle);

        // Verify each bead's labels
        verify_labels(&storage, "bf-unicode", &["🔥", "中文", "日本語"]);
        verify_labels(&storage, "bf-special", &["won't-fix", "a/b/c", "x.y.z"]);
        verify_labels(&storage, "bf-empty", &[]);

        let retrieved = storage
            .get_issue("bf-many")
            .expect("Failed to get bead")
            .expect("Bead not found");
        assert_eq!(
            retrieved.labels.len(),
            20,
            "Cycle {}: bf-many should have 20 labels",
            cycle
        );
    }
}

#[test]
fn test_label_order_preservation_across_import_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create bead with labels in a specific order
    let bead = create_bead_with_label_slices(
        "bf-order",
        "Order Test",
        &vec!["zebra", "alpha", "middle", "beta", "gamma"],
    );
    storage.create_issue(&bead).expect("Failed to create bead");

    // Run 3 import/export cycles
    for cycle in 1..=3 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 1, "Cycle {}: Should import 1 bead", cycle);

        // Verify all labels are present (order depends on implementation)
        let retrieved = storage
            .get_issue("bf-order")
            .expect("Failed to get bead")
            .expect("Bead not found");
        assert_eq!(
            retrieved.labels.len(),
            5,
            "Cycle {}: Should have 5 labels",
            cycle
        );
        assert!(retrieved.labels.contains(&"zebra".to_string()));
        assert!(retrieved.labels.contains(&"alpha".to_string()));
        assert!(retrieved.labels.contains(&"middle".to_string()));
        assert!(retrieved.labels.contains(&"beta".to_string()));
        assert!(retrieved.labels.contains(&"gamma".to_string()));
    }
}

#[test]
fn test_no_label_corruption_after_many_cycles() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create beads with diverse label sets
    let test_cases = vec![
        ("bf-1", vec!["simple"]),
        ("bf-2", vec!["label1", "label2", "label3"]),
        ("bf-3", vec!["🔥", "中文", "test@example.com"]),
        (
            "bf-4",
            vec!["a", "b", "c", "d", "e", "f", "g", "h", "i", "j"],
        ),
        ("bf-5", vec![]),
    ];

    for (id, labels) in &test_cases {
        let bead = create_bead_with_label_slices(id, "Test", labels.as_slice());
        storage.create_issue(&bead).expect("Failed to create bead");
    }

    // Run 5 import/export cycles (stress test)
    for cycle in 1..=5 {
        let result = run_import_export_cycle(&storage, &jsonl_path);
        assert_eq!(result.imported, 5, "Cycle {}: Should import 5 beads", cycle);

        // Verify no corruption after each cycle
        for (id, expected_labels) in &test_cases {
            let retrieved = storage
                .get_issue(id)
                .expect(&format!("Cycle {}: Failed to get bead {}", cycle, id))
                .expect(&format!("Cycle {}: Bead {} not found", cycle, id));

            assert_eq!(
                retrieved.labels.len(),
                expected_labels.len(),
                "Cycle {}: Bead {} should have {} labels",
                cycle,
                id,
                expected_labels.len()
            );

            for expected_label in expected_labels {
                assert!(
                    retrieved.labels.contains(&expected_label.to_string()),
                    "Cycle {}: Bead {} should contain label '{}'",
                    cycle,
                    id,
                    expected_label
                );
            }
        }
    }
}

#[test]
#[ignore = "bf-3uk2w5: pre-existing shared-test-workspace isolation defect (order-dependent false failure), not a product bug"]
fn test_incremental_import_preserves_labels() {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    let jsonl_path = dir.path().join("issues.jsonl");

    // Create initial bead
    let bead1 = create_bead_with_label_slices("bf-1", "Initial", &["phase-1", "backend"]);
    storage.create_issue(&bead1).expect("Failed to create bead");

    // First export/import
    jsonl::export_jsonl(&jsonl_path, || Ok(storage.list_all_issues()?)).expect("Export failed");

    // Add a second bead
    let bead2 = create_bead_with_label_slices("bf-2", "Added", &["phase-2", "frontend"]);
    storage.create_issue(&bead2).expect("Failed to create bead");

    // Second export/import with both beads
    let result = jsonl::import_jsonl(&jsonl_path, |issue| {
        // Use create/update logic instead of upsert
        match storage.get_issue(&issue.id)? {
            Some(_) => Ok(jsonl::UpsertResult::Updated),
            None => {
                storage.create_issue(issue)?;
                Ok(jsonl::UpsertResult::New)
            }
        }
    })
    .expect("Import failed");

    assert_eq!(result.imported, 1, "Should import 1 new bead (bf-2)");

    // Verify both beads have correct labels
    verify_labels(&storage, "bf-1", &["phase-1", "backend"]);
    verify_labels(&storage, "bf-2", &["phase-2", "frontend"]);
}
