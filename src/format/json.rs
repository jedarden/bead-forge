use crate::format::{ClaimResultOutput, Formatter, JsonEnvelope, StatsOutput};
use crate::model::Issue;
use crate::velocity::VelocityStats;
use serde_json::{self, Value};

#[derive(Debug, Clone, Copy)]
pub struct JsonFormatter;

impl JsonFormatter {
    /// Create a JsonFormatter with envelope mode enabled.
    pub fn with_envelope_enabled() -> Self {
        JsonFormatter
    }
}

/// Serialize a single issue to a JSON object, stripping the bulky
/// dependencies/comments relations for `br` compatibility.
///
/// ## Why Manual Stripping is Necessary (Cannot Use #[serde(skip)])
///
/// The manual stripping of `dependencies` and `comments` in this function
/// is intentional and CANNOT be replaced with serde `#[serde(skip)]` attributes
/// on the Issue struct fields. Here's why:
///
/// 1. **Selective Exclusion**: We want to exclude relations ONLY in JSON formatter
///    output (list/ready/search commands), but preserve them for:
///    - JSONL export/import roundtrips (src/jsonl.rs)
///    - API responses that include full issue data
///    - Debug/inspection commands that need complete issue state
///
/// 2. **br Compatibility**: The original `br` tool strips these relations in its
///    JSON output to keep lines short and readable. Breaking this would be a
///    breaking format change.
///
/// 3. **JSONL Line Length**: dependencies/comments can be deeply nested and very
///    large. Including them would make JSONL lines extremely long and harder to
///    work with (grep, jq, etc.).
///
/// 4. **Skip Serializing If is Not Enough**: The Issue struct already has
///    `#[serde(skip_serializing_if = "Vec::is_empty")]` on these fields, which
///    skips them when empty. But we need to ALWAYS skip them for JSON formatter
///    output, even when they're populated.
///
/// ## Alternative Approaches Considered
///
/// - `#[serde(skip)]` on dependencies/comments fields: Would prevent serialization
///   in ALL contexts, breaking JSONL export/import and API responses.
///
/// - Custom serde serializer for Issue: Would be more complex than this simple
///   manual stripping and harder to maintain.
///
/// - Separate struct for JSON output: Would require duplicating the entire Issue
///   struct or complex conversion logic.
///
/// Manual stripping is the simplest and most maintainable solution for this
/// selective exclusion requirement.
///
/// ## Uses the Standard Issue Serde Attributes
///
/// After stripping relations, serialization uses the standard Issue serde
/// attributes, which skip empty collections and None values for compact output.
/// This ensures consistency with storage and other export paths.
fn issue_to_value(issue: &Issue) -> Value {
    let mut stripped = issue.clone();
    stripped.dependencies = vec![];
    stripped.comments = vec![];

    serde_json::to_value(&stripped).unwrap_or(Value::Null)
}

