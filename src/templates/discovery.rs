//! Template discovery for GitHub issue templates.
//!
//! Discovers canonical templates (bug_report, feature_request, question) in
//! `.github/ISSUE_TEMPLATE/` directory, supporting both .md (legacy) and .yml
//! (GitHub forms) formats.

use crate::error::{Result, RogersError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Canonical GitHub issue template names that Rodgers checks for.
pub const CANONICAL_TEMPLATE_NAMES: &[&str] = &["bug_report", "feature_request", "question"];

/// Supported template file extensions.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["md", "yml", "yaml"];

/// A discovered template status for a single canonical template.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TemplateStatus {
    /// Whether at least one matching file was found for this template.
    pub found: bool,
    /// Path to the found template file(s), if any.
    pub paths: Vec<String>,
}

/// The result of template discovery for all canonical templates.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DiscoveryResult {
    /// Status for each canonical template.
    pub templates: std::collections::HashMap<String, TemplateStatus>,
    /// Whether the ISSUE_TEMPLATE directory exists at all.
    pub directory_exists: bool,
    /// The path that was checked (for reporting).
    pub checked_path: String,
    /// Any warnings encountered during discovery.
    pub warnings: Vec<String>,
}

impl DiscoveryResult {
    /// Returns true if all canonical templates are found.
    pub fn all_found(&self) -> bool {
        CANONICAL_TEMPLATE_NAMES
            .iter()
            .all(|name| self.templates.get(*name).is_some_and(|s| s.found))
    }

    /// Returns true if no templates are found.
    pub fn none_found(&self) -> bool {
        CANONICAL_TEMPLATE_NAMES
            .iter()
            .all(|name| self.templates.get(*name).is_none_or(|s| !s.found))
    }

    /// Returns a list of missing template names.
    pub fn missing_templates(&self) -> Vec<&'static str> {
        CANONICAL_TEMPLATE_NAMES
            .iter()
            .filter(|name| self.templates.get(**name).is_none_or(|s| !s.found))
            .copied()
            .collect()
    }

    /// Returns a list of found template names.
    pub fn found_templates(&self) -> Vec<&'static str> {
        CANONICAL_TEMPLATE_NAMES
            .iter()
            .filter(|name| self.templates.get(**name).is_some_and(|s| s.found))
            .copied()
            .collect()
    }
}

/// Discovers GitHub issue templates in the given repository root.
///
/// Searches `.github/ISSUE_TEMPLATE/` for canonical templates (bug_report,
/// feature_request, question) with both .md and .yml/.yaml extensions.
/// Handles subdirectories recursively and performs case-insensitive matching.
///
/// # Arguments
///
/// * `repo_root` - Path to the repository root
///
/// # Returns
///
/// Discovery result containing found/missing status for each canonical template.
pub fn discover_templates(repo_root: &Path) -> DiscoveryResult {
    discover_templates_impl(repo_root)
}

/// Internal implementation of template discovery.
fn discover_templates_impl(repo_root: &Path) -> DiscoveryResult {
    let issue_template_dir = repo_root.join(".github").join("ISSUE_TEMPLATE");
    let checked_path = issue_template_dir.display().to_string();

    // Check if directory exists
    if !issue_template_dir.exists() || !issue_template_dir.is_dir() {
        let mut result = DiscoveryResult {
            directory_exists: false,
            checked_path,
            ..Default::default()
        };

        // Initialize all templates as not found
        for name in CANONICAL_TEMPLATE_NAMES {
            result.templates.insert(
                (*name).to_string(),
                TemplateStatus {
                    found: false,
                    paths: Vec::new(),
                },
            );
        }

        return result;
    }

    let mut result = DiscoveryResult {
        directory_exists: true,
        checked_path,
        ..Default::default()
    };

    // Initialize all templates as not found
    for name in CANONICAL_TEMPLATE_NAMES {
        result.templates.insert(
            (*name).to_string(),
            TemplateStatus {
                found: false,
                paths: Vec::new(),
            },
        );
    }

    // Search recursively for template files
    let mut all_files: Vec<String> = Vec::new();
    if let Err(e) =
        collect_template_files(&issue_template_dir, &mut all_files, &mut result.warnings)
    {
        result
            .warnings
            .push(format!("Error reading directory: {}", e));
    }

    // Match files against canonical template names (case-insensitive)
    for file_path in &all_files {
        if let Some(template_name) = match_template_name(file_path) {
            let status = result
                .templates
                .entry(template_name.to_string())
                .or_default();
            status.found = true;
            status.paths.push(file_path.clone());
        }
    }

    result
}

