use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

// Rusqlite support for Priority
impl rusqlite::types::ToSql for Priority {
    fn to_sql(&self) -> Result<rusqlite::types::ToSqlOutput<'_>, rusqlite::Error> {
        let val = self.0;
        Ok(rusqlite::types::ToSqlOutput::Owned(val.into()))
    }
}

impl rusqlite::types::FromSql for Priority {
    fn column_result(value: rusqlite::types::ValueRef<'_>) -> Result<Self, rusqlite::types::FromSqlError> {
        i32::column_result(value).map(Priority)
    }
}

#[allow(clippy::trivially_copy_pass_by_ref)]
const fn is_false(b: &bool) -> bool {
    !*b
}

/// Serialize Option<i32> as 0 when None (for bd conformance - bd expects integer, not null)
#[allow(clippy::ref_option, clippy::trivially_copy_pass_by_ref)]
fn serialize_compaction_level<S>(value: &Option<i32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_i32(value.unwrap_or(0))
}

/// Issue lifecycle status.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    #[default]
    Open,
    InProgress,
    Blocked,
    Deferred,
    Draft,
    Closed,
    #[serde(rename = "tombstone")]
    Tombstone,
    #[serde(rename = "pinned")]
    Pinned,
    #[serde(untagged)]
    Custom(String),
}

impl Status {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Open => "open",
            Self::InProgress => "in_progress",
            Self::Blocked => "blocked",
            Self::Deferred => "deferred",
            Self::Draft => "draft",
            Self::Closed => "closed",
            Self::Tombstone => "tombstone",
            Self::Pinned => "pinned",
            Self::Custom(value) => value,
        }
    }

    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(self, Self::Closed | Self::Tombstone)
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Open | Self::InProgress)
    }

    /// Returns true if the issue is in draft state (not yet ready for execution).
    #[must_use]
    pub const fn is_draft(&self) -> bool {
        matches!(self, Self::Draft)
    }
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for Status {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "in_progress" | "inprogress" => Ok(Self::InProgress),
            "blocked" => Ok(Self::Blocked),
            "deferred" => Ok(Self::Deferred),
            "draft" => Ok(Self::Draft),
            "closed" => Ok(Self::Closed),
            "tombstone" => Ok(Self::Tombstone),
            "pinned" => Ok(Self::Pinned),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

/// Issue priority (0=Critical, 4=Backlog).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub struct Priority(pub i32);

impl Default for Priority {
    fn default() -> Self {
        Self::MEDIUM
    }
}

impl Priority {
    pub const CRITICAL: Self = Self(0);
    pub const HIGH: Self = Self(1);
    pub const MEDIUM: Self = Self(2);
    pub const LOW: Self = Self(3);
    pub const BACKLOG: Self = Self(4);
}

impl fmt::Display for Priority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "P{}", self.0)
    }
}

impl FromStr for Priority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim().to_uppercase();
        let val = s.strip_prefix('P').unwrap_or(&s);

        match val.parse::<i32>() {
            Ok(p) if (0..=4).contains(&p) => Ok(Self(p)),
            Ok(p) => Err(format!("Invalid priority: {}", p)),
            Err(_) => Err(format!("Invalid priority: {}", s)),
        }
    }
}

/// Issue type category.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    #[default]
    Task,
    Bug,
    Feature,
    Epic,
    Chore,
    Docs,
    Question,
    #[serde(untagged)]
    Custom(String),
}

impl IssueType {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Task => "task",
            Self::Bug => "bug",
            Self::Feature => "feature",
            Self::Epic => "epic",
            Self::Chore => "chore",
            Self::Docs => "docs",
            Self::Question => "question",
            Self::Custom(value) => value,
        }
    }

    /// Returns true if this is a standard (non-custom) issue type.
    /// Used for bd conformance validation in CLI commands.
    #[must_use]
    pub const fn is_standard(&self) -> bool {
        !matches!(self, Self::Custom(_))
    }
}

impl fmt::Display for IssueType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for IssueType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "task" => Ok(Self::Task),
            "bug" => Ok(Self::Bug),
            "feature" => Ok(Self::Feature),
            "epic" => Ok(Self::Epic),
            "chore" => Ok(Self::Chore),
            "docs" => Ok(Self::Docs),
            "question" => Ok(Self::Question),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

