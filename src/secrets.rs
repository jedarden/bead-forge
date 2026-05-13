//! Secret detection and prevention for bead fields.
//!
//! Scans string fields for common secret patterns before writing to storage.
//! Refuses operations that match configured patterns.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Configuration for secret protection patterns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretProtectionConfig {
    /// Enable secret scanning (default: true)
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Custom regex patterns to block (in addition to built-ins)
    #[serde(default)]
    pub custom_patterns: Vec<String>,

    /// Patterns to allow-list (never block)
    #[serde(default)]
    pub allowlist: Vec<String>,
}

impl Default for SecretProtectionConfig {
    fn default() -> Self {
        SecretProtectionConfig {
            enabled: default_enabled(),
            custom_patterns: Vec::new(),
            allowlist: Vec::new(),
        }
    }
}

fn default_enabled() -> bool {
    true
}

/// A detected secret match.
#[derive(Debug, Clone)]
pub struct SecretMatch {
    /// The pattern name that matched
    pub pattern_name: String,
    /// The matched text
    pub matched_text: String,
}

/// Secret scanner with built-in patterns.
pub struct SecretScanner {
    built_in_patterns: Vec<(&'static str, Regex)>,
    custom_patterns: Vec<Regex>,
    allowlist: Vec<Regex>,
}

impl SecretScanner {
    /// Create a new scanner with built-in patterns.
    pub fn new() -> Result<Self, regex::Error> {
        Ok(SecretScanner {
            built_in_patterns: Self::built_in_patterns()?,
            custom_patterns: Vec::new(),
            allowlist: Vec::new(),
        })
    }

    /// Create a scanner from config.
    pub fn from_config(config: &SecretProtectionConfig) -> Result<Self, regex::Error> {
        let mut scanner = Self::new()?;

        // Compile custom patterns
        for pattern in &config.custom_patterns {
            match Regex::new(pattern) {
                Ok(re) => scanner.custom_patterns.push(re),
                Err(e) => {
                    eprintln!("Warning: invalid custom regex pattern '{}': {}", pattern, e);
                }
            }
        }

        // Compile allowlist patterns
        for pattern in &config.allowlist {
            match Regex::new(pattern) {
                Ok(re) => scanner.allowlist.push(re),
                Err(e) => {
                    eprintln!(
                        "Warning: invalid allowlist regex pattern '{}': {}",
                        pattern, e
                    );
                }
            }
        }

        Ok(scanner)
    }

