//! Git tag creation with semantic version.
//!
//! This module handles creating annotated git tags for releases using the
//! `vX.Y.Z` convention. Tags are created with a release message that includes
//! version information and optional changelog content.
//!
//! ## Tag Format
//!
//! Tags use the `vX.Y.Z` format (with `v` prefix), following standard
//! semantic versioning convention. For example, `v1.2.3`.
//!
//! ## Tag Message
//!
//! Annotated tags include a message in the format:
//!
//! ```text
//! Release {version}
//!
//! {optional_changelog_or_description}
//! ```
//!
//! ## Workflow
//!
//! ```mermaid
//! flowchart TD
//!     A[Create release branch] --> B{Tag exists?}
//!     B -->|Yes| C[Error: TagAlreadyExists]
//!     B -->|No| D[Create annotated tag vX.Y.Z]
//!     D --> E[Push tag to origin]
//!     E --> F{Push succeeded?}
//!     F -->|Yes| G[Success]
//!     F -->|No| H[Error: PushFailed]
//! ```

use serde::{Deserialize, Serialize};

use crate::git::client::GitClient;

/// Configuration for creating a release tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagConfig {
    /// Repository path.
    pub repo_path: String,
    /// Semantic version (without 'v' prefix), e.g. "1.2.3".
    pub version: String,
    /// Release message for the annotated tag.
    pub message: String,
    /// Optional changelog content to append to the message.
    pub changelog: Option<String>,
    /// Remote to push the tag to.
    pub remote: String,
}

impl TagConfig {
    /// Create a new tag configuration.
    pub fn new(repo_path: &str, version: &str, message: &str) -> Self {
        Self {
            repo_path: repo_path.to_string(),
            version: version.to_string(),
            message: message.to_string(),
            changelog: None,
            remote: "origin".to_string(),
        }
    }

    /// Set the changelog content to include in the tag message.
    pub fn with_changelog(mut self, changelog: &str) -> Self {
        self.changelog = Some(changelog.to_string());
        self
    }

    /// Set the remote to push to.
    pub fn with_remote(mut self, remote: &str) -> Self {
        self.remote = remote.to_string();
        self
    }

    /// Build the full tag message, including changelog if present.
    pub fn build_message(&self) -> String {
        let mut msg = format!("Release v{}", self.version);
        if !self.message.is_empty() && self.message != "Release" {
            msg.push_str("\n\n");
            msg.push_str(&self.message);
        }
        if let Some(ref changelog) = self.changelog {
            msg.push_str("\n\n");
            msg.push_str(changelog);
        }
        msg
    }

    /// Get the tag name with 'v' prefix.
    pub fn tag_name(&self) -> String {
        format!("v{}", self.version)
    }
}

/// Result of creating a release tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagResult {
    /// The tag name (e.g., "v1.2.3").
    pub tag_name: String,
    /// The version string (e.g., "1.2.3").
    pub version: String,
    /// Whether the tag was newly created.
    pub is_new: bool,
    /// Whether the tag was pushed to the remote.
    pub was_pushed: bool,
    /// Message describing what happened.
    pub message: String,
}

/// Create an annotated release tag.
///
/// This function:
/// 1. Checks if the tag already exists
/// 2. Creates the annotated tag with `vX.Y.Z` format
/// 3. Optionally pushes the tag to the remote
///
/// Returns `TagResult` with details about the created tag.
pub fn create_tag(config: &TagConfig) -> Result<TagResult, crate::git::client::GitError> {
    let client = GitClient::new(&config.repo_path);
    let tag_name = config.tag_name();
    let full_message = config.build_message();

    // Check if tag already exists
    let exists = client.tag_exists(&tag_name)?;
    if exists {
        return Err(crate::git::client::GitError::TagAlreadyExists(
            tag_name.clone(),
        ));
    }

    // Create the annotated tag
    let create_result = client.create_annotated_tag(&tag_name, &full_message)?;

    // Push the tag to remote
    let push_result = client.push_tag(&tag_name, &config.remote);
    let was_pushed = push_result.is_ok();

    let message = match push_result {
        Ok(_) => format!("{} Tag pushed to {}", create_result, config.remote),
        Err(e) => {
            // Tag was created locally but push failed
            format!("{} Push failed: {}", create_result, e)
        }
    };

    Ok(TagResult {
        tag_name,
        version: config.version.clone(),
        is_new: true,
        was_pushed,
        message,
    })
}