/// Dependency relationship type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DependencyType {
    Blocks,
    ParentChild,
    ConditionalBlocks,
    WaitsFor,
    Related,
    DiscoveredFrom,
    RepliesTo,
    RelatesTo,
    Duplicates,
    Supersedes,
    CausedBy,
    #[serde(untagged)]
    Custom(String),
}

impl DependencyType {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Blocks => "blocks",
            Self::ParentChild => "parent-child",
            Self::ConditionalBlocks => "conditional-blocks",
            Self::WaitsFor => "waits-for",
            Self::Related => "related",
            Self::DiscoveredFrom => "discovered-from",
            Self::RepliesTo => "replies-to",
            Self::RelatesTo => "relates-to",
            Self::Duplicates => "duplicates",
            Self::Supersedes => "supersedes",
            Self::CausedBy => "caused-by",
            Self::Custom(value) => value,
        }
    }

    #[must_use]
    pub const fn affects_ready_work(&self) -> bool {
        matches!(
            self,
            Self::Blocks | Self::ParentChild | Self::ConditionalBlocks | Self::WaitsFor
        )
    }

    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        matches!(
            self,
            Self::Blocks | Self::ParentChild | Self::ConditionalBlocks | Self::WaitsFor
        )
    }
}

impl fmt::Display for DependencyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for DependencyType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "blocks" => Ok(Self::Blocks),
            "parent-child" => Ok(Self::ParentChild),
            "conditional-blocks" => Ok(Self::ConditionalBlocks),
            "waits-for" => Ok(Self::WaitsFor),
            "related" => Ok(Self::Related),
            "discovered-from" => Ok(Self::DiscoveredFrom),
            "replies-to" => Ok(Self::RepliesTo),
            "relates-to" => Ok(Self::RelatesTo),
            "duplicates" => Ok(Self::Duplicates),
            "supersedes" => Ok(Self::Supersedes),
            "caused-by" => Ok(Self::CausedBy),
            other => Ok(Self::Custom(other.to_string())),
        }
    }
}

/// Audit event type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventType {
    Created,
    Updated,
    StatusChanged,
    PriorityChanged,
    AssigneeChanged,
    Commented,
    Closed,
    Reopened,
    DependencyAdded,
    DependencyRemoved,
    LabelAdded,
    LabelRemoved,
    Compacted,
    Deleted,
    Restored,
    Custom(String),
}

impl EventType {
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Created => "created",
            Self::Updated => "updated",
            Self::StatusChanged => "status_changed",
            Self::PriorityChanged => "priority_changed",
            Self::AssigneeChanged => "assignee_changed",
            Self::Commented => "commented",
            Self::Closed => "closed",
            Self::Reopened => "reopened",
            Self::DependencyAdded => "dependency_added",
            Self::DependencyRemoved => "dependency_removed",
            Self::LabelAdded => "label_added",
            Self::LabelRemoved => "label_removed",
            Self::Compacted => "compacted",
            Self::Deleted => "deleted",
            Self::Restored => "restored",
            Self::Custom(value) => value,
        }
    }
}

impl Serialize for EventType {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for EventType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        let event_type = match value.as_str() {
            "created" => Self::Created,
            "updated" => Self::Updated,
            "status_changed" => Self::StatusChanged,
            "priority_changed" => Self::PriorityChanged,
            "assignee_changed" => Self::AssigneeChanged,
            "commented" => Self::Commented,
            "closed" => Self::Closed,
            "reopened" => Self::Reopened,
            "dependency_added" => Self::DependencyAdded,
            "dependency_removed" => Self::DependencyRemoved,
            "label_added" => Self::LabelAdded,
            "label_removed" => Self::LabelRemoved,
            "compacted" => Self::Compacted,
            "deleted" => Self::Deleted,
            "restored" => Self::Restored,
            _ => Self::Custom(value),
        };
        Ok(event_type)
    }
}

