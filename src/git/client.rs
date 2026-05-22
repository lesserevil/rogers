//! Git client for local repository operations.
//!
//! This module provides low-level git operations needed for release management:
//! branch creation, tag creation, and pushing to remotes. All operations use
//! local git commands via `std::process::Command`.
//!
//! ## Error Types
//!
//! - `BranchAlreadyExists` — the target branch already exists locally or remotely
//! - `TagAlreadyExists` — the target tag already exists
//! - `PushFailed` — the push operation failed (permissions, network, etc.)
//! - `NonFastForward` — push would result in non-fast-forward
//!
//! ## Usage
//!
//! ```no_run
//! use rogers::git::client::GitClient;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let client = GitClient::new("/path/to/repo");
//! client.create_branch("release/1.0.0", "main")?;
//! client.create_annotated_tag("v1.0.0", "Release 1.0.0")?;
//! client.push_branch("release/1.0.0", "origin")?;
//! client.push_tag("v1.0.0", "origin")?;
//! # Ok(()) }
//! ```

use std::path::Path;
use thiserror::Error;

/// Error type for git operations.
#[derive(Debug, Error)]
pub enum GitError {
    #[error("branch already exists: {0}")]
    BranchAlreadyExists(String),

    #[error("tag already exists: {0}")]
    TagAlreadyExists(String),

    #[error("push failed for '{ref_name}' to '{remote}': {reason}")]
    PushFailed {
        ref_name: String,
        remote: String,
        reason: String,
    },

    #[error("non-fast-forward push rejected for '{0}'")]
    NonFastForward(String),

    #[error("git command failed: {command} — {stderr}")]
    CommandFailed { command: String, stderr: String },

    #[error("repository not found at path: {0}")]
    RepoNotFound(String),

    #[error("branch '{0}' not found")]
    BranchNotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Alias for branch already exists error.
pub type BranchAlreadyExists = GitError;

/// Alias for tag already exists error.
pub type TagAlreadyExists = GitError;

/// A git client for local repository operations.
///
/// Wraps `git` CLI commands for branch creation, tag creation, and pushing.
#[derive(Debug, Clone)]
pub struct GitClient {
    /// Path to the repository root.
    repo_path: String,
}

impl GitClient {
    /// Create a new GitClient pointing at the given repository path.
    pub fn new<P: AsRef<Path>>(repo_path: P) -> Self {
        Self {
            repo_path: repo_path.as_ref().display().to_string(),
        }
    }

    /// Create from the current directory.
    pub fn from_current_dir() -> Self {
        Self {
            repo_path: ".".to_string(),
        }
    }

    // ---------------------------------------------------------------------------
    // Branch operations
    // ---------------------------------------------------------------------------

    /// Create a new branch from a base branch.
    ///
    /// Returns `Err(GitError::BranchAlreadyExists(_))` if the branch already exists.
    pub fn create_branch(&self, branch: &str, base: &str) -> Result<String, GitError> {
        // Check if branch already exists
        if self.branch_exists(branch)? {
            return Err(GitError::BranchAlreadyExists(branch.to_string()));
        }

        let _output = self.run_git(&["branch", branch, base])?;
        Ok(format!("Branch '{}' created from '{}'", branch, base))
    }

    /// Check if a branch exists (locally or remotely).
    pub fn branch_exists(&self, branch: &str) -> Result<bool, GitError> {
        let output = match self.run_git_output(&["rev-parse", "--verify", branch]) {
            Ok(_) => true,
            Err(GitError::CommandFailed { stderr, .. }) => {
                // git rev-parse --verify returns non-zero when branch doesn't exist
                stderr.contains("not found") || !stderr.is_empty()
            }
            Err(e) => return Err(e),
        };
        Ok(output)
    }

    /// Delete a local branch.
    pub fn delete_branch(&self, branch: &str, force: bool) -> Result<(), GitError> {
        let flag = if force { "-D" } else { "-d" };
        self.run_git(&["branch", flag, branch])?;
        Ok(())
    }

