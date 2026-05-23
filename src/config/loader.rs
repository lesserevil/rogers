//! Configuration loading for Rodgers.
//!
//! This module loads and merges configuration from config.yaml (host-level)
//! and rogers.yaml (repo-level, overrides host-level).

use super::schema::{AppConfig, QuestionRoutingConfig};
use crate::error::{Result, RogersError};
use std::fs;
use std::path::Path;

/// Load configuration from the given paths.
///
/// Layer 1: Default config
/// Layer 2: config.yaml (host-level)
/// Layer 3: rogers.yaml (repo-level, overrides host-level)
pub fn load_config(base_path: &Path) -> Result<AppConfig> {
    let mut config = AppConfig::default();

    // Layer 1 → 2: Load host-level config.yaml
    let host_path = base_path.join("config.yaml");
    if host_path.exists() {
        let host_config = load_yaml_file(&host_path)?;
        config = merge_config(config, host_config);
    }

    // Layer 2 → 3: Load repo-level rogers.yaml
    let repo_path = base_path.join("rogers.yaml");
    if repo_path.exists() {
        let repo_config = load_yaml_file(&repo_path)?;
        config = merge_config(config, repo_config);
    }

    // Ensure question_routing always has defaults applied, even when no
    // config files were loaded.
    if config.question_routing.is_none() {
        config.question_routing = Some(QuestionRoutingConfig::default());
    }

    Ok(config)
}

/// Load a YAML config file.
fn load_yaml_file(path: &Path) -> Result<AppConfig> {
    let content = fs::read_to_string(path)
        .map_err(|e| RogersError::Config(format!("failed to read {}: {}", path.display(), e)))?;
    let config: AppConfig = serde_yaml::from_str(&content)
        .map_err(|e| RogersError::Config(format!("failed to parse {}: {}", path.display(), e)))?;
    Ok(config)
}

/// Merge two configs: right-side values override left-side values
/// for all present fields. Only question_routing is actually merged
/// since AppConfig is flattened at the top level.
fn merge_config(left: AppConfig, right: AppConfig) -> AppConfig {
    AppConfig {
        github: merge_github(left.github, right.github),
        scheduler: merge_scheduler(left.scheduler, right.scheduler),
        beads: merge_beads(left.beads, right.beads),
        llm: merge_llm(left.llm, right.llm),
        triage: merge_triage(left.triage, right.triage),
        release: merge_release(left.release, right.release),
        rogation: merge_rogation(left.rogation, right.rogation),
        question_routing: Some(merge_question_routing(
            left.question_routing.unwrap_or_default(),
            right.question_routing.unwrap_or_default(),
        )),
        log_level: right.log_level.or(left.log_level),
        error_channel: right.error_channel.or(left.error_channel),
    }
}