/// Collects all template files from a directory recursively.
fn collect_template_files(
    dir: &Path,
    files: &mut Vec<String>,
    warnings: &mut Vec<String>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            // Recurse into subdirectories
            if let Err(e) = collect_template_files(&path, files, warnings) {
                warnings.push(format!(
                    "Warning reading subdirectory {}: {}",
                    path.display(),
                    e
                ));
            }
        } else if path.is_file()
            && let Some(ext) = path.extension()
        {
            let ext_str = ext.to_string_lossy().to_lowercase();
            if SUPPORTED_EXTENSIONS.contains(&ext_str.as_str()) {
                files.push(path.display().to_string());
            }
        }
    }
    Ok(())
}

/// Attempts to match a file path against canonical template names.
/// Returns the canonical name if matched, None otherwise.
/// Performs case-insensitive matching.
fn match_template_name(file_path: &str) -> Option<&'static str> {
    let path = Path::new(file_path);
    let stem = path.file_stem()?;
    let stem_lower = stem.to_string_lossy().to_lowercase();

    for name in CANONICAL_TEMPLATE_NAMES {
        let name_lower = name.to_lowercase();
        if stem_lower == name_lower || stem_lower.starts_with(&format!("{}_", name_lower)) {
            return Some(*name);
        }
    }

    None
}

