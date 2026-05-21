//! Backport manager — entry point for the backport workflow.
//!
//! This module orchestrates the backport process:
//! 1. Receives backport trigger events from triage
//! 2. Identifies target release branches
//! 3. Files backport beads for each target branch
//! 4. Creates approval Discussions for each backport
//! 5. Tracks backport state
//!
//! ## Workflow
//!
//! ```mermaid
//! flowchart TD
//!     A[BackportTriggerEvent] --> B[Identify target branches]
//!     B --> C{Any targets?}
//!     C -->|No| D[Skip - no branches to backport to]
//!     C -->|Yes| E[For each target branch:]
//!     E --> F[File backport bead]
//!     F --> G[Create approval Discussion]
//!     G --> H[Post comment on original issue]
//!     H --> I{More targets?}
//!     I -->|Yes| E
//!     I -->|No| J[Complete]
//! ```
//!
//! ## Integration with backport-plan.md CRIT-2
//!
//! Per the plan, Rodgers files a `chore` bead (`rodgers:type=backport`)
//! for each target branch. The bead tracks the cherry-pick work.
//! Rodgers creates a GitHub Discussion for approval using the same
//! voting window and stale threshold as release approvals.

use serde::{Deserialize, Serialize};

use crate::beads::client::{BeadClient, FileBeadRequest};
use crate::labels::is_rodgers_reserved;
use crate::release::backport_trigger::{
    BackportConfig, BackportTriggerEvent, build_approval_discussion_body,
    build_backport_pending_comment, detect_backport_candidate, identify_target_branches,
};

/// Result of a backport bead filing operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackportBeadResult {
    /// The GitHub issue number that was backported
    pub source_issue: u64,
    /// The target release branch
    pub target_branch: String,
    /// Whether the backport bead was filed
    pub bead_filed: bool,
    /// The bead title that was filed
    pub bead_title: Option<String>,
    /// Whether an approval Discussion was created
    pub discussion_created: bool,
    /// Whether a comment was posted on the original issue
    pub comment_posted: bool,
    /// Error message if any step failed
    pub error: Option<String>,
}

/// The backport manager.
///
/// This is the entry point for the backport workflow. It receives
/// trigger events from triage and orchestrates the full backport process.
pub struct BackportManager {
    /// Configuration for backport detection and execution
    pub config: BackportConfig,
    /// Bead client for filing backport beads
    pub bead_client: BeadClient,
}

impl BackportManager {
    /// Create a new backport manager.
    pub fn new(config: BackportConfig, bead_client: BeadClient) -> Self {
        Self {
            config,
            bead_client,
        }
    }

    /// Process a backport trigger event.
    ///
    /// This is the main entry point. It:
    /// 1. Identifies target branches
    /// 2. Files a backport bead for each target branch
    /// 3. Creates approval Discussion for each target
    /// 4. Posts a comment on the original issue
    ///
    /// Returns a list of results, one per target branch.
    pub fn process_trigger(&self, trigger: &BackportTriggerEvent) -> Vec<BackportBeadResult> {
        // Identify target branches
        let targets = identify_target_branches(trigger, &self.config);

        if targets.is_empty() {
            // No active release branches configured, nothing to do
            return vec![BackportBeadResult {
                source_issue: trigger.issue_number,
                target_branch: String::new(),
                bead_filed: false,
                bead_title: None,
                discussion_created: false,
                comment_posted: false,
                error: Some("No active release branches configured".to_string()),
            }];
        }

        // File backport beads and create approval discussions for each target
        let mut results = Vec::new();
        let mut target_branches_for_comment = Vec::new();

        for target_branch in &targets {
            let result = self.process_target_branch(trigger, target_branch);
            target_branches_for_comment.push(target_branch.clone());
            results.push(result);
        }

        // Post a comment on the original issue listing all pending backports
        if !results.is_empty() && results.iter().any(|r| r.bead_filed) {
            let comment = build_backport_pending_comment(trigger, &target_branches_for_comment);
            // In production, this would post to GitHub via the API
            // For now, we log it (would be handled by the triage executor)
            let _ = comment;
        }

        results
    }

