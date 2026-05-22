//! Configuration schema definitions for Rodgers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

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

/// Interpolate environment variables in a string value.
/// Supports ${ENV_VAR} syntax. If the env var is not set, the placeholder
/// is left unchanged (allowing config files to be shared with documentation).
pub fn interpolate_env_var(value: &str) -> String {
    let mut result = value.to_string();

    // Simple scan for ${VAR} pattern without regex
    let mut start = 0;
    while let Some(dollar_pos) = result[start..].find('$') {
        let abs_pos = start + dollar_pos;

        // Check if followed by ${...}
        if abs_pos + 1 < result.len() && result.chars().nth(abs_pos + 1) == Some('{') {
            if let Some(close_pos) = result[abs_pos + 2..].find('}') {
                let var_start = abs_pos + 2;
                let var_end = var_start + close_pos;
                let var_name = &result[var_start..var_end];

                if let Ok(env_value) = env::var(var_name) {
                    let full_match = format!("${{{}}}", var_name);
                    result = result.replace(&full_match, &env_value);
                    // Don't advance start - check for more occurrences in modified string
                } else {
                    // Env var not set, skip past this pattern to avoid infinite loop
                    start = var_end + 1;
                }
                continue;
            }
        }

        // No ${...}, move past this $
        start = abs_pos + 1;
    }

    result
}

/// Apply environment variable interpolation to all string fields in the config.
pub fn apply_env_interpolation(config: &mut Config) {
    config.github.token = interpolate_env_var(&config.github.token);
    if let Some(api_url) = &config.github.api_url {
        config.github.api_url = Some(interpolate_env_var(api_url));
    }

    config.beads.remote = interpolate_env_var(&config.beads.remote);
    if let Some(database) = &config.beads.database {
        config.beads.database = Some(interpolate_env_var(database));
    }

    if let Some(base_url) = &config.llm.base_url {
        config.llm.base_url = Some(interpolate_env_var(base_url));
    }
    config.llm.api_key = interpolate_env_var(&config.llm.api_key);

    if let Some(level) = &config.log_level {
        config.log_level = Some(interpolate_env_var(level));
    }

    if let Some(channel) = &config.error_channel {
        config.error_channel = Some(interpolate_env_var(channel));
    }
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

/// LLM configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub provider: Option<String>,
    pub base_url: Option<String>,
    pub model: String,
    pub api_key: String,
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

/// Placeholder token patterns to detect.
pub const PLACEHOLDER_TOKEN_PATTERNS: &[&str] = &[
    "YOUR_TOKEN",
    "YOUR_GITHUB_TOKEN",
    "ghp_sample",
    "ghp_xxxxxxxxxxxx",
    "github_pat_sample",
    "REPLACE_ME",
    "CHANGE_ME",
    "INSERT_TOKEN_HERE",
];

/// Rodgers-required labels that should never be in labels_never_bot_managed.
pub fn rodgers_required_label_names() -> &'static [&'static str] {
    &[
        "bug",
        "feature",
        "question",
        "needs-information",
        "needs-documentation",
        "ready-for-review",
        "will-not-do",
        "ready-for-work",
        "in-progress",
    ]
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

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: Some("openai".to_string()),
            base_url: Some("https://api.openai.com/v1".to_string()),
            model: String::new(),
            api_key: String::new(),
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
            approval_discussion_category: Some("Release Proposals".to_string()),
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
