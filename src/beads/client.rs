#![allow(dead_code)]

//! Bead client for filing epic and child beads.
//!
//! This module provides the interface for creating beads in the beads database.
//! Beads track work items derived from GitHub issues.
//!
//! ## Bead Types
//!
//! - **Epic bead**: Top-level work unit (type=epic) derived from a GitHub issue
//! - **Child beads**: Sub-work items (one per logical unit of work)
//!
//! ## Bead Status
//!
//! When filed from ready-for-work, beads are created with `deferred` status.
//! They are promoted to `open` when a human signal is received (modification
//! or comment on any child bead).

use serde::{Deserialize, Serialize};

/// A bead creation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBeadRequest {
    /// Bead title (matches GitHub issue title for epic, logical unit for children)
    pub title: String,
    /// Bead description with plan reference and GitHub issue link
    pub description: String,
    /// Bead type (epic, feature, bug, chore)
    pub bead_type: BeadType,
    /// Bead priority (0=critical, 1=high, 2=medium, 3=low, 4=backlog)
    pub priority: u8,
    /// Whether this is an epic bead
    pub is_epic: bool,
    /// Parent bead ID (for child beads)
    pub parent_id: Option<String>,
    /// Status (deferred until human signal)
    pub status: BeadStatus,
    /// Labels to apply
    pub labels: Vec<String>,
}

/// Bead type classification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BeadType {
    /// Epic - top-level work unit
    Epic,
    /// Feature implementation
    Feature,
    /// Bug fix
    Bug,
    /// Administrative/task work
    Chore,
    /// Infrastructure work
    Infra,
    /// Testing work
    Test,
}

impl BeadType {
    /// Convert from GitHub labels to bead type.
    pub fn from_github_labels(labels: &[String]) -> Self {
        if labels.iter().any(|l| l == "bug") {
            BeadType::Bug
        } else {
            BeadType::Feature
        }
    }

    /// Get the string representation for beads database.
    pub fn as_str(&self) -> &'static str {
        match self {
            BeadType::Epic => "epic",
            BeadType::Feature => "feature",
            BeadType::Bug => "bug",
            BeadType::Chore => "chore",
            BeadType::Infra => "infra",
            BeadType::Test => "test",
        }
    }
}

/// Bead status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum BeadStatus {
    /// Deferred - awaiting human review/signal
    Deferred,
    /// Open - actively being worked
    Open,
    /// InProgress - work started
    InProgress,
    /// Closed - work completed
    Closed,
    /// Blocked - cannot proceed
    Blocked,
}

impl BeadStatus {
    /// Get the string representation for beads database.
    pub fn as_str(&self) -> &'static str {
        match self {
            BeadStatus::Deferred => "deferred",
            BeadStatus::Open => "open",
            BeadStatus::InProgress => "in-progress",
            BeadStatus::Closed => "closed",
            BeadStatus::Blocked => "blocked",
        }
    }
}

/// Result from filing an epic bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicBeadResult {
    /// The epic bead ID
    pub bead_id: String,
    /// Bead title
    pub title: String,
    /// GitHub issue number this epic came from
    pub github_issue: u64,
    /// List of child bead IDs (empty if single epic)
    pub child_bead_ids: Vec<String>,
    /// Whether this was an epic-scale analysis
    pub is_epic_scale: bool,
    /// Breakdown comment to post on GitHub issue
    pub breakdown_comment: String,
}

/// A child bead created from epic breakdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildBeadSpec {
    /// Bead title (single codebase part or logical unit)
    pub title: String,
    /// Description with scope and constraints
    pub description: String,
    /// Priority for this child bead
    pub priority: u8,
}

/// Result from epic-scale analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicScaleResult {
    /// Whether the work requires epic-scale breakdown
    pub is_epic_scale: bool,
    /// Reasons for epic-scale determination
    pub reasons: Vec<String>,
    /// Child bead specifications (if epic-scale)
    pub child_beads: Vec<ChildBeadSpec>,
    /// Recommendation text
    pub recommendation: String,
}

/// Bead client for API operations.
///
/// This is an interface - actual HTTP calls go through the CLI which
/// wraps the beads CLI (bd). In this implementation we generate the
/// command structures that would be used by the actual client.
pub struct BeadClient;

impl BeadClient {
    /// Create a new bead client.
    pub fn new() -> Self {
        Self
    }