impl Formatter for JsonFormatter {
    fn format_issue(&self, issue: &Issue) -> String {
        serde_json::to_string(&issue_to_value(issue)).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_issues(&self, issues: &[Issue]) -> String {
        issues
            .iter()
            .map(|issue| serde_json::to_string(&issue_to_value(issue)))
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default()
            .join("\n")
    }

    fn format_error(&self, message: &str) -> String {
        serde_json::json!({"error": message}).to_string()
    }

    fn format_claim_result(&self, result: &ClaimResultOutput) -> String {
        serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_no_claim(&self) -> String {
        "{}".to_string()
    }

    fn format_stats(&self, stats: &StatsOutput) -> String {
        serde_json::to_string(stats).unwrap_or_else(|_| "{}".to_string())
    }

    fn format_velocity(&self, stats: &[VelocityStats]) -> String {
        serde_json::to_string(stats).unwrap_or_else(|_| "[]".to_string())
    }

    fn format_with_envelope(&self, kind: &str, data: &str) -> String {
        // Parse the data string as JSON
        let json_value: Value =
            serde_json::from_str(data).unwrap_or_else(|_| Value::String(data.to_string()));

        // Wrap in envelope and serialize
        JsonEnvelope::new(kind, json_value)
            .to_json_compact()
            .unwrap_or_else(|_| "{}".to_string())
    }

    fn format_with_envelope_and_warning(
        &self,
        kind: &str,
        data: &str,
        warning: Option<&str>,
    ) -> String {
        // Parse the data string as JSON
        let json_value: Value =
            serde_json::from_str(data).unwrap_or_else(|_| Value::String(data.to_string()));

        // Wrap in envelope with optional warning and serialize
        let envelope = JsonEnvelope::new(kind, json_value);
        let envelope_with_warning = match warning {
            Some(w) => envelope.with_warning(w),
            None => envelope,
        };
        envelope_with_warning
            .to_json_compact()
            .unwrap_or_else(|_| "{}".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Issue;
    use serde_json::Value;

    fn parse(line: &str) -> Value {
        serde_json::from_str(line).expect("formatter must emit valid JSON")
    }

    #[test]
    fn assignee_skipped_when_unset() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        // With standard Issue serde, assignee is skipped when None (skip_serializing_if)
        assert_eq!(v.get("assignee"), None);
    }

    #[test]
    fn assignee_and_labels_populated_when_present() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.assignee = Some("claude-code-glm-4.7-alpha".to_string());
        issue.labels = vec!["split-child".to_string()];
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(
            v.get("assignee").and_then(|a| a.as_str()),
            Some("claude-code-glm-4.7-alpha")
        );
        assert_eq!(
            v.get("labels"),
            Some(&Value::Array(vec![Value::String(
                "split-child".to_string()
            )]))
        );
    }

    #[test]
    fn format_issues_guarantees_fields_per_line() {
        let a = Issue::new("bf-a".to_string(), "A".to_string(), ".".to_string());
        let mut b = Issue::new("bf-b".to_string(), "B".to_string(), ".".to_string());
        b.assignee = Some("worker".to_string());
        b.labels = vec!["x".to_string()];
        let out = JsonFormatter.format_issues(&[a, b]);
        for line in out.lines() {
            let v = parse(line);
            assert!(v.get("assignee").is_some(), "assignee key must be present");
            assert!(v.get("labels").is_some(), "labels key must be present");
            assert!(
                v.get("labels").unwrap().is_array(),
                "labels must be an array"
            );
        }
    }

    /// Empty input emits no lines at all — `.join("\n")` over an empty slice is
    /// the empty string, so `bf list --format json` on an empty workspace prints
    /// nothing (as opposed to `bf ready`, which special-cases `[]`).
    #[test]
    fn format_issues_empty_yields_empty_string() {
        let out = JsonFormatter.format_issues(&[]);
        assert!(
            out.is_empty(),
            "empty input must produce empty output, got {out:?}"
        );
        assert_eq!(out.lines().count(), 0);
    }

    /// A single issue emits exactly one JSON object on one line — neither an
    /// array-wrapped value nor a trailing newline.
    #[test]
    fn format_issues_single_yields_one_valid_json_line() {
        let issue = Issue::new("bf-solo".to_string(), "Solo".to_string(), ".".to_string());
        let out = JsonFormatter.format_issues(&[issue]);
        assert_eq!(
            out.lines().count(),
            1,
            "single issue must be exactly one line"
        );

        let v = parse(&out);
        assert_eq!(v.get("id").and_then(|i| i.as_str()), Some("bf-solo"));
        assert_eq!(v.get("title").and_then(|t| t.as_str()), Some("Solo"));
        // Display normalization applies per-line, even with one entry.
        assert!(v.get("assignee").is_some());
        assert!(v.get("labels").is_some());
    }

    /// Multiple issues emit JSONL — one self-contained JSON object per line, in
    /// input order, with no array wrapper or comma separators between them.
    #[test]
    fn format_issues_multiple_yields_jsonl_one_object_per_line() {
        let a = Issue::new("bf-a".to_string(), "A".to_string(), ".".to_string());
        let b = Issue::new("bf-b".to_string(), "B".to_string(), ".".to_string());
        let c = Issue::new("bf-c".to_string(), "C".to_string(), ".".to_string());
        let out = JsonFormatter.format_issues(&[a, b, c]);

        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(
            lines.len(),
            3,
            "three issues must produce three JSONL lines"
        );

        // Each line is independently valid JSON; ids preserve input order.
        let ids: Vec<String> = lines
            .iter()
            .map(|line| {
                let v = parse(line);
                v.get("id")
                    .and_then(|i| i.as_str())
                    .unwrap_or_else(|| panic!("line {line:?} must have a string id, got {v}"))
                    .to_string()
            })
            .collect();
        assert_eq!(
            ids,
            vec!["bf-a".to_string(), "bf-b".to_string(), "bf-c".to_string()]
        );

        // No array wrapper or comma separators: the whole output is not valid
        // JSON, but each line is.
        assert!(
            serde_json::from_str::<Value>(&out).is_err(),
            "concatenated JSONL must not parse as a single JSON value"
        );
    }

    #[test]
    fn claim_dry_run_emits_only_preview_keys() {
        // dry-run: bead_id/assignee always present, plus title/priority/impact/
        // workspace/dry_run; `reclaimed` is never set so it must be omitted.
        let mut out = ClaimResultOutput::new("bf-9", "claude-x");
        out.title = Some("T".to_string());
        out.priority = Some(2);
        out.downstream_impact = Some(7);
        out.workspace = Some("/repo".to_string());
        out.dry_run = Some(true);

        let v = parse(&JsonFormatter.format_claim_result(&out));
        assert_eq!(v.get("bead_id").and_then(|x| x.as_str()), Some("bf-9"));
        assert_eq!(v.get("assignee").and_then(|x| x.as_str()), Some("claude-x"));
        assert_eq!(v.get("dry_run").and_then(|x| x.as_bool()), Some(true));
        assert_eq!(v.get("priority").and_then(|x| x.as_i64()), Some(2));
        assert_eq!(v.get("downstream_impact").and_then(|x| x.as_i64()), Some(7));
        assert_eq!(v.get("workspace").and_then(|x| x.as_str()), Some("/repo"));
        assert!(
            v.get("reclaimed").is_none(),
            "reclaimed key must be omitted when unset"
        );
    }

    #[test]
    fn claim_single_workspace_omits_workspace_key() {
        // normal single-workspace claim: bead_id + reclaimed + assignee only.
        let mut out = ClaimResultOutput::new("bf-1", "claude-y");
        out.reclaimed = Some(0);

        let v = parse(&JsonFormatter.format_claim_result(&out));
        assert_eq!(v.get("bead_id").and_then(|x| x.as_str()), Some("bf-1"));
        assert_eq!(v.get("assignee").and_then(|x| x.as_str()), Some("claude-y"));
        assert_eq!(v.get("reclaimed").and_then(|x| x.as_i64()), Some(0));
        assert!(
            v.get("workspace").is_none(),
            "workspace key must be omitted on a single-workspace claim"
        );
        assert!(
            v.get("dry_run").is_none(),
            "dry_run key must be omitted on a real claim"
        );
    }

    #[test]
    fn no_claim_is_empty_object() {
        assert_eq!(JsonFormatter.format_no_claim(), "{}");
    }

    // Skip serializing if tests for Vec fields

    #[test]
    fn dependencies_skipped_when_empty() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        // dependencies is manually stripped to empty by issue_to_value(), then skipped by serde
        assert_eq!(v.get("dependencies"), None, "dependencies should be omitted when empty");
    }

    #[test]
    fn comments_skipped_when_empty() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        // comments is manually stripped to empty by issue_to_value(), then skipped by serde
        assert_eq!(v.get("comments"), None, "comments should be omitted when empty");
    }

