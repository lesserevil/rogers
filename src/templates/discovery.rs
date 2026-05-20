//! Template discovery, validation, and management.
//!
//! This module handles:
//! - Discovery of project issue templates in `.github/ISSUE_TEMPLATE/`
//! - Validation that required templates are present
//! - Filing a bead (GitHub issue) with suggested templates when none found

use super::defaults;
use serde::{Deserialize, Serialize};

/// Required template files for Rodgers to function.
pub const REQUIRED_TEMPLATES: &[&str] = &["bug_report.md", "feature_request.md", "question.md"];

/// Result of template discovery for a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateDiscovery {
    /// The repository checked.
    pub repository: String,
    /// Path that was checked.
    pub template_dir: String,
    /// Whether the template directory exists.
    pub directory_exists: bool,
    /// Templates that were found.
    pub found_templates: Vec<String>,
    /// Templates that are missing.
    pub missing_templates: Vec<String>,
    /// Whether all required templates are present.
    pub is_complete: bool,
}

impl TemplateDiscovery {
    /// Create a new discovery result.
    pub fn new(repository: String) -> Self {
        Self {
            repository,
            template_dir: ".github/ISSUE_TEMPLATE/".to_string(),
            directory_exists: false,
            found_templates: Vec::new(),
            missing_templates: REQUIRED_TEMPLATES.iter().map(|s| s.to_string()).collect(),
            is_complete: false,
        }
    }

    /// Check if any templates were found.
    pub fn has_any_templates(&self) -> bool {
        !self.found_templates.is_empty()
    }

    /// Check if all required templates are present.
    pub fn all_required_present(&self) -> bool {
        self.directory_exists && self.missing_templates.is_empty()
    }

    /// Update with found templates.
    pub fn with_templates(mut self, found: Vec<String>) -> Self {
        self.directory_exists = true;
        self.found_templates = found.clone();
        self.missing_templates = REQUIRED_TEMPLATES
            .iter()
            .filter(|t| !found.contains(&t.to_string()))
            .map(|s| s.to_string())
            .collect();
        self.is_complete = self.missing_templates.is_empty();
        self
    }

    /// Generate bead issue content with suggested templates.
    ///
    /// Returns a formatted GitHub issue body containing all three default
    /// templates that can be used to populate `.github/ISSUE_TEMPLATE/`.
    pub fn generate_bead_body(&self) -> String {
        let mut body = String::new();

        body.push_str("# Suggested Issue Templates\n\n");
        body.push_str("This project is missing issue templates. ");
        body.push_str("Rodgers provides the following suggested templates.\n\n");
        body.push_str("To use these, create the files in `.github/ISSUE_TEMPLATE/`:\n\n");
        body.push_str("---\n\n");

        for (filename, title, content) in defaults::ALL_DEFAULT_TEMPLATES {
            body.push_str(&format!("## `{}` — {}\n\n", filename, title));
            body.push_str("```markdown\n");
            body.push_str(content);
            body.push_str("\n```\n\n");
            body.push_str("---\n\n");
        }

        body.push_str("## Usage Notes\n\n");
        body.push_str("- Copy each template into `.github/ISSUE_TEMPLATE/<filename>`\n");
        body.push_str(
            "- Templates are governed by project decision — review and modify before committing\n",
        );
        body.push_str(
            "- Rodgers uses these templates to structure issue routing and completeness checking\n",
        );

        body
    }

    /// Check if a bead should be filed based on discovery and auto_suggest config.
    pub fn should_file_bead(&self, auto_suggest: bool) -> bool {
        // Bead is filed when:
        // - Templates directory missing OR no valid templates found
        // - AND auto_suggest is enabled (default)
        (!self.directory_exists || !self.is_complete) && auto_suggest
    }
}

/// The bead title when filing for missing templates.
pub const TEMPLATE_BEAD_TITLE: &str =
    "Project missing issue templates - suggested templates available";

/// The label to apply for template infrastructure beads.
pub const TEMPLATE_BEAD_TYPE_LABEL: &str = "infra";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discovery_with_no_templates() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string());

        assert!(!discovery.directory_exists);
        assert!(!discovery.has_any_templates());
        assert!(!discovery.all_required_present());
        assert_eq!(3, discovery.missing_templates.len());
    }

    #[test]
    fn test_discovery_with_some_templates() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string())
            .with_templates(vec!["bug_report.md".to_string()]);

        assert!(discovery.directory_exists);
        assert!(discovery.has_any_templates());
        assert!(!discovery.all_required_present());
        assert_eq!(2, discovery.missing_templates.len());
        assert!(
            discovery
                .missing_templates
                .contains(&"feature_request.md".to_string())
        );
        assert!(
            discovery
                .missing_templates
                .contains(&"question.md".to_string())
        );
    }

    #[test]
    fn test_discovery_with_all_templates() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string()).with_templates(vec![
            "bug_report.md".to_string(),
            "feature_request.md".to_string(),
            "question.md".to_string(),
        ]);

        assert!(discovery.directory_exists);
        assert!(discovery.has_any_templates());
        assert!(discovery.all_required_present());
        assert!(discovery.missing_templates.is_empty());
        assert!(discovery.is_complete);
    }

    #[test]
    fn test_should_file_bead_when_no_templates_and_auto_suggest_true() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string());
        assert!(discovery.should_file_bead(true));
    }

    #[test]
    fn test_should_not_file_bead_when_no_templates_and_auto_suggest_false() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string());
        assert!(!discovery.should_file_bead(false));
    }

    #[test]
    fn test_should_not_file_bead_when_templates_complete() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string()).with_templates(vec![
            "bug_report.md".to_string(),
            "feature_request.md".to_string(),
            "question.md".to_string(),
        ]);
        assert!(!discovery.should_file_bead(true));
    }

    #[test]
    fn test_should_file_bead_when_partial_templates() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string())
            .with_templates(vec!["bug_report.md".to_string()]);
        assert!(discovery.should_file_bead(true));
    }

    #[test]
    fn test_bead_body_contains_all_templates() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string())
            .with_templates(vec!["bug_report.md".to_string()]);

        let body = discovery.generate_bead_body();

        assert!(body.contains("bug_report.md"));
        assert!(body.contains("feature_request.md"));
        assert!(body.contains("question.md"));
        assert!(body.contains(&defaults::BUG_REPORT_TEMPLATE));
        assert!(body.contains(&defaults::FEATURE_REQUEST_TEMPLATE));
        assert!(body.contains(&defaults::QUESTION_TEMPLATE));
    }

    #[test]
    fn test_bead_body_contains_usage_notes() {
        let discovery = TemplateDiscovery::new("owner/repo".to_string());
        let body = discovery.generate_bead_body();

        assert!(body.contains("Usage Notes"));
        assert!(body.contains(".github/ISSUE_TEMPLATE/"));
    }

    #[test]
    fn test_bead_title_is_correct() {
        assert_eq!(
            TEMPLATE_BEAD_TITLE,
            "Project missing issue templates - suggested templates available"
        );
    }

    #[test]
    fn test_bead_type_label_is_infra() {
        assert_eq!(TEMPLATE_BEAD_TYPE_LABEL, "infra");
    }
}
