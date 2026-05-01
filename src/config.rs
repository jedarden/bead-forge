use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_issue_prefixes")]
    pub issue_prefixes: Vec<String>,
    #[serde(default = "default_default_priority")]
    pub default_priority: i32,
    #[serde(default = "default_default_type")]
    pub default_type: String,
    #[serde(default)]
    pub scoring: ScoringConfig,
    #[serde(default)]
    pub claim_ttl_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringConfig {
    #[serde(default = "default_priority_weight")]
    pub priority_weight: f64,
    #[serde(default = "default_blockers_weight")]
    pub blockers_weight: f64,
    #[serde(default = "default_age_weight")]
    pub age_weight: f64,
    #[serde(default = "default_labels_weight")]
    pub labels_weight: f64,
    #[serde(default = "default_max_age_hours")]
    pub max_age_hours: i64,
    #[serde(default = "default_max_blockers")]
    pub max_blockers: i32,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        ScoringConfig {
            priority_weight: default_priority_weight(),
            blockers_weight: default_blockers_weight(),
            age_weight: default_age_weight(),
            labels_weight: default_labels_weight(),
            max_age_hours: default_max_age_hours(),
            max_blockers: default_max_blockers(),
        }
    }
}

fn default_issue_prefixes() -> Vec<String> {
    vec!["bf".to_string()]
}

fn default_default_priority() -> i32 {
    2
}

fn default_default_type() -> String {
    "task".to_string()
}

fn default_priority_weight() -> f64 {
    0.4
}

fn default_blockers_weight() -> f64 {
    0.3
}

fn default_age_weight() -> f64 {
    0.2
}

fn default_labels_weight() -> f64 {
    0.1
}

fn default_max_age_hours() -> i64 {
    20
}

fn default_max_blockers() -> i32 {
    3
}

impl Default for Config {
    fn default() -> Self {
        Config {
            issue_prefixes: default_issue_prefixes(),
            default_priority: default_default_priority(),
            default_type: default_default_type(),
            scoring: ScoringConfig::default(),
            claim_ttl_minutes: 30,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub database: String,
    #[serde(rename = "jsonl_export")]
    pub jsonl_export: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Metadata {
            database: "beads.db".to_string(),
            jsonl_export: "issues.jsonl".to_string(),
        }
    }
}

pub fn find_beads_dir(start_dir: &Path) -> Option<PathBuf> {
    let mut current = Some(start_dir);
    while let Some(dir) = current {
        let beads_dir = dir.join(".beads");
        if beads_dir.is_dir() {
            return Some(beads_dir);
        }
        current = dir.parent();
    }
    None
}

pub fn load_config(beads_dir: &Path) -> Result<Config> {
    let config_path = beads_dir.join("config.yaml");
    if config_path.exists() {
        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    } else {
        Ok(Config::default())
    }
}

pub fn load_metadata(beads_dir: &Path) -> Result<Metadata> {
    let metadata_path = beads_dir.join("metadata.json");
    if metadata_path.exists() {
        let content = std::fs::read_to_string(&metadata_path)?;
        let metadata: Metadata = serde_json::from_str(&content)?;
        Ok(metadata)
    } else {
        Ok(Metadata::default())
    }
}

pub fn get_default_prefix(config: &Config) -> &str {
    config.issue_prefixes.first().map(|s| s.as_str()).unwrap_or("bf")
}

/// Initialize a new workspace directory with default config and metadata.
///
/// Creates the .beads directory with default configuration files.
/// Used primarily for testing.
pub fn init_workspace(beads_dir: &Path, prefix: &str) -> Result<()> {
    std::fs::create_dir_all(beads_dir)?;

    // Write default config.yaml
    let config = Config {
        issue_prefixes: vec![prefix.to_string()],
        ..Default::default()
    };
    let config_yaml = serde_yaml::to_string(&config)?;
    std::fs::write(beads_dir.join("config.yaml"), config_yaml)?;

    // Write default metadata.json
    let metadata = Metadata::default();
    let metadata_json = serde_json::to_string_pretty(&metadata)?;
    std::fs::write(beads_dir.join("metadata.json"), metadata_json)?;

    Ok(())
}