/// The primary issue entity.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Issue {
    /// Unique ID (e.g., "bd-abc123").
    pub id: String,

    /// Content hash for deduplication and sync.
    #[serde(skip)]
    pub content_hash: Option<String>,

    /// Title (1-500 chars).
    pub title: String,

    /// Detailed description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Technical design notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design: Option<String>,

    /// Acceptance criteria.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub acceptance_criteria: Option<String>,

    /// Additional notes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,

    /// Workflow status.
    #[serde(default)]
    pub status: Status,

    /// Priority (0=Critical, 4=Backlog).
    #[serde(default)]
    pub priority: Priority,

    /// Issue type (bug, feature, etc.).
    #[serde(default)]
    pub issue_type: IssueType,

    /// Assigned user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,

    /// Issue owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Estimated effort in minutes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub estimated_minutes: Option<i32>,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Creator username.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,

    /// Closure timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<DateTime<Utc>>,

    /// Reason for closure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_reason: Option<String>,

    /// Session ID that closed this issue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closed_by_session: Option<String>,

    /// Due date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub due_at: Option<DateTime<Utc>>,

    /// Defer until date.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defer_until: Option<DateTime<Utc>>,

    /// External reference (e.g., JIRA-123).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_ref: Option<String>,

    /// Source system for imported issues.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_system: Option<String>,

    /// Source repository for multi-repo support.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_repo: Option<String>,

    // Tombstone fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delete_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_type: Option<String>,

    // Compaction (legacy/compat)
    // Note: Always serialize compaction_level as integer (0 when None) for bd conformance
    // bd's Go sql scanner cannot handle NULL for integer columns
    #[serde(default, serialize_with = "serialize_compaction_level")]
    pub compaction_level: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compacted_at_commit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub original_size: Option<i32>,

    // Messaging
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub ephemeral: bool,

    // Context
    #[serde(default, skip_serializing_if = "is_false")]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_template: bool,

    // Relations (for export/display, not always in DB table directly)
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub labels: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub dependencies: Vec<Dependency>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub comments: Vec<Comment>,
}

impl Default for Issue {
    fn default() -> Self {
        Self {
            id: String::new(),
            content_hash: None,
            title: String::new(),
            description: None,
            design: None,
            acceptance_criteria: None,
            notes: None,
            status: Status::default(),
            priority: Priority::default(),
            issue_type: IssueType::default(),
            assignee: None,
            owner: None,
            estimated_minutes: None,
            created_at: Utc::now(),
            created_by: None,
            updated_at: Utc::now(),
            closed_at: None,
            close_reason: None,
            closed_by_session: None,
            due_at: None,
            defer_until: None,
            external_ref: None,
            source_system: None,
            source_repo: None,
            deleted_at: None,
            deleted_by: None,
            delete_reason: None,
            original_type: None,
            compaction_level: None,
            compacted_at: None,
            compacted_at_commit: None,
            original_size: None,
            sender: None,
            ephemeral: false,
            pinned: false,
            is_template: false,
            labels: Vec::new(),
            dependencies: Vec::new(),
            comments: Vec::new(),
        }
    }
}

impl Issue {
    pub fn new(id: String, title: String, source_repo: String) -> Self {
        let now = Utc::now();
        Issue {
            id,
            title,
            source_repo: Some(source_repo),
            created_at: now,
            updated_at: now,
            status: Status::default(),
            priority: Priority::default(),
            issue_type: IssueType::default(),
            ..Default::default()
        }
    }

    pub fn is_blocked(&self) -> bool {
        self.dependencies
            .iter()
            .filter(|d| matches!(d.dep_type, DependencyType::Blocks))
            .any(|d| {
                self.source_repo
                    .as_ref()
                    .map(|repo| repo == &d.depends_on_id)
                    .unwrap_or(false)
            })
    }

