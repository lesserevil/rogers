//! Release task creation with full metadata for audit trail.
//!
//! This module handles filing a release task when a release is approved.
//! The release task serves as the audit trail for the entire release process,
//! tracking branch, tag, merge commit, and GitHub Release linkage.
//!
//! ## Task Metadata
//!
//! When a release is approved, a `chore` task is filed with:
//! - Title: `Release vX.Y.Z`
//! - Type: `chore`
//! - Tag: `rodgers:type=release`
//! - Priority: 1 (high priority)
//! - Status: `open` (progresses to `in_progress` then `closed`)
//! - Description includes all audit metadata

use serde::{Deserialize, Serialize};

use crate::backlog::client::{FileTaskRequest, TaskStatus, TaskType};

/// Metadata required to file a release task.
///
/// This struct captures all the information needed to construct a
/// comprehensive release task description for the audit trail.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseTaskMetadata {
    /// Version string, e.g. "1.2.3" (without leading "v")
    pub version: String,
    /// Source branch the release was cut from ("main" or "release/X.Y")
    pub source_branch: String,
    /// Number of commits since the last release
    pub commits_since_last: u64,
    /// Reason for the version bump (e.g., "feature release", "hotfix", "maintenance")
    pub version_bump_reason: String,
    /// SHA of the merge commit that enabled release candidacy
    pub merge_commit_sha: String,
    /// The release branch name (e.g., "release/1.2.3")
    pub release_branch_name: String,
    /// Git tag name (e.g., "v1.2.3")
    pub git_tag_name: String,
    /// GitHub Release URL (populated after release creation, empty before)
    pub github_release_url: String,
    /// Plan reference for traceability
    pub plan_ref: String,
}

impl ReleaseTaskMetadata {
    /// Build a description string for the release task that includes
    /// all required audit metadata fields.
    pub fn build_description(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("Plan: {}", self.plan_ref));
        lines.push(String::new());

        lines.push("## Release Metadata".to_string());
        lines.push(String::new());

        lines.push(format!("**Source:** {}", self.source_branch));
        lines.push(format!(
            "**Commits since last release:** {}",
            self.commits_since_last
        ));
        lines.push(format!(
            "**Version bump reason:** {}",
            self.version_bump_reason
        ));
        lines.push(format!("**Merge commit:** {}", self.merge_commit_sha));
        lines.push(format!("**Release branch:** {}", self.release_branch_name));
        lines.push(format!("**Git tag:** {}", self.git_tag_name));

        if !self.github_release_url.is_empty() {
            lines.push(format!(
                "**GitHub Release:** [{}]({})",
                self.github_release_url, self.github_release_url
            ));
        } else {
            lines.push(String::from("**GitHub Release:** Pending"));
        }

        lines.join("\n")
    }
}

/// Build a FileTaskRequest for a new release task.
///
/// This constructs the complete task filing request with all required
/// metadata fields:
/// - Title: "Release vX.Y.Z"
/// - Type: chore
/// - Tag: rodgers:type=release
/// - Priority: 1
/// - Status: open
/// - Description with full audit metadata
/// - No parent (top-level task)
pub fn build_release_task_request(metadata: &ReleaseTaskMetadata) -> FileTaskRequest {
    let title = format!("Release v{}", metadata.version);
    let description = metadata.build_description();

    FileTaskRequest {
        title,
        description,
        task_type: TaskType::Chore,
        priority: 1,
        is_epic: false,
        parent_id: None,
        status: TaskStatus::Open,
        labels: vec!["rodgers:type=release".to_string()],
    }
}