    /// Define built-in secret patterns.
    fn built_in_patterns() -> Result<Vec<(&'static str, Regex)>, regex::Error> {
        Ok(vec![
            // AWS Access Key ID
            (
                "AWS Access Key",
                Regex::new(r"(?i)(?:aws_access_key_id|aws.*key|access key)[\s=:]+[A-Z0-9]{20}")?,
            ),
            (
                "AWS Access Key (AKIA)",
                Regex::new(r"\bAKIA[0-9A-Z]{16}\b")?,
            ),
            // AWS Secret Access Key
            (
                "AWS Secret Key",
                Regex::new(r"(?i)(?:aws_secret_access_key|secret.*key)[\s=:]+[a-zA-Z0-9+/]{40}")?,
            ),
            // Generic API tokens
            ("API Token (sk-)", Regex::new(r"\bsk-[a-zA-Z0-9]{20,}\b")?),
            (
                "API Token (xoxb-)",
                Regex::new(r"\bxoxb-[0-9]{11,13}-[0-9]{11,13}-[a-zA-Z0-9]{24}\b")?,
            ),
            ("API Token (ghp_)", Regex::new(r"\bghp_[a-zA-Z0-9]{36}\b")?),
            ("API Token (gho_)", Regex::new(r"\bgho_[a-zA-Z0-9]{36}\b")?),
            ("API Token (ghu_)", Regex::new(r"\bghu_[a-zA-Z0-9]{36}\b")?),
            ("API Token (ghs_)", Regex::new(r"\bghs_[a-zA-Z0-9]{36}\b")?),
            ("API Token (ghr_)", Regex::new(r"\bghr_[a-zA-Z0-9]{36}\b")?),
            (
                "API Token (github_pat_)",
                Regex::new(r"\bgithub_pat_[a-zA-Z0-9_]{82}\b")?,
            ),
            // Private keys (PEM headers and base64 blocks)
            (
                "RSA Private Key",
                Regex::new(r"-----BEGIN RSA PRIVATE KEY-----")?,
            ),
            ("Private Key", Regex::new(r"-----BEGIN PRIVATE KEY-----")?),
            (
                "EC Private Key",
                Regex::new(r"-----BEGIN EC PRIVATE KEY-----")?,
            ),
            (
                "OpenSSH Private Key",
                Regex::new(r"-----BEGIN OPENSSH PRIVATE KEY-----")?,
            ),
            // JWT tokens
            (
                "JWT Token",
                Regex::new(r"\beyJ[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\.[a-zA-Z0-9_-]+\b")?,
            ),
            // Bearer tokens
            (
                "Bearer Token",
                Regex::new(r"(?i)bearer\s+[a-zA-Z0-9_\-\.~+/]+")?,
            ),
            // Database connection strings
            (
                "PostgreSQL URL",
                Regex::new(r"(?i)postgresql://[^\s]+:[^\s]+@[^\s]+")?,
            ),
            (
                "MySQL URL",
                Regex::new(r"(?i)mysql://[^\s]+:[^\s]+@[^\s]+")?,
            ),
            (
                "MongoDB URL",
                Regex::new(r"(?i)mongodb(?:\+srv)://[^\s]+:[^\s]+@[^\s]+")?,
            ),
            // API keys in query parameters
            (
                "API Key in URL",
                Regex::new(r"[?&](?:api[_-]?key|apikey|token|auth)[\s=]+[a-zA-Z0-9_\-]{16,}")?,
            ),
            // Stripe keys
            (
                "Stripe Key",
                Regex::new(r"\bsk_(?:live|test)_[a-zA-Z0-9]{24,}\b")?,
            ),
            (
                "Stripe Publishable Key",
                Regex::new(r"\bpk_(?:live|test)_[a-zA-Z0-9]{24,}\b")?,
            ),
            // SendGrid keys
            (
                "SendGrid Key",
                Regex::new(r"\bSG\.[a-zA-Z0-9_-]{22,}\.[a-zA-Z0-9_-]{43,}\b")?,
            ),
            // Slack webhooks
            (
                "Slack Webhook",
                Regex::new(
                    r"\bhttps://hooks\.slack\.com/services/T[A-Z0-9]{8}/B[A-Z0-9]{8}/[a-zA-Z0-9]{24}\b",
                )?,
            ),
            // Google Cloud credentials
            (
                "Google Cloud Key",
                Regex::new(r#""type":\s*"service_account""#)?,
            ),
            (
                "Google OAuth",
                Regex::new(r"\b[0-9]+-[a-zA-Z0-9_]{32}\.apps\.googleusercontent\.com\b")?,
            ),
            // Azure keys
            ("Azure Key", Regex::new(r"[a-zA-Z0-9/_-]{44}")?),
            // Password fields (common patterns)
            (
                "Password in Key",
                Regex::new(r"(?i)(?:password|passwd|pwd)[\s':=]+[^\s]{4,}")?,
            ),
        ])
    }

    /// Scan a single string for secrets.
    pub fn scan_string(&self, value: &str) -> Vec<SecretMatch> {
        let mut matches = Vec::new();

        // Check allowlist first
        for allow_re in &self.allowlist {
            if allow_re.is_match(value) {
                return Vec::new();
            }
        }

        // Check built-in patterns
        for (name, pattern) in &self.built_in_patterns {
            if let Some(m) = pattern.find(value) {
                matches.push(SecretMatch {
                    pattern_name: name.to_string(),
                    matched_text: m.as_str().to_string(),
                });
            }
        }

        // Check custom patterns
        for pattern in &self.custom_patterns {
            if let Some(m) = pattern.find(value) {
                matches.push(SecretMatch {
                    pattern_name: "custom".to_string(),
                    matched_text: m.as_str().to_string(),
                });
            }
        }

        matches
    }

    /// Scan all string fields of an issue for secrets.
    pub fn scan_issue(&self, issue: &crate::model::Issue) -> Vec<SecretMatch> {
        let mut all_matches = Vec::new();
        let mut seen = HashSet::new();

        // Scan all string fields
        for field in [
            &issue.id,
            &issue.title,
            issue.description.as_deref().unwrap_or(""),
            issue.design.as_deref().unwrap_or(""),
            issue.acceptance_criteria.as_deref().unwrap_or(""),
            issue.notes.as_deref().unwrap_or(""),
            issue.assignee.as_deref().unwrap_or(""),
            issue.owner.as_deref().unwrap_or(""),
            issue.created_by.as_deref().unwrap_or(""),
            issue.close_reason.as_deref().unwrap_or(""),
            issue.closed_by_session.as_deref().unwrap_or(""),
            issue.external_ref.as_deref().unwrap_or(""),
            issue.source_system.as_deref().unwrap_or(""),
            issue.source_repo.as_deref().unwrap_or(""),
            issue.deleted_by.as_deref().unwrap_or(""),
            issue.delete_reason.as_deref().unwrap_or(""),
            issue.original_type.as_deref().unwrap_or(""),
            issue.compacted_at_commit.as_deref().unwrap_or(""),
            issue.sender.as_deref().unwrap_or(""),
        ] {
            for m in self.scan_string(field) {
                // Deduplicate by matched text
                if seen.insert(m.matched_text.clone()) {
                    all_matches.push(m);
                }
            }
        }

        // Scan labels
        for label in &issue.labels {
            for m in self.scan_string(label) {
                if seen.insert(m.matched_text.clone()) {
                    all_matches.push(m);
                }
            }
        }

        // Scan annotations
        for (key, value) in &issue.annotations {
            for m in self.scan_string(key) {
                if seen.insert(m.matched_text.clone()) {
                    all_matches.push(m);
                }
            }
            for m in self.scan_string(value) {
                if seen.insert(m.matched_text.clone()) {
                    all_matches.push(m);
                }
            }
        }

        all_matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Issue, Priority, Status};
    use chrono::Utc;

    #[test]
    fn test_detects_aws_key() {
        let scanner = SecretScanner::new().unwrap();
        let matches = scanner.scan_string("AKIAIOSFODNN7EXAMPLE");
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.pattern_name.contains("AKIA")));
    }