    pub fn content_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let serialized = serde_json::to_string(self).unwrap_or_default();
        let hash = Sha256::digest(serialized.as_bytes());
        format!("{:x}", hash)
    }

    /// Compare two issues using sync semantics instead of raw struct equality.
    ///
    /// This ignores derived or volatile audit fields that would otherwise make
    /// semantically identical issues look different across import/export
    /// boundaries, while still comparing the full synced payload including
    /// labels, dependencies, comments, and user-visible timestamps like `due_at`.
    #[must_use]
    pub fn sync_equals(&self, other: &Self) -> bool {
        if self.id != other.id
            || self.title != other.title
            || self.description != other.description
            || self.design != other.design
            || self.acceptance_criteria != other.acceptance_criteria
            || self.notes != other.notes
            || self.status != other.status
            || self.priority != other.priority
            || self.issue_type != other.issue_type
            || self.assignee != other.assignee
            || self.owner != other.owner
            || self.estimated_minutes != other.estimated_minutes
            || self.created_by != other.created_by
            || self.closed_at != other.closed_at
            || self.close_reason != other.close_reason
            || self.closed_by_session != other.closed_by_session
            || self.due_at != other.due_at
            || self.defer_until != other.defer_until
            || self.external_ref != other.external_ref
            || self.source_system != other.source_system
            || self.source_repo != other.source_repo
            || self.deleted_at != other.deleted_at
            || self.deleted_by != other.deleted_by
            || self.delete_reason != other.delete_reason
            || self.original_type != other.original_type
            || self.compacted_at != other.compacted_at
            || self.compacted_at_commit != other.compacted_at_commit
            || self.original_size != other.original_size
            || self.sender != other.sender
            || self.ephemeral != other.ephemeral
            || self.pinned != other.pinned
            || self.is_template != other.is_template
        {
            return false;
        }

        // Handle compaction_level serialization quirk where None == 0
        if self.compaction_level.unwrap_or(0) != other.compaction_level.unwrap_or(0) {
            return false;
        }

        // Fast path for relations: if lengths differ, they can't be equal
        if self.dependencies.len() != other.dependencies.len()
            || self.comments.len() != other.comments.len()
        {
            return false;
        }

        // Compare labels (order independent)
        let mut self_labels = self.labels.clone();
        self_labels.sort_unstable();
        self_labels.dedup();
        let mut other_labels = other.labels.clone();
        other_labels.sort_unstable();
        other_labels.dedup();
        if self_labels != other_labels {
            return false;
        }

        // Compare dependencies (order independent)
        let mut self_deps = self.dependencies.clone();
        self_deps.sort_by(|left, right| {
            left.issue_id
                .cmp(&right.issue_id)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
                .then_with(|| left.dep_type.as_str().cmp(right.dep_type.as_str()))
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.created_by.cmp(&right.created_by))
                .then_with(|| left.metadata.cmp(&right.metadata))
                .then_with(|| left.thread_id.cmp(&right.thread_id))
        });
        let mut other_deps = other.dependencies.clone();
        other_deps.sort_by(|left, right| {
            left.issue_id
                .cmp(&right.issue_id)
                .then_with(|| left.depends_on_id.cmp(&right.depends_on_id))
                .then_with(|| left.dep_type.as_str().cmp(right.dep_type.as_str()))
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.created_by.cmp(&right.created_by))
                .then_with(|| left.metadata.cmp(&right.metadata))
                .then_with(|| left.thread_id.cmp(&right.thread_id))
        });
        if self_deps != other_deps {
            return false;
        }

        // Compare comments (order independent)
        let mut self_comments = self.comments.clone();
        self_comments.sort_by(|left, right| {
            left.issue_id
                .cmp(&right.issue_id)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.author.cmp(&right.author))
                .then_with(|| left.body.cmp(&right.body))
                .then_with(|| left.id.cmp(&right.id))
        });
        let mut other_comments = other.comments.clone();
        other_comments.sort_by(|left, right| {
            left.issue_id
                .cmp(&right.issue_id)
                .then_with(|| left.created_at.cmp(&right.created_at))
                .then_with(|| left.author.cmp(&right.author))
                .then_with(|| left.body.cmp(&right.body))
                .then_with(|| left.id.cmp(&right.id))
        });
        if self_comments != other_comments {
            return false;
        }

        true
    }

    /// Check if this issue is a tombstone that has exceeded its TTL.
    #[must_use]
    pub fn is_expired_tombstone(&self, retention_days: Option<u64>) -> bool {
        if self.status != Status::Tombstone {
            return false;
        }

        let Some(days) = retention_days else {
            return false;
        };

        if days == 0 {
            return false;
        }

        let Some(deleted_at) = self.deleted_at else {
            return false;
        };

        // Clamp days to a safe maximum to avoid panic in Duration::days().
        let max_safe_days = 365_u64 * 1000;
        let days_i64 = i64::try_from(days.min(max_safe_days)).unwrap_or(365_000);
        let expiration_time = deleted_at + chrono::Duration::days(days_i64);
        Utc::now() > expiration_time
    }
}

/// Epic completion status with child counts.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EpicStatus {
    pub epic: Issue,
    pub total_children: usize,
    pub closed_children: usize,
    pub eligible_for_close: bool,
}

/// Relationship between two issues.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dependency {
    /// The issue that has the dependency (source).
    pub issue_id: String,

    /// The issue being depended on (target).
    pub depends_on_id: String,

    /// Type of dependency.
    #[serde(rename = "type")]
    pub dep_type: DependencyType,

    /// Creation timestamp.
    pub created_at: DateTime<Utc>,

    /// Creator.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub created_by: Option<String>,

    /// Optional metadata (JSON).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>,

    /// Thread ID for conversation linking.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
}

