//! Init module - repository initialization and template discovery.
//!
//! This module handles:
//! - Repository initialization checks
//! - Template discovery and validation
//! - Bead filing when templates are missing and auto_suggest=true

use crate::templates::{TEMPLATE_BEAD_TITLE, TEMPLATE_BEAD_TYPE_LABEL, TemplateDiscovery};

/// Configuration for templates section.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TemplatesConfig {
    /// Whether to file a bead with suggested templates when none found.
    #[serde(default = "default_auto_suggest")]
    pub auto_suggest: bool,
}

fn default_auto_suggest() -> bool {
    true
}

impl Default for TemplatesConfig {
    fn default() -> Self {
        Self { auto_suggest: true }
    }
}

/// Full Rodgers configuration.
#[derive(Debug, Clone, Default, serde::Deserialize)]
pub struct RodgersConfig {
    pub templates: TemplatesConfig,
}

/// Result of running init checks.
#[derive(Debug, Clone)]
pub struct InitCheckResult {
    /// The repository checked.
    pub repository: String,
    /// Template discovery result.
    pub template_discovery: TemplateDiscovery,
    /// Whether a bead was (or would be) filed.
    pub bead_filed: bool,
    /// The bead content if applicable.
    pub bead_body: Option<String>,
}

impl InitCheckResult {
    /// Create a new init check result.
    pub fn new(repository: String) -> Self {
        Self {
            repository: repository.clone(),
            template_discovery: TemplateDiscovery::new(repository),
            bead_filed: false,
            bead_body: None,
        }
    }

    /// Check templates and determine if a bead should be filed.
    ///
    /// In a real implementation, this would query the GitHub API to check
    /// for existing templates. For now, it uses the provided discovery result.
    pub fn with_template_discovery(
        mut self,
        discovery: TemplateDiscovery,
        auto_suggest: bool,
    ) -> Self {
        self.template_discovery = discovery;

        if self.template_discovery.should_file_bead(auto_suggest) {
            self.bead_filed = true;
            self.bead_body = Some(self.template_discovery.generate_bead_body());
        }

        self
    }

    /// Get the bead title for filing.
    pub fn bead_title(&self) -> &'static str {
        TEMPLATE_BEAD_TITLE
    }

    /// Get the bead type label.
    pub fn bead_type_label(&self) -> &'static str {
        TEMPLATE_BEAD_TYPE_LABEL
    }
}

/// Check templates for a repository and file a bead if needed.
///
/// This function is called during `rogers init` to check if the target
/// repository has issue templates. If none are found and auto_suggest is true,
/// a bead is generated with suggested default templates.
///
/// Returns the init check result with bead information if applicable.
pub fn check_and_suggest_templates(
    repository: &str,
    found_templates: Vec<String>,
    auto_suggest: bool,
) -> InitCheckResult {
    let discovery = TemplateDiscovery::new(repository.to_string()).with_templates(found_templates);

    let result = InitCheckResult::new(repository.to_string())
        .with_template_discovery(discovery, auto_suggest);

    if result.bead_filed {
        tracing::info!(
            repository = result.repository,
            title = result.bead_title(),
            "Filing bead for missing issue templates"
        );
    } else {
        tracing::debug!(
            repository = result.repository,
            "Templates complete or auto_suggest disabled, no bead filed"
        );
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_auto_suggest_is_true() {
        let config = TemplatesConfig::default();
        assert!(config.auto_suggest);
    }

    #[test]
    fn test_init_result_no_bead_when_templates_complete() {
        let result = InitCheckResult::new("owner/repo".to_string()).with_template_discovery(
            TemplateDiscovery::new("owner/repo".to_string()).with_templates(vec![
                "bug_report.md".to_string(),
                "feature_request.md".to_string(),
                "question.md".to_string(),
            ]),
            true,
        );

        assert!(!result.bead_filed);
        assert!(result.bead_body.is_none());
    }

    #[test]
    fn test_init_result_bead_when_no_templates_and_auto_suggest() {
        let result = InitCheckResult::new("owner/repo".to_string())
            .with_template_discovery(TemplateDiscovery::new("owner/repo".to_string()), true);

        assert!(result.bead_filed);
        assert!(result.bead_body.is_some());
        let body = result.bead_body.unwrap();
        assert!(body.contains("bug_report.md"));
        assert!(body.contains("feature_request.md"));
        assert!(body.contains("question.md"));
    }

    #[test]
    fn test_init_result_no_bead_when_no_templates_and_auto_suggest_false() {
        let result = InitCheckResult::new("owner/repo".to_string())
            .with_template_discovery(TemplateDiscovery::new("owner/repo".to_string()), false);

        assert!(!result.bead_filed);
        assert!(result.bead_body.is_none());
    }

    #[test]
    fn test_bead_title_is_correct() {
        let result = InitCheckResult::new("owner/repo".to_string());
        assert_eq!(
            result.bead_title(),
            "Project missing issue templates - suggested templates available"
        );
    }

    #[test]
    fn test_bead_type_label_is_infra() {
        let result = InitCheckResult::new("owner/repo".to_string());
        assert_eq!(result.bead_type_label(), "infra");
    }

    #[test]
    fn test_check_and_suggest_templates_creates_result() {
        let result =
            check_and_suggest_templates("owner/repo", vec!["bug_report.md".to_string()], true);

        assert_eq!(result.repository, "owner/repo");
        assert!(result.bead_filed);
        assert!(result.bead_body.is_some());
    }
}