    #[test]
    fn test_detects_private_key() {
        let scanner = SecretScanner::new().unwrap();
        let matches = scanner.scan_string("-----BEGIN RSA PRIVATE KEY-----");
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.pattern_name.contains("RSA")));
    }

    #[test]
    fn test_detects_jwt() {
        let scanner = SecretScanner::new().unwrap();
        let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let matches = scanner.scan_string(jwt);
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.pattern_name.contains("JWT")));
    }

    #[test]
    fn test_detects_api_token() {
        let scanner = SecretScanner::new().unwrap();
        let matches = scanner.scan_string("sk-1234567890abcdefghijklmnopqrst");
        assert!(!matches.is_empty());
        assert!(matches.iter().any(|m| m.pattern_name.contains("sk-")));
    }

    #[test]
    fn test_allowlist() {
        let mut config = SecretProtectionConfig::default();
        config.allowlist = vec![r"^AKIAEXAMPLE$".to_string()];

        let scanner = SecretScanner::from_config(&config).unwrap();
        let matches = scanner.scan_string("AKIAEXAMPLE");
        assert!(matches.is_empty());
    }

    #[test]
    fn test_scans_issue() {
        let scanner = SecretScanner::new().unwrap();
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Test issue".to_string(),
            description: Some("Use AKIAIOSFODNN7EXAMPLE for auth".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        let matches = scanner.scan_issue(&issue);
        assert!(!matches.is_empty());
    }

    #[test]
    fn test_safe_content_passes() {
        let scanner = SecretScanner::new().unwrap();
        let issue = Issue {
            id: "bf-test".to_string(),
            title: "Implement feature X".to_string(),
            description: Some("Add a new API endpoint for fetching user data".to_string()),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            ..Default::default()
        };

        let matches = scanner.scan_issue(&issue);
        assert!(matches.is_empty());
    }
}