/// A comment on an issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Comment {
    pub id: i64,
    pub issue_id: String,
    pub author: String,
    #[serde(rename = "text")]
    pub body: String,
    pub created_at: DateTime<Utc>,
}

/// An event in the issue's history (audit log).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Event {
    pub id: i64,
    pub issue_id: String,
    #[serde(rename = "type")]
    pub event_type: EventType,
    pub actor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub old_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub new_value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Changes that can be applied to an issue (non-Serde, for internal updates).
#[derive(Debug, Clone, Default)]
pub struct IssueChanges {
    pub title: Option<String>,
    pub description: Option<String>,
    pub design: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub notes: Option<String>,
    pub status: Option<Status>,
    pub priority: Option<i32>,
    pub issue_type: Option<IssueType>,
    pub assignee: Option<String>,
    pub owner: Option<String>,
    pub estimated_minutes: Option<i32>,
    pub due_at: Option<DateTime<Utc>>,
    pub defer_until: Option<DateTime<Utc>>,
    pub external_ref: Option<String>,
    pub labels: Option<Vec<String>>,
}

/// Filter for listing issues (non-Serde, for queries).
#[derive(Debug, Clone, Default)]
pub struct IssueFilter {
    pub status: Option<Status>,
    pub issue_type: Option<IssueType>,
    pub assignee: Option<String>,
    pub priority: Option<i32>,
    pub labels: Option<Vec<String>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_issue_roundtrip_from_br_format() {
        // This is the exact format br (beads_rust) exports to issues.jsonl
        let json = r#"{
  "id": "bf-1bz",
  "title": "Port br-compatible Issue data model with identical Serde attributes (src/model.rs)",
  "description": "Port Issue, Status, Priority, IssueType, Dependency, DependencyType, Comment, Event from beads_rust. Every field, every skip_serializing_if, every rename must match the JSONL wire format exactly. Acceptance: a br-exported issues.jsonl round-trips through serde_json with zero field loss.",
  "status": "in_progress",
  "priority": 0,
  "issue_type": "task",
  "assignee": "claude-code-glm-4.7-november",
  "created_at": "2026-04-29T23:21:25.206825571Z",
  "created_by": "coding",
  "updated_at": "2026-04-29T23:51:34.458178699Z",
  "source_repo": ".",
  "compaction_level": 0,
  "original_size": 0,
  "labels": ["deferred", "model", "phase-1"]
}"#;

        // Deserialize
        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize br format");

        // Verify key fields
        assert_eq!(issue.id, "bf-1bz");
        assert_eq!(issue.status, Status::InProgress);
        assert_eq!(issue.priority, Priority::CRITICAL);
        assert_eq!(issue.issue_type, IssueType::Task);
        assert_eq!(issue.assignee.as_deref(), Some("claude-code-glm-4.7-november"));
        assert_eq!(issue.source_repo.as_deref(), Some("."));
        assert_eq!(issue.compaction_level, Some(0));
        assert_eq!(issue.original_size, Some(0));
        assert_eq!(issue.labels, vec!["deferred", "model", "phase-1"]);

        // Serialize back and verify
        let serialized = serde_json::to_string(&issue).expect("Failed to serialize");
        let roundtrip: Issue = serde_json::from_str(&serialized).expect("Failed to deserialize roundtrip");