    /// Build an epic bead request.
    ///
    /// Description includes plan reference and acceptance criteria.
    #[allow(clippy::too_many_arguments)]
    pub fn build_epic_request(
        &self,
        github_issue_number: u64,
        github_issue_title: &str,
        _github_issue_body: &str,
        github_issue_url: &str,
        acceptance_criteria: &str,
        is_epic_scale: bool,
        bead_type: BeadType,
        priority: u8,
    ) -> FileBeadRequest {
        let title = github_issue_title.to_string();
        let description = format!(
            "Plan: plans/feature-bug-plan.md §Bead Breakdown. GitHub Issue: #{issue_number}. {url}\n\n\
            ## Acceptance Criteria\n\n{criteria}",
            issue_number = github_issue_number,
            url = github_issue_url,
            criteria = acceptance_criteria
        );

        FileBeadRequest {
            title,
            description,
            bead_type: if is_epic_scale {
                BeadType::Epic
            } else {
                bead_type
            },
            priority,
            is_epic: true,
            parent_id: None,
            status: BeadStatus::Deferred,
            labels: vec!["rodgers:parent=rogers-ch2".to_string()],
        }
    }

    /// Build an enriched epic bead request with CRIT-6 description.
    ///
    /// The description includes:
    /// - Plan: plans/feature-bug-plan.md §Bead Breakdown
    /// - GitHub Issue: #<number> with discovered-from link
    /// - Full acceptance criteria from issue body AND comments
    /// - LLM-summarized What and Why summary
    ///
    /// The enriched description is passed directly rather than being
    /// constructed here.
    #[allow(clippy::too_many_arguments)]
    pub fn build_epic_request_enriched(
        &self,
        _github_issue_number: u64,
        github_issue_title: &str,
        description: &str,
        _github_issue_url: &str,
        _acceptance_criteria: &crate::feature_bug::AllAcceptanceCriteria,
        is_epic_scale: bool,
        bead_type: BeadType,
        priority: u8,
    ) -> FileBeadRequest {
        FileBeadRequest {
            title: github_issue_title.to_string(),
            description: description.to_string(),
            bead_type: if is_epic_scale {
                BeadType::Epic
            } else {
                bead_type
            },
            priority,
            is_epic: true,
            parent_id: None,
            status: BeadStatus::Deferred,
            labels: vec!["rodgers:parent=rogers-ch2".to_string()],
        }
    }

    /// Build a child bead request.
    pub fn build_child_request(
        &self,
        spec: &ChildBeadSpec,
        parent_id: &str,
        github_issue_number: u64,
        bead_type: BeadType,
    ) -> FileBeadRequest {
        let description = format!(
            "{}\n\n__Child bead of epic derived from GitHub Issue #{issue_number}__",
            spec.description,
            issue_number = github_issue_number
        );

        FileBeadRequest {
            title: spec.title.clone(),
            description,
            bead_type,
            priority: spec.priority,
            is_epic: false,
            parent_id: Some(parent_id.to_string()),
            status: BeadStatus::Deferred,
            labels: vec!["rodgers:parent=rogers-ch2".to_string()],
        }
    }

    /// Build the breakdown comment linking epic and child beads.
    pub fn build_breakdown_comment(
        &self,
        epic_bead_id: &str,
        child_bead_ids: &[String],
        is_epic_scale: bool,
    ) -> String {
        if is_epic_scale {
            let child_items: Vec<String> = child_bead_ids
                .iter()
                .map(|id| format!("- [ ] {id}"))
                .collect();

            format!(
                "## Rodgers Break Down\n\n\
                 This issue has been accepted for implementation and broken down into the following work units:\n\n\
                 ### Epic\n\
                 {epic}\n\n\
                 ### Child Beads\n\
                 {children}\n\n\
                 All beads are in deferred status — a human review signal (any modification or comment) will batch-open all non-closed children.\n",
                epic = epic_bead_id,
                children = child_items.join("\n")
            )
        } else {
            format!(
                "## Rodgers Work Tracking\n\n\
                 This issue has been accepted for implementation.\n\n\
                 ### Epic\n\
                 {epic}\n\n\
                 The epic bead tracks all work for this issue.\n",
                epic = epic_bead_id
            )
        }
    }
}

