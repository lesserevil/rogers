//! Release configuration schema.
//!
//! Defines the structure and defaults for release-related configuration keys.
//! These are loaded from `config.yaml` and can be overridden by environment
//! variables (e.g., `ROGERS_RELEASE_APPROVAL_DISCUSSION_CATEGORY`).
//!
//! ## Configuration keys
//!
//! - `release.approval_discussion_category` — GitHub Discussion category for proposals (default: "Announcements")
//! - `release.active_branches` — List of active release branches for backports (default: [])
//! - `release.voting_window_days` — Days before nudging stale proposal (default: 2)
//! - `release.stale_threshold_days` — Days before closing stale proposal (default: 7)
//!
//! ## Validation rules
//!
//! - `active_branches` may be empty — warns but continues (backport manager inactive)
//! - `voting_window_days` must be non-negative — validation error if negative
//! - `stale_threshold_days` must be non-negative — validation error if negative
//! - `approval_discussion_category` defaults to "Announcements" if empty or invalid

use serde::{Deserialize, Serialize};
use std::env;

/// Default values for release configuration.
pub const DEFAULT_APPROVAL_DISCUSSION_CATEGORY: &str = "Announcements";
pub const DEFAULT_VOTING_WINDOW_DAYS: i32 = 2;
pub const DEFAULT_STALE_THRESHOLD_DAYS: i32 = 7;

/// Release configuration loaded from config.yaml.
///
/// All fields are optional because the YAML may omit them entirely.
/// Default values are applied via `ReleaseConfig::apply_defaults()` or `ReleaseConfig::validate_and_defaults()`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReleaseConfig {
    /// GitHub Discussion category for release/backport approval proposals.
    #[serde(default)]
    pub approval_discussion_category: Option<String>,

    /// Branches receiving backports (maintenance releases).
    /// If empty, the backport manager is inactive.
    #[serde(default)]
    pub active_branches: Option<Vec<String>>,

    /// Time in days before a reminder ping is sent for stale proposals.
    #[serde(default)]
    pub voting_window_days: Option<i32>,

    /// Time in days before a proposal is closed as stale.
    #[serde(default)]
    pub stale_threshold_days: Option<i32>,
}