/// Create a tag without pushing (local-only operation).
pub fn create_tag_local(config: &TagConfig) -> Result<TagResult, crate::git::client::GitError> {
    let client = GitClient::new(&config.repo_path);
    let tag_name = config.tag_name();
    let full_message = config.build_message();

    // Check if tag already exists
    let exists = client.tag_exists(&tag_name)?;
    if exists {
        return Err(crate::git::client::GitError::TagAlreadyExists(
            tag_name.clone(),
        ));
    }

    // Create the annotated tag
    let create_result = client.create_annotated_tag(&tag_name, &full_message)?;

    Ok(TagResult {
        tag_name,
        version: config.version.clone(),
        is_new: true,
        was_pushed: false,
        message: create_result,
    })
}

/// Build a standard release tag message.
///
/// Creates a message in the format:
///
/// ```text
/// Release vX.Y.Z
///
/// {description}
/// ```
pub fn build_release_message(version: &str, description: &str) -> String {
    format!("Release v{}\n\n{}", version, description)
}

/// Build a release tag message with changelog.
///
/// Creates a message in the format:
///
/// ```text
/// Release vX.Y.Z
///
/// {description}
///
/// {changelog}
/// ```
pub fn build_release_message_with_changelog(
    version: &str,
    description: &str,
    changelog: &str,
) -> String {
    format!("Release v{}\n\n{}\n\n{}", version, description, changelog)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // TagConfig tests
    // =============================================================================

    #[test]
    fn test_tag_config_new() {
        let config = TagConfig::new(".", "1.2.3", "Stable release");
        assert_eq!(config.repo_path, ".");
        assert_eq!(config.version, "1.2.3");
        assert_eq!(config.message, "Stable release");
        assert!(config.changelog.is_none());
        assert_eq!(config.remote, "origin");
    }

    #[test]
    fn test_tag_config_with_changelog() {
        let config = TagConfig::new(".", "1.2.3", "Stable release")
            .with_changelog("### Features\n- New login");
        assert_eq!(
            config.changelog,
            Some("### Features\n- New login".to_string())
        );
    }

    #[test]
    fn test_tag_config_with_remote() {
        let config = TagConfig::new(".", "1.2.3", "Stable release").with_remote("upstream");
        assert_eq!(config.remote, "upstream");
    }

    #[test]
    fn test_tag_name_format() {
        let config = TagConfig::new(".", "1.2.3", "Stable release");
        assert_eq!(config.tag_name(), "v1.2.3");
    }

    #[test]
    fn test_tag_name_format_zero_version() {
        let config = TagConfig::new(".", "0.0.1", "Initial release");
        assert_eq!(config.tag_name(), "v0.0.1");
    }

    // =============================================================================
    // build_message tests
    // =============================================================================

    #[test]
    fn test_build_message_basic() {
        let config = TagConfig::new(".", "1.0.0", "First release");
        let msg = config.build_message();
        assert_eq!(msg, "Release v1.0.0\n\nFirst release");
    }

    #[test]
    fn test_build_message_with_changelog() {
        let config = TagConfig::new(".", "1.0.0", "First release")
            .with_changelog("### Features\n- New feature");
        let msg = config.build_message();
        assert_eq!(
            msg,
            "Release v1.0.0\n\nFirst release\n\n### Features\n- New feature"
        );
    }

    #[test]
    fn test_build_message_empty_description() {
        let config = TagConfig::new(".", "1.0.0", "Release").with_changelog("### Changes");
        // "Release" as message is treated as empty
        let msg = config.build_message();
        assert_eq!(msg, "Release v1.0.0\n\n### Changes");
    }

    #[test]
    fn test_build_message_no_changelog() {
        let config = TagConfig::new(".", "1.0.0", "Stable release");
        let msg = config.build_message();
        assert!(!msg.contains("###"));
        assert_eq!(msg, "Release v1.0.0\n\nStable release");
    }

    // =============================================================================
    // TagResult tests
    // =============================================================================

    #[test]
    fn test_tag_result_fields() {
        let result = TagResult {
            tag_name: "v1.0.0".to_string(),
            version: "1.0.0".to_string(),
            is_new: true,
            was_pushed: true,
            message: "Tag created and pushed".to_string(),
        };

        assert_eq!(result.tag_name, "v1.0.0");
        assert_eq!(result.version, "1.0.0");
        assert!(result.is_new);
        assert!(result.was_pushed);
    }

    #[test]
    fn test_tag_result_serialization() {
        let result = TagResult {
            tag_name: "v2.0.0".to_string(),
            version: "2.0.0".to_string(),
            is_new: true,
            was_pushed: true,
            message: "Created and pushed".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("v2.0.0"));
        assert!(json.contains("2.0.0"));
        assert!(json.contains("true"));

        let deserialized: TagResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.tag_name, "v2.0.0");
        assert!(deserialized.was_pushed);
    }

    // =============================================================================
    // build_release_message tests
    // =============================================================================

    #[test]
    fn test_build_release_message() {
        let msg = build_release_message("1.0.0", "First stable release");
        assert_eq!(msg, "Release v1.0.0\n\nFirst stable release");
    }

    #[test]
    fn test_build_release_message_hotfix() {
        let msg = build_release_message("1.0.1", "Security hotfix");
        assert_eq!(msg, "Release v1.0.1\n\nSecurity hotfix");
    }

    #[test]
    fn test_build_release_message_with_changelog() {
        let msg = build_release_message_with_changelog(
            "1.0.0",
            "Initial release",
            "### Features\n- Login page",
        );
        assert_eq!(
            msg,
            "Release v1.0.0\n\nInitial release\n\n### Features\n- Login page"
        );
    }

    // =============================================================================
    // create_tag tests (local-only verification)
    // =============================================================================

    #[test]
    fn test_create_tag_nonexistent_repo() {
        let config = TagConfig::new("/tmp/does_not_exist", "1.0.0", "Test");
        let result = create_tag(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_tag_local_nonexistent_repo() {
        let config = TagConfig::new("/tmp/does_not_exist", "1.0.0", "Test");
        let result = create_tag_local(&config);
        assert!(result.is_err());
    }

    // =============================================================================
    // Edge cases
    // =============================================================================

    #[test]
    fn test_tag_config_major_version() {
        let config = TagConfig::new(".", "10.20.30", "Large version numbers");
        assert_eq!(config.tag_name(), "v10.20.30");
    }

    #[test]
    fn test_tag_config_prerelease_version() {
        let config = TagConfig::new(".", "1.0.0-alpha.1", "Pre-release");
        assert_eq!(config.tag_name(), "v1.0.0-alpha.1");
    }

    #[test]
    fn test_build_release_message_empty_description() {
        let msg = build_release_message("0.1.0", "");
        assert_eq!(msg, "Release v0.1.0\n\n");
    }

    #[test]
    fn test_tag_result_not_pushed() {
        let result = TagResult {
            tag_name: "v1.0.0".to_string(),
            version: "1.0.0".to_string(),
            is_new: true,
            was_pushed: false,
            message: "Tag created locally but push failed".to_string(),
        };
        assert!(!result.was_pushed);
    }
}
