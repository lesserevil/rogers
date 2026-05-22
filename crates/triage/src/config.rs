//! Configuration types for the triage crate.
//!
//! Mirrors the config types from the root crate for use in this workspace member.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export LLM config from rogers_llm crate
pub use rogers_llm::client::LlmConfig;

/// Top-level configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub github: GitHubConfig,
    pub scheduler: SchedulerConfig,
    pub beads: BeadsConfig,
    pub llm: LlmConfig,
    pub triage: Option<TriageConfig>,
    pub release: Option<ReleaseConfig>,
    pub rogation: Option<RogationConfig>,
    pub log_level: Option<String>,
    pub error_channel: Option<String>,
}

/// GitHub configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub owner: String,
    pub repo: String,
    pub token: String,
    pub api_url: Option<String>,
}

/// Scheduler configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    pub interval_minutes: u32,
    pub enabled: Option<bool>,
}

/// Beads (dolt) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeadsConfig {
    pub remote: String,
    pub database: Option<String>,
}

/// Triage configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageConfig {
    pub default_labels: Option<Vec<String>>,
    pub bot_labels: Option<Vec<String>>,
    pub close_labels: Option<Vec<String>>,
    pub assignees: Option<Vec<String>>,
}

/// Release configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseConfig {
    pub approval_discussion_category: Option<String>,
    pub active_branches: Option<Vec<String>>,
    pub voting_window_days: Option<u32>,
    pub stale_threshold_days: Option<u32>,
}

/// Rogation (project-level) configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RogationConfig {
    pub ignore_labels: Option<Vec<String>>,
    pub labels_never_bot_managed: Option<Vec<String>>,
    pub custom_type_names: Option<HashMap<String, String>>,
    pub format: Option<String>,
    pub agent_file: Option<String>,
    pub template_dir: Option<String>,
    pub security_label: Option<String>,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_minutes: 15,
            enabled: Some(true),
        }
    }
}

impl Default for BeadsConfig {
    fn default() -> Self {
        Self {
            remote: String::new(),
            database: Some("message.hibernate".to_string()),
        }
    }
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            default_labels: Some(vec![
                "bug".to_string(),
                "enhancement".to_string(),
                "question".to_string(),
            ]),
            bot_labels: Some(vec![]),
            close_labels: Some(vec![
                "wontfix".to_string(),
                "duplicate".to_string(),
                "not planned".to_string(),
            ]),
            assignees: Some(vec![]),
        }
    }
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            approval_discussion_category: Some("Announcements".to_string()),
            active_branches: Some(vec![]),
            voting_window_days: Some(2),
            stale_threshold_days: Some(7),
        }
    }
}

impl Default for RogationConfig {
    fn default() -> Self {
        Self {
            ignore_labels: Some(vec![]),
            labels_never_bot_managed: Some(vec![]),
            custom_type_names: Some(HashMap::new()),
            format: None,
            agent_file: None,
            template_dir: None,
            security_label: Some("security".to_string()),
        }
    }
}
