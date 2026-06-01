//! Configuration schema definitions for Rodgers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::path::PathBuf;

/// Top-level configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub github: GitHubConfig,
    pub scheduler: SchedulerConfig,
    pub backlog: BacklogConfig,
    pub llm: LlmConfig,
    pub triage: Option<TriageConfig>,
    pub release: Option<ReleaseConfig>,
    pub rogation: Option<RogationConfig>,
    pub log_level: Option<String>,
    pub error_channel: Option<String>,
}

/// Partial configuration used for layered config loading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub github: Option<GitHubConfig>,
    pub scheduler: Option<SchedulerConfig>,
    pub backlog: Option<BacklogConfig>,
    pub llm: Option<LlmConfig>,
    pub triage: Option<TriageConfig>,
    pub release: Option<ReleaseConfig>,
    pub rogation: Option<RogationConfig>,
    pub question_routing: Option<QuestionRoutingConfig>,
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

    if let Some(path) = &config.backlog.path {
        config.backlog.path = Some(interpolate_env_var(path));
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

/// Backlog.md task store configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklogConfig {
    pub path: Option<String>,
}

/// Question routing configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestionRoutingConfig {
    pub code_search_keywords: Vec<String>,
    pub doc_search_path: PathBuf,
    pub code_search_path: String,
}

impl QuestionRoutingConfig {
    pub fn keywords(&self) -> &[String] {
        &self.code_search_keywords
    }

    pub fn has_keywords(&self) -> bool {
        !self.code_search_keywords.is_empty()
    }

    pub fn matches_question(&self, text: &str) -> bool {
        let text = text.to_ascii_lowercase();
        self.code_search_keywords
            .iter()
            .any(|keyword| text.contains(&keyword.to_ascii_lowercase()))
    }
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
pub const DEFAULT_APPROVAL_DISCUSSION_CATEGORY: &str = "Release Proposals";
pub const DEFAULT_VOTING_WINDOW_DAYS: i32 = 2;
pub const DEFAULT_STALE_THRESHOLD_DAYS: i32 = 7;

/// Release configuration.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseConfig {
    pub approval_discussion_category: Option<String>,
    pub active_branches: Option<Vec<String>>,
    pub voting_window_days: Option<i32>,
    pub stale_threshold_days: Option<i32>,
}