/// Validates a template file format.
/// Returns Ok if valid, Err with warning message if invalid.
#[allow(dead_code)]
pub fn validate_template(file_path: &Path) -> Result<()> {
    let ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "md" => {
            // Markdown files - basic validation (non-empty)
            let content = fs::read_to_string(file_path)?;
            if content.trim().is_empty() {
                return Err(RogersError::Config(format!(
                    "Template file {} is empty",
                    file_path.display()
                )));
            }
            Ok(())
        }
        "yml" | "yaml" => {
            // YAML files - validate as valid YAML with YAML parser
            let content = fs::read_to_string(file_path)?;
            serde_yaml::from_str::<serde_yaml::Value>(&content).map_err(RogersError::Yaml)?;
            Ok(())
        }
        _ => {
            // Unknown extension - warn but treat as valid for extension handling above
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_template_dir(base: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
        let dir = base.path().join(".github").join("ISSUE_TEMPLATE");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    fn create_template_in_subdir(
        base: &TempDir,
        subdir: &str,
        name: &str,
        content: &str,
    ) -> std::path::PathBuf {
        let dir = base
            .path()
            .join(".github")
            .join("ISSUE_TEMPLATE")
            .join(subdir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        fs::write(&path, content).unwrap();
        path
    }

    // Tests for .md template discovery

    #[test]
    fn test_discover_md_templates() {
        let temp = TempDir::new().unwrap();
        create_template_dir(&temp, "bug_report.md", "# Bug Report\n");
        create_template_dir(&temp, "feature_request.md", "# Feature Request\n");
        create_template_dir(&temp, "question.md", "# Question\n");

        let result = discover_templates(temp.path());

        assert!(result.directory_exists);
        assert_eq!(
            result.found_templates(),
            vec!["bug_report", "feature_request", "question"]
        );
        assert!(result.all_found());
        assert!(result.missing_templates().is_empty());
    }

    #[test]
    fn test_discover_yml_templates() {
        let temp = TempDir::new().unwrap();
        create_template_dir(&temp, "bug_report.yml", "name: Bug Report\n");
        create_template_dir(&temp, "feature_request.yml", "name: Feature Request\n");
        create_template_dir(&temp, "question.yml", "name: Question\n");

        let result = discover_templates(temp.path());

        assert!(result.directory_exists);
        assert!(result.all_found());
    }

    #[test]
    fn test_discover_mixed_templates() {
        let temp = TempDir::new().unwrap();
        create_template_dir(&temp, "bug_report.md", "# Bug Report\n");
        create_template_dir(&temp, "feature_request.yml", "name: Feature Request\n");
        create_template_dir(&temp, "question.md", "# Question\n");

        let result = discover_templates(temp.path());

        assert!(result.directory_exists);
        assert!(result.all_found());
    }

    #[test]
    fn test_reports_all_three_found() {
        let temp = TempDir::new().unwrap();
        create_template_dir(&temp, "bug_report.md", "# Bug Report\n");
        create_template_dir(&temp, "feature_request.md", "# Feature Request\n");
        create_template_dir(&temp, "question.md", "# Question\n");

        let result = discover_templates(temp.path());

        assert_eq!(result.found_templates().len(), 3);
        assert!(result.all_found());
    }

    #[test]
    fn test_reports_missing_templates_individually() {
        let temp = TempDir::new().unwrap();
        // Only bug_report exists
        create_template_dir(&temp, "bug_report.md", "# Bug Report\n");

        let result = discover_templates(temp.path());

        let missing = result.missing_templates();
        assert_eq!(missing, vec!["feature_request", "question"]);
        assert!(!result.all_found());

        let found = result.found_templates();
        assert_eq!(found, vec!["bug_report"]);
    }

    // Tests for directory not found

    #[test]
    fn test_handles_directory_not_found() {
        let temp = TempDir::new().unwrap();
        // Don't create .github/ISSUE_TEMPLATE directory

        let result = discover_templates(temp.path());

        assert!(!result.directory_exists);
        assert!(result.none_found());
        assert_eq!(
            result.missing_templates(),
            vec!["bug_report", "feature_request", "question"]
        );
    }

    // Tests for subdirectory search

    #[test]
    fn test_searches_subdirectories() {
        let temp = TempDir::new().unwrap();
        create_template_in_subdir(&temp, "forms", "bug_report.md", "# Bug Report\n");
        create_template_dir(&temp, "feature_request.md", "# Feature Request\n");
        create_template_in_subdir(&temp, "legacy", "question.md", "# Question\n");

        let result = discover_templates(temp.path());

        assert!(result.all_found());

        // Verify paths include subdirectories
        let bug_status = result.templates.get("bug_report").unwrap();
        assert!(bug_status.paths.iter().any(|p| p.contains("forms")));
    }

    // Tests for case insensitive matching

    #[test]
    fn test_case_insensitive_matching() {
        let temp = TempDir::new().unwrap();
        create_template_dir(&temp, "Bug_Report.md", "# Bug Report\n");
        create_template_dir(&temp, "FEATURE_REQUEST.md", "# Feature Request\n");
        create_template_dir(&temp, "Question.md", "# Question\n");

        let result = discover_templates(temp.path());

        assert!(result.all_found());
    }

    // Test for validate_template

    #[test]
    fn test_validate_markdown_template() {
        let temp = TempDir::new().unwrap();
        let path = create_template_dir(&temp, "test.md", "# Test Template\nSome content\n");

        assert!(validate_template(&path).is_ok());
    }

    #[test]
    fn test_validate_empty_markdown_template() {
        let temp = TempDir::new().unwrap();
        let path = create_template_dir(&temp, "test.md", "");

        assert!(validate_template(&path).is_err());
    }

    #[test]
    fn test_validate_yaml_template() {
        let temp = TempDir::new().unwrap();
        let path = create_template_dir(&temp, "test.yml", "name: Test Template\nabout: Testing\n");

        assert!(validate_template(&path).is_ok());
    }

    #[test]
    fn test_validate_invalid_yaml_template() {
        let temp = TempDir::new().unwrap();
        let path = create_template_dir(&temp, "test.yml", "name: Test\n  invalid: yaml\n");

        assert!(validate_template(&path).is_err());
    }
}