impl ReleaseConfig {
    /// Create a new ReleaseConfig with explicit values (no defaults applied).
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
    ///
    /// This does NOT perform validation — use `validate_and_defaults()` for
    /// combined validation + default application.
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
    ///
    /// Returns a resolved config on success, or a vector of validation error
    /// messages if any field fails validation.
    ///
    /// Validation rules:
    /// - `voting_window_days` must be >= 0
    /// - `stale_threshold_days` must be >= 0
    /// - Empty `active_branches` generates a warning (not an error)
    pub fn validate_and_defaults(&self) -> Result<ResolvedReleaseConfig, Vec<String>> {
        let mut errors = Vec::new();

        // Validate voting_window_days
        if let Some(days) = self.voting_window_days {
            if days < 0 {
                errors.push(format!(
                    "release.voting_window_days must be non-negative, got {}",
                    days
                ));
            }
        }

        // Validate stale_threshold_days
        if let Some(days) = self.stale_threshold_days {
            if days < 0 {
                errors.push(format!(
                    "release.stale_threshold_days must be non-negative, got {}",
                    days
                ));
            }
        }

        if !errors.is_empty() {
            return Err(errors);
        }

        let resolved = self.apply_defaults();

        // Warn about empty active_branches (not an error, just a warning)
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
///
/// Every field is guaranteed to have a valid value.
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

    /// Check if a proposal is stale (has not been voted on within the voting window).
    pub fn is_stale_after_days(&self, days_since_creation: i32) -> bool {
        days_since_creation > self.stale_threshold_days
    }

    /// Check if a proposal should be nudged (past voting window but not yet stale).
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

/// Load release configuration with environment variable overrides.
///
/// Environment variable overrides follow the pattern `ROGERS_RELEASE_<FIELD>`.
/// For example:
/// - `ROGERS_RELEASE_ACTIVE_BRANCHES` — comma-separated list of branches
/// - `ROGERS_RELEASE_VOTING_WINDOW_DAYS` — integer days
/// - `ROGERS_RELEASE_STALE_THRESHOLD_DAYS` — integer days
/// - `ROGERS_RELEASE_APPROVAL_DISCUSSION_CATEGORY` — category name
///
/// The YAML config takes precedence; env vars only override if set.
pub fn load_release_config_with_env(path: &std::path::Path) -> Result<ReleaseConfig, String> {
    let mut config = load_release_config_from_yaml(path)?;

    // Apply env var overrides for fields not already set
    if config.approval_discussion_category.is_none() {
        if let Ok(val) = env::var("ROGERS_RELEASE_APPROVAL_DISCUSSION_CATEGORY") {
            config.approval_discussion_category = Some(val);
        }
    }

    if config.active_branches.is_none() {
        if let Ok(val) = env::var("ROGERS_RELEASE_ACTIVE_BRANCHES") {
            let branches: Vec<String> = val
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !branches.is_empty() {
                config.active_branches = Some(branches);
            }
        }
    }

    if config.voting_window_days.is_none() {
        if let Ok(val) = env::var("ROGERS_RELEASE_VOTING_WINDOW_DAYS") {
            if let Ok(days) = val.parse::<i32>() {
                config.voting_window_days = Some(days);
            }
        }
    }

    if config.stale_threshold_days.is_none() {
        if let Ok(val) = env::var("ROGERS_RELEASE_STALE_THRESHOLD_DAYS") {
            if let Ok(days) = val.parse::<i32>() {
                config.stale_threshold_days = Some(days);
            }
        }
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize env var access to avoid race conditions in parallel tests
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // Helper to safely set env vars in tests
    fn set_env_var(key: &str, value: &str) {
        unsafe { std::env::set_var(key, value) };
    }

    fn remove_env_var(key: &str) {
        unsafe { std::env::remove_var(key) };
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

    #[test]
    fn test_load_release_config_env_override() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Config has no release section
        let content = r#"
github:
  owner: test
  repo: test
"#;
        std::fs::write(&config_path, content).unwrap();

        // Save original env var values
        let orig_voting = save_env_var("ROGERS_RELEASE_VOTING_WINDOW_DAYS");
        let orig_stale = save_env_var("ROGERS_RELEASE_STALE_THRESHOLD_DAYS");
        let orig_branches = save_env_var("ROGERS_RELEASE_ACTIVE_BRANCHES");
        let orig_category = save_env_var("ROGERS_RELEASE_APPROVAL_DISCUSSION_CATEGORY");

        // Set env vars
        set_env_var("ROGERS_RELEASE_VOTING_WINDOW_DAYS", "5");
        set_env_var("ROGERS_RELEASE_STALE_THRESHOLD_DAYS", "14");
        set_env_var("ROGERS_RELEASE_ACTIVE_BRANCHES", "release/1.x,release/2.x");
        set_env_var("ROGERS_RELEASE_APPROVAL_DISCUSSION_CATEGORY", "Security");

        let config = load_release_config_with_env(&config_path).unwrap();
        assert_eq!(
            config.approval_discussion_category,
            Some("Security".to_string())
        );
        assert_eq!(
            config.active_branches,
            Some(vec!["release/1.x".to_string(), "release/2.x".to_string()])
        );
        assert_eq!(config.voting_window_days, Some(5));
        assert_eq!(config.stale_threshold_days, Some(14));

        // Restore original env var values
        restore_env_var("ROGERS_RELEASE_VOTING_WINDOW_DAYS", orig_voting.as_deref());
        restore_env_var("ROGERS_RELEASE_STALE_THRESHOLD_DAYS", orig_stale.as_deref());
        restore_env_var("ROGERS_RELEASE_ACTIVE_BRANCHES", orig_branches.as_deref());
        restore_env_var(
            "ROGERS_RELEASE_APPROVAL_DISCUSSION_CATEGORY",
            orig_category.as_deref(),
        );
    }

    #[test]
    fn test_load_release_config_yaml_takes_precedence_over_env() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Config has voting_window_days = 3
        let content = r#"
github:
  owner: test
  repo: test
release:
  voting_window_days: 3
"#;
        std::fs::write(&config_path, content).unwrap();

        // Save original
        let orig_voting = save_env_var("ROGERS_RELEASE_VOTING_WINDOW_DAYS");
        // Env var tries to override to 5
        set_env_var("ROGERS_RELEASE_VOTING_WINDOW_DAYS", "5");

        let config = load_release_config_with_env(&config_path).unwrap();
        // YAML value takes precedence
        assert_eq!(config.voting_window_days, Some(3));

        // Restore original
        restore_env_var("ROGERS_RELEASE_VOTING_WINDOW_DAYS", orig_voting.as_deref());
    }

    #[test]
    fn test_load_release_config_env_branches_whitespace() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
"#;
        std::fs::write(&config_path, content).unwrap();

        // Save original
        let orig_branches = save_env_var("ROGERS_RELEASE_ACTIVE_BRANCHES");

        set_env_var(
            "ROGERS_RELEASE_ACTIVE_BRANCHES",
            " release/1.x , release/2.x , ",
        );

        let config = load_release_config_with_env(&config_path).unwrap();
        assert_eq!(
            config.active_branches,
            Some(vec!["release/1.x".to_string(), "release/2.x".to_string()])
        );

        // Restore original
        restore_env_var("ROGERS_RELEASE_ACTIVE_BRANCHES", orig_branches.as_deref());
    }

    // =============================================================================
    // active_branches parsing tests
    // =============================================================================

    #[test]
    fn test_active_branches_parsed_as_list() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
release:
  active_branches:
    - release/1.x
    - release/2.x
    - release/3.x
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        let branches = config.active_branches.unwrap();
        assert_eq!(branches.len(), 3);
        assert_eq!(branches[0], "release/1.x");
        assert_eq!(branches[1], "release/2.x");
        assert_eq!(branches[2], "release/3.x");
    }