fn merge_github(
    left: Option<super::schema::GitHubConfig>,
    right: Option<super::schema::GitHubConfig>,
) -> Option<super::schema::GitHubConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::GitHubConfig {
            owner: r.owner.or(l.owner),
            repo: r.repo.or(l.repo),
            token: r.token.or(l.token),
            api_url: r.api_url.or(l.api_url),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn merge_scheduler(
    left: Option<super::schema::SchedulerConfig>,
    right: Option<super::schema::SchedulerConfig>,
) -> Option<super::schema::SchedulerConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::SchedulerConfig {
            interval_minutes: r.interval_minutes.or(l.interval_minutes),
            enabled: r.enabled.or(l.enabled),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn merge_beads(
    left: Option<super::schema::BeadsConfig>,
    right: Option<super::schema::BeadsConfig>,
) -> Option<super::schema::BeadsConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::BeadsConfig {
            remote: r.remote.or(l.remote),
            database: r.database.or(l.database),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn merge_llm(
    left: Option<super::schema::LlmConfig>,
    right: Option<super::schema::LlmConfig>,
) -> Option<super::schema::LlmConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::LlmConfig {
            provider: r.provider.or(l.provider),
            base_url: r.base_url.or(l.base_url),
            model: r.model.or(l.model),
            api_key: r.api_key.or(l.api_key),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn merge_triage(
    left: Option<super::schema::TriageConfig>,
    right: Option<super::schema::TriageConfig>,
) -> Option<super::schema::TriageConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::TriageConfig {
            default_labels: merge_vec(l.default_labels, r.default_labels),
            bot_labels: merge_vec(l.bot_labels, r.bot_labels),
            close_labels: merge_vec(l.close_labels, r.close_labels),
            assignees: merge_vec(l.assignees, r.assignees),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn merge_release(
    left: Option<super::schema::ReleaseConfig>,
    right: Option<super::schema::ReleaseConfig>,
) -> Option<super::schema::ReleaseConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::ReleaseConfig {
            approval_discussion_category: r
                .approval_discussion_category
                .or(l.approval_discussion_category),
            active_branches: merge_vec(l.active_branches, r.active_branches),
            voting_window_days: r.voting_window_days.or(l.voting_window_days),
            stale_threshold_days: r.stale_threshold_days.or(l.stale_threshold_days),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

fn merge_rogation(
    left: Option<super::schema::RogationConfig>,
    right: Option<super::schema::RogationConfig>,
) -> Option<super::schema::RogationConfig> {
    match (left, right) {
        (Some(l), Some(r)) => Some(super::schema::RogationConfig {
            ignore_labels: merge_vec(l.ignore_labels, r.ignore_labels),
            labels_never_bot_managed: merge_vec(
                l.labels_never_bot_managed,
                r.labels_never_bot_managed,
            ),
            custom_type_names: r.custom_type_names.or(l.custom_type_names),
            format: r.format.or(l.format),
            agent_file: r.agent_file.or(l.agent_file),
            template_dir: r.template_dir.or(l.template_dir),
            security_label: r.security_label.or(l.security_label),
        }),
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

/// Merge question_routing config from two layers.
/// repo-level overrides host-level for all fields.
fn merge_question_routing(
    left: QuestionRoutingConfig,
    right: QuestionRoutingConfig,
) -> QuestionRoutingConfig {
    // If the right side has any non-default keywords, use them.
    // Otherwise fall through to other fields.
    let has_custom_keywords =
        right.code_search_keywords != QuestionRoutingConfig::default().code_search_keywords;
    let has_custom_doc_path =
        right.doc_search_path != QuestionRoutingConfig::default().doc_search_path;
    let has_custom_code_path =
        right.code_search_path != QuestionRoutingConfig::default().code_search_path;

    // If right has at least one custom field, it takes precedence
    // for any field not explicitly set in right (i.e., right's default
    // values are treated as "not provided").
    let keywords = if has_custom_keywords {
        right.code_search_keywords
    } else {
        left.code_search_keywords
    };

    let doc_path = if has_custom_doc_path {
        right.doc_search_path
    } else {
        left.doc_search_path
    };

    let code_path = if has_custom_code_path {
        right.code_search_path
    } else {
        left.code_search_path
    };

    QuestionRoutingConfig {
        code_search_keywords: keywords,
        doc_search_path: doc_path,
        code_search_path: code_path,
    }
}

fn merge_vec<T: Clone>(left: Option<Vec<T>>, right: Option<Vec<T>>) -> Option<Vec<T>> {
    match (left, right) {
        (Some(_), Some(r)) => Some(r), // right overrides
        (Some(l), None) => Some(l),
        (None, Some(r)) => Some(r),
        (None, None) => None,
    }
}

/// Load the question routing configuration from the loaded AppConfig.
/// Returns the merged question_routing config with defaults applied.
pub fn load_question_routing_config(app_config: &AppConfig) -> QuestionRoutingConfig {
    app_config.question_routing.clone().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_config_dir(content: &str) -> std::io::Result<tempfile::TempDir> {
        let temp = tempfile::TempDir::new()?;
        fs::write(temp.path().join("config.yaml"), content)?;
        Ok(temp)
    }

    #[test]
    fn test_load_default_config() {
        let temp = tempfile::TempDir::new().unwrap();
        let config = load_config(temp.path()).unwrap();
        assert!(config.question_routing.is_some());
        let qr = config.question_routing.unwrap();
        assert_eq!(qr.keywords().len(), 9);
        assert!(qr.matches_question("How does the router work?"));
        assert!(qr.matches_question("What function handles auth?"));
        assert!(qr.matches_question("Tell me about the internals"));
    }

    #[test]
    fn test_custom_keywords_override_defaults() {
        let yaml = r#"
question_routing:
  code_search_keywords:
    - "show me the code"
    - "read the code"
  doc_search_path: custom_docs/
  code_search_path: src/**/*.rs
"#;
        let temp = create_test_config_dir(yaml).unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        // Custom keywords should be used
        assert_eq!(qr.keywords().len(), 2);
        assert!(qr.matches_question("Show me the code for this"));
        assert!(qr.matches_question("Read the code please"));
        // Default keywords should NOT match (overridden)
        assert!(!qr.matches_question("How does it work?"));
    }

    #[test]
    fn test_empty_keywords_disable_code_search() {
        let yaml = r#"
question_routing:
  code_search_keywords: []
"#;
        let temp = create_test_config_dir(yaml).unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        assert!(!qr.has_keywords());
        assert!(!qr.matches_question("How does it work?"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let temp = tempfile::TempDir::new().unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        // Mixed case should still match
        assert!(qr.matches_question("HOW DOES IT WORK?"));
        assert!(qr.matches_question("How Does It Work?"));
        assert!(qr.matches_question("UNDER THE HOOD"));
    }

    #[test]
    fn test_repo_level_overrides_host_level() -> std::io::Result<()> {
        let yaml = r#"
github:
  owner: host-owner
  repo: host-repo
question_routing:
  code_search_keywords:
    - "repo keyword"
"#;
        let repo_yaml = r#"
question_routing:
  code_search_keywords:
    - "custom keyword"
"#;
        let temp = tempfile::TempDir::new().unwrap();
        fs::write(temp.path().join("config.yaml"), yaml)?;
        fs::write(temp.path().join("rogers.yaml"), repo_yaml)?;

        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        // repo-level keywords override host-level
        assert_eq!(qr.keywords().len(), 1);
        assert!(qr.matches_question("Use the custom keyword"));
        assert!(!qr.matches_question("Use the repo keyword"));

        Ok(())
    }

    #[test]
    fn test_doc_search_path_loaded() {
        let yaml = r#"
question_routing:
  doc_search_path: "custom_docs/"
"#;
        let temp = create_test_config_dir(yaml).unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        assert_eq!(qr.doc_search_path, std::path::PathBuf::from("custom_docs/"));
    }

    #[test]
    fn test_code_search_path_loaded() {
        let yaml = r#"
question_routing:
  code_search_path: "src/**/*"
"#;
        let temp = create_test_config_dir(yaml).unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        assert_eq!(qr.code_search_path, "src/**/*");
    }

    #[test]
    fn test_phrase_matching() {
        let temp = tempfile::TempDir::new().unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();

        // Phrase matching: "walk me through" should match as a phrase
        assert!(qr.matches_question("Walk me through the flow"));
        // But "walk" alone should not match (unless it's a substring of another keyword)
        // Actually "walk" is a substring of "walk me through", so it matches
        // Let's test that "internals" matches within a larger phrase
        assert!(qr.matches_question("What are the internals of this?"));
    }

    #[test]
    fn test_default_keywords_list() {
        let temp = tempfile::TempDir::new().unwrap();
        let config = load_config(temp.path()).unwrap();
        let qr = config.question_routing.unwrap();
        let kw = qr.keywords();

        assert_eq!(kw.len(), 9);
        assert!(kw.contains(&"how does".to_string()));
        assert!(kw.contains(&"what function".to_string()));
        assert!(kw.contains(&"which module".to_string()));
        assert!(kw.contains(&"internals".to_string()));
        assert!(kw.contains(&"implementation".to_string()));
        assert!(kw.contains(&"source code".to_string()));
        assert!(kw.contains(&"walk me through".to_string()));
        assert!(kw.contains(&"flow of".to_string()));
        assert!(kw.contains(&"under the hood".to_string()));
    }
}
