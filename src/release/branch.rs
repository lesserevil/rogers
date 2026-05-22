//! Release branch creation.
//!
//! This module orchestrates the creation of release branches from a source
//! branch (typically `main` or an existing release branch for hotfixes).
//!
//! ## Release Branch Naming
//!
//! Release branches follow the pattern `release/X.Y.Z` where `X.Y.Z` is the
//! semantic version. For example, `release/1.2.3`.
//!
//! ## Source Branch Selection
//!
//! - **Normal release**: Branch from `main`
//! - **Hotfix release**: Branch from existing `release/X.Y` (the release line)
//!
//! ## Workflow
//!
//! ```mermaid
//! flowchart TD
//!     A[Compute next version] --> B{Source branch?}
//!     B -->|main| C[Create release/X.Y.Z from main]
//!     B -->|release/X.Y| D[Create release/X.Y.Z from release/X.Y]
//!     C --> E{Branch exists?}
//!     D --> E
//!     E -->|Yes| F[Error: BranchAlreadyExists]
//!     E -->|No| G[Success: branch created]
//! ```

use serde::{Deserialize, Serialize};

use crate::git::client::GitClient;
use crate::release::changelog::ConventionalCommitType;

/// Configuration for creating a release branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseBranchConfig {
    /// Repository path.
    pub repo_path: String,
    /// Current version (base for computing next version).
    pub current_major: u64,
    pub current_minor: u64,
    pub current_patch: u64,
    /// Source branch to create the release branch from.
    pub source_branch: String,
    /// Commit types for version computation.
    pub commit_types: Vec<ConventionalCommitType>,
    /// Whether there are breaking changes.
    pub has_breaking_changes: bool,
    /// Optional override for the release version (bypasses computation).
    pub override_version: Option<String>,
}

impl ReleaseBranchConfig {
    /// Create a new release branch configuration.
    pub fn new(
        repo_path: &str,
        current_major: u64,
        current_minor: u64,
        current_patch: u64,
        source_branch: &str,
    ) -> Self {
        Self {
            repo_path: repo_path.to_string(),
            current_major,
            current_minor,
            current_patch,
            source_branch: source_branch.to_string(),
            commit_types: Vec::new(),
            has_breaking_changes: false,
            override_version: None,
        }
    }

    /// Add a commit type for version computation.
    pub fn add_commit_type(mut self, commit_type: ConventionalCommitType) -> Self {
        self.commit_types.push(commit_type);
        self
    }

    /// Mark this release as having breaking changes.
    pub fn with_breaking_changes(mut self) -> Self {
        self.has_breaking_changes = true;
        self
    }

    /// Override the computed version with a specific version string.
    pub fn with_version_override(mut self, version: &str) -> Self {
        self.override_version = Some(version.to_string());
        self
    }

    /// Compute the next version string based on the configuration.
    pub fn compute_version(&self) -> String {
        if let Some(ref version) = self.override_version {
            return version.clone();
        }

        let client = GitClient::new(&self.repo_path);
        let (major, minor, patch) = client.compute_next_version(
            self.current_major,
            self.current_minor,
            self.current_patch,
            &self.commit_types,
            self.has_breaking_changes,
        );
        format!("{}.{}.{}", major, minor, patch)
    }

    /// Get the release branch name.
    pub fn branch_name(&self) -> String {
        format!("release/{}", self.compute_version())
    }

    /// Get the tag name for this release.
    pub fn tag_name(&self) -> String {
        format!("v{}", self.compute_version())
    }
}

/// Result of creating a release branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseBranchResult {
    /// The release branch name (e.g., "release/1.2.3").
    pub branch_name: String,
    /// The version string (e.g., "1.2.3").
    pub version: String,
    /// The tag name (e.g., "v1.2.3").
    pub tag_name: String,
    /// The source branch the release was cut from.
    pub source_branch: String,
    /// Whether the branch was newly created.
    pub is_new: bool,
    /// Message describing what happened.
    pub message: String,
}