impl Default for BeadClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bead_type_from_github_labels_bug() {
        let labels = vec!["bug".to_string(), "needs-information".to_string()];
        assert_eq!(BeadType::from_github_labels(&labels), BeadType::Bug);
    }

    #[test]
    fn test_bead_type_from_github_labels_feature() {
        let labels = vec!["feature".to_string()];
        assert_eq!(BeadType::from_github_labels(&labels), BeadType::Feature);
    }

    #[test]
    fn test_bead_type_defaults_to_feature() {
        let labels = vec!["documentation".to_string()];
        assert_eq!(BeadType::from_github_labels(&labels), BeadType::Feature);
    }

    #[test]
    fn test_bead_type_as_str() {
        assert_eq!(BeadType::Epic.as_str(), "epic");
        assert_eq!(BeadType::Feature.as_str(), "feature");
        assert_eq!(BeadType::Bug.as_str(), "bug");
    }

    #[test]
    fn test_bead_status_deferred_as_str() {
        assert_eq!(BeadStatus::Deferred.as_str(), "deferred");
    }

    #[test]
    fn test_build_epic_request() {
        let client = BeadClient::new();
        let request = client.build_epic_request(
            42,
            "Implement feature X",
            "Body content",
            "https://github.com/org/repo/issues/42",
            "- [ ] AC-1: Works\n- [ ] AC-2: Correct",
            false, // not epic-scale
            BeadType::Feature,
            2,
        );

        assert!(request.is_epic);
        assert_eq!(request.title, "Implement feature X");
        assert!(request.description.contains("plans/feature-bug-plan.md"));
        assert!(request.description.contains("#42"));
        assert!(request.description.contains("AC-1"));
        assert_eq!(request.status, BeadStatus::Deferred);
        assert!(request.parent_id.is_none());
        assert_eq!(request.bead_type, BeadType::Feature);
    }

    #[test]
    fn test_build_epic_request_epic_type_for_large_work() {
        let client = BeadClient::new();
        let request = client.build_epic_request(
            42,
            "Complex feature",
            "Body",
            "https://github.com/org/repo/issues/42",
            "AC-1",
            true, // epic-scale
            BeadType::Feature,
            1,
        );

        // When epic-scale, type should be epic
        assert_eq!(request.bead_type, BeadType::Epic);
    }

    #[test]
    fn test_build_child_request() {
        let client = BeadClient::new();
        let spec = ChildBeadSpec {
            title: "Implement CLI argument parsing".to_string(),
            description: "Add CLI argument parsing for the new feature. Use clap.".to_string(),
            priority: 2,
        };

        let request = client.build_child_request(&spec, "epic-001", 42, BeadType::Feature);

        assert!(!request.is_epic);
        assert_eq!(request.title, "Implement CLI argument parsing");
        assert!(request.description.contains("clap"));
        assert!(request.description.contains("#42"));
        assert_eq!(request.parent_id, Some("epic-001".to_string()));
        assert_eq!(request.status, BeadStatus::Deferred);
        assert_eq!(request.priority, 2);
    }

    #[test]
    fn test_build_breakdown_comment_epic_scale() {
        let client = BeadClient::new();
        let comment = client.build_breakdown_comment(
            "epic-001",
            &["child-001".to_string(), "child-002".to_string()],
            true,
        );

        assert!(comment.contains("epic-001"));
        assert!(comment.contains("child-001"));
        assert!(comment.contains("child-002"));
        assert!(comment.contains("deferred"));
        assert!(comment.contains("Rodgers Break Down"));
    }

    #[test]
    fn test_build_breakdown_comment_single_epic() {
        let client = BeadClient::new();
        let comment = client.build_breakdown_comment("epic-001", &[], false);

        assert!(comment.contains("epic-001"));
        assert!(comment.contains("Rodgers Work Tracking"));
        assert!(!comment.contains("Child Beads"));
    }

    #[test]
    fn test_epic_bead_result_serialization() {
        let result = EpicBeadResult {
            bead_id: "epic-001".to_string(),
            title: "Test issue".to_string(),
            github_issue: 42,
            child_bead_ids: vec!["child-001".to_string(), "child-002".to_string()],
            is_epic_scale: true,
            breakdown_comment: "Comment".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("epic-001"));
        assert!(json.contains("42"));
    }

    #[test]
    fn test_child_bead_spec_serialization() {
        let spec = ChildBeadSpec {
            title: "Test bead".to_string(),
            description: "Test description".to_string(),
            priority: 1,
        };

        let json = serde_json::to_string(&spec).unwrap();
        assert!(json.contains("Test bead"));
    }

    #[test]
    fn test_file_bead_request_serialization() {
        let request = FileBeadRequest {
            title: "Test".to_string(),
            description: "Test desc".to_string(),
            bead_type: BeadType::Feature,
            priority: 2,
            is_epic: true,
            parent_id: None,
            status: BeadStatus::Deferred,
            labels: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("deferred"));
        assert!(json.contains("feature"));
    }

    #[test]
    fn test_epic_scale_result_serialization() {
        let result = EpicScaleResult {
            is_epic_scale: true,
            reasons: vec!["Multiple areas".to_string(), "Sequential work".to_string()],
            child_beads: vec![ChildBeadSpec {
                title: "Child 1".to_string(),
                description: "Desc".to_string(),
                priority: 2,
            }],
            recommendation: "Break into epic + children".to_string(),
        };

        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("epic_scale"));
        assert!(json.contains("Multiple areas"));
    }

    #[test]
    fn test_client_default() {
        let client = BeadClient::default();
        let request = client.build_epic_request(
            1,
            "Title",
            "Body",
            "https://github.com/org/repo/issues/1",
            "AC-1",
            false,
            BeadType::Feature,
            2,
        );
        assert_eq!(request.title, "Title");
    }
}
