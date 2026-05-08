//! Event log query functionality for bf log command.
//!
//! Provides filtering and formatting of audit events from the events table.

use crate::model::{Event, EventType};
use crate::storage::Storage;
use anyhow::Result;
use chrono::{DateTime, Utc};

/// Filter options for querying events.
#[derive(Debug, Clone, Default)]
pub struct EventFilter {
    /// Filter by issue ID (None = all issues)
    pub issue_id: Option<String>,
    /// Filter by actor (None = all actors)
    pub actor: Option<String>,
    /// Filter by events since this timestamp (None = no time filter)
    pub since: Option<DateTime<Utc>>,
    /// Filter by event type (None = all types)
    pub event_type: Option<EventType>,
    /// Only show status change events
    pub status_changes_only: bool,
    /// Show field-level diff between old_value and new_value
    pub show_diff: bool,
    /// Limit number of results (None = unlimited)
    pub limit: Option<usize>,
}

impl EventFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_issue_id(mut self, id: String) -> Self {
        self.issue_id = Some(id);
        self
    }

    pub fn with_actor(mut self, actor: String) -> Self {
        self.actor = Some(actor);
        self
    }

    pub fn with_since(mut self, since: DateTime<Utc>) -> Self {
        self.since = Some(since);
        self
    }

    pub fn with_event_type(mut self, event_type: EventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    pub fn status_changes_only(mut self) -> Self {
        self.status_changes_only = true;
        self
    }

    pub fn with_diff(mut self) -> Self {
        self.show_diff = true;
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Query events from storage with optional filtering.
pub fn query_events(storage: &Storage, filter: &EventFilter) -> Result<Vec<Event>> {
    let mut events = if filter.issue_id.is_some()
        || filter.since.is_some()
        || filter.actor.is_some()
        || filter.event_type.is_some()
    {
        storage.list_events_filtered(
            filter.issue_id.as_deref(),
            filter.since.as_ref(),
            filter.actor.as_deref(),
            filter.event_type.as_ref(),
            filter.limit,
        )?
    } else {
        // No filters, get all events with limit
        storage.list_events_filtered(None, None, None, None, filter.limit)?
    };

    // Apply status_changes_only filter in-memory (not a SQL column)
    if filter.status_changes_only {
        events.retain(|e| {
            matches!(
                e.event_type,
                EventType::StatusChanged
                    | EventType::Closed
                    | EventType::Reopened
                    | EventType::PriorityChanged
                    | EventType::AssigneeChanged
            )
        });
    }

    Ok(events)
}

/// Format a single event for text output.
pub fn format_event_text(event: &Event, show_diff: bool) -> String {
    let mut s = String::new();

    // Timestamp
    let ts = event.created_at.format("%Y-%m-%d %H:%M:%S UTC");
    s.push_str(&format!("[{}] ", ts));

    // Event type
    s.push_str(&format!("{} ", event.event_type.as_str()));

    // Actor
    s.push_str(&format!("by {}", event.actor));

    // Old/New values
    if let (Some(old), Some(new)) = (&event.old_value, &event.new_value) {
        if show_diff && old != new {
            s.push_str(&format!(": {} → {}", old, new));
        } else if old != new {
            s.push_str(&format!(": {} → {}", old, new));
        }
    } else if let Some(new) = &event.new_value {
        s.push_str(&format!(": {}", new));
    } else if let Some(old) = &event.old_value {
        s.push_str(&format!(": {} (removed)", old));
    }

    // Comment if present
    if let Some(comment) = &event.comment {
        s.push_str(&format!(" // {}", comment));
    }

    s
}

/// Format events as JSON.
pub fn format_events_json(events: &[Event]) -> Result<String> {
    Ok(serde_json::to_string_pretty(events)?)
}

/// Format events for toon output (compact).
pub fn format_event_toon(event: &Event) -> String {
    format!(
        "{}|{}|{}",
        event.created_at.format("%Y-%m-%dT%H:%M:%SZ"),
        event.event_type.as_str(),
        event.actor
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_builder() {
        let filter = EventFilter::new()
            .with_issue_id("bf-123".to_string())
            .with_actor("claude".to_string())
            .with_limit(10);

        assert_eq!(filter.issue_id, Some("bf-123".to_string()));
        assert_eq!(filter.actor, Some("claude".to_string()));
        assert_eq!(filter.limit, Some(10));
    }

    #[test]
    fn test_format_event_text() {
        let event = Event {
            id: 1,
            issue_id: "bf-123".to_string(),
            event_type: EventType::StatusChanged,
            actor: "claude".to_string(),
            old_value: Some("open".to_string()),
            new_value: Some("in_progress".to_string()),
            comment: Some("Started work".to_string()),
            created_at: DateTime::parse_from_rfc3339("2026-05-08T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
        };

        let formatted = format_event_text(&event, false);
        assert!(formatted.contains("status_changed"));
        assert!(formatted.contains("by claude"));
        assert!(formatted.contains("open → in_progress"));
        assert!(formatted.contains("Started work"));
    }
}
