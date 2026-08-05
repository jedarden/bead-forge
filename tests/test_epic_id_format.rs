// Test: bf-58gaqu - Verify epic ID format follows prefix-hash pattern

use bead_forge::id::{generate_id, is_valid_bead_id, optimal_hash_length};

#[test]
fn test_epic_id_format() {
    // Test 1: Verify epic creation produces valid ID format
    let test_cases = vec![
        ("test-epic", 10),
        ("another-epic", 100),
        ("my-epic", 1000),
    ];

    for (title, count) in test_cases {
        let id = generate_id("bf", count);

        // Verify pattern: <prefix>-<hash>
        assert!(
            id.starts_with("bf-"),
            "ID should start with 'bf-' prefix: {}",
            id
        );

        // Verify hash part is lowercase alphanumeric
        let hash_part = id.split('-').nth(1).expect("Should have hash part");
        assert!(
            hash_part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()),
            "Hash should be lowercase alphanumeric: {}",
            id
        );

        // Verify hash length is in valid range [3, 8]
        assert!(
            hash_part.len() >= 3 && hash_part.len() <= 8,
            "Hash length should be 3-8 chars: {} (got {})",
            id,
            hash_part.len()
        );

        // Verify ID passes validation function
        assert!(is_valid_bead_id(&id), "ID should be valid: {}", id);
    }
}

#[test]
fn test_epic_id_uniqueness() {
    // Test 2: Verify uniqueness across multiple generations
    let mut ids = std::collections::HashSet::new();
    let corpus_size = 100;

    for _ in 0..corpus_size {
        let id = generate_id("bf", corpus_size);
        assert!(
            ids.insert(id.clone()),
            "Duplicate ID detected: {}",
            id
        );
    }

    assert_eq!(
        ids.len(),
        corpus_size,
        "Should generate {} unique IDs",
        corpus_size
    );
}

#[test]
fn test_epic_id_adaptive_length() {
    // Test 3: Verify hash length adapts to corpus size
    let small_corpus = generate_id("bf", 10);
    let medium_corpus = generate_id("bf", 5000);
    let large_corpus = generate_id("bf", 10000);

    let small_hash = small_corpus.split('-').nth(1).unwrap();
    let medium_hash = medium_corpus.split('-').nth(1).unwrap();
    let large_hash = large_corpus.split('-').nth(1).unwrap();

    // Verify expected lengths based on optimal_hash_length function
    assert_eq!(small_hash.len(), optimal_hash_length(10));
    assert_eq!(medium_hash.len(), optimal_hash_length(5000));
    assert_eq!(large_hash.len(), optimal_hash_length(10000));

    // Higher corpus should produce longer or equal hash
    assert!(large_hash.len() >= medium_hash.len());
    assert!(medium_hash.len() >= small_hash.len());
}

#[test]
fn test_real_epic_creation() {
    // Test 4: Integration test with actual epic creation
    // This verifies the actual bf create --type epic produces valid IDs
    let output = std::process::Command::new("bf")
        .args(&["create", "--title", "Integration Test Epic", "--type", "epic", "--priority", "0"])
        .output()
        .expect("Failed to execute bf create");

    assert!(output.status.success(), "bf create should succeed");

    let id = String::from_utf8_lossy(&output.stdout).trim().to_string();

    // Verify the created ID follows the pattern
    assert!(id.starts_with("bf-"), "Created ID should have bf- prefix: {}", id);
    assert!(is_valid_bead_id(&id), "Created ID should be valid: {}", id);

    // Cleanup: delete the test epic
    let _ = std::process::Command::new("bf")
        .args(&["close", &id, "--reason", "Test cleanup"])
        .output();
}