        // All fields should match after roundtrip
        assert_eq!(issue, roundtrip);
    }

    #[test]
    fn test_compaction_level_serializes_as_zero_when_none() {
        // bd conformance: compaction_level must serialize as 0, not null
        let issue = Issue {
            id: "test-1".to_string(),
            title: "Test".to_string(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            compaction_level: None,
            ..Default::default()
        };

        let json = serde_json::to_string(&issue).unwrap();
        assert!(json.contains(r#""compaction_level":0"#));
        assert!(!json.contains(r#""compaction_level":null"#));
    }

    #[test]
    fn test_empty_vectors_skipped_in_serialization() {
        let issue = Issue {
            id: "test-2".to_string(),
            title: "Test".to_string(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            labels: vec![],
            dependencies: vec![],
            comments: vec![],
            ..Default::default()
        };

        let json = serde_json::to_string(&issue).unwrap();
        // Empty vectors should be skipped
        assert!(!json.contains(r#""labels":"#));
        assert!(!json.contains(r#""dependencies":"#));
        assert!(!json.contains(r#""comments":"#));
    }

    #[test]
    fn test_custom_status_roundtrip() {
        let status: Status = serde_json::from_str("\"custom_status\"").unwrap();
        assert_eq!(status, Status::Custom("custom_status".to_string()));
        let serialized = serde_json::to_string(&status).unwrap();
        assert_eq!(serialized, "\"custom_status\"");
    }

    #[test]
    fn test_custom_issue_type_roundtrip() {
        let issue_type: IssueType = serde_json::from_str("\"spike\"").unwrap();
        assert_eq!(issue_type, IssueType::Custom("spike".to_string()));
        let serialized = serde_json::to_string(&issue_type).unwrap();
        assert_eq!(serialized, "\"spike\"");
    }

    #[test]
    fn test_custom_dependency_type_roundtrip() {
        let dep_type: DependencyType = serde_json::from_str("\"custom-dep\"").unwrap();
        assert_eq!(dep_type, DependencyType::Custom("custom-dep".to_string()));
        let serialized = serde_json::to_string(&dep_type).unwrap();
        assert_eq!(serialized, "\"custom-dep\"");
    }

    #[test]
    fn test_custom_event_type_roundtrip() {
        let event_type: EventType = serde_json::from_str("\"custom_event\"").unwrap();
        assert_eq!(event_type, EventType::Custom("custom_event".to_string()));
        let serialized = serde_json::to_string(&event_type).unwrap();
        assert_eq!(serialized, "\"custom_event\"");
    }

    #[test]
    fn test_dependency_type_field_renamed() {
        let json = r#"{"issue_id":"bd-1","depends_on_id":"bd-2","type":"blocks","created_at":"2026-01-01T00:00:00Z"}"#;
        let dep: Dependency = serde_json::from_str(json).unwrap();
        assert_eq!(dep.dep_type, DependencyType::Blocks);
    }

    #[test]
    fn test_comment_text_field_renamed() {
        let json = r#"{"id":1,"issue_id":"bd-123","author":"user","text":"comment body","created_at":"2026-01-01T00:00:00Z"}"#;
        let comment: Comment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.body, "comment body");
    }

    #[test]
    fn test_event_type_field_renamed() {
        let json = r#"{"id":1,"issue_id":"bd-123","type":"status_changed","actor":"user","created_at":"2026-01-01T00:00:00Z"}"#;
        let event: Event = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, EventType::StatusChanged);
    }

    #[test]
    fn test_tombstone_status_renamed() {
        let json = r#"{"id":"bd-1","title":"Test","status":"tombstone","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.status, Status::Tombstone);
    }

    #[test]
    fn test_pinned_status_renamed() {
        let json = r#"{"id":"bd-1","title":"Test","status":"pinned","created_at":"2026-01-01T00:00:00Z","updated_at":"2026-01-01T00:00:00Z"}"#;
        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.status, Status::Pinned);
    }

    #[test]
    fn test_priority_transparent_serialization() {
        let p = Priority::CRITICAL;
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "0");

        let deserialized: Priority = serde_json::from_str("2").unwrap();
        assert_eq!(deserialized, Priority::MEDIUM);
    }

    #[test]
    fn test_dependency_type_kebab_case() {
        let json = r#"{"issue_id":"bd-1","depends_on_id":"bd-2","type":"parent-child","created_at":"2026-01-01T00:00:00Z"}"#;
        let dep: Dependency = serde_json::from_str(json).unwrap();
        assert_eq!(dep.dep_type, DependencyType::ParentChild);

        let serialized = serde_json::to_string(&dep.dep_type).unwrap();
        assert_eq!(serialized, "\"parent-child\"");
    }

    #[test]
    fn test_full_issue_with_all_fields() {
        let json = r#"{
  "id": "bd-full",
  "title": "Full Issue",
  "description": "Description",
  "design": "Design notes",
  "acceptance_criteria": "Criteria",
  "notes": "Additional notes",
  "status": "open",
  "priority": 1,
  "issue_type": "bug",
  "assignee": "alice",
  "owner": "bob",
  "estimated_minutes": 120,
  "created_at": "2026-01-01T00:00:00Z",
  "created_by": "charlie",
  "updated_at": "2026-01-02T00:00:00Z",
  "closed_at": "2026-01-03T00:00:00Z",
  "close_reason": "Fixed",
  "closed_by_session": "session-123",
  "due_at": "2026-01-05T00:00:00Z",
  "defer_until": "2026-01-06T00:00:00Z",
  "external_ref": "JIRA-123",
  "source_system": "jira",
  "source_repo": "myrepo",
  "deleted_at": "2026-01-07T00:00:00Z",
  "deleted_by": "admin",
  "delete_reason": "Duplicate",
  "original_type": "bug",
  "compaction_level": 1,
  "compacted_at": "2026-01-08T00:00:00Z",
  "compacted_at_commit": "abc123",
  "original_size": 1000,
  "sender": "system",
  "ephemeral": true,
  "pinned": true,
  "is_template": false,
  "labels": ["urgent", "backend"],
  "dependencies": [
    {
      "issue_id": "bd-full",
      "depends_on_id": "bd-dep",
      "type": "blocks",
      "created_at": "2026-01-01T00:00:00Z",
      "created_by": "alice",
      "metadata": "{\"key\":\"value\"}",
      "thread_id": "thread-1"
    }
  ],
  "comments": [
    {
      "id": 1,
      "issue_id": "bd-full",
      "author": "alice",
      "text": "First comment",
      "created_at": "2026-01-01T00:00:00Z"
    }
  ]
}"#;

        let issue: Issue = serde_json::from_str(json).expect("Failed to deserialize full issue");

        // Verify all fields are preserved
        assert_eq!(issue.id, "bd-full");
        assert_eq!(issue.title, "Full Issue");
        assert_eq!(issue.description.as_deref(), Some("Description"));
        assert_eq!(issue.design.as_deref(), Some("Design notes"));
        assert_eq!(issue.acceptance_criteria.as_deref(), Some("Criteria"));
        assert_eq!(issue.notes.as_deref(), Some("Additional notes"));
        assert_eq!(issue.status, Status::Open);
        assert_eq!(issue.priority, Priority::HIGH);
        assert_eq!(issue.issue_type, IssueType::Bug);
        assert_eq!(issue.assignee.as_deref(), Some("alice"));
        assert_eq!(issue.owner.as_deref(), Some("bob"));
        assert_eq!(issue.estimated_minutes, Some(120));
        assert_eq!(issue.created_by.as_deref(), Some("charlie"));
        assert_eq!(issue.close_reason.as_deref(), Some("Fixed"));
        assert_eq!(issue.closed_by_session.as_deref(), Some("session-123"));
        assert_eq!(issue.external_ref.as_deref(), Some("JIRA-123"));
        assert_eq!(issue.source_system.as_deref(), Some("jira"));
        assert_eq!(issue.source_repo.as_deref(), Some("myrepo"));
        assert_eq!(issue.deleted_by.as_deref(), Some("admin"));
        assert_eq!(issue.delete_reason.as_deref(), Some("Duplicate"));
        assert_eq!(issue.original_type.as_deref(), Some("bug"));
        assert_eq!(issue.compaction_level, Some(1));
        assert_eq!(issue.compacted_at_commit.as_deref(), Some("abc123"));
        assert_eq!(issue.original_size, Some(1000));
        assert_eq!(issue.sender.as_deref(), Some("system"));
        assert!(issue.ephemeral);
        assert!(issue.pinned);
        assert!(!issue.is_template);
        assert_eq!(issue.labels, vec!["urgent", "backend"]);
        assert_eq!(issue.dependencies.len(), 1);
        assert_eq!(issue.dependencies[0].dep_type, DependencyType::Blocks);
        assert_eq!(issue.comments.len(), 1);
        assert_eq!(issue.comments[0].body, "First comment");

        // Roundtrip test
        let serialized = serde_json::to_string(&issue).unwrap();
        let roundtrip: Issue = serde_json::from_str(&serialized).unwrap();
        assert_eq!(issue, roundtrip);
    }

    #[test]
    fn test_epic_status_serialization() {
        let epic_status = EpicStatus {
            epic: Issue {
                id: "bd-epic".to_string(),
                title: "Epic".to_string(),
                created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
                updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
                ..Default::default()
            },
            total_children: 10,
            closed_children: 7,
            eligible_for_close: false,
        };

        let json = serde_json::to_string(&epic_status).unwrap();
        assert!(json.contains("\"total_children\":10"));
        assert!(json.contains("\"closed_children\":7"));
        assert!(json.contains("\"eligible_for_close\":false"));
    }

    #[test]
    fn test_sync_equals_ignores_audit_timestamps_and_relation_order() {
        let mut issue1 = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            labels: vec!["backend".to_string(), "bug".to_string()],
            dependencies: vec![
                Dependency {
                    issue_id: "bd-test".to_string(),
                    depends_on_id: "bd-parent".to_string(),
                    dep_type: DependencyType::Blocks,
                    created_at: Utc.timestamp_opt(1_700_000_100, 0).unwrap(),
                    created_by: Some("alice".to_string()),
                    metadata: Some("{\"source\":\"cli\"}".to_string()),
                    thread_id: Some("br-1".to_string()),
                },
                Dependency {
                    issue_id: "bd-test".to_string(),
                    depends_on_id: "bd-epic".to_string(),
                    dep_type: DependencyType::ParentChild,
                    created_at: Utc.timestamp_opt(1_700_000_200, 0).unwrap(),
                    created_by: Some("alice".to_string()),
                    metadata: None,
                    thread_id: None,
                },
            ],
            comments: vec![
                Comment {
                    id: 2,
                    issue_id: "bd-test".to_string(),
                    author: "alice".to_string(),
                    body: "second".to_string(),
                    created_at: Utc.timestamp_opt(1_700_000_200, 0).unwrap(),
                },
                Comment {
                    id: 1,
                    issue_id: "bd-test".to_string(),
                    author: "alice".to_string(),
                    body: "first".to_string(),
                    created_at: Utc.timestamp_opt(1_700_000_100, 0).unwrap(),
                },
            ],
            ..Default::default()
        };

        let mut issue2 = issue1.clone();
        issue2.created_at = Utc.timestamp_opt(1_800_000_000, 0).unwrap();
        issue2.updated_at = Utc.timestamp_opt(1_800_000_500, 0).unwrap();
        issue2.labels.reverse();
        issue2.dependencies.reverse();
        issue2.comments.reverse();
        issue2.content_hash = Some("stale-hash".to_string());

        assert!(issue1.sync_equals(&issue2));
        assert!(issue2.sync_equals(&issue1));
    }

    #[test]
    fn test_sync_equals_detects_semantic_changes() {
        let issue1 = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        let mut issue2 = issue1.clone();
        issue2.due_at = Some(Utc.timestamp_opt(1_800_000_000, 0).unwrap());

        assert!(!issue1.sync_equals(&issue2));
    }

    #[test]
    fn test_sync_equals_treats_duplicate_labels_as_equivalent() {
        let mut issue1 = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        issue1.labels = vec![
            "backend".to_string(),
            "backend".to_string(),
            "urgent".to_string(),
        ];

        let mut issue2 = issue1.clone();
        issue2.labels = vec!["urgent".to_string(), "backend".to_string()];

        assert!(issue1.sync_equals(&issue2));
        assert!(issue2.sync_equals(&issue1));
    }

    #[test]
    fn test_is_expired_tombstone_not_tombstone() {
        let issue = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            status: Status::Open,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        assert!(!issue.is_expired_tombstone(Some(30)));
    }

    #[test]
    fn test_is_expired_tombstone_no_retention() {
        let mut issue = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            status: Status::Tombstone,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        issue.deleted_at = Some(Utc::now() - chrono::Duration::days(100));
        assert!(!issue.is_expired_tombstone(None));
    }

    #[test]
    fn test_is_expired_tombstone_zero_retention() {
        let mut issue = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            status: Status::Tombstone,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        issue.deleted_at = Some(Utc::now() - chrono::Duration::days(100));
        assert!(!issue.is_expired_tombstone(Some(0)));
    }

    #[test]
    fn test_is_expired_tombstone_no_deleted_at() {
        let issue = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            status: Status::Tombstone,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        assert!(!issue.is_expired_tombstone(Some(30)));
    }

    #[test]
    fn test_is_expired_tombstone_not_expired() {
        let mut issue = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            status: Status::Tombstone,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        issue.deleted_at = Some(Utc::now() - chrono::Duration::days(10));
        assert!(!issue.is_expired_tombstone(Some(30)));
    }

    #[test]
    fn test_is_expired_tombstone_expired() {
        let mut issue = Issue {
            id: "bd-test".to_string(),
            title: "Test".to_string(),
            status: Status::Tombstone,
            created_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            updated_at: Utc.timestamp_opt(1_700_000_000, 0).unwrap(),
            ..Default::default()
        };
        issue.deleted_at = Some(Utc::now() - chrono::Duration::days(40));
        assert!(issue.is_expired_tombstone(Some(30)));
    }
}