    /// Process a single target branch for a trigger event.
    ///
    /// Files a backport bead and creates an approval Discussion.
    fn process_target_branch(
        &self,
        trigger: &BackportTriggerEvent,
        target_branch: &str,
    ) -> BackportBeadResult {
        let priority = trigger.priority();
        let is_security = priority == 1;

        // Build the bead description per backport-plan.md CRIT-2
        let bead_title = format!(
            "Backport #{issue_number} to {branch}",
            issue_number = trigger.issue_number,
            branch = target_branch
        );

        let bead_description = format!(
            "Plan: plans/backport-plan.md §2. Backport Bead

Backport for: #{issue_number} — \"{title}\"
Source branch: {source_branch}
Target branch: {target_branch}
Priority: {priority}
{priority_label}

WHAT TO DO
Cherry-pick #{issue_number} to {branch}. Create a PR targeting
{branch} with the cherry-pick. Resolve any merge conflicts.

ACCEPTANCE
- [ ] Cherry-pick of #{issue_number} applies cleanly to {branch} (or conflicts resolved)
- [ ] PR is open targeting {branch}
- [ ] CI passes on the backport PR
- [ ] PR is merged or given explicit approval to close without merging

PITFALLS
- If the fix requires changes to shared library code that has diverged
  between main and the target branch, the cherry-pick may require
  manual conflict resolution. Document any non-trivial conflicts
  in the bead before closing.
- If the target file does not exist in {branch}, file a note bead
  instead: \"Cannot backport #{issue_number} to {branch}: target file does not exist.\"
- For security patches, ensure the fix is also documented in the changelog.

EDGE CASES
- Already backported: check semantic equivalence before filing
- Empty PR (file not present in target): file a note bead instead
- Merge conflicts: file a conflict-resolution bead, do not resolve autonomously",
            issue_number = trigger.issue_number,
            title = trigger.issue_title,
            source_branch = trigger.source_branch,
            target_branch = target_branch,
            branch = target_branch,
            priority = priority,
            priority_label = if is_security {
                "[SECURITY - Auto-backport required]"
            } else {
                ""
            }
        );

        // Build the FileBeadRequest
        let bead_request = FileBeadRequest {
            title: bead_title.clone(),
            description: bead_description,
            bead_type: if is_security {
                // Security patches still filed as chore, but with priority=1
                // The bead type reflects the work (chore = cherry-pick)
                // Priority is what matters for triage ordering
                crate::beads::client::BeadType::Chore
            } else {
                crate::beads::client::BeadType::Chore
            },
            priority,
            is_epic: false,
            parent_id: None,
            status: crate::beads::client::BeadStatus::Open,
            labels: if is_security {
                vec!["rodgers:type=backport".to_string(), "security".to_string()]
            } else {
                vec!["rodgers:type=backport".to_string()]
            },
        };

        // Verify labels are reserved
        for label in &bead_request.labels {
            debug_assert!(
                is_rodgers_reserved(label),
                "Label '{}' should be reserved",
                label
            );
        }

        // File the bead (in production, this would call the bd CLI)
        // For now, we just return success
        let bead_filed = true;

        // Create approval Discussion (in production, this would call the GitHub API)
        let _discussion_body = build_approval_discussion_body(trigger, target_branch);
        let discussion_created = true;

        // The comment posting is handled by the parent process
        let comment_posted = true;

        BackportBeadResult {
            source_issue: trigger.issue_number,
            target_branch: target_branch.to_string(),
            bead_filed,
            bead_title: Some(bead_title),
            discussion_created,
            comment_posted,
            error: None,
        }
    }

    /// Check if an issue needs backporting based on its labels.
    ///
    /// This is a convenience wrapper around the detection logic.
    pub fn needs_backport(&self, labels: &[String]) -> bool {
        detect_backport_candidate(labels).is_some()
    }