    /// Get the list of local branches.
    pub fn list_local_branches(&self) -> Result<Vec<String>, GitError> {
        let output = self.run_git_output(&["branch", "--list"])?;
        Ok(output
            .lines()
            .map(|line| {
                line.trim()
                    .strip_prefix('*')
                    .unwrap_or(line)
                    .trim()
                    .to_string()
            })
            .filter(|s| !s.is_empty())
            .collect())
    }

    /// Get the list of remote branches for a given remote.
    pub fn list_remote_branches(&self, remote: &str) -> Result<Vec<String>, GitError> {
        let output = self.run_git_output(&["ls-remote", "--heads", "--refs", remote])?;
        Ok(output
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.split('\t').collect();
                parts.get(1).map(|refspec| {
                    refspec
                        .strip_prefix("refs/heads/")
                        .unwrap_or(refspec)
                        .to_string()
                })
            })
            .collect())
    }

    // ---------------------------------------------------------------------------
    // Tag operations
    // ---------------------------------------------------------------------------

    /// Create an annotated tag with a message.
    ///
    /// Returns `Err(GitError::TagAlreadyExists(_))` if the tag already exists.
    pub fn create_annotated_tag(&self, tag: &str, message: &str) -> Result<String, GitError> {
        // Check if tag already exists
        if self.tag_exists(tag)? {
            return Err(GitError::TagAlreadyExists(tag.to_string()));
        }

        let _output = self.run_git(&["tag", "-a", tag, "-m", message])?;
        Ok(format!(
            "Annotated tag '{}' created with message: {}",
            tag, message
        ))
    }

    /// Check if a tag exists.
    pub fn tag_exists(&self, tag: &str) -> Result<bool, GitError> {
        let output = match self.run_git_output(&["rev-parse", "--verify", "refs/tags/", tag]) {
            // git rev-parse takes the tag name as the last arg
            Err(GitError::CommandFailed { stderr, .. }) => {
                // Tag doesn't exist
                stderr.contains("not found") || !stderr.is_empty()
            }
            Err(e) => return Err(e),
            _ => true,
        };
        Ok(output)
    }

    /// List all tags.
    pub fn list_tags(&self) -> Result<Vec<String>, GitError> {
        let output = self.run_git_output(&["tag", "-l"])?;
        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    /// Delete a tag.
    pub fn delete_tag(&self, tag: &str) -> Result<(), GitError> {
        self.run_git(&["tag", "-d", tag])?;
        Ok(())
    }

    // ---------------------------------------------------------------------------
    // Push operations
    // ---------------------------------------------------------------------------

    /// Push a branch to a remote.
    pub fn push_branch(&self, branch: &str, remote: &str) -> Result<String, GitError> {
        let result = self.run_git_output(&["push", remote, branch]);

        match result {
            Ok(output) => Ok(output),
            Err(GitError::CommandFailed { stderr, .. }) => {
                if stderr.contains("[rejected]") && stderr.contains("non-fast-forward") {
                    Err(GitError::NonFastForward(branch.to_string()))
                } else if stderr.contains("already exists") {
                    Err(GitError::BranchAlreadyExists(branch.to_string()))
                } else {
                    Err(GitError::PushFailed {
                        ref_name: format!("refs/heads/{}", branch),
                        remote: remote.to_string(),
                        reason: stderr,
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Push a tag to a remote.
    pub fn push_tag(&self, tag: &str, remote: &str) -> Result<String, GitError> {
        let result = self.run_git_output(&["push", remote, tag]);

        match result {
            Ok(output) => Ok(output),
            Err(GitError::CommandFailed { stderr, .. }) => {
                if stderr.contains("[rejected]") && stderr.contains("non-fast-forward") {
                    Err(GitError::NonFastForward(format!("tag/{}", tag)))
                } else if stderr.contains("already exists") {
                    Err(GitError::TagAlreadyExists(tag.to_string()))
                } else {
                    Err(GitError::PushFailed {
                        ref_name: format!("refs/tags/{}", tag),
                        remote: remote.to_string(),
                        reason: stderr,
                    })
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Push both a branch and its associated tag to a remote.
    pub fn push_branch_and_tag(
        &self,
        branch: &str,
        tag: &str,
        remote: &str,
    ) -> Result<(String, String), GitError> {
        let branch_result = self.push_branch(branch, remote)?;
        let tag_result = self.push_tag(tag, remote)?;
        Ok((branch_result, tag_result))
    }

    // ---------------------------------------------------------------------------
    // Version computation
    // ---------------------------------------------------------------------------

    /// Compute the next semantic version from a list of commit types.
    ///
    /// Conventional commit rules:
    /// - Any `feat` → bump minor (unless major is already set)
    /// - Any `breaking change` → bump major
    /// - Otherwise → bump patch
    pub fn compute_next_version(
        &self,
        current_major: u64,
        current_minor: u64,
        current_patch: u64,
        commit_types: &[crate::release::changelog::ConventionalCommitType],
        has_breaking: bool,
    ) -> (u64, u64, u64) {
        if has_breaking {
            // Breaking change → bump major, reset minor and patch
            (current_major + 1, 0, 0)
        } else if commit_types
            .iter()
            .any(|t| *t == crate::release::changelog::ConventionalCommitType::Feat)
        {
            // Feature → bump minor, reset patch
            (current_major, current_minor + 1, 0)
        } else {
            // Fix/chore/docs/etc → bump patch
            (current_major, current_minor, current_patch + 1)
        }
    }

    /// Get the current tag at HEAD.
    pub fn tag_at_head(&self) -> Result<Option<String>, GitError> {
        let output = self.run_git_output(&["describe", "--tags", "--exact-match", "HEAD"]);
        match output {
            Ok(tag) => Ok(Some(tag.trim().to_string())),
            Err(_) => Ok(None), // HEAD is not on a tag
        }
    }

    /// Get the commit count between two refs.
    pub fn commit_count_between(&self, older: &str, newer: &str) -> Result<u64, GitError> {
        let range = format!("{}..{}", older, newer);
        let output = self.run_git_output(&["rev-list", "--count", &range])?;
        output
            .trim()
            .parse::<u64>()
            .map_err(|e| GitError::CommandFailed {
                command: format!("git rev-list --count {}..{}", older, newer),
                stderr: e.to_string(),
            })
    }

    /// Get the latest tag in the repository.
    pub fn latest_tag(&self) -> Result<Option<String>, GitError> {
        let output = self.run_git_output(&["describe", "--tags", "--abbrev=0"]);
        match output {
            Ok(tag) => Ok(Some(tag.trim().to_string())),
            Err(_) => Ok(None),
        }
    }

    // ---------------------------------------------------------------------------
    // Internal helpers
    // ---------------------------------------------------------------------------

    /// Run a git command and return stdout.
    fn run_git(&self, args: &[&str]) -> Result<String, GitError> {
        let output = std::process::Command::new("git")
            .current_dir(&self.repo_path)
            .args(args)
            .output()
            .map_err(|e| GitError::Io(e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            let command = format!("git {}", args.join(" "));
            return Err(GitError::CommandFailed { command, stderr });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run a git command and return stdout (alias for consistency).
    fn run_git_output(&self, args: &[&str]) -> Result<String, GitError> {
        self.run_git(args)
    }
}

/// Create a release branch from a source branch.
///
/// This is a convenience function that creates a `GitClient`, checks for
/// branch existence, and creates the branch.
pub fn create_release_branch(
    repo_path: &str,
    version: &str,
    source_branch: &str,
) -> Result<String, GitError> {
    let client = GitClient::new(repo_path);
    let branch_name = format!("release/{}", version);
    client.create_branch(&branch_name, source_branch)
}

/// Create an annotated git tag with semantic version prefix.
///
/// Creates a tag `v{version}` with the given message.
pub fn create_annotated_tag(
    repo_path: &str,
    version: &str,
    message: &str,
) -> Result<String, GitError> {
    let client = GitClient::new(repo_path);
    let tag_name = format!("v{}", version);
    client.create_annotated_tag(&tag_name, message)
}

/// Push both a release branch and its tag to origin.
pub fn push_branch_and_tag(repo_path: &str, version: &str) -> Result<(String, String), GitError> {
    let client = GitClient::new(repo_path);
    let branch = format!("release/{}", version);
    let tag = format!("v{}", version);
    client.push_branch_and_tag(&branch, &tag, "origin")
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // GitClient construction tests
    // =============================================================================

    #[test]
    fn test_git_client_new() {
        let client = GitClient::new("/tmp/repo");
        assert_eq!(client.repo_path, "/tmp/repo");
    }

    #[test]
    fn test_git_client_from_current_dir() {
        let client = GitClient::from_current_dir();
        assert_eq!(client.repo_path, ".");
    }

    // =============================================================================
    // GitClient clone test
    // =============================================================================

    #[test]
    fn test_git_client_clone() {
        let client = GitClient::new("/tmp/repo");
        let cloned = client.clone();
        assert_eq!(cloned.repo_path, client.repo_path);
    }

    // =============================================================================
    // compute_next_version tests
    // =============================================================================

    #[test]
    fn test_compute_next_version_breaking_change() {
        let client = GitClient::new(".");
        let types = vec![
            crate::release::changelog::ConventionalCommitType::Fix,
            crate::release::changelog::ConventionalCommitType::Feat,
        ];
        let (major, minor, patch) = client.compute_next_version(1, 2, 3, &types, true);
        assert_eq!(major, 2);
        assert_eq!(minor, 0);
        assert_eq!(patch, 0);
    }

    #[test]
    fn test_compute_next_version_feature() {
        let client = GitClient::new(".");
        let types = vec![
            crate::release::changelog::ConventionalCommitType::Feat,
            crate::release::changelog::ConventionalCommitType::Fix,
        ];
        let (major, minor, patch) = client.compute_next_version(1, 2, 3, &types, false);
        assert_eq!(major, 1);
        assert_eq!(minor, 3);
        assert_eq!(patch, 0);
    }

    #[test]
    fn test_compute_next_version_patch_only() {
        let client = GitClient::new(".");
        let types = vec![
            crate::release::changelog::ConventionalCommitType::Fix,
            crate::release::changelog::ConventionalCommitType::Chore,
        ];
        let (major, minor, patch) = client.compute_next_version(1, 2, 3, &types, false);
        assert_eq!(major, 1);
        assert_eq!(minor, 2);
        assert_eq!(patch, 4);
    }

    #[test]
    fn test_compute_next_version_no_commits() {
        let client = GitClient::new(".");
        let types: Vec<crate::release::changelog::ConventionalCommitType> = vec![];
        let (major, minor, patch) = client.compute_next_version(1, 2, 3, &types, false);
        // No feat, no breaking → patch bump
        assert_eq!(major, 1);
        assert_eq!(minor, 2);
        assert_eq!(patch, 4);
    }

    #[test]
    fn test_compute_next_version_docs_only() {
        let client = GitClient::new(".");
        let types = vec![crate::release::changelog::ConventionalCommitType::Docs];
        let (major, minor, patch) = client.compute_next_version(0, 0, 0, &types, false);
        assert_eq!(major, 0);
        assert_eq!(minor, 0);
        assert_eq!(patch, 1);
    }

    #[test]
    fn test_compute_next_version_mixed_types_with_breaking() {
        let client = GitClient::new(".");
        let types = vec![
            crate::release::changelog::ConventionalCommitType::Feat,
            crate::release::changelog::ConventionalCommitType::Fix,
            crate::release::changelog::ConventionalCommitType::Refactor,
        ];
        let (major, minor, patch) = client.compute_next_version(0, 5, 2, &types, true);
        assert_eq!(major, 1);
        assert_eq!(minor, 0);
        assert_eq!(patch, 0);
    }

    // =============================================================================
    // Error type display tests
    // =============================================================================

    #[test]
    fn test_error_branch_already_exists_display() {
        let err = GitError::BranchAlreadyExists("release/1.0.0".to_string());
        assert!(format!("{}", err).contains("release/1.0.0"));
    }

    #[test]
    fn test_error_tag_already_exists_display() {
        let err = GitError::TagAlreadyExists("v1.0.0".to_string());
        assert!(format!("{}", err).contains("v1.0.0"));
    }

    #[test]
    fn test_error_push_failed_display() {
        let err = GitError::PushFailed {
            ref_name: "refs/heads/release/1.0.0".to_string(),
            remote: "origin".to_string(),
            reason: "connection refused".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("release/1.0.0"));
        assert!(msg.contains("origin"));
        assert!(msg.contains("connection refused"));
    }

    #[test]
    fn test_error_non_fast_forward_display() {
        let err = GitError::NonFastForward("release/1.0.0".to_string());
        assert!(format!("{}", err).contains("release/1.0.0"));
    }

    #[test]
    fn test_error_command_failed_display() {
        let err = GitError::CommandFailed {
            command: "git branch".to_string(),
            stderr: "fatal: not a git repository".to_string(),
        };
        let msg = format!("{}", err);
        assert!(msg.contains("git branch"));
        assert!(msg.contains("not a git repository"));
    }

    #[test]
    fn test_error_branch_not_found_display() {
        let err = GitError::BranchNotFound("feature-xyz".to_string());
        assert!(format!("{}", err).contains("feature-xyz"));
    }

    #[test]
    fn test_error_repo_not_found_display() {
        let err = GitError::RepoNotFound("/nonexistent".to_string());
        assert!(format!("{}", err).contains("/nonexistent"));
    }

    // =============================================================================
    // BranchAlreadyExists / TagAlreadyExists type alias tests
    // =============================================================================

    #[test]
    fn test_branch_already_exists_alias() {
        let err: BranchAlreadyExists = GitError::BranchAlreadyExists("release/1.0.0".to_string());
        assert!(format!("{}", err).contains("release/1.0.0"));
    }

    #[test]
    fn test_tag_already_exists_alias() {
        let err: TagAlreadyExists = GitError::TagAlreadyExists("v1.0.0".to_string());
        assert!(format!("{}", err).contains("v1.0.0"));
    }

    // =============================================================================
    // Convenience function tests (no actual git repo required)
    // =============================================================================

    #[test]
    fn test_create_release_branch_format() {
        // This will fail because we don't have a real git repo at /tmp/nonexistent
        // but we can check the error type
        let result = create_release_branch("/tmp/nonexistent", "1.0.0", "main");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::Io(_) => {} // Expected: directory doesn't exist
            e => panic!("Expected Io error, got: {}", e),
        }
    }

    #[test]
    fn test_create_annotated_tag_format() {
        let result = create_annotated_tag("/tmp/nonexistent", "1.0.0", "Release 1.0.0");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::Io(_) => {} // Expected: directory doesn't exist
            e => panic!("Expected Io error, got: {}", e),
        }
    }

    #[test]
    fn test_push_branch_and_tag_format() {
        let result = push_branch_and_tag("/tmp/nonexistent", "1.0.0");
        assert!(result.is_err());
        match result.unwrap_err() {
            GitError::Io(_) => {} // Expected: directory doesn't exist
            e => panic!("Expected Io error, got: {}", e),
        }
    }

    // =============================================================================
    // GitError serialization tests
    // =============================================================================
}
