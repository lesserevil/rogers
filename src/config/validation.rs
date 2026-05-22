//! Configuration validation logic for Rodgers.

use crate::config::schema::{
    apply_env_interpolation, interpolate_env_var, rodgers_required_label_names, Config,
    PLACEHOLDER_TOKEN_PATTERNS,
};
use rogers_core::error::{Result, RogersError};
use std::path::Path;

/// Validation result containing any warnings.
#[derive(Debug, Default)]
pub struct ValidationResult {
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

/// Validate configuration, returning descriptive errors on failure.
pub fn validate_config(config: &Config) -> Result<ValidationResult> {
    let mut result = ValidationResult::new();

    // Validate GitHub config
    validate_github_config(&config.github)?;

    // Validate scheduler config
    validate_scheduler_config(&config.scheduler)?;

    // Validate beads config
    validate_beads_config(&config.beads)?;

    // Validate LLM config
    validate_llm_config(&config.llm, &mut result)?;

    // Validate triage config (if present)
    if let Some(triage) = &config.triage {
        validate_triage_config(triage, &mut result)?;
    }

    // Validate release config (if present)
    if let Some(release) = &config.release {
        validate_release_config(release, &mut result)?;
    }

    // Validate rogation config (if present)
    if let Some(rogation) = &config.rogation {
        validate_rogation_config(rogation, &mut result)?;
    }

    // Validate log_level
    if let Some(level) = &config.log_level {
        validate_log_level(level, &mut result)?;
    }

    Ok(result)
}

/// Validate GitHub configuration.
fn validate_github_config(github: &crate::config::schema::GitHubConfig) -> Result<()> {
    // Check owner
    if github.owner.trim().is_empty() {
        return Err(RogersError::Config(
            "github.owner: required key missing or empty. Set the GitHub organization or username that owns the repository.".to_string(),
        ));
    }

    // Check repo
    if github.repo.trim().is_empty() {
        return Err(RogersError::Config(
            "github.repo: required key missing or empty. Set the GitHub repository name."
                .to_string(),
        ));
    }

    // Check token
    if github.token.trim().is_empty() {
        return Err(RogersError::Config(
            "github.token: required key missing or empty. Set a GitHub personal access token (classic or fine-grained) with repo scope. Use ${RODGERS_GITHUB_TOKEN} for env var injection.".to_string(),
        ));
    }

    // Check for placeholder tokens
    if is_placeholder_token(&github.token) {
        return Err(RogersError::Config(
            "github.token: appears to be a placeholder value. Replace with a real GitHub personal access token. Use ${RODGERS_GITHUB_TOKEN} for env var injection.".to_string(),
        ));
    }

    Ok(())
}

/// Validate scheduler configuration.
fn validate_scheduler_config(scheduler: &crate::config::schema::SchedulerConfig) -> Result<()> {
    if scheduler.interval_minutes == 0 {
        return Err(RogersError::Config(
            "scheduler.interval_minutes: must be a positive integer (minimum 1). Current value: 0. Set to at least 1 minute.".to_string(),
        ));
    }

    // Warn if interval is very small (though with u32, only 0 is checked above for error)
    // Note: In practice scheduler.interval_minutes < 1 only means 0 for u32
    // This warning exists for future if u32 becomes i32 or for documentation
    if scheduler.interval_minutes < 5 {
        // This is informational, not an error - can remove if too noisy
    }

    Ok(())
}

/// Validate beads configuration.
fn validate_beads_config(beads: &crate::config::schema::BeadsConfig) -> Result<()> {
    if beads.remote.trim().is_empty() {
        return Err(RogersError::Config(
            "beads.remote: required key missing or empty. Set the Dolt remote URL for bead storage (e.g., 'doltremote://user@host/db' or 'ssh://user@host/path'). Run 'dolt remote add origin <url>' first.".to_string(),
        ));
    }

    Ok(())
}

/// Validate LLM configuration.
fn validate_llm_config(
    llm: &crate::config::schema::LlmConfig,
    result: &mut ValidationResult,
) -> Result<()> {
    // Check model
    if llm.model.trim().is_empty() {
        return Err(RogersError::Config(
            "llm.model: required key missing or empty. Set the model name (e.g., 'gpt-4o', 'gpt-4o-mini', 'claude-3-opus').".to_string(),
        ));
    }

    // Check api_key
    if llm.api_key.trim().is_empty() {
        return Err(RogersError::Config(
            "llm.api_key: required key missing or empty. Set the API key for your LLM provider. Use ${OPENAI_API_KEY} for env var injection.".to_string(),
        ));
    }

    // Check for placeholder tokens in api_key
    if is_placeholder_token(&llm.api_key) {
        result.add_warning(
            "llm.api_key: appears to be a placeholder value. Replace with a real API key. Use ${OPENAI_API_KEY} for env var injection.".to_string(),
        );
    }

    // Validate base_url format if present
    if let Some(base_url) = &llm.base_url {
        if !base_url.trim().is_empty() && !is_valid_url(base_url) {
            result.add_warning(format!(
                "llm.base_url: '{}' may not be a valid URL. Expected format: https://api.example.com/v1",
                base_url
            ));
        }
    }

    Ok(())
}

/// Validate triage configuration.
fn validate_triage_config(
    triage: &crate::config::schema::TriageConfig,
    result: &mut ValidationResult,
) -> Result<()> {
    // Validate default_labels
    if let Some(labels) = &triage.default_labels {
        for label in labels {
            if label.trim().is_empty() {
                result.add_warning("triage.default_labels: contains empty label name".to_string());
            }
        }
    }

    // Validate bot_labels
    if let Some(labels) = &triage.bot_labels {
        for label in labels {
            if label.trim().is_empty() {
                result.add_warning("triage.bot_labels: contains empty label name".to_string());
            }
        }
    }

    // Validate close_labels
    if let Some(labels) = &triage.close_labels {
        for label in labels {
            if label.trim().is_empty() {
                result.add_warning("triage.close_labels: contains empty label name".to_string());
            }
        }
    }

    Ok(())
}

/// Validate release configuration.
fn validate_release_config(
    release: &crate::config::schema::ReleaseConfig,
    result: &mut ValidationResult,
) -> Result<()> {
    // Check active_branches if releases are configured
    if let Some(branches) = &release.active_branches {
        if branches.is_empty() {
            result.add_warning(
                "release.active_branches: empty list. Rodgers will not evaluate backports for any release branches. Add branch names like ['release/1.x', 'release/2.x'].".to_string(),
            );
        } else {
            for branch in branches {
                if branch.trim().is_empty() {
                    result.add_warning(
                        "release.active_branches: contains empty branch name".to_string(),
                    );
                }
            }
        }
    }

    // Validate voting_window_days
    if let Some(days) = release.voting_window_days {
        if days == 0 {
            result.add_warning("release.voting_window_days: 0 means no voting window. Consider setting to at least 1.".to_string());
        }
    }

    // Validate stale_threshold_days
    if let Some(days) = release.stale_threshold_days {
        if days == 0 {
            result.add_warning("release.stale_threshold_days: 0 means proposals never go stale. Consider setting to at least 1.".to_string());
        }
    }

    Ok(())
}

/// Validate rogation configuration.
fn validate_rogation_config(
    rogation: &crate::config::schema::RogationConfig,
    result: &mut ValidationResult,
) -> Result<()> {
    // Check labels_never_bot_managed for Rodgers-required labels
    if let Some(labels) = &rogation.labels_never_bot_managed {
        let required_labels = rodgers_required_label_names();
        for label in labels {
            let label_lower = label.to_lowercase();
            if required_labels.contains(&label_lower.as_str()) {
                result.add_warning(format!(
                    "rogation.labels_never_bot_managed: contains '{}' which is a Rodgers-required label. Rodgers manages this label automatically; excluding it may break workflow.",
                    label
                ));
            }
            if label.trim().is_empty() {
                result.add_warning(
                    "rogation.labels_never_bot_managed: contains empty label name".to_string(),
                );
            }
        }
    }

    // Validate ignore_labels
    if let Some(labels) = &rogation.ignore_labels {
        for label in labels {
            if label.trim().is_empty() {
                result.add_warning("rogation.ignore_labels: contains empty label name".to_string());
            }
        }
    }

    // Validate security_label
    if let Some(label) = &rogation.security_label {
        if label.trim().is_empty() {
            result.add_warning(
                "rogation.security_label: empty string. Using default 'security'.".to_string(),
            );
        }
    }

    Ok(())
}

/// Validate log level.
fn validate_log_level(level: &str, result: &mut ValidationResult) -> Result<()> {
    let valid_levels = ["debug", "info", "warn", "error", "trace"];
    let level_lower = level.to_lowercase();
    if !valid_levels.contains(&level_lower.as_str()) {
        result.add_warning(format!(
            "log_level: '{}' is not a standard level. Valid levels: {}. Using 'info'.",
            level,
            valid_levels.join(", ")
        ));
    }
    Ok(())
}

/// Check if a token value appears to be a placeholder.
fn is_placeholder_token(token: &str) -> bool {
    let token_upper = token.to_uppercase();
    for pattern in PLACEHOLDER_TOKEN_PATTERNS {
        if token_upper.contains(&pattern.to_uppercase()) {
            return true;
        }
    }
    false
}

/// Simple URL validation.
fn is_valid_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Load and validate configuration from a YAML file.
/// Applies environment variable interpolation to config values before validation.
pub fn load_and_validate_config(path: &Path) -> Result<(Config, ValidationResult)> {
    // Check if file exists
    if !path.exists() {
        return Err(RogersError::Config(format!(
            "config.yaml not found at '{}'. Copy config.example.yaml to config.yaml and fill in your values.",
            path.display()
        )));
    }

    // Read file
    let content = std::fs::read_to_string(path).map_err(|e| {
        RogersError::Config(format!(
            "failed to read config.yaml at '{}': {}",
            path.display(),
            e
        ))
    })?;

    // Parse YAML
    let mut config: Config = serde_yaml::from_str(&content).map_err(|e| {
        // Provide descriptive error with location
        let location = match e.location() {
            Some(loc) => format!("line {}, column {}", loc.line(), loc.column()),
            None => "unknown location".to_string(),
        };
        RogersError::Config(format!("config.yaml: invalid YAML at {}. {}", location, e))
    })?;

    // Apply environment variable interpolation BEFORE validation
    apply_env_interpolation(&mut config);

    // Validate
    let result = validate_config(&config)?;

    Ok((config, result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::*;

    fn valid_config() -> Config {
        Config {
            github: GitHubConfig {
                owner: "test-owner".to_string(),
                repo: "test-repo".to_string(),
                token: "ghp_real_token_1234567890abcdef".to_string(),
                api_url: Some("https://api.github.com".to_string()),
            },
            scheduler: SchedulerConfig {
                interval_minutes: 5,
                enabled: Some(true),
            },
            beads: BeadsConfig {
                remote: "doltremote://user@host/db".to_string(),
                database: Some("message.hibernate".to_string()),
            },
            llm: LlmConfig {
                provider: Some("openai".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model: "gpt-4o-mini".to_string(),
                api_key: "sk-real-key-1234567890abcdef".to_string(),
            },
            triage: Some(TriageConfig::default()),
            release: Some(ReleaseConfig::default()),
            rogation: Some(RogationConfig::default()),
            log_level: Some("info".to_string()),
            error_channel: None,
        }
    }

    #[test]
    fn test_valid_config_passes() {
        let config = valid_config();
        let result = validate_config(&config);
        assert!(
            result.is_ok(),
            "Valid config should pass: {:?}",
            result.err()
        );
    }

    #[test]
    fn test_missing_github_owner_fails() {
        let mut config = valid_config();
        config.github.owner = "".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("github.owner"));
        assert!(err.contains("required"));
    }

    #[test]
    fn test_missing_github_repo_fails() {
        let mut config = valid_config();
        config.github.repo = "".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("github.repo"));
        assert!(err.contains("required"));
    }

    #[test]
    fn test_missing_github_token_fails() {
        let mut config = valid_config();
        config.github.token = "".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("github.token"));
        assert!(err.contains("required"));
    }

    #[test]
    fn test_placeholder_github_token_fails() {
        let mut config = valid_config();
        config.github.token = "YOUR_TOKEN".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("github.token"));
        assert!(err.contains("placeholder"));
    }