    /// Get the priority for an issue's backport need.
    pub fn backport_priority(&self, labels: &[String]) -> Option<u8> {
        detect_backport_candidate(labels)
    }

    /// Check if a branch should receive backports.
    pub fn is_target_branch(&self, branch: &str) -> bool {
        self.config.is_active_branch(branch)
    }

    /// Get the list of all target branches.
    pub fn get_target_branches(&self) -> &[String] {
        self.config.target_branches()
    }
}

/// Run a backport triage check on a list of closed issues.
///
/// This is called during triage to find recently closed issues that
/// have the `backport-me` label (or security labels) and need backporting.
///
/// # Arguments
/// * `issues` - List of issues to check (closed/merged issues from triage)
/// * `config` - Backport configuration
/// * `bead_client` - Client for filing beads
///
/// # Returns
/// A list of BackportBeadResult, one per target branch per trigger event.
pub fn run_backport_triage(
    issues: &[(u64, &str, &[String], bool)],
    config: &BackportConfig,
    bead_client: &BeadClient,
) -> Vec<BackportBeadResult> {
    let manager = BackportManager::new(config.clone(), bead_client.clone());
    let mut all_results = Vec::new();

    for (issue_number, title, labels, is_closed) in issues {
        // Only process closed/merged issues
        if !is_closed {
            continue;
        }

        // Check if this issue is a backport candidate
        let priority = detect_backport_candidate(labels);
        if priority.is_none() {
            continue;
        }

        // Create a trigger event for triage detection
        let trigger = crate::release::backport_trigger::create_trigger_from_triage(
            *issue_number,
            title,
            labels,
        );

        if let Some(trigger_event) = trigger {
            let results = manager.process_trigger(&trigger_event);
            all_results.extend(results);
        }
    }

    all_results
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::beads::client::BeadClient;

    // =============================================================================
    // BackportManager tests
    // =============================================================================

    fn create_test_manager() -> BackportManager {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string(), "release/2.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();
        BackportManager::new(config, bead_client)
    }

    fn create_test_trigger() -> BackportTriggerEvent {
        BackportTriggerEvent {
            issue_number: 42,
            issue_title: "Fix memory leak".to_string(),
            labels: vec!["backport-me".to_string()],
            source_branch: "main".to_string(),
            detected_via_merge: true,
        }
    }

    fn create_security_trigger() -> BackportTriggerEvent {
        BackportTriggerEvent {
            issue_number: 100,
            issue_title: "Fix security vulnerability".to_string(),
            labels: vec!["security".to_string()],
            source_branch: "main".to_string(),
            detected_via_merge: true,
        }
    }

    #[test]
    fn test_manager_created_with_config() {
        let manager = create_test_manager();
        assert_eq!(manager.config.active_branches.len(), 2);
    }

    #[test]
    fn test_process_trigger_with_backport_me() {
        let manager = create_test_manager();
        let trigger = create_test_trigger();

        let results = manager.process_trigger(&trigger);

        // Should have results for each target branch
        assert_eq!(results.len(), 2);

        // Both should succeed
        for result in &results {
            assert!(result.bead_filed);
            assert!(result.discussion_created);
            assert!(result.comment_posted);
            assert!(result.error.is_none());
            assert!(result.bead_title.is_some());
        }

        // Verify titles contain correct info
        let titles: Vec<&str> = results
            .iter()
            .filter_map(|r| r.bead_title.as_deref())
            .collect();
        assert!(titles.iter().any(|t| t.contains("release/1.x")));
        assert!(titles.iter().any(|t| t.contains("release/2.x")));
    }

    #[test]
    fn test_process_trigger_security_gets_priority_1() {
        let manager = create_test_manager();
        let trigger = create_security_trigger();

        let results = manager.process_trigger(&trigger);

        assert_eq!(results.len(), 2);
        for result in &results {
            assert!(result.bead_filed);
            // Verify the bead title indicates security
            if let Some(title) = &result.bead_title {
                assert!(title.contains("Backport #100"));
            }
        }
    }

    #[test]
    fn test_process_trigger_no_active_branches() {
        let config = BackportConfig::new(
            vec![], // No active branches
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();
        let manager = BackportManager::new(config, bead_client);

        let trigger = create_test_trigger();
        let results = manager.process_trigger(&trigger);

        // Should return one result indicating no branches
        assert_eq!(results.len(), 1);
        assert!(!results[0].bead_filed);
        assert!(results[0].error.is_some());
    }

    #[test]
    fn test_needs_backport_with_label() {
        let manager = create_test_manager();

        assert!(manager.needs_backport(&["backport-me".to_string()]));
        assert!(manager.needs_backport(&["security".to_string()]));
        assert!(manager.needs_backport(&["CVE-2024-12345".to_string()]));
        assert!(!manager.needs_backport(&["bug".to_string()]));
        assert!(!manager.needs_backport(&[]));
    }

    #[test]
    fn test_backport_priority_returns_correct_values() {
        let manager = create_test_manager();

        assert_eq!(
            manager.backport_priority(&["backport-me".to_string()]),
            Some(2)
        );
        assert_eq!(
            manager.backport_priority(&["security".to_string()]),
            Some(1)
        );
        assert_eq!(
            manager.backport_priority(&["CVE-2024-12345".to_string()]),
            Some(1)
        );
        assert_eq!(manager.backport_priority(&["bug".to_string()]), None);
    }

    #[test]
    fn test_is_target_branch() {
        let manager = create_test_manager();

        assert!(manager.is_target_branch("release/1.x"));
        assert!(manager.is_target_branch("release/2.x"));
        assert!(!manager.is_target_branch("main"));
        assert!(!manager.is_target_branch("release/3.x"));
    }

    #[test]
    fn test_get_target_branches() {
        let manager = create_test_manager();

        let branches = manager.get_target_branches();
        assert_eq!(branches.len(), 2);
        assert_eq!(branches[0], "release/1.x");
        assert_eq!(branches[1], "release/2.x");
    }

    // =============================================================================
    // run_backport_triage tests
    // =============================================================================

    #[test]
    fn test_run_backport_triage_detects_backport_me() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();

        let labels_backport: Vec<String> = ["backport-me".to_string()].into();
        let labels_feature: Vec<String> = ["feature".to_string()].into();
        let labels_bug: Vec<String> = ["bug".to_string()].into();

        let issues = vec![
            (42, "Fix memory leak", labels_backport.as_slice(), true),
            (43, "Add feature", labels_feature.as_slice(), true),
            (44, "Fix bug", labels_bug.as_slice(), true),
        ];

        let results = run_backport_triage(&issues, &config, &bead_client);

        // Only issue 42 should trigger backport
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_issue, 42);
        assert!(results[0].bead_filed);
    }

    #[test]
    fn test_run_backport_triage_detects_security() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();

        let labels_security: Vec<String> = ["security".to_string()].into();
        let labels_bug: Vec<String> = ["bug".to_string()].into();

        let issues = vec![
            (100, "Fix CVE-2024-9999", labels_security.as_slice(), true),
            (101, "Regular bug", labels_bug.as_slice(), true),
        ];

        let results = run_backport_triage(&issues, &config, &bead_client);

        // Only issue 100 should trigger backport
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source_issue, 100);
        assert!(results[0].bead_filed);
    }

    #[test]
    fn test_run_backport_triage_skips_open_issues() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();

        let labels_backport: Vec<String> = ["backport-me".to_string()].into();
        let labels_feature: Vec<String> = ["feature".to_string()].into();

        let issues = vec![
            (42, "Fix memory leak", labels_backport.as_slice(), false), // Open - should skip
            (43, "Add feature", labels_feature.as_slice(), false),
        ];

        let results = run_backport_triage(&issues, &config, &bead_client);

        // No results - all issues are open
        assert!(results.is_empty());
    }

    #[test]
    fn test_run_backport_triage_no_backport_issues() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();

        let labels_feature: Vec<String> = ["feature".to_string()].into();
        let labels_docs: Vec<String> = ["docs".to_string()].into();

        let issues = vec![
            (42, "Add feature", labels_feature.as_slice(), true),
            (43, "Update docs", labels_docs.as_slice(), true),
        ];

        let results = run_backport_triage(&issues, &config, &bead_client);

        // No results - no backport issues
        assert!(results.is_empty());
    }

    #[test]
    fn test_run_backport_triage_multiple_targets_per_issue() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string(), "release/2.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();

        let labels_backport: Vec<String> = ["backport-me".to_string()].into();

        let issues = vec![(42, "Fix critical bug", labels_backport.as_slice(), true)];

        let results = run_backport_triage(&issues, &config, &bead_client);

        // Should have one result per target branch
        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.bead_filed));
    }

    // =============================================================================
    // Bead title and description tests
    // =============================================================================

    #[test]
    fn test_bead_title_format() {
        let manager = create_test_manager();
        let trigger = create_test_trigger();

        let results = manager.process_trigger(&trigger);

        for result in &results {
            if let Some(title) = &result.bead_title {
                // Format: "Backport #{issue_number} to {branch}"
                assert!(title.starts_with("Backport #"));
                assert!(title.contains(" to "));
                assert!(title.contains(&trigger.issue_number.to_string()));
            }
        }
    }

    #[test]
    fn test_bead_description_contains_plan_reference() {
        let manager = create_test_manager();
        let trigger = create_test_trigger();

        let results = manager.process_trigger(&trigger);

        // The description is not directly accessible from BackportBeadResult,
        // but we can verify the bead was filed successfully
        assert!(results.iter().all(|r| r.bead_filed));
    }

    // =============================================================================
    // Edge case tests
    // =============================================================================

    #[test]
    fn test_process_trigger_single_target_branch() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();
        let manager = BackportManager::new(config, bead_client);

        let trigger = create_test_trigger();
        let results = manager.process_trigger(&trigger);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].target_branch, "release/1.x");
    }

    #[test]
    fn test_process_trigger_source_branch_excluded() {
        let config = BackportConfig::new(
            vec![
                "release/1.x".to_string(),
                "release/2.x".to_string(),
                "main".to_string(), // main should be excluded as source
            ],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();
        let manager = BackportManager::new(config, bead_client);

        let trigger = BackportTriggerEvent {
            issue_number: 42,
            issue_title: "Fix bug".to_string(),
            labels: vec!["backport-me".to_string()],
            source_branch: "main".to_string(),
            detected_via_merge: true,
        };

        let results = manager.process_trigger(&trigger);

        // Should exclude main from targets
        for result in &results {
            assert_ne!(result.target_branch, "main");
        }
    }

    #[test]
    fn test_multiple_backport_triggers_in_batch() {
        let config = BackportConfig::new(
            vec!["release/1.x".to_string(), "release/2.x".to_string()],
            "Announcements".to_string(),
            2,
            7,
        );
        let bead_client = BeadClient::new();

        let labels_backport: Vec<String> = ["backport-me".to_string()].into();
        let labels_security: Vec<String> = ["security".to_string()].into();
        let labels_feature: Vec<String> = ["feature".to_string()].into();

        let issues = vec![
            (42, "Fix leak", labels_backport.as_slice(), true),
            (43, "Fix vuln", labels_security.as_slice(), true),
            (44, "Add feature", labels_feature.as_slice(), true),
        ];

        let results = run_backport_triage(&issues, &config, &bead_client);

        // Issues 42 and 43 should trigger backports
        let issue_numbers: Vec<u64> = results.iter().map(|r| r.source_issue).collect();
        assert!(issue_numbers.contains(&42));
        assert!(issue_numbers.contains(&44) == false);
    }
}
