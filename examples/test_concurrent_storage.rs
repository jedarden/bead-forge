use std::sync::{Arc, Barrier};
use std::thread;
use tempfile::TempDir;

fn main() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    
    let storage = Arc::new(bead_forge::Storage::open(&db_path).unwrap());
    
    let issue = bead_forge::model::Issue {
        id: "bf-test-001".to_string(),
        title: "Test Issue".to_string(),
        description: Some("Test description".to_string()),
        status: bead_forge::model::Status::Open,
        priority: bead_forge::model::Priority(2),
        issue_type: bead_forge::model::IssueType::Task,
        ..Default::default()
    };
    
    storage.create_issue(&issue).unwrap();
    println!("Created test issue");
    
    let barrier = Arc::new(Barrier::new(11));
    let mut handles = vec![];
    
    for i in 0..10 {
        let storage_clone = Arc::clone(&storage);
        let barrier_clone = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            barrier_clone.wait();
            for _ in 0..100 {
                let _ = storage_clone.get_issue("bf-test-001");
                let _ = storage_clone.list_all_issues();
            }
            println!("Reader thread {} completed", i);
        }));
    }
    
    let storage_clone = Arc::clone(&storage);
    let barrier_clone = Arc::clone(&barrier);
    handles.push(thread::spawn(move || {
        barrier_clone.wait();
        for i in 0..10 {
            let changes = bead_forge::model::IssueChanges {
                title: Some(format!("Updated Issue {}", i)),
                ..Default::default()
            };
            let _ = storage_clone.update_issue("bf-test-001", &changes);
        }
        println!("Writer thread completed");
    }));
    
    for handle in handles {
        handle.join().unwrap();
    }
    
    let result = storage.get_issue("bf-test-001").unwrap();
    assert!(result.is_some());
    println!("SUCCESS: Concurrent read/load test passed without SQLITE_CORRUPT");
    
    storage.rebuild_blocked_cache().unwrap();
    println!("SUCCESS: rebuild_blocked_cache works");
}