    #[test]
    fn test_placeholder_github_token_ghp_sample_fails() {
        let mut config = valid_config();
        config.github.token = "ghp_sample".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("placeholder"));
    }

    #[test]
    fn test_scheduler_interval_zero_fails() {
        let mut config = valid_config();
        config.scheduler.interval_minutes = 0;
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("scheduler.interval_minutes"));
        assert!(err.contains("positive"));
    }

    #[test]
    fn test_missing_beads_remote_fails() {
        let mut config = valid_config();
        config.beads.remote = "".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("beads.remote"));
        assert!(err.contains("required"));
    }

    #[test]
    fn test_missing_llm_model_fails() {
        let mut config = valid_config();
        config.llm.model = "".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("llm.model"));
        assert!(err.contains("required"));
    }

    #[test]
    fn test_missing_llm_api_key_fails() {
        let mut config = valid_config();
        config.llm.api_key = "".to_string();
        let result = validate_config(&config);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("llm.api_key"));
        assert!(err.contains("required"));
    }

    #[test]
    fn test_placeholder_llm_api_key_warning() {
        let mut config = valid_config();
        config.llm.api_key = "YOUR_TOKEN".to_string();
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("llm.api_key") && w.contains("placeholder")));
    }

    #[test]
    fn test_empty_release_active_branches_warning() {
        let mut config = valid_config();
        config.release = Some(ReleaseConfig {
            active_branches: Some(vec![]),
            ..ReleaseConfig::default()
        });
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("release.active_branches") && w.contains("empty")));
    }

    #[test]
    fn test_labels_never_bot_managed_with_required_label_warning() {
        let mut config = valid_config();
        config.rogation = Some(RogationConfig {
            labels_never_bot_managed: Some(vec!["needs-information".to_string()]),
            ..RogationConfig::default()
        });
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("labels_never_bot_managed") && w.contains("needs-information")));
    }

    #[test]
    fn test_labels_never_bot_managed_with_multiple_required_labels_warning() {
        let mut config = valid_config();
        config.rogation = Some(RogationConfig {
            labels_never_bot_managed: Some(vec![
                "bug".to_string(),
                "feature".to_string(),
                "ready-for-review".to_string(),
            ]),
            ..RogationConfig::default()
        });
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        // Should have warnings for each required label
        let warning_count = result
            .warnings
            .iter()
            .filter(|w| w.contains("labels_never_bot_managed"))
            .count();
        assert_eq!(warning_count, 3);
    }

    #[test]
    fn test_invalid_yaml_gives_descriptive_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        std::fs::write(&config_path, "invalid: yaml: [unclosed").unwrap();

        let result = load_and_validate_config(&config_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("invalid YAML"));
        assert!(err.contains("line"));
    }

    #[test]
    fn test_missing_config_file_gives_descriptive_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("nonexistent.yaml");

        let result = load_and_validate_config(&config_path);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("config.yaml not found"));
        assert!(err.contains("config.example.yaml"));
    }

    #[test]
    fn test_invalid_log_level_warning() {
        let mut config = valid_config();
        config.log_level = Some("verbose".to_string());
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("log_level") && w.contains("verbose")));
    }

    #[test]
    fn test_empty_label_in_default_labels_warning() {
        let mut config = valid_config();
        config.triage = Some(TriageConfig {
            default_labels: Some(vec![
                "bug".to_string(),
                "".to_string(),
                "feature".to_string(),
            ]),
            ..TriageConfig::default()
        });
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("default_labels") && w.contains("empty")));
    }

    #[test]
    fn test_empty_branch_in_active_branches_warning() {
        let mut config = valid_config();
        config.release = Some(ReleaseConfig {
            active_branches: Some(vec!["release/1.x".to_string(), "".to_string()]),
            ..ReleaseConfig::default()
        });
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("active_branches") && w.contains("empty")));
    }

    #[test]
    fn test_invalid_base_url_warning() {
        let mut config = valid_config();
        config.llm.base_url = Some("not-a-url".to_string());
        let result = validate_config(&config);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert!(result.has_warnings());
        assert!(result
            .warnings
            .iter()
            .any(|w| w.contains("base_url") && w.contains("not-a-url")));
    }

    // === Schema tests ===

    #[test]
    fn test_config_has_all_required_keys() {
        // Verify all top-level required structs exist
        let config = valid_config();
        assert!(config.github.owner.len() > 0);
        assert!(config.github.repo.len() > 0);
        assert!(config.github.token.len() > 0);
        assert!(config.beads.remote.len() > 0);
        assert!(config.llm.model.len() > 0);
    }

    #[test]
    fn test_scheduler_type_validation() {
        // scheduler.interval_minutes is correct type (u32)
        let config = valid_config();
        assert!(config.scheduler.interval_minutes >= 1);

        // Verify we can set values beyond int16 range to confirm u32
        let mut config = valid_config();
        config.scheduler.interval_minutes = 1000000;
        assert_eq!(config.scheduler.interval_minutes, 1000000);
    }

    #[test]
    fn test_release_config_types() {
        let config = ReleaseConfig::default();
        // approval_discussion_category is String
        assert!(config.approval_discussion_category.is_some());
        // active_branches is Vec<String>
        assert!(config.active_branches.is_some());
        // voting_window_days is u32
        assert!(config.voting_window_days.unwrap() >= 1);
        // stale_threshold_days is u32
        assert!(config.stale_threshold_days.unwrap() >= 1);
    }

    #[test]
    fn test_triage_config_types() {
        let config = TriageConfig::default();
        // All lists are Vec<String>
        assert!(config.default_labels.is_some());
        assert!(config.bot_labels.is_some());
        assert!(config.close_labels.is_some());
        assert!(config.assignees.is_some());
    }

    #[test]
    fn test_rogation_config_types() {
        let config = RogationConfig::default();
        // custom_type_names is HashMap<String, String>
        assert!(config.custom_type_names.is_some());
        assert!(config.custom_type_names.unwrap().is_empty());
    }

    // === Defaults applied tests ===

    #[test]
    fn test_scheduler_defaults() {
        let default = SchedulerConfig::default();
        assert_eq!(default.interval_minutes, 15);
        assert_eq!(default.enabled, Some(true));
    }

    #[test]
    fn test_llm_defaults() {
        let default = LlmConfig::default();
        assert_eq!(default.provider, Some("openai".to_string()));
        assert_eq!(
            default.base_url,
            Some("https://api.openai.com/v1".to_string())
        );
    }

    #[test]
    fn test_triage_defaults() {
        let default = TriageConfig::default();
        assert_eq!(
            default.default_labels,
            Some(vec![
                "bug".to_string(),
                "enhancement".to_string(),
                "question".to_string()
            ])
        );
        assert_eq!(
            default.close_labels,
            Some(vec![
                "wontfix".to_string(),
                "duplicate".to_string(),
                "not planned".to_string()
            ])
        );
    }

    #[test]
    fn test_rogation_defaults() {
        let default = RogationConfig::default();
        assert_eq!(default.security_label, Some("security".to_string()));
    }

    #[test]
    fn test_beads_defaults() {
        let default = BeadsConfig::default();
        assert_eq!(default.database, Some("message.hibernate".to_string()));
    }

    // === Env var interpolation tests ===

    #[test]
    fn test_interpolate_env_var_simple() {
        // Test when env var is set - use unsafe block for env var manipulation
        unsafe {
            std::env::set_var("RODGERS_TEST_VAR", "secret123");
        }
        let result = interpolate_env_var("prefix_${RODGERS_TEST_VAR}_suffix");
        assert_eq!(result, "prefix_secret123_suffix");
        unsafe {
            std::env::remove_var("RODGERS_TEST_VAR");
        }
    }

    #[test]
    fn test_interpolate_env_var_not_set() {
        // Test when env var is not set - placeholder remains
        let result = interpolate_env_var("prefix_${NONEXISTENT_VAR_12345}_suffix");
        assert_eq!(result, "prefix_${NONEXISTENT_VAR_12345}_suffix");
    }

    #[test]
    fn test_interpolate_env_var_multiple() {
        // Test with multiple env vars
        unsafe {
            std::env::set_var("VAR1", "a");
            std::env::set_var("VAR2", "b");
        }
        let result = interpolate_env_var("${VAR1}-${VAR2}");
        assert_eq!(result, "a-b");
        unsafe {
            std::env::remove_var("VAR1");
            std::env::remove_var("VAR2");
        }
    }

    #[test]
    fn test_interpolate_env_var_no_placeholder() {
        // Test string without placeholders
        let result = interpolate_env_var("plain_string");
        assert_eq!(result, "plain_string");
    }

    #[test]
    fn test_interpolate_env_var_empty_string() {
        let result = interpolate_env_var("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_interpolate_env_var_preserves_non_dollar() {
        // Ensure $ without ${} is preserved
        let result = interpolate_env_var("price: $100");
        assert_eq!(result, "price: $100");
    }

    #[test]
    fn test_apply_env_interpolation_to_config() {
        use crate::config::schema::apply_env_interpolation;

        unsafe {
            std::env::set_var("RODGERS_GITHUB_TOKEN", "real_github_token");
            std::env::set_var("OPENAI_API_KEY", "real_openai_key");
        }

        let mut config = valid_config();
        let original_token = format!("${{{}}}", "RODGERS_GITHUB_TOKEN");
        config.github.token = original_token.clone();
        let original_api_key = format!("${{{}}}", "OPENAI_API_KEY");
        config.llm.api_key = original_api_key.clone();

        apply_env_interpolation(&mut config);

        assert_eq!(config.github.token, "real_github_token");
        assert_eq!(config.llm.api_key, "real_openai_key");

        unsafe {
            std::env::remove_var("RODGERS_GITHUB_TOKEN");
            std::env::remove_var("OPENAI_API_KEY");
        }
    }

    // === Integration tests ===

    #[test]
    fn test_load_config_from_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
github:
  owner: myorg
  repo: myrepo
  token: "${MY_TEST_TOKEN}"
  api_url: https://api.github.com
scheduler:
  interval_minutes: 10
  enabled: true
beads:
  remote: doltremote://localhost/myorg/rogers
  database: test.hibernate
llm:
  provider: openai
  base_url: https://api.openai.com/v1
  model: gpt-4o-mini
  api_key: "${MY_TEST_API_KEY}"
triage:
  default_labels:
    - bug
    - feature
  bot_labels: []
  close_labels:
    - wontfix
  assignees:
    - octocat
release:
  approval_discussion_category: Announcements
  active_branches:
    - release/1.0
    - release/2.0
rogation:
  security_label: security-policy
"#;

        std::fs::write(&config_path, yaml_content).unwrap();
        unsafe {
            std::env::set_var("MY_TEST_TOKEN", "token_from_env");
            std::env::set_var("MY_TEST_API_KEY", "api_key_from_env");
        }

        let result = load_and_validate_config(&config_path);
        assert!(result.is_ok(), "Failed to load config: {:?}", result.err());

        let (config, validation_result) = result.unwrap();

        // Verify env interpolation worked
        assert_eq!(config.github.token, "token_from_env");
        assert_eq!(config.llm.api_key, "api_key_from_env");

        // Verify schema parsed correctly
        assert_eq!(config.github.owner, "myorg");
        assert_eq!(config.github.repo, "myrepo");
        assert_eq!(config.scheduler.interval_minutes, 10);
        assert_eq!(config.beads.remote, "doltremote://localhost/myorg/rogers");
        assert_eq!(config.llm.model, "gpt-4o-mini");

        // Verify structure types
        assert_eq!(
            config
                .triage
                .as_ref()
                .unwrap()
                .default_labels
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            config
                .release
                .as_ref()
                .unwrap()
                .active_branches
                .as_ref()
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            config
                .rogation
                .as_ref()
                .unwrap()
                .security_label
                .as_ref()
                .unwrap(),
            "security-policy"
        );

        // Check no required field errors
        assert!(
            !validation_result.has_warnings(),
            "Unexpected warnings: {:?}",
            validation_result.warnings
        );

        unsafe {
            std::env::remove_var("MY_TEST_TOKEN");
            std::env::remove_var("MY_TEST_API_KEY");
        }
    }

    #[test]
    fn test_load_and_validate_rejects_missing_required() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Missing github.token
        let yaml_content = r#"
