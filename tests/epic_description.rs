//! Epic description tests (bf-1uu38).
//!
//! Each test builds an `IssueType::Epic` issue carrying a particular flavor of
//! description and checks that the value survives three trips:
//!   1. in-memory construction,
//!   2. a JSON serialize/deserialize roundtrip (the JSONL sync format),
//!   3. a SQLite store/retrieve roundtrip.
//!
//! Note on the storage layer: `create_issue` writes
//! `issue.description.as_deref().unwrap_or("")`, so a `None` description is
//! persisted as the empty string and reads back as `Some("")`. The `None` and
//! empty-string tests below pin that normalization explicitly.

use bead_forge::model::{Issue, IssueType, Priority, Status};
use bead_forge::storage::Storage;

/// Build an epic with the given id/title/description; everything else default.
fn epic(id: &str, title: &str, description: Option<&str>) -> Issue {
    Issue {
        id: id.to_string(),
        title: title.to_string(),
        issue_type: IssueType::Epic,
        status: Status::Open,
        priority: Priority::MEDIUM,
        description: description.map(str::to_string),
        ..Default::default()
    }
}

/// Serialize to JSON and back, returning the deserialized issue.
fn json_roundtrip(issue: &Issue) -> Issue {
    let json = serde_json::to_string(issue).unwrap();
    serde_json::from_str(&json).unwrap()
}

/// Store the issue in a fresh temp-dir database and read it back.
fn storage_roundtrip(issue: &Issue) -> Issue {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(&dir.path().join("test.db")).unwrap();
    storage.create_issue(issue).unwrap();
    storage.get_issue(&issue.id).unwrap().unwrap()
}

/// Assert the description survives construction, JSON, and storage unchanged.
fn assert_description_roundtrips(issue: &Issue, expected: &str) {
    assert_eq!(issue.description.as_deref(), Some(expected));

    let from_json = json_roundtrip(issue);
    assert_eq!(from_json.issue_type, IssueType::Epic);
    assert_eq!(from_json.description.as_deref(), Some(expected));

    let from_storage = storage_roundtrip(issue);
    assert_eq!(from_storage.issue_type, IssueType::Epic);
    assert_eq!(from_storage.description.as_deref(), Some(expected));
}

#[test]
fn test_epic_basic_description() {
    let description = "This epic tracks the rollout of the new claiming subsystem.";
    let e = epic("epic-desc-basic", "Basic Description Epic", Some(description));
    assert_description_roundtrips(&e, description);
}

#[test]
fn test_epic_markdown_description() {
    let description = "\
# Claiming Subsystem

Rework `bf claim` so that it uses **BEGIN IMMEDIATE**.

## Tasks

- [x] Add `with_immediate_transaction()`
- [ ] Wire it into `claim.rs`
- [ ] Backfill tests

> See [the plan](docs/plan/plan.md) for details.

```rust
storage.with_immediate_transaction(|tx| Ok(tx))?;
```

| Phase | Owner |
|-------|-------|
| 1     | core  |
";
    let e = epic(
        "epic-desc-markdown",
        "Markdown Description Epic",
        Some(description),
    );
    assert_description_roundtrips(&e, description);

    // Markdown structure specifically must not be mangled or escaped away.
    let from_storage = storage_roundtrip(&e);
    let stored = from_storage.description.unwrap();
    assert!(stored.contains("# Claiming Subsystem"));
    assert!(stored.contains("- [x] Add `with_immediate_transaction()`"));
    assert!(stored.contains("| Phase | Owner |"));
    assert!(stored.contains("```rust"));
}

#[test]
fn test_epic_long_description() {
    // ~5000 characters: a 100-char unit repeated 50 times.
    let unit = "The epic description must survive storage without truncation at any layer. Padding padding. ";
    let description: String = unit.repeat(5000 / unit.len() + 1);
    assert!(
        description.len() >= 5000,
        "fixture should be at least 5000 chars, got {}",
        description.len()
    );

    let e = epic("epic-desc-long", "Long Description Epic", Some(&description));
    assert_description_roundtrips(&e, &description);

    // Length is the point of this test: assert it explicitly after storage.
    let from_storage = storage_roundtrip(&e);
    assert_eq!(from_storage.description.unwrap().len(), description.len());
}

#[test]
fn test_epic_special_chars_description() {
    let description = r#"Special: <>&"'\/@#$%^*()_+-=[]{}|;:,.<>?/~`"#;
    let e = epic(
        "epic-desc-special",
        "Special Chars Description Epic",
        Some(description),
    );
    assert_description_roundtrips(&e, description);

    // JSON must escape the quote/backslash rather than drop them.
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains(r#"\""#));
    assert!(json.contains(r"\\"));
    assert_eq!(
        json_roundtrip(&e).description.as_deref(),
        Some(description),
        "escaping must be reversible"
    );
}

#[test]
fn test_epic_unicode_description() {
    let description = "你好 🚀 café 日精 👍 高优先级";
    let e = epic(
        "epic-desc-unicode",
        "Unicode Description Epic",
        Some(description),
    );
    assert_description_roundtrips(&e, description);

    let from_storage = storage_roundtrip(&e);
    let stored = from_storage.description.unwrap();
    // Multi-byte code points survive intact -- no replacement chars, no
    // byte-boundary truncation of the 4-byte emoji.
    assert!(stored.contains("你好"));
    assert!(stored.contains("🚀"));
    assert!(stored.contains("café"));
    assert!(stored.contains("日精"));
    assert!(stored.contains("👍"));
    assert!(stored.contains("高优先级"));
    assert!(!stored.contains('\u{FFFD}'));
    assert_eq!(stored.chars().count(), description.chars().count());
}

#[test]
fn test_epic_multiline_description() {
    let description = "First line\nSecond line\n\nFourth line after a blank\nTrailing line\n";
    let e = epic(
        "epic-desc-multiline",
        "Multiline Description Epic",
        Some(description),
    );
    assert_description_roundtrips(&e, description);

    let from_storage = storage_roundtrip(&e);
    let stored = from_storage.description.unwrap();
    assert_eq!(stored.matches('\n').count(), 5);
    assert_eq!(stored.lines().next(), Some("First line"));

    // In JSON the newline is escaped, not literal -- otherwise a description
    // would break the one-issue-per-line JSONL format.
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains("First line\\nSecond line"));
    assert!(!json.contains('\n'));
}

#[test]
fn test_epic_empty_description() {
    let e = epic("epic-desc-empty", "Empty Description Epic", Some(""));

    assert_eq!(e.description.as_deref(), Some(""));

    // `skip_serializing_if = "Option::is_none"` only skips None, so an empty
    // string is still emitted and comes back as Some("").
    let json = serde_json::to_string(&e).unwrap();
    assert!(json.contains(r#""description":"""#));
    assert_eq!(json_roundtrip(&e).description.as_deref(), Some(""));

    assert_eq!(storage_roundtrip(&e).description.as_deref(), Some(""));
}

#[test]
fn test_epic_none_description() {
    let e = epic("epic-desc-none", "None Description Epic", None);

    assert_eq!(e.description, None);

    // None is skipped entirely in JSON and deserializes back to None.
    let json = serde_json::to_string(&e).unwrap();
    assert!(
        !json.contains("\"description\""),
        "None description must be omitted from JSON, got: {json}"
    );
    assert_eq!(json_roundtrip(&e).description, None);

    // Storage normalizes None to the empty string (create_issue writes
    // `unwrap_or("")`), so it reads back as Some("") rather than None.
    assert_eq!(storage_roundtrip(&e).description.as_deref(), Some(""));
}