/// Build a release task request for a release that already has a GitHub Release URL.
///
/// This is the same as `build_release_task_request` but with the URL already
/// populated in the metadata.
#[allow(clippy::too_many_arguments)]
pub fn build_release_task_with_url(
    version: &str,
    source_branch: &str,
    commits_since_last: u64,
    version_bump_reason: &str,
    merge_commit_sha: &str,
    release_branch_name: &str,
    git_tag_name: &str,
    github_release_url: &str,
    plan_ref: &str,
) -> FileTaskRequest {
    let metadata = ReleaseTaskMetadata {
        version: version.to_string(),
        source_branch: source_branch.to_string(),
        commits_since_last,
        version_bump_reason: version_bump_reason.to_string(),
        merge_commit_sha: merge_commit_sha.to_string(),
        release_branch_name: release_branch_name.to_string(),
        git_tag_name: git_tag_name.to_string(),
        github_release_url: github_release_url.to_string(),
        plan_ref: plan_ref.to_string(),
    };
    build_release_task_request(&metadata)
}

/// Build a release task request for a release that does not yet have a
/// GitHub Release URL (filed at release start).
#[allow(clippy::too_many_arguments)]
pub fn build_release_task_start(
    version: &str,
    source_branch: &str,
    commits_since_last: u64,
    version_bump_reason: &str,
    merge_commit_sha: &str,
    release_branch_name: &str,
    git_tag_name: &str,
    plan_ref: &str,
) -> FileTaskRequest {
    build_release_task_with_url(
        version,
        source_branch,
        commits_since_last,
        version_bump_reason,
        merge_commit_sha,
        release_branch_name,
        git_tag_name,
        "", // no GitHub Release URL yet
        plan_ref,
    )
}