/// Create a release branch based on the configuration.
///
/// This function:
/// 1. Computes the next version from commit analysis (or uses override)
/// 2. Constructs the branch name `release/X.Y.Z`
/// 3. Creates the branch from the source branch
///
/// Returns `ReleaseBranchResult` with details about the created branch.
pub fn create_branch(
    config: &ReleaseBranchConfig,
) -> Result<ReleaseBranchResult, crate::git::client::GitError> {
    let client = GitClient::new(&config.repo_path);
    let version = config.compute_version();
    let branch_name = format!("release/{}", version);

    // Check if branch already exists
    let exists = client.branch_exists(&branch_name)?;
    if exists {
        return Err(crate::git::client::GitError::BranchAlreadyExists(
            branch_name.clone(),
        ));
    }

    // Create the branch
    let message = client.create_branch(&branch_name, &config.source_branch)?;

    Ok(ReleaseBranchResult {
        branch_name,
        version: version.clone(),
        tag_name: format!("v{}", version),
        source_branch: config.source_branch.clone(),
        is_new: true,
        message,
    })
}

/// Determine the source branch for a release.
///
/// - If `source_hint` is "hotfix" and the current version has a non-zero minor,
///   the source is `release/X.Y` (the release line).
/// - Otherwise, the source is `main`.
pub fn determine_source_branch(
    source_hint: &str,
    current_major: u64,
    current_minor: u64,
) -> String {
    if source_hint == "hotfix" && current_minor > 0 {
        format!("release/{}.{}", current_major, current_minor)
    } else {
        "main".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // ReleaseBranchConfig tests
    // =============================================================================

    #[test]
    fn test_release_branch_config_new() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main");
        assert_eq!(config.repo_path, ".");
        assert_eq!(config.current_major, 1);
        assert_eq!(config.current_minor, 2);
        assert_eq!(config.current_patch, 3);
        assert_eq!(config.source_branch, "main");
        assert!(config.commit_types.is_empty());
        assert!(!config.has_breaking_changes);
        assert!(config.override_version.is_none());
    }

    #[test]
    fn test_release_branch_config_add_commit_type() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main")
            .add_commit_type(ConventionalCommitType::Feat)
            .add_commit_type(ConventionalCommitType::Fix);
        assert_eq!(config.commit_types.len(), 2);
        assert_eq!(config.commit_types[0], ConventionalCommitType::Feat);
        assert_eq!(config.commit_types[1], ConventionalCommitType::Fix);
    }

    #[test]
    fn test_release_branch_config_with_breaking_changes() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main").with_breaking_changes();
        assert!(config.has_breaking_changes);
    }

    #[test]
    fn test_release_branch_config_with_version_override() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main").with_version_override("2.0.0");
        assert_eq!(config.override_version, Some("2.0.0".to_string()));
    }

    #[test]
    fn test_compute_version_with_override() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main").with_version_override("3.0.0");
        assert_eq!(config.compute_version(), "3.0.0");
    }

    #[test]
    fn test_compute_version_feature_bump() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main")
            .add_commit_type(ConventionalCommitType::Feat);
        assert_eq!(config.compute_version(), "1.3.0");
    }

    #[test]
    fn test_compute_version_patch_bump() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main")
            .add_commit_type(ConventionalCommitType::Fix);
        assert_eq!(config.compute_version(), "1.2.4");
    }

    #[test]
    fn test_compute_version_major_bump() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main").with_breaking_changes();
        assert_eq!(config.compute_version(), "2.0.0");
    }

    #[test]
    fn test_compute_version_no_commits_patch_bump() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main");
        // No commits means patch bump
        assert_eq!(config.compute_version(), "1.2.4");
    }

    // =============================================================================
    // branch_name and tag_name tests
    // =============================================================================

    #[test]
    fn test_branch_name_format() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main")
            .add_commit_type(ConventionalCommitType::Feat);
        assert_eq!(config.branch_name(), "release/1.3.0");
    }

    #[test]
    fn test_tag_name_format() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main")
            .add_commit_type(ConventionalCommitType::Fix);
        assert_eq!(config.tag_name(), "v1.2.4");
    }

    #[test]
    fn test_branch_name_with_override() {
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main").with_version_override("2.0.0");
        assert_eq!(config.branch_name(), "release/2.0.0");
        assert_eq!(config.tag_name(), "v2.0.0");
    }

    // =============================================================================
    // determine_source_branch tests
    // =============================================================================

    #[test]
    fn test_determine_source_branch_normal_release() {
        let source = determine_source_branch("normal", 1, 2);
        assert_eq!(source, "main");
    }

    #[test]
    fn test_determine_source_branch_hotfix() {
        let source = determine_source_branch("hotfix", 1, 2);
        assert_eq!(source, "release/1.2");
    }

    #[test]
    fn test_determine_source_branch_hotfix_zero_minor() {
        // Hotfix with minor=0 should still use main (no release line exists)
        let source = determine_source_branch("hotfix", 1, 0);
        assert_eq!(source, "main");
    }

    #[test]
    fn test_determine_source_branch_main_hint() {
        let source = determine_source_branch("main", 2, 5);
        assert_eq!(source, "main");
    }

    // =============================================================================
    // ReleaseBranchResult tests
    // =============================================================================

    #[test]
    fn test_release_branch_result_fields() {
        let result = ReleaseBranchResult {
            branch_name: "release/1.0.0".to_string(),
            version: "1.0.0".to_string(),
            tag_name: "v1.0.0".to_string(),
            source_branch: "main".to_string(),
            is_new: true,
            message: "Branch created".to_string(),
        };

        assert_eq!(result.branch_name, "release/1.0.0");
        assert_eq!(result.version, "1.0.0");
        assert_eq!(result.tag_name, "v1.0.0");
        assert_eq!(result.source_branch, "main");
        assert!(result.is_new);
    }

    #[test]
    fn test_release_branch_result_serialization() {
        let result = ReleaseBranchResult {
            branch_name: "release/2.0.0".to_string(),
            version: "2.0.0".to_string(),
            tag_name: "v2.0.0".to_string(),
            source_branch: "release/1.0".to_string(),
            is_new: true,
            message: "Created from release line".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("release/2.0.0"));
        assert!(json.contains("v2.0.0"));
        assert!(json.contains("release/1.0"));

        let deserialized: ReleaseBranchResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, "2.0.0");
        assert_eq!(deserialized.source_branch, "release/1.0");
    }

    // =============================================================================
    // create_branch tests (requires actual git repo)
    // =============================================================================

    #[test]
    fn test_create_branch_nonexistent_repo() {
        let config = ReleaseBranchConfig::new("/tmp/does_not_exist", 1, 0, 0, "main");
        let result = create_branch(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_branch_version_computation_hotfix() {
        // Simulate a hotfix: fix-only commits on release line
        let config = ReleaseBranchConfig::new(".", 1, 0, 0, "release/1.0")
            .add_commit_type(ConventionalCommitType::Fix);
        assert_eq!(config.compute_version(), "1.0.1");
        assert_eq!(config.branch_name(), "release/1.0.1");
        assert_eq!(config.source_branch, "release/1.0");
    }

    #[test]
    fn test_create_branch_version_computation_major_release() {
        // Simulate a major release: has breaking changes
        let config = ReleaseBranchConfig::new(".", 0, 9, 5, "main")
            .add_commit_type(ConventionalCommitType::Feat)
            .with_breaking_changes();
        assert_eq!(config.compute_version(), "1.0.0");
        assert_eq!(config.branch_name(), "release/1.0.0");
        assert_eq!(config.tag_name(), "v1.0.0");
    }

    #[test]
    fn test_create_branch_version_computation_minor_release() {
        // Simulate a minor release: feature commits, no breaking
        let config = ReleaseBranchConfig::new(".", 1, 0, 3, "main")
            .add_commit_type(ConventionalCommitType::Feat)
            .add_commit_type(ConventionalCommitType::Docs);
        assert_eq!(config.compute_version(), "1.1.0");
        assert_eq!(config.branch_name(), "release/1.1.0");
    }

    #[test]
    fn test_create_branch_version_computation_patch_release() {
        // Simulate a patch release: fix-only commits
        let config = ReleaseBranchConfig::new(".", 1, 2, 3, "main")
            .add_commit_type(ConventionalCommitType::Fix)
            .add_commit_type(ConventionalCommitType::Chore);
        assert_eq!(config.compute_version(), "1.2.4");
        assert_eq!(config.branch_name(), "release/1.2.4");
    }
}
