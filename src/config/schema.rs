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

/// Default values for release configuration.
pub const DEFAULT_APPROVAL_DISCUSSION_CATEGORY: &str = "Announcements";
pub const DEFAULT_VOTING_WINDOW_DAYS: u32 = 2;
pub const DEFAULT_STALE_THRESHOLD_DAYS: u32 = 7;

/// Release configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseConfig {
    pub approval_discussion_category: Option<String>,
    pub active_branches: Option<Vec<String>>,
    pub voting_window_days: Option<u32>,
    pub stale_threshold_days: Option<u32>,
}

impl ReleaseConfig {
    /// Create a new ReleaseConfig with explicit values.
    pub fn new(
        approval_discussion_category: Option<String>,
        active_branches: Option<Vec<String>>,
        voting_window_days: Option<u32>,
        stale_threshold_days: Option<u32>,
    ) -> Self {
        Self {
            approval_discussion_category,
            active_branches,
            voting_window_days,
            stale_threshold_days,
        }
    }

    /// Apply default values for any fields that are None.
    pub fn apply_defaults(&self) -> ResolvedReleaseConfig {
        ResolvedReleaseConfig {
            approval_discussion_category: self
                .approval_discussion_category
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| DEFAULT_APPROVAL_DISCUSSION_CATEGORY.to_string()),
            active_branches: self.active_branches.clone().unwrap_or_default(),
            voting_window_days: self
                .voting_window_days
                .unwrap_or(DEFAULT_VOTING_WINDOW_DAYS),
            stale_threshold_days: self
                .stale_threshold_days
                .unwrap_or(DEFAULT_STALE_THRESHOLD_DAYS),
        }
    }

    /// Validate the config and apply defaults in one step.
    pub fn validate_and_defaults(&self) -> Result<ResolvedReleaseConfig, Vec<String>> {
        let mut errors = Vec::new();

        if let Some(days) = self.voting_window_days {
            if days == 0 {
                errors.push("release.voting_window_days must be positive".to_string());
            }
        }
        if let Some(days) = self.stale_threshold_days {
            if days == 0 {
                errors.push("release.stale_threshold_days must be positive".to_string());
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let resolved = self.apply_defaults();

        if resolved.active_branches.is_empty() {
            tracing::warn!("release.active_branches is empty — backport manager will not operate");
        }

        Ok(resolved)
    }

    /// Check if any release configuration fields are set (non-None).
    pub fn is_configured(&self) -> bool {
        self.approval_discussion_category.is_some()
            || self.active_branches.is_some()
            || self.voting_window_days.is_some()
            || self.stale_threshold_days.is_some()
    }
}

/// Resolved release configuration with all defaults applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedReleaseConfig {
    /// GitHub Discussion category for approval proposals.
    pub approval_discussion_category: String,

    /// Active release branches for backports.
    pub active_branches: Vec<String>,

    /// Days before reminder ping for stale proposals.
    pub voting_window_days: u32,

    /// Days before closing a stale proposal.
    pub stale_threshold_days: u32,
}

impl ResolvedReleaseConfig {
    /// Check if a branch is an active release branch.
    pub fn is_active_branch(&self, branch: &str) -> bool {
        self.active_branches.contains(&branch.to_string())
    }

    /// Get the list of active branches.
    pub fn active_branches(&self) -> &[String] {
        &self.active_branches
    }

    /// Check if backport management is active (has at least one branch).
    pub fn is_backport_active(&self) -> bool {
        !self.active_branches.is_empty()
    }

    /// Check if a proposal is stale.
    pub fn is_stale_after_days(&self, days_since_creation: i32) -> bool {
        days_since_creation > self.stale_threshold_days as i32
    }

    /// Check if a proposal should be nudged.
    pub fn should_nudge_after_days(&self, days_since_creation: i32) -> bool {
        days_since_creation > self.voting_window_days as i32
            && !self.is_stale_after_days(days_since_creation)
    }
}

/// Load release configuration from a YAML file.
///
/// Returns a `ReleaseConfig` parsed from the YAML. If the file does not
/// exist or the release section is absent, returns `ReleaseConfig::default()`.
pub fn load_release_config_from_yaml(path: &std::path::Path) -> Result<ReleaseConfig, String> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to read config file {:?}: {}", path, e))?;

    #[derive(Debug, Deserialize)]
    struct ConfigWrapper {
        release: Option<ReleaseConfig>,
    }

    let wrapper: ConfigWrapper =
        serde_yaml::from_str(&content).map_err(|e| format!("YAML parse error: {}", e))?;

    Ok(wrapper.release.unwrap_or_default())
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