/// Update a release task description when a GitHub Release is created.
///
/// Replaces the "Pending" placeholder with the actual URL.
pub fn update_release_task_for_github_release(
    request: &FileTaskRequest,
    github_release_url: &str,
) -> FileTaskRequest {
    let mut updated = request.clone();
    // Replace the "Pending" line with the actual URL
    let description = request.description.replace(
        "**GitHub Release:** Pending",
        &format!(
            "**GitHub Release:** [{}]({})",
            github_release_url, github_release_url
        ),
    );
    updated.description = description;
    updated
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // ReleaseTaskMetadata tests
    // =============================================================================

    #[test]
    fn test_build_description_all_fields() {
        let metadata = ReleaseTaskMetadata {
            version: "1.2.3".to_string(),
            source_branch: "main".to_string(),
            commits_since_last: 42,
            version_bump_reason: "feature release".to_string(),
            merge_commit_sha: "abc123def456".to_string(),
            release_branch_name: "release/1.2.3".to_string(),
            git_tag_name: "v1.2.3".to_string(),
            github_release_url: "https://github.com/org/repo/releases/tag/v1.2.3".to_string(),
            plan_ref: "plans/release-management-plan.md §Release Execution".to_string(),
        };

        let description = metadata.build_description();

        assert!(description.contains("Plan: plans/release-management-plan.md §Release Execution"));
        assert!(description.contains("## Release Metadata"));
        assert!(description.contains("**Source:** main"));
        assert!(description.contains("**Commits since last release:** 42"));
        assert!(description.contains("**Version bump reason:** feature release"));
        assert!(description.contains("**Merge commit:** abc123def456"));
        assert!(description.contains("**Release branch:** release/1.2.3"));
        assert!(description.contains("**Git tag:** v1.2.3"));
        assert!(description.contains("**GitHub Release:** [https://github.com/org/repo/releases/tag/v1.2.3](https://github.com/org/repo/releases/tag/v1.2.3)"));
    }

    #[test]
    fn test_build_description_pending_release_url() {
        let metadata = ReleaseTaskMetadata {
            version: "1.0.0".to_string(),
            source_branch: "main".to_string(),
            commits_since_last: 10,
            version_bump_reason: "initial release".to_string(),
            merge_commit_sha: "deadbeef".to_string(),
            release_branch_name: "release/1.0.0".to_string(),
            git_tag_name: "v1.0.0".to_string(),
            github_release_url: String::new(),
            plan_ref: "plans/release-management-plan.md §Release Execution".to_string(),
        };

        let description = metadata.build_description();

        assert!(description.contains("**GitHub Release:** Pending"));
    }

    #[test]
    fn test_build_description_source_branch_release() {
        let metadata = ReleaseTaskMetadata {
            version: "1.1.0".to_string(),
            source_branch: "release/1.0".to_string(),
            commits_since_last: 3,
            version_bump_reason: "hotfix".to_string(),
            merge_commit_sha: "cafebabe".to_string(),
            release_branch_name: "release/1.1.0".to_string(),
            git_tag_name: "v1.1.0".to_string(),
            github_release_url: String::new(),
            plan_ref: "plans/release-management-plan.md §Release Execution".to_string(),
        };

        let description = metadata.build_description();

        assert!(description.contains("**Source:** release/1.0"));
        assert!(description.contains("**Commits since last release:** 3"));
        assert!(description.contains("**Version bump reason:** hotfix"));
    }

    // =============================================================================
    // build_release_task_request tests
    // =============================================================================

    #[test]
    fn test_build_release_task_request_all_fields() {
        let metadata = ReleaseTaskMetadata {
            version: "1.2.3".to_string(),
            source_branch: "main".to_string(),
            commits_since_last: 42,
            version_bump_reason: "feature release".to_string(),
            merge_commit_sha: "abc123".to_string(),
            release_branch_name: "release/1.2.3".to_string(),
            git_tag_name: "v1.2.3".to_string(),
            github_release_url: String::new(),
            plan_ref: "plans/release-management-plan.md §Release Execution".to_string(),
        };

        let request = build_release_task_request(&metadata);

        assert_eq!(request.title, "Release v1.2.3");
        assert_eq!(request.task_type, TaskType::Chore);
        assert_eq!(request.priority, 1);
        assert_eq!(request.status, TaskStatus::Open);
        assert!(!request.is_epic);
        assert!(request.parent_id.is_none());
        assert!(request.labels.contains(&"rodgers:type=release".to_string()));
        assert!(request.description.contains("Plan:"));
        assert!(request.description.contains("Source"));
        assert!(request.description.contains("Commits since last release"));
    }

    #[test]
    fn test_build_release_task_request_priority_and_type() {
        let metadata = ReleaseTaskMetadata {
            version: "1.0.0".to_string(),
            source_branch: "main".to_string(),
            commits_since_last: 0,
            version_bump_reason: "initial".to_string(),
            merge_commit_sha: "0000".to_string(),
            release_branch_name: "release/1.0.0".to_string(),
            git_tag_name: "v1.0.0".to_string(),
            github_release_url: String::new(),
            plan_ref: "plans/release-management-plan.md §Release Execution".to_string(),
        };

        let request = build_release_task_request(&metadata);

        assert_eq!(request.task_type, TaskType::Chore);
        assert_eq!(request.priority, 1);
        assert!(request.labels.contains(&"rodgers:type=release".to_string()));
    }

    #[test]
    fn test_build_release_task_request_no_parent() {
        let metadata = ReleaseTaskMetadata {
            version: "1.0.0".to_string(),
            source_branch: "main".to_string(),
            commits_since_last: 5,
            version_bump_reason: "test".to_string(),
            merge_commit_sha: "1111".to_string(),
            release_branch_name: "release/1.0.0".to_string(),
            git_tag_name: "v1.0.0".to_string(),
            github_release_url: String::new(),
            plan_ref: "plans/release-management-plan.md §Release Execution".to_string(),
        };

        let request = build_release_task_request(&metadata);

        assert!(request.parent_id.is_none());
        assert!(!request.is_epic);
    }

    // =============================================================================
    // build_release_task_with_url tests
    // =============================================================================

    #[test]
    fn test_build_release_task_with_url_includes_github_release() {
        let request = build_release_task_with_url(
            "2.0.0",
            "main",
            100,
            "major release",
            "abcdef",
            "release/2.0.0",
            "v2.0.0",
            "https://github.com/org/repo/releases/tag/v2.0.0",
            "plans/release-management-plan.md §Release Execution",
        );

        assert_eq!(request.title, "Release v2.0.0");
        assert!(request
            .description
            .contains("https://github.com/org/repo/releases/tag/v2.0.0"));
        assert!(!request.description.contains("Pending"));
    }

    // =============================================================================
    // build_release_task_start tests
    // =============================================================================

    #[test]
    fn test_build_release_task_start_has_pending_url() {
        let request = build_release_task_start(
            "1.5.0",
            "main",
            25,
            "feature release",
            "fedcba",
            "release/1.5.0",
            "v1.5.0",
            "plans/release-management-plan.md §Release Execution",
        );

        assert_eq!(request.title, "Release v1.5.0");
        assert!(request.description.contains("Pending"));
        assert_eq!(request.status, TaskStatus::Open);
        assert_eq!(request.task_type, TaskType::Chore);
        assert_eq!(request.priority, 1);
    }

    // =============================================================================
    // update_release_task_for_github_release tests
    // =============================================================================

    #[test]
    fn test_update_release_task_for_github_release_replaces_pending() {
        let request = build_release_task_start(
            "1.0.0",
            "main",
            5,
            "initial",
            "abc123",
            "release/1.0.0",
            "v1.0.0",
            "plans/release-management-plan.md §Release Execution",
        );

        let updated = update_release_task_for_github_release(
            &request,
            "https://github.com/org/repo/releases/tag/v1.0.0",
        );

        assert!(!updated.description.contains("Pending"));
        assert!(updated
            .description
            .contains("https://github.com/org/repo/releases/tag/v1.0.0"));
        // Verify other fields unchanged
        assert_eq!(updated.title, request.title);
        assert_eq!(updated.task_type, request.task_type);
        assert_eq!(updated.priority, request.priority);
    }

    #[test]
    fn test_update_release_task_for_github_release_already_has_url() {
        let request = build_release_task_with_url(
            "1.0.0",
            "main",
            5,
            "initial",
            "abc123",
            "release/1.0.0",
            "v1.0.0",
            "https://github.com/org/repo/releases/tag/v1.0.0",
            "plans/release-management-plan.md §Release Execution",
        );

        // Updating with same URL should not break anything
        let updated = update_release_task_for_github_release(
            &request,
            "https://github.com/org/repo/releases/tag/v1.0.0",
        );

        assert!(updated
            .description
            .contains("https://github.com/org/repo/releases/tag/v1.0.0"));
    }

    // =============================================================================
    // Serialization tests
    // =============================================================================

    #[test]
    fn test_release_task_metadata_serialization() {
        let metadata = ReleaseTaskMetadata {
            version: "1.2.3".to_string(),
            source_branch: "main".to_string(),
            commits_since_last: 42,
            version_bump_reason: "feature release".to_string(),
            merge_commit_sha: "abc123".to_string(),
            release_branch_name: "release/1.2.3".to_string(),
            git_tag_name: "v1.2.3".to_string(),
            github_release_url: "https://github.com/org/releases/v1.2.3".to_string(),
            plan_ref: "plans/release-management-plan.md".to_string(),
        };

        let json = serde_json::to_string(&metadata).unwrap();
        assert!(json.contains("\"version\":\"1.2.3\""));
        assert!(json.contains("\"source_branch\":\"main\""));
        assert!(json.contains("\"commits_since_last\":42"));

        let deserialized: ReleaseTaskMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.version, "1.2.3");
        assert_eq!(deserialized.source_branch, "main");
    }

    #[test]
    fn test_file_task_request_serialization() {
        let request = build_release_task_start(
            "1.0.0",
            "main",
            10,
            "initial release",
            "abc123",
            "release/1.0.0",
            "v1.0.0",
            "plans/release-management-plan.md §Release Execution",
        );

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Release v1.0.0"));
        assert!(json.contains("chore"));
        assert!(json.contains("rodgers:type=release"));
        assert!(json.contains("\"priority\":1"));
        assert!(json.contains("\"open\""));
    }
}
