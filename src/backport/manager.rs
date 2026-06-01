//! Backport manager orchestrator.
//!
//! Coordinates detection, approval, and execution of backports.
//! Runs on each scheduler cycle and:
//! 1. Detects candidates from recent merged commits
//! 2. Files backport tasks per target branch
//! 3. Creates approval Discussions
//! 4. Checks approval status on existing Discussions
//! 5. Executes approved backports (branch + PR)
//! 6. Handles conflicts via conflict-resolution tasks
//! 7. Marks backport tasks as closed when PRs are merged

use crate::backlog::controller::TaskController;
use crate::backlog::schema::{status, task_type};
use crate::error::Result;
use crate::github::auth::GitHubAuth;
use crate::github::client::GitHubClient;
use crate::llm::client::LlmClient;

use super::approval::{ApprovalResult, BackportApproval, BackportApprovalManager};
use super::detector::{BackportCandidate, CandidateReason, DetectionResult};
use super::execution::{BackportExecutor, BackportResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State keys for the backport manager.
pub mod state_keys {
    /// The last run timestamp.
    pub const LAST_RUN: &str = "backport.last_run";

    /// The last processed commit SHA per branch (JSON map: branch -> sha).
    pub const LAST_PROCESSED: &str = "backport.last_processed";
}

/// Backport manager state for persistence between runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackportState {
    /// Last run timestamp.
    pub last_run: Option<DateTime<Utc>>,
    /// Map of source branch -> last processed commit SHA.
    pub last_processed: HashMap<String, String>,
    /// Pending approvals (task_id -> approval data).
    #[serde(default)]
    pub pending_approvals: HashMap<String, PendingApproval>,
}

/// A pending approval record (tracked between runs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    /// Task ID.
    pub task_id: String,
    /// Discussion number.
    pub discussion_number: i32,
    /// Discussion global ID.
    pub discussion_id: String,
    /// Target branch.
    pub target_branch: String,
    /// Commit SHA being backported.
    pub commit_sha: String,
    /// When the discussion was created.
    pub created_at: DateTime<Utc>,
    /// Last reminder time.
    pub last_reminder_at: Option<DateTime<Utc>>,
}

impl PendingApproval {
    /// Create a new pending approval record.
    pub fn new(
        task_id: String,
        discussion_number: i32,
        discussion_id: String,
        target_branch: String,
        commit_sha: String,
    ) -> Self {
        Self {
            task_id,
            discussion_number,
            discussion_id,
            target_branch,
            commit_sha,
            created_at: Utc::now(),
            last_reminder_at: None,
        }
    }

    /// Convert to a BackportApproval.
    pub fn into_approval(self) -> BackportApproval {
        BackportApproval::new(
            self.task_id,
            self.discussion_number,
            self.discussion_id,
            self.target_branch,
            self.commit_sha,
        )
    }
}

/// Backport manager.
///
/// Orchestrates the full backport lifecycle: detection, approval, execution.
#[derive(Debug, Clone)]
pub struct BackportManager {
    /// GitHub client.
    github: GitHubClient,
    /// LLM client (optional, for semantic equivalence).
    llm: Option<LlmClient>,
    /// Task controller.
    task_controller: TaskController,
    /// Approval manager.
    approval_manager: BackportApprovalManager,
    /// Executor.
    executor: BackportExecutor,
    /// Active release branches from config.
    active_branches: Vec<String>,
    /// Security label name.
    security_label: String,
}

impl BackportManager {
    /// Create a new backport manager from configuration.
    pub fn new(
        github: GitHubClient,
        llm: Option<LlmClient>,
        task_controller: TaskController,
        release_config: &crate::config::ReleaseConfig,
        rogation_config: &crate::config::RogationConfig,
    ) -> Self {
        let security_label = rogation_config
            .security_label
            .as_deref()
            .unwrap_or("security")
            .to_string();

        let active_branches = release_config.active_branches.clone().unwrap_or_default();

        let voting_window = release_config.voting_window_days.unwrap_or(2).max(0) as u32;
        let stale_threshold = release_config.stale_threshold_days.unwrap_or(7).max(0) as u32;

        let approval_category = release_config
            .approval_discussion_category
            .as_deref()
            .unwrap_or("Announcements")
            .to_string();

        // Extract strings before moving github
        let github_owner = github.owner().to_string();
        let github_repo = github.repo().to_string();
        let github_token = github.auth().token().to_string();

        Self {
            github: github.clone(),
            llm,
            task_controller,
            approval_manager: BackportApprovalManager::new(
                GitHubClient::new(
                    github_owner.clone(),
                    github_repo.clone(),
                    GitHubAuth::new_with_default_api(&github_token),
                ),
                approval_category,
                voting_window,
                stale_threshold,
            ),
            executor: BackportExecutor::new(
                GitHubClient::new(
                    github_owner,
                    github_repo,
                    GitHubAuth::new_with_default_api(&github_token),
                ),
                Some(github_token),
            ),
            active_branches,
            security_label,
        }
    }