    #[test]
    fn test_voting_window_days_parsed_as_integer() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
release:
  voting_window_days: 2
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        assert_eq!(config.voting_window_days, Some(2));
        assert!(matches!(config.voting_window_days, Some(2i32)));
    }

    #[test]
    fn test_stale_threshold_days_parsed_as_integer() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
release:
  stale_threshold_days: 7
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        assert_eq!(config.stale_threshold_days, Some(7));
        assert!(matches!(config.stale_threshold_days, Some(7i32)));
    }

    // =============================================================================
    // Integration-style: release manager uses config values
    // =============================================================================

    #[test]
    fn test_release_manager_uses_config_values() {
        // Simulates what the release manager does with the config
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
release:
  approval_discussion_category: "Release Proposals"
  active_branches:
    - release/1.x
    - release/2.x
  voting_window_days: 3
  stale_threshold_days: 14
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        let resolved = config.validate_and_defaults().unwrap();

        // Release manager checks which branches are active
        assert!(resolved.is_active_branch("release/1.x"));
        assert!(resolved.is_active_branch("release/2.x"));
        assert!(!resolved.is_active_branch("main"));

        // Release manager checks stale proposals
        assert!(!resolved.is_stale_after_days(5));
        assert!(!resolved.is_stale_after_days(10));
        assert!(!resolved.is_stale_after_days(14));
        assert!(resolved.is_stale_after_days(15));

        // Release manager nudges stale proposals
        assert!(resolved.should_nudge_after_days(4));
        assert!(resolved.should_nudge_after_days(10));
        assert!(!resolved.should_nudge_after_days(15)); // stale, not nudged

        // Release manager checks if backport is active
        assert!(resolved.is_backport_active());
    }

    #[test]
    fn test_release_manager_empty_branches_inactive() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let content = r#"
github:
  owner: test
  repo: test
release:
  approval_discussion_category: "Announcements"
  active_branches: []
  voting_window_days: 2
  stale_threshold_days: 7
"#;
        std::fs::write(&config_path, content).unwrap();

        let config = load_release_config_from_yaml(&config_path).unwrap();
        let resolved = config.validate_and_defaults().unwrap();

        assert!(!resolved.is_backport_active());
        assert_eq!(resolved.active_branches().len(), 0);
    }
}