github:
  owner: myorg
  repo: myrepo
  token: ""
scheduler:
  interval_minutes: 5
beads:
  remote: doltremote://localhost/db
llm:
  model: gpt-4o-mini
  api_key: ""
"#;

        std::fs::write(&config_path, yaml_content).unwrap();
        let result = load_and_validate_config(&config_path);
        assert!(result.is_err(), "Should reject missing tokens");
        let err = result.unwrap_err().to_string();
        assert!(err.contains("token") || err.contains("required"));
    }

    // === Unknown keys tests ===

    #[test]
    fn test_unknown_top_level_keys_behavior() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        let yaml_content = r#"
github:
  owner: myorg
  repo: myrepo
  token: "${MY_TEST_TOKEN}"
unknown_key:
  foo: bar
scheduler:
  interval_minutes: 5
beads:
  remote: doltremote://localhost/db
llm:
  model: gpt-4o-mini
  api_key: "${MY_TEST_API_KEY}"
"#;

        std::fs::write(&config_path, yaml_content).unwrap();
        unsafe {
            std::env::set_var("MY_TEST_TOKEN", "token");
            std::env::set_var("MY_TEST_API_KEY", "key");
        }

        // Note: serde_yaml silently ignores unknown keys that don't match the struct.
        // This test documents current behavior - unknown keys are silently ignored.
        // Required fields are validated even with unknown keys present.
        let result = load_and_validate_config(&config_path);
        assert!(
            result.is_ok(),
            "Config with unknown keys should still load: {:?}",
            result.err()
        );

        unsafe {
            std::env::remove_var("MY_TEST_TOKEN");
            std::env::remove_var("MY_TEST_API_KEY");
        }
    }
}
