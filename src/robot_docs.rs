//! `bf robot-docs` command: machine-readable enumeration of command contracts.
//!
//! Outputs a JSON schema describing every command's `--json` output contract,
//! enabling agent consumers (NEEDLE worker prompts) to programmatically parse
//! responses without hardcoding shapes.

use serde::{Deserialize, Serialize};
use serde_json::json;

/// Robot docs: describes every command's JSON contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RobotDocs {
    /// Envelope version (must match `JsonEnvelope::version`).
    pub envelope_version: u32,
    /// Schema for the envelope structure itself.
    pub envelope_schema: EnvelopeSchema,
    /// Per-command contracts.
    pub commands: Vec<CommandDoc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvelopeSchema {
    /// Description of the envelope structure.
    pub description: String,
    /// Fields present in all envelopes.
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandDoc {
    /// Command name (as used in envelope `kind` field).
    pub command: String,
    /// Human-readable description.
    pub description: String,
    /// Expected CLI invocation (with flags).
    pub example: String,
    /// Shape of `data` field in the envelope.
    pub data_shape: DataShape,
    /// Schema for the data field (when applicable).
    pub data_schema: Option<serde_json::Value>,
    /// Notes for consumers (e.g., empty result handling).
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum DataShape {
    /// A single object.
    Object,
    /// An array of objects.
    Array,
    /// A specific structured object (e.g., stats).
    Structured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldSchema {
    /// Field name.
    pub name: String,
    /// Field type (JSON Schema type).
    pub r#type: String,
    /// Whether this field is always present.
    pub required: bool,
    /// Description.
    pub description: String,
}

impl RobotDocs {
    /// Generate the complete robot docs.
    pub fn generate() -> Self {
        Self {
            envelope_version: crate::format::ENVELOPE_VERSION,
            envelope_schema: EnvelopeSchema {
                description: "All --json outputs are wrapped in an envelope with version, kind, data, and optional warning fields.".to_string(),
                fields: vec![
                    FieldSchema {
                        name: "version".to_string(),
                        r#type: "integer".to_string(),
                        required: true,
                        description: "Envelope version (currently 1). Enables future compatibility.".to_string(),
                    },
                    FieldSchema {
                        name: "kind".to_string(),
                        r#type: "string".to_string(),
                        required: true,
                        description: "Command identifier (e.g., 'list', 'ready', 'claim').".to_string(),
                    },
                    FieldSchema {
                        name: "data".to_string(),
                        r#type: "any".to_string(),
                        required: true,
                        description: "Command-specific data. Structure varies by kind; see individual command docs.".to_string(),
                    },
                    FieldSchema {
                        name: "warning".to_string(),
                        r#type: "string".to_string(),
                        required: false,
                        description: "Auto-flush failure message (present only when flush fails).".to_string(),
                    },
                ],
            },
            commands: vec![
                CommandDoc {
                    command: "create".to_string(),
                    description: "Create a new bead and emit its ID.".to_string(),
                    example: "bf create --title 'Fix bug' --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "Generated bead ID"}
                        },
                        "required": ["id"]
                    })),
                    notes: vec![
                        "Always succeeds; never returns an empty data field.".to_string(),
                        "May include 'warning' field if auto-flush fails.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "list".to_string(),
                    description: "List beads in the workspace.".to_string(),
                    example: "bf list --status open --json".to_string(),
                    data_shape: DataShape::Array,
                    data_schema: Some(json!({
                        "type": "array",
                        "items": {"type": "object"},
                        "description": "Array of bead objects (same shape as Issue model, with dependencies/comments stripped)"
                    })),
                    notes: vec![
                        "Empty result: data is an empty array [].".to_string(),
                        "Each bead object has 'assignee' (null when unset) and 'labels' (array, empty when unset) fields present.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "ready".to_string(),
                    description: "Show ready (unblocked) beads ranked by impact.".to_string(),
                    example: "bf ready --limit 10 --json".to_string(),
                    data_shape: DataShape::Array,
                    data_schema: Some(json!({
                        "type": "array",
                        "items": {"type": "object"},
                        "description": "Array of bead objects (same shape as Issue model)"
                    })),
                    notes: vec![
                        "Empty result: data is an empty array [].".to_string(),
                        "Previously emitted '[]' on empty and NDJSON on non-empty; now always a JSON array inside the envelope.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "show".to_string(),
                    description: "Show full details for a single bead.".to_string(),
                    example: "bf show bf-123 --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "description": "Full Issue object (including dependencies and comments)"
                    })),
                    notes: vec![
                        "Errors when bead not found (not a null/empty data field).".to_string(),
                    ],
                },
                CommandDoc {
                    command: "claim".to_string(),
                    description: "Atomically claim a bead for a worker.".to_string(),
                    example: "bf claim --assignee worker-123 --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "bead_id": {"type": "string"},
                            "assignee": {"type": "string"},
                            "reclaimed": {"type": "integer", "description": "Number of stale beads reclaimed before claiming"},
                            "workspace": {"type": "string", "description": "Workspace path (only in multi-workspace mode)"},
                            "title": {"type": "string", "description": "Bead title (present in dry-run mode)"},
                            "priority": {"type": "integer", "description": "Bead priority (present in dry-run mode)"},
                            "downstream_impact": {"type": "integer", "description": "Number of dependent beads (present in dry-run mode)"},
                            "dry_run": {"type": "boolean", "description": "True if this was a dry-run"}
                        },
                        "required": ["bead_id", "assignee"]
                    })),
                    notes: vec![
                        "When no bead is available, data is an empty object {}.".to_string(),
                        "Fields beyond bead_id/assignee are optional and depend on the claim mode.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "update".to_string(),
                    description: "Update one or more fields of a bead.".to_string(),
                    example: "bf update bf-123 --status in_progress --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "The updated bead ID"}
                        },
                        "required": ["id"]
                    })),
                    notes: vec![
                        "Always returns the bead ID; empty data is not a valid response.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "close".to_string(),
                    description: "Mark a bead as closed.".to_string(),
                    example: "bf close bf-123 --reason 'Completed' --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "The closed bead ID"}
                        },
                        "required": ["id"]
                    })),
                    notes: vec![
                        "Always returns the bead ID.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "reopen".to_string(),
                    description: "Reopen a closed bead.".to_string(),
                    example: "bf reopen bf-123 --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "The reopened bead ID"}
                        },
                        "required": ["id"]
                    })),
                    notes: vec![
                        "Always returns the bead ID.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "delete".to_string(),
                    description: "Permanently delete a bead.".to_string(),
                    example: "bf delete bf-123 --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "id": {"type": "string", "description": "The deleted bead ID"}
                        },
                        "required": ["id"]
                    })),
                    notes: vec![
                        "Destructive operation; cannot be undone.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "stats".to_string(),
                    description: "Show workspace statistics.".to_string(),
                    example: "bf stats --by-type --json".to_string(),
                    data_shape: DataShape::Structured,
                    data_schema: Some(json!({
                        "type": "object",
                        "properties": {
                            "total": {"type": "integer"},
                            "open": {"type": "integer"},
                            "in_progress": {"type": "integer"},
                            "closed": {"type": "integer"},
                            "by_type": {"type": "object", "description": "Counts by issue type (present with --by-type)"},
                            "by_priority": {"type": "object", "description": "Counts by priority (present with --by-priority)"},
                            "by_assignee": {"type": "object", "description": "Counts by assignee (present with --by-assignee)"},
                            "by_label": {"type": "object", "description": "Counts by label (present with --by-label)"}
                        },
                        "required": ["total", "open", "in_progress", "closed"]
                    })),
                    notes: vec![
                        "Breakdown fields are optional and omitted when not requested.".to_string(),
                    ],
                },
                CommandDoc {
                    command: "velocity".to_string(),
                    description: "Show velocity statistics.".to_string(),
                    example: "bf velocity --json".to_string(),
                    data_shape: DataShape::Array,
                    data_schema: Some(json!({
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "assignee": {"type": "string"},
                                "completed_24h": {"type": "integer"},
                                "completed_7d": {"type": "integer"},
                                "completed_30d": {"type": "integer"}
                            }
                        }
                    })),
                    notes: vec![
                        "Empty result: data is an empty array [] (no beads with velocity data).".to_string(),
                    ],
                },
                CommandDoc {
                    command: "search".to_string(),
                    description: "Search beads by query text and filters.".to_string(),
                    example: "bf search --query 'bug' --status open --json".to_string(),
                    data_shape: DataShape::Array,
                    data_schema: Some(json!({
                        "type": "array",
                        "items": {"type": "object"},
                        "description": "Array of matching bead objects"
                    })),
                    notes: vec![
                        "Empty result: data is an empty array [] (no matches).".to_string(),
                    ],
                },
                CommandDoc {
                    command: "recent".to_string(),
                    description: "Show recently modified beads.".to_string(),
                    example: "bf recent --limit 20 --json".to_string(),
                    data_shape: DataShape::Array,
                    data_schema: Some(json!({
                        "type": "array",
                        "items": {"type": "object"},
                        "description": "Array of recently modified bead objects"
                    })),
                    notes: vec![
                        "Empty result: data is an empty array [] (no activity).".to_string(),
                    ],
                },
                CommandDoc {
                    command: "batch".to_string(),
                    description: "Execute multiple operations in one transaction.".to_string(),
                    example: "bf batch --file ops.json --json".to_string(),
                    data_shape: DataShape::Array,
                    data_schema: Some(json!({
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "op": {"type": "string", "description": "Operation type (create, close, etc.)"},
                                "result": {"description": "Operation result (varies by op)"}
                            }
                        }
                    })),
                    notes: vec![
                        "Each array element corresponds to one operation in the batch file.".to_string(),
                        "Operations are executed atomically; any failure rolls back the entire batch.".to_string(),
                    ],
                },
            ],
        }
    }

    /// Render as formatted JSON (for `bf robot-docs --json`).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