impl ReleaseConfig {
    /// Create a new ReleaseConfig with explicit values.
    pub fn new(
        approval_discussion_category: Option<String>,
        active_branches: Option<Vec<String>>,
        voting_window_days: Option<i32>,
        stale_threshold_days: Option<i32>,
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
            if days < 0 {
                errors.push("release.voting_window_days must be non-negative".to_string());
            }
        }
        if let Some(days) = self.stale_threshold_days {
            if days < 0 {
                errors.push("release.stale_threshold_days must be non-negative".to_string());
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
    pub voting_window_days: i32,

    /// Days before closing a stale proposal.
    pub stale_threshold_days: i32,
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
        days_since_creation > self.stale_threshold_days
    }

    /// Check if a proposal should be nudged.
    pub fn should_nudge_after_days(&self, days_since_creation: i32) -> bool {
        days_since_creation > self.voting_window_days
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

impl Default for BacklogConfig {
    fn default() -> Self {
        Self {
            path: Some("backlog".to_string()),
        }
    }
}

impl Default for QuestionRoutingConfig {
    fn default() -> Self {
        Self {
            code_search_keywords: vec![
                "how does".to_string(),
                "what function".to_string(),
                "which module".to_string(),
                "internals".to_string(),
                "implementation".to_string(),
                "source code".to_string(),
                "walk me through".to_string(),
                "flow of".to_string(),
                "under the hood".to_string(),
            ],
            doc_search_path: PathBuf::from("docs/"),
            code_search_path: "**/*".to_string(),
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

#[cfg(test)]
mod tests {
    use super::*;

    // Helper to safely set env vars in tests
    fn set_env_var(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    // =============================================================================
    // ReleaseConfig — schema validation tests
    // =============================================================================

    #[test]
    fn test_release_config_defaults() {
        let config = ReleaseConfig::default();
        assert!(config.approval_discussion_category.is_none());
        assert!(config.active_branches.is_none());
        assert!(config.voting_window_days.is_none());
        assert!(config.stale_threshold_days.is_none());
        assert!(!config.is_configured());
    }

    #[test]
    fn test_release_config_applies_defaults() {
        let config = ReleaseConfig::default();
        let resolved = config.apply_defaults();

        assert_eq!(
            resolved.approval_discussion_category,
            DEFAULT_APPROVAL_DISCUSSION_CATEGORY
        );
        assert!(resolved.active_branches.is_empty());
        assert_eq!(resolved.voting_window_days, DEFAULT_VOTING_WINDOW_DAYS);
        assert_eq!(resolved.stale_threshold_days, DEFAULT_STALE_THRESHOLD_DAYS);
    }

    #[test]
    fn test_release_config_custom_values() {
        let config = ReleaseConfig::new(
            Some("Security Advisories".to_string()),
            Some(vec!["release/1.x".to_string(), "release/2.x".to_string()]),
            Some(3),
            Some(10),
        );
        let resolved = config.apply_defaults();

        assert_eq!(resolved.approval_discussion_category, "Security Advisories");
        assert_eq!(resolved.active_branches.len(), 2);
        assert!(resolved.is_active_branch("release/1.x"));
        assert!(resolved.is_active_branch("release/2.x"));
        assert!(!resolved.is_active_branch("main"));
        assert_eq!(resolved.voting_window_days, 3);
        assert_eq!(resolved.stale_threshold_days, 10);
    }

    #[test]
    fn test_release_config_empty_category_defaults() {
        // Empty string should fall back to default
        let config = ReleaseConfig::new(Some("".to_string()), None, None, None);
        let resolved = config.apply_defaults();
        assert_eq!(
            resolved.approval_discussion_category,
            DEFAULT_APPROVAL_DISCUSSION_CATEGORY
        );
    }

    #[test]
    fn test_release_config_is_configured() {
        let empty = ReleaseConfig::default();
        assert!(!empty.is_configured());

        let configured = ReleaseConfig::new(Some("Announcements".to_string()), None, None, None);
        assert!(configured.is_configured());
    }

    // =============================================================================
    // ResolvedReleaseConfig tests
    // =============================================================================

    #[test]
    fn test_is_active_branch() {
        let config = ResolvedReleaseConfig {
            approval_discussion_category: "Announcements".to_string(),
            active_branches: vec!["release/1.x".to_string(), "release/2.x".to_string()],
            voting_window_days: 2,
            stale_threshold_days: 7,
        };

        assert!(config.is_active_branch("release/1.x"));
        assert!(config.is_active_branch("release/2.x"));
        assert!(!config.is_active_branch("main"));
        assert!(!config.is_active_branch("develop"));
    }

    #[test]
    fn test_is_backport_active() {
        let with_branches = ResolvedReleaseConfig {
            approval_discussion_category: "Announcements".to_string(),
            active_branches: vec!["release/1.x".to_string()],
            voting_window_days: 2,
            stale_threshold_days: 7,
        };
        assert!(with_branches.is_backport_active());

        let empty_branches = ResolvedReleaseConfig {
            approval_discussion_category: "Announcements".to_string(),
            active_branches: vec![],
            voting_window_days: 2,
            stale_threshold_days: 7,
        };
        assert!(!empty_branches.is_backport_active());
    }

    #[test]
    fn test_is_stale_after_days() {
        let config = ResolvedReleaseConfig {
            approval_discussion_category: "Announcements".to_string(),
            active_branches: vec!["release/1.x".to_string()],
            voting_window_days: 2,
            stale_threshold_days: 7,
        };

        assert!(!config.is_stale_after_days(0));
        assert!(!config.is_stale_after_days(5));
        assert!(!config.is_stale_after_days(7));
        assert!(config.is_stale_after_days(8));
        assert!(config.is_stale_after_days(30));
    }

    #[test]
    fn test_should_nudge_after_days() {
        let config = ResolvedReleaseConfig {
            approval_discussion_category: "Announcements".to_string(),
            active_branches: vec!["release/1.x".to_string()],
            voting_window_days: 2,
            stale_threshold_days: 7,
        };

        // Before voting window: no nudge
        assert!(!config.should_nudge_after_days(0));
        assert!(!config.should_nudge_after_days(1));
        // At voting window boundary: no nudge (must be > voting_window)
        assert!(!config.should_nudge_after_days(2));
        // Past voting window but not stale: nudge
        assert!(config.should_nudge_after_days(3));
        assert!(config.should_nudge_after_days(6));
        // At stale threshold: not stale yet (must be > stale_threshold)
        assert!(!config.is_stale_after_days(7));
        assert!(config.should_nudge_after_days(7));
        // Past stale threshold: stale, not nudged
        assert!(config.is_stale_after_days(8));
        assert!(!config.should_nudge_after_days(8));
    }

    // =============================================================================
    // validate_and_defaults tests
    // =============================================================================

    #[test]
    fn test_validate_and_defaults_valid() {
        let config = ReleaseConfig::new(
            Some("Announcements".to_string()),
            Some(vec!["release/1.x".to_string()]),
            Some(3),
            Some(10),
        );
        let result = config.validate_and_defaults();
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.voting_window_days, 3);
        assert_eq!(resolved.stale_threshold_days, 10);
    }

    #[test]
    fn test_validate_and_defaults_negative_voting_window() {
        let config = ReleaseConfig::new(None, None, Some(-1), None);
        let result = config.validate_and_defaults();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("voting_window_days"));
        assert!(errors[0].contains("non-negative"));
    }

    #[test]
    fn test_validate_and_defaults_negative_stale_threshold() {
        let config = ReleaseConfig::new(None, None, None, Some(-5));
        let result = config.validate_and_defaults();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("stale_threshold_days"));
    }

    #[test]
    fn test_validate_and_defaults_both_negative() {
        let config = ReleaseConfig::new(None, None, Some(-1), Some(-1));
        let result = config.validate_and_defaults();
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn test_validate_and_defaults_zero_days_ok() {
        // Zero is valid (non-negative)
        let config = ReleaseConfig::new(None, None, Some(0), Some(0));
        let result = config.validate_and_defaults();
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert_eq!(resolved.voting_window_days, 0);
        assert_eq!(resolved.stale_threshold_days, 0);
    }

    #[test]
    fn test_validate_and_defaults_empty_branches_warns() {
        // Empty branches should still succeed (with a warning logged)
        let config = ReleaseConfig::new(None, Some(vec![]), None, None);
        let result = config.validate_and_defaults();
        assert!(result.is_ok());
        assert!(result.unwrap().active_branches.is_empty());
    }

    #[test]
    fn test_validate_and_defaults_all_defaults_applied() {
        let config = ReleaseConfig::default();
        let resolved = config.validate_and_defaults().unwrap();
        assert_eq!(
            resolved.approval_discussion_category,
            DEFAULT_APPROVAL_DISCUSSION_CATEGORY
        );
        assert!(resolved.active_branches.is_empty());
        assert_eq!(resolved.voting_window_days, DEFAULT_VOTING_WINDOW_DAYS);
        assert_eq!(resolved.stale_threshold_days, DEFAULT_STALE_THRESHOLD_DAYS);
    }

    // =============================================================================
    // load_release_config_from_yaml tests
    // =============================================================================

    #[test]
    fn test_load_release_config_from_yaml_with_release_section() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
release:
  approval_discussion_category: "Security Advisories"
  active_branches:
    - release/1.x
    - release/2.x
  voting_window_days: 3
  stale_threshold_days: 10
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        assert_eq!(
            config.approval_discussion_category,
            Some("Security Advisories".to_string())
        );
        assert_eq!(config.active_branches.as_ref().unwrap().len(), 2);
        assert_eq!(config.voting_window_days, Some(3));
        assert_eq!(config.stale_threshold_days, Some(10));
    }

    #[test]
    fn test_load_release_config_from_yaml_no_release_section() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
llm:
  model: gpt-4
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        assert_eq!(config, ReleaseConfig::default());
    }

    #[test]
    fn test_load_release_config_from_yaml_missing_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");
        let result = load_release_config_from_yaml(&config_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No such file"));
    }

    #[test]
    fn test_load_release_config_from_yaml_invalid_yaml() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = "{ invalid: yaml: [";
        std::fs::write(&config_path, content).unwrap();

        let result = load_release_config_from_yaml(&config_path);
        assert!(result.is_err());
    }

    // =============================================================================
    // load_release_config_with_env tests
    // =============================================================================

    fn save_env_var(key: &str) -> Option<String> {
        std::env::var(key).ok()
    }

    fn restore_env_var(key: &str, original: Option<&str>) {
        match original {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
    }
}