    #[test]
    fn events_skipped_when_empty() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        // events uses standard serde skip_serializing_if = "Vec::is_empty"
        assert_eq!(v.get("events"), None, "events should be omitted when empty");
    }

    #[test]
    fn labels_skipped_when_empty() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        // labels uses standard serde skip_serializing_if = "Vec::is_empty"
        assert_eq!(v.get("labels"), None, "labels should be omitted when empty");
    }

    // Skip serializing if tests for Option fields

    #[test]
    fn description_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("description"), None, "description should be omitted when None");
    }

    #[test]
    fn design_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("design"), None, "design should be omitted when None");
    }

    #[test]
    fn acceptance_criteria_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("acceptance_criteria"), None, "acceptance_criteria should be omitted when None");
    }

    #[test]
    fn notes_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("notes"), None, "notes should be omitted when None");
    }

    #[test]
    fn owner_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("owner"), None, "owner should be omitted when None");
    }

    #[test]
    fn estimated_minutes_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("estimated_minutes"), None, "estimated_minutes should be omitted when None");
    }

    #[test]
    fn created_by_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("created_by"), None, "created_by should be omitted when None");
    }

    #[test]
    fn closed_at_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("closed_at"), None, "closed_at should be omitted when None");
    }

    #[test]
    fn close_reason_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("close_reason"), None, "close_reason should be omitted when None");
    }

    #[test]
    fn closed_by_session_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("closed_by_session"), None, "closed_by_session should be omitted when None");
    }

    #[test]
    fn due_at_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("due_at"), None, "due_at should be omitted when None");
    }

    #[test]
    fn defer_until_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("defer_until"), None, "defer_until should be omitted when None");
    }

    #[test]
    fn external_ref_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("external_ref"), None, "external_ref should be omitted when None");
    }

    #[test]
    fn source_system_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("source_system"), None, "source_system should be omitted when None");
    }

    #[test]
    fn deleted_at_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("deleted_at"), None, "deleted_at should be omitted when None");
    }

    #[test]
    fn deleted_by_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("deleted_by"), None, "deleted_by should be omitted when None");
    }

    #[test]
    fn delete_reason_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("delete_reason"), None, "delete_reason should be omitted when None");
    }

    #[test]
    fn original_type_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("original_type"), None, "original_type should be omitted when None");
    }

    #[test]
    fn compacted_at_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("compacted_at"), None, "compacted_at should be omitted when None");
    }

    #[test]
    fn compacted_at_commit_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("compacted_at_commit"), None, "compacted_at_commit should be omitted when None");
    }

    #[test]
    fn original_size_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("original_size"), None, "original_size should be omitted when None");
    }

    #[test]
    fn sender_skipped_when_none() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("sender"), None, "sender should be omitted when None");
    }

    #[test]
    fn annotations_skipped_when_empty() {
        let issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        // annotations uses BTreeMap with skip_serializing_if = "BTreeMap::is_empty"
        assert_eq!(v.get("annotations"), None, "annotations should be omitted when empty");
    }

    // Tests to verify fields ARE present when populated

    #[test]
    fn description_present_when_some() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.description = Some("A description".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("description").and_then(|d| d.as_str()), Some("A description"));
    }

    #[test]
    fn design_present_when_some() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.design = Some("Design notes".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("design").and_then(|d| d.as_str()), Some("Design notes"));
    }

    #[test]
    fn acceptance_criteria_present_when_some() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.acceptance_criteria = Some("Criteria".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("acceptance_criteria").and_then(|d| d.as_str()), Some("Criteria"));
    }

    #[test]
    fn notes_present_when_some() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.notes = Some("Some notes".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("notes").and_then(|d| d.as_str()), Some("Some notes"));
    }

    #[test]
    fn owner_present_when_some() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.owner = Some("owner-name".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("owner").and_then(|d| d.as_str()), Some("owner-name"));
    }

    #[test]
    fn estimated_minutes_present_when_some() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.estimated_minutes = Some(120);
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert_eq!(v.get("estimated_minutes").and_then(|d| d.as_i64()), Some(120));
    }

    #[test]
    fn annotations_present_when_populated() {
        let mut issue = Issue::new("bf-test".to_string(), "Test".to_string(), ".".to_string());
        issue.annotations.insert("key".to_string(), "value".to_string());
        let v = parse(&JsonFormatter.format_issue(&issue));
        assert!(v.get("annotations").is_some(), "annotations should be present when populated");
        assert_eq!(v.get("annotations").and_then(|a| a.get("key")).and_then(|v| v.as_str()), Some("value"));
    }
}
