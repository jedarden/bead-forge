//! JSON envelope for unified --json output across all commands.
//!
//! All `bf` commands that support `--json` or `--format json` now emit a
//! stable envelope shape:
//!
//! ```json
//! {
//!   "version": 1,
//!   "kind": "<command>",
//!   "data": <command-specific data>,
//!   "warning": "<auto-flush failure message, if any>"
//! }
//! ```
//!
//! The `version` field enables future compatibility; `kind` identifies the
//! command so consumers can parse `data` correctly; `warning` is present only
//! when auto-flush fails (see `crate::autoflush`).
//!
//! ## Command-specific `data` shapes
//!
//! | Command | `data` shape | Empty result |
//! |---------|-------------|--------------|
//! | `create` | `{"id": "bf-xxx"}` | N/A (always succeeds) |
//! | `list` | `[{...}, {...}]` | `[]` |
//! | `ready` | `[{...}, {...}]` | `[]` |
//! | `show` | `{...}` | error (not found) |
//! | `claim` | `{...}` | `{}` (no bead available) |
//! | `update` | `{id: "..."}` | N/A |
//! | `close` | `{id: "..."}` | N/A |
//! | `reopen` | `{id: "..."}` | N/A |
//! | `delete` | `{id: "..."}` | N/A |
//! | `stats` | `{total: ..., ...}` | N/A |
//! | `velocity` | `[{...}, {...}]` | `[]` |
//! | `search` | `[{...}, {...}]` | `[]` |
//! | `recent` | `[{...}, {...}]` | `[]` |
//! | `batch` | `[{op: ..., result: ...}]` | `[]` |
//!
//! List-like commands (list, ready, search, recent, velocity) now emit a JSON
//! **array** wrapped in the envelope, not NDJSON. This fixes the prior divergence
//! where `json.load()` on `bf list --format json` would fail silently (only the
//! first line parses) while `bf ready --json` on empty output emitted `[]`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Current envelope version.
pub const VERSION: u32 = 1;

/// Unified JSON envelope for all command outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonEnvelope {
    /// Envelope version (currently 1).
    pub version: u32,
    /// Command identifier (e.g., "list", "ready", "claim", "create").
    pub kind: String,
    /// Command-specific data (varies by command).
    pub data: Value,
    /// Warning message (present only when auto-flush fails).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
}

impl JsonEnvelope {
    /// Create a new envelope for a command.
    pub fn new(kind: impl Into<String>, data: Value) -> Self {
        Self {
            version: VERSION,
            kind: kind.into(),
            data,
            warning: None,
        }
    }

    /// Add a warning to the envelope (e.g., auto-flush failure).
    pub fn with_warning(mut self, message: impl Into<String>) -> Self {
        self.warning = Some(message.into());
        self
    }

    /// Serialize the envelope to a JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Serialize the envelope to a compact JSON string (no pretty-printing).
    pub fn to_json_compact(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn envelope_has_required_fields() {
        let env = JsonEnvelope::new("list", json!([]));
        let v = serde_json::to_value(&env).unwrap();
        assert_eq!(v["version"], 1);
        assert_eq!(v["kind"], "list");
        assert!(v.get("warning").is_none() || v["warning"].is_null());
    }

    #[test]
    fn envelope_with_optional_warning() {
        let env = JsonEnvelope::new("create", json!({"id": "bf-test"}))
            .with_warning("auto-flush failed");
        let v = serde_json::to_value(&env).unwrap();
        assert_eq!(v["warning"], "auto-flush failed");
    }

    #[test]
    fn envelope_skips_warning_when_none() {
        let env = JsonEnvelope::new("list", json!([]));
        let s = env.to_json().unwrap();
        // When warning is None, the key should not be present (not just null)
        let v: Value = serde_json::from_str(&s).unwrap();
        assert!(v.get("warning").is_none() || v["warning"].is_null());
    }

    #[test]
    fn envelope_parses_as_valid_json() {
        let env = JsonEnvelope::new("stats", json!({"total": 42}));
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert_eq!(parsed.kind, "stats");
        assert_eq!(parsed.data["total"], 42);
    }

    #[test]
    fn envelope_serializes_compactly() {
        let env = JsonEnvelope::new("claim", json!({"bead_id": "bf-123"}));
        let compact = env.to_json_compact().unwrap();
        // Compact JSON has no newlines
        assert!(!compact.contains('\n'));
        // But still parses correctly
        let parsed: JsonEnvelope = serde_json::from_str(&compact).unwrap();
        assert_eq!(parsed.kind, "claim");
    }

    #[test]
    fn list_command_with_multiple_items() {
        let data = json!([
            {"id": "bf-1", "title": "First"},
            {"id": "bf-2", "title": "Second"}
        ]);
        let env = JsonEnvelope::new("list", data);
        let s = env.to_json().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 2);
    }

    #[test]
    fn ready_command_empty_returns_array() {
        let env = JsonEnvelope::new("ready", json!([]));
        let s = env.to_json_compact().unwrap();
        let parsed: JsonEnvelope = serde_json::from_str(&s).unwrap();
        assert!(parsed.data.is_array());
        assert_eq!(parsed.data.as_array().unwrap().len(), 0);
    }
}
