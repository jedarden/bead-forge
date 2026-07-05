// Test file to verify Epic implementation meets acceptance criteria
// bf-4w6yj: Add Epic type to model and serialization

#[cfg(test)]
mod epic_verification_tests {
    use bead_forge::model::IssueType;
    use serde_json;

    #[test]
    fn verify_epic_variant_exists() {
        // AC: Add Epic variant to IssueType enum in src/model.rs
        let epic = IssueType::Epic;
        assert_eq!(epic.as_str(), "epic");
    }

    #[test]
    fn verify_epic_serializes_to_epic() {
        // AC: Epic serializes to JSON as "issue_type":"epic"
        let epic = IssueType::Epic;
        let serialized = serde_json::to_string(&epic).unwrap();
        assert_eq!(serialized, "\"epic\"");
    }

    #[test]
    fn verify_epic_deserializes_from_epic() {
        // AC: Ensure serde deserialization preserves epic type
        let deserialized: IssueType = serde_json::from_str("\"epic\"").unwrap();
        assert_eq!(deserialized, IssueType::Epic);
    }

    #[test]
    fn verify_epic_as_str_returns_epic() {
        // AC: epic_type.as_str() returns "epic"
        assert_eq!(IssueType::Epic.as_str(), "epic");
    }

    #[test]
    fn verify_default_is_task_not_epic() {
        // AC: Default IssueType remains Task (not Epic)
        let default: IssueType = Default::default();
        assert_eq!(default, IssueType::Task);
        assert_ne!(default, IssueType::Epic);
    }

    #[test]
    fn verify_all_standard_types_roundtrip() {
        // AC: All standard issue types roundtrip correctly
        let types = vec![
            ("task", IssueType::Task),
            ("bug", IssueType::Bug),
            ("feature", IssueType::Feature),
            ("epic", IssueType::Epic),
            ("chore", IssueType::Chore),
            ("docs", IssueType::Docs),
            ("question", IssueType::Question),
        ];

        for (json_str, expected) in types {
            // Serialize
            let serialized = serde_json::to_string(&expected).unwrap();
            assert_eq!(serialized, format!("\"{}\"", json_str));

            // Deserialize
            let deserialized: IssueType = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, expected);
        }
    }
}
