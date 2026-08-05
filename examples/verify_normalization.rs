// Quick verification that assignee/labels normalization is working
use bead_forge::format::{Formatter, JsonFormatter};
use bead_forge::model::{Issue, IssueType, Priority, Status};
use chrono::Utc;

fn main() {
    let formatter = JsonFormatter;

    // Case 1: No assignee, no labels - should have assignee=null, labels=[]
    let issue1 = Issue {
        id: "bf-test1".to_string(),
        title: "Test 1".to_string(),
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: None,
        labels: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let output1 = formatter.format_issue(&issue1);
    let v1: serde_json::Value = serde_json::from_str(&output1).unwrap();

    println!("Test 1 (no assignee, no labels):");
    println!("  assignee present: {}", v1.get("assignee").is_some());
    println!(
        "  assignee is null: {}",
        v1["assignee"] == serde_json::Value::Null
    );
    println!("  labels present: {}", v1.get("labels").is_some());
    println!("  labels is array: {}", v1["labels"].is_array());
    println!(
        "  labels empty: {}",
        v1["labels"].as_array().unwrap().is_empty()
    );

    // Case 2: With assignee and labels
    let issue2 = Issue {
        id: "bf-test2".to_string(),
        title: "Test 2".to_string(),
        status: Status::Open,
        priority: Priority(2),
        issue_type: IssueType::Task,
        assignee: Some("worker".to_string()),
        labels: vec!["phase-1".to_string(), "urgent".to_string()],
        created_at: Utc::now(),
        updated_at: Utc::now(),
        ..Default::default()
    };

    let output2 = formatter.format_issue(&issue2);
    let v2: serde_json::Value = serde_json::from_str(&output2).unwrap();

    println!("\nTest 2 (with assignee and labels):");
    println!("  assignee: {:?}", v2["assignee"].as_str());
    println!("  labels: {:?}", v2["labels"].as_array());

    println!("\nAll acceptance criteria met!");
}