    /// Run the backport manager cycle.
    ///
    /// This is the main entry point called by the scheduler on each run.
    pub async fn run(&mut self, state: &mut BackportState) -> Result<BackportRunResult> {
        tracing::info!("Starting backport manager run");
        let start = Utc::now();
        let mut result = BackportRunResult::default();

        // 1. Detect candidates since last run
        let last_run = state.last_run.filter(|_| true);
        let detection_result = self.detect_candidates(last_run).await?;

        for candidate in &detection_result.candidates {
            tracing::info!(
                "Backport candidate: {} ({}) on {}",
                candidate.commit_sha_short,
                candidate.reason,
                candidate.landed_on_branch
            );
        }
        result.candidates_found = detection_result.candidates.len();
        result.checked = detection_result.checked;

        // 2. For each target branch, file a backport task and create discussion
        // Clone active_branches to avoid holding &self while calling &mut self methods
        let active_branches: Vec<_> = self.active_branches.clone();
        for candidate in &detection_result.candidates {
            for branch in &active_branches {
                // Skip if same as landed branch
                if &candidate.landed_on_branch == branch {
                    continue;
                }

                // Check semantic equivalence
                let already_backported = self.is_already_backported(candidate, branch).await?;

                if already_backported {
                    tracing::debug!(
                        "Commit {} is already present on {}, skipping",
                        candidate.commit_sha_short,
                        branch
                    );
                    continue;
                }

                // File backport task
                match self.file_backport_task(candidate, branch).await {
                    Ok(task_id) => {
                        result.tasks_filed += 1;

                        // Create approval discussion
                        match self
                            .create_approval_discussion(candidate, branch, &task_id, None)
                            .await
                        {
                            Ok((disc_num, disc_id)) => {
                                result.approvals_created += 1;
                                state.pending_approvals.insert(
                                    task_id.clone(),
                                    PendingApproval::new(
                                        task_id,
                                        disc_num,
                                        disc_id,
                                        branch.to_string(),
                                        candidate.commit_sha.clone(),
                                    ),
                                );
                            }
                            Err(e) => {
                                tracing::error!(
                                    "Failed to create approval discussion for {}: {}",
                                    task_id,
                                    e
                                );
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to file backport task: {}", e);
                    }
                }
            }
        }

        // 3. Check pending approvals
        if !state.pending_approvals.is_empty() {
            let approval_entries: Vec<_> = self
                .check_pending_approvals(state)
                .await?
                .into_iter()
                .collect();

            result.approvals_checked = state.pending_approvals.len();

            // 4. Execute approved backports
            for (task_id, approval_result) in approval_entries {
                match approval_result {
                    ApprovalResult::Approved => {
                        if let Some(approval) = state.pending_approvals.remove(&task_id) {
                            let exec_result = self.execute_approved_backport(&approval).await;

                            // Handle the result which may use anyhow::Error internally
                            match exec_result {
                                Ok(backport_result) => {
                                    result.executions_succeeded += 1;
                                    result.push_files_created += 1;
                                    result
                                        .prs_created
                                        .insert(task_id.clone(), backport_result.pr_number);
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Backport execution failed for {}: {}",
                                        task_id,
                                        e
                                    );
                                    result.execution_errors.push(e.to_string());
                                }
                            }

                            // Clean up approval (he's done, either succeeded or failed)
                            if let Err(e) = self
                                .task_controller
                                .update_child_status(&task_id, status::CLOSED)
                                .await
                            {
                                tracing::warn!("Failed to close task {}: {}", task_id, e);
                            }
                        }
                    }
                    ApprovalResult::Rejected => {
                        // Mark the task as closed with note about rejection
                        tracing::info!("Backport {} rejected by human, closing task", task_id);
                        if let Err(e) = self
                            .task_controller
                            .update_child_status(&task_id, status::CLOSED)
                            .await
                        {
                            tracing::warn!("Failed to close task {}: {}", task_id, e);
                        }
                        state.pending_approvals.remove(&task_id);
                    }
                    _ => {
                        // Still pending or other status, do nothing
                    }
                }
            }

            // 5. Handle stale discussions (reminder or close)
            self.handle_stale_discussions(state).await?;
        }

        result.elapsed_ms = (Utc::now() - start).num_milliseconds() as u64;
        state.last_run = Some(Utc::now());

        tracing::info!(
            "Backport run complete: {} candidates, {} tasks, {} approvals, {} executions",
            result.candidates_found,
            result.tasks_filed,
            result.approvals_created,
            result.executions_succeeded
        );

        Ok(result)
    }

    /// Detect backport candidates from merged commits.
    async fn detect_candidates(&mut self, since: Option<DateTime<Utc>>) -> Result<DetectionResult> {
        use super::detector::BackportDetector;

        let mut detector = BackportDetector::new(
            self.github.clone(),
            self.llm.clone(),
            crate::config::ReleaseConfig {
                active_branches: Some(self.active_branches.clone()),
                ..Default::default()
            },
            self.security_label.clone(),
        );

        detector.detect_candidates(since).await
    }

    /// Check if a candidate has already been backported to a branch.
    async fn is_already_backported(
        &mut self,
        candidate: &BackportCandidate,
        target_branch: &str,
    ) -> Result<bool> {
        use super::detector::BackportDetector;

        let mut detector = BackportDetector::new(
            self.github.clone(),
            self.llm.clone(),
            crate::config::ReleaseConfig {
                active_branches: Some(self.active_branches.clone()),
                ..Default::default()
            },
            self.security_label.clone(),
        );

        detector
            .is_semantically_equivalent(&candidate.commit_sha, target_branch)
            .await
    }

    /// File a backport task for a candidate targeting a branch.
    async fn file_backport_task(
        &self,
        candidate: &BackportCandidate,
        target_branch: &str,
    ) -> Result<String> {
        use crate::backlog::controller::CreateChildRequest;

        let request = CreateChildRequest {
            title: candidate.task_title(target_branch),
            description: Some(candidate.task_description(target_branch)),
            task_type: Some(task_type::CHORE.to_string()),
            rodgers_type: Some("backport".to_string()),
            rodgers_labels: Some("rodgers:type=backport".to_string()),
            acceptance_criteria: Some(format!(
                "- [ ] Backport {} to {} is merged or explicitly closed without merging",
                candidate.commit_sha_short, target_branch
            )),
            priority: Some(candidate.priority),
        };

        let children = self
            .task_controller
            .file_children("", vec![request])
            .await?;
        Ok(children.first().map(|c| c.id.clone()).unwrap_or_default())
    }

    /// Create an approval discussion for a backport.
    async fn create_approval_discussion(
        &mut self,
        candidate: &BackportCandidate,
        target_branch: &str,
        _task_id: &str,
        _parent_task_id: Option<&str>,
    ) -> Result<(i32, String)> {
        let title = format!(
            "Backport approval: {} to {}",
            candidate.commit_sha_short, target_branch
        );

        let issue_ref = candidate
            .issue_number
            .map(|n| format!("#{n}"))
            .unwrap_or_else(|| "(none)".to_string());

        let body = format!(
            r#"## Backport Proposal

**Commit:** {sha} — "{msg}"
**Source issue:** {issue_ref}
**Target branch:** {branch}

This fix meets backport criteria{reason}.

Approve by reacting 👍.
Reject by reacting 👎 (backport will not proceed).

The backport will be filed as a PR targeting **{branch}**."#,
            sha = candidate.commit_sha,
            msg = candidate.commit_message,
            issue_ref = issue_ref,
            branch = target_branch,
            reason = match candidate.reason {
                CandidateReason::BugFix => " (bug fix)",
                CandidateReason::SecurityPatch => " (security patch — priority 1)",
                CandidateReason::BackportMe => " (backport-me label)",
                CandidateReason::DocumentationFix => " (documentation fix)",
            }
        );

        self.approval_manager
            .create_approval_discussion(&title, &body)
            .await
    }

    /// Check status of all pending approvals.
    async fn check_pending_approvals(
        &mut self,
        state: &mut BackportState,
    ) -> Result<HashMap<String, ApprovalResult>> {
        let mut results = HashMap::new();

        for (task_id, approval) in &state.pending_approvals {
            match self
                .approval_manager
                .check_approval(approval.discussion_number)
                .await
            {
                Ok(result) => {
                    if result != ApprovalResult::Pending {
                        results.insert(task_id.clone(), result);
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to check approval for discussion #{}: {}",
                        approval.discussion_number,
                        e
                    );
                }
            }
        }

        Ok(results)
    }

    /// Execute an approved backport (creates branch + PR).
    async fn execute_approved_backport(
        &mut self,
        approval: &PendingApproval,
    ) -> Result<BackportResult> {
        let title = format!(
            "Backport {} to {}",
            &approval.commit_sha[..7.min(approval.commit_sha.len())],
            approval.target_branch
        );

        self.executor
            .execute(&approval.commit_sha, &approval.target_branch, &title, None)
            .await
            .map_err(crate::error::RogersError::from)
    }

    /// Handle stale discussions (send reminders or close).
    async fn handle_stale_discussions(&mut self, state: &mut BackportState) -> Result<()> {
        let now = Utc::now();
        let voting_window =
            chrono::Duration::days(self.approval_manager.voting_window_days() as i64);
        let stale_threshold =
            chrono::Duration::days(self.approval_manager.stale_threshold_days() as i64);

        // Collect stale task IDs first to avoid double mutable borrow
        let stale_ids: Vec<String> = state
            .pending_approvals
            .iter()
            .filter(|(_, approval)| {
                let age = now - approval.created_at;
                age > stale_threshold
            })
            .map(|(task_id, _)| task_id.clone())
            .collect();

        for task_id in stale_ids {
            tracing::info!("Backport {} is stale, closing", task_id);
            if let Some(approval) = state.pending_approvals.remove(&task_id) {
                if let Err(e) = self.close_discussion(approval.discussion_number).await {
                    tracing::warn!(
                        "Failed to close discussion #{}: {}",
                        approval.discussion_number,
                        e
                    );
                }
                if let Err(e) = self.file_revisit_task(&approval).await {
                    tracing::warn!("Failed to file revisit task for {}: {}", task_id, e);
                }
            }
        }

        // Collect task IDs needing reminders
        let to_remind: Vec<String> = state
            .pending_approvals
            .iter()
            .filter(|(_, approval)| {
                let age = now - approval.created_at;
                age > voting_window && approval.last_reminder_at.is_none()
            })
            .map(|(task_id, _)| task_id.clone())
            .collect();

        for task_id in to_remind {
            if let Some(approval) = state.pending_approvals.get(&task_id) {
                tracing::info!("Posting voting reminder for backport {}", task_id);
                let reminder_body = format!(
                    r#"## Backport Reminder

This backport proposal has been waiting for approval.

- **{}** → **{}** (commit `{}`)

Please react with 👍 to approve or 👎 to reject.

This discussion will be closed as stale if there is no response."#,
                    approval.commit_sha.chars().take(7).collect::<String>(),
                    approval.target_branch,
                    approval.commit_sha
                );

                if let Err(e) = self
                    .approval_manager
                    .post_reminder(approval.discussion_number, &reminder_body)
                    .await
                {
                    tracing::warn!("Failed to post reminder: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Close a discussion as stale.
    async fn close_discussion(&mut self, discussion_number: i32) -> Result<()> {
        // Use the GraphQL updateDiscussion mutation
        // Get discussions to find the one with this number
        let discussions = self.github.get_discussions(None, Some(100), None).await?;

        let disc = discussions
            .nodes
            .iter()
            .find(|d| d.number == discussion_number);

        if let Some(disc) = disc {
            let mutation = r#"
                mutation($id: ID!) {
                    updateDiscussion(input: {discussionId: $id, state: CLOSED}) {
                        discussion {
                            id
                        }
                    }
                }
            "#;

            #[derive(serde::Serialize)]
            struct Variables {
                id: String,
            }

            #[derive(serde::Deserialize)]
            struct UpdateResult {
                #[serde(rename = "updateDiscussion")]
                _update_discussion: DiscClose,
            }

            #[derive(serde::Deserialize)]
            struct DiscClose {
                #[serde(rename = "discussion")]
                _discussion: DiscId,
            }

            #[derive(serde::Deserialize)]
            struct DiscId {
                #[serde(rename = "id")]
                _id: String,
            }

            let variables = Variables {
                id: disc.id.clone(),
            };

            let _: Option<crate::github::models::GraphQLResponse<UpdateResult>> =
                self.github.graphql(mutation, Some(variables)).await.ok();
        }

        Ok(())
    }

    /// File a revisit task for a stale backport.
    async fn file_revisit_task(&self, approval: &PendingApproval) -> Result<()> {
        use crate::backlog::controller::CreateChildRequest;

        let request = CreateChildRequest {
            title: format!(
                "Revisit: backport {} to {} (stale)",
                approval.commit_sha.chars().take(7).collect::<String>(),
                approval.target_branch
            ),
            description: Some(format!(
                r#"Plan: plans/backport-plan.md §Stale Discussion Handling

The backport discussion for commit {} to **{}** was closed as stale.

WHAT TO DO
Re-evaluate whether this backport is still needed. If so, manually file
the backport PR and close this task.

ORIGINAL APPROVAL DISCUSSION
#{} (may be closed)"#,
                approval.commit_sha, approval.target_branch, approval.discussion_number,
            )),
            task_type: Some(task_type::CHORE.to_string()),
            rodgers_type: Some("backport".to_string()),
            rodgers_labels: None,
            acceptance_criteria: Some(
                "- [ ] Backport is evaluated and either filed manually or explicitly closed"
                    .to_string(),
            ),
            priority: Some(3), // Lower priority for stales
        };

        self.task_controller
            .file_children("", vec![request])
            .await?;

        Ok(())
    }
}

/// Result of a single backport run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BackportRunResult {
    /// Candidates found in this run.
    pub candidates_found: usize,
    /// Total merged PRs checked.
    pub checked: usize,
    /// Backport tasks filed.
    pub tasks_filed: usize,
    /// Approval discussions created.
    pub approvals_created: usize,
    /// Pending approvals checked.
    pub approvals_checked: usize,
    /// Backport executions attempted.
    pub executions_attempted: usize,
    /// Backport executions that succeeded.
    pub executions_succeeded: usize,
    /// Push files created (branches pushed).
    pub push_files_created: usize,
    /// PR numbers created (task_id -> pr_number).
    #[serde(default)]
    pub prs_created: HashMap<String, Option<i32>>,
    /// Execution errors encountered.
    #[serde(default)]
    pub execution_errors: Vec<String>,
    /// Time elapsed in milliseconds.
    pub elapsed_ms: u64,
}

impl BackportRunResult {
    /// Whether any action was taken.
    pub fn took_action(&self) -> bool {
        self.tasks_filed > 0 || self.approvals_created > 0 || self.executions_succeeded > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backport_state_default() {
        let state = BackportState::default();
        assert!(state.last_run.is_none());
        assert!(state.last_processed.is_empty());
        assert!(state.pending_approvals.is_empty());
    }

    #[test]
    fn test_pending_approval_new() {
        let approval = PendingApproval::new(
            "task-1".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "release/1.x".to_string(),
            "abc123def456789".to_string(),
        );

        assert_eq!(approval.task_id, "task-1");
        assert_eq!(approval.discussion_number, 42);
        assert_eq!(approval.target_branch, "release/1.x");
        assert!(approval.last_reminder_at.is_none());
    }

    #[test]
    fn test_pending_approval_into_approval() {
        let pending = PendingApproval::new(
            "task-1".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "release/1.x".to_string(),
            "abc123def456789".to_string(),
        );

        let approval = pending.into_approval();
        assert_eq!(approval.task_id, "task-1");
        assert_eq!(approval.discussion_number, 42);
        assert_eq!(approval.target_branch, "release/1.x");
        assert!(approval.is_pending());
    }

    #[test]
    fn test_backport_run_result_took_action() {
        let mut result = BackportRunResult::default();
        assert!(!result.took_action());

        result.tasks_filed = 1;
        assert!(result.took_action());

        result.tasks_filed = 0;
        result.approvals_created = 1;
        assert!(result.took_action());

        result.approvals_created = 0;
        result.executions_succeeded = 1;
        assert!(result.took_action());

        result.executions_succeeded = 0;
        assert!(!result.took_action());
    }

    #[test]
    fn test_backport_manager_github_config() {
        // Verify GitHubAuth token access works
        let auth = GitHubAuth::new_with_default_api("ghp_abcdefghijklmnopqrstuvwxyz1234567890");
        assert!(auth.validate_token().is_ok());
    }
}
