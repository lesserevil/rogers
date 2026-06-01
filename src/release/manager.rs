//! Release manager orchestrator.
//!
//! Coordinates detection, proposal, and execution of releases.
//! Runs on each scheduler cycle and:
//! 1. Detects release candidates from merged PRs since last release
//! 2. Surfaces potential blockers (blocker label, priority, human-flagged)
//! 3. Creates Release Proposal Discussions
//! 4. Checks approval status on existing Discussions
//! 5. Executes approved releases (branch + tag + GitHub Release)
//! 6. Handles stale proposals (reminder, then close + revisit task)

use crate::backlog::controller::TaskController;
use crate::backlog::schema::task_type;
use crate::error::{Result, RogersError};
use crate::github::auth::GitHubAuth;
use crate::github::client::GitHubClient;

use super::detector::{CandidacyResult, ReleaseCandidate, ReleaseSource};
use super::execution::ReleaseExecutor;
use super::proposal::{ApprovalResult, ReleaseApproval, ReleaseProposalManager};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State keys for the release manager.
pub mod state_keys {
    /// The last run timestamp.
    pub const LAST_RUN: &str = "release.last_run";

    /// The last processed release version (for detecting new releases).
    pub const LAST_RELEASE: &str = "release.last_processed";
}

/// Release manager state for persistence between runs.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseState {
    /// Last run timestamp.
    pub last_run: Option<DateTime<Utc>>,
    /// Last processed release version.
    pub last_processed: Option<String>,
    /// Pending approvals (version -> approval data).
    #[serde(default)]
    pub pending_approvals: HashMap<String, PendingApproval>,
}

/// A pending approval record (tracked between runs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    /// Version of the release.
    pub version: String,
    /// Discussion number.
    pub discussion_number: i32,
    /// Discussion global ID.
    pub discussion_id: String,
    /// Source branch.
    pub source: String,
    /// When the discussion was created.
    pub created_at: DateTime<Utc>,
    /// Last reminder time.
    pub last_reminder_at: Option<DateTime<Utc>>,
}

impl PendingApproval {
    /// Create a new pending approval record.
    pub fn new(
        version: String,
        discussion_number: i32,
        discussion_id: String,
        source: String,
    ) -> Self {
        Self {
            version,
            discussion_number,
            discussion_id,
            source,
            created_at: Utc::now(),
            last_reminder_at: None,
        }
    }

    /// Convert to a ReleaseApproval.
    pub fn into_approval(self) -> ReleaseApproval {
        ReleaseApproval::new(
            self.version,
            self.discussion_number,
            self.discussion_id,
            self.source,
        )
    }
}

/// Release manager.
///
/// Orchestrates the full release lifecycle: detection, proposal, execution.
#[derive(Debug, Clone)]
pub struct ReleaseManager {
    /// GitHub client.
    github: GitHubClient,
    /// Task controller.
    task_controller: TaskController,
    /// Approval manager.
    approval_manager: ReleaseProposalManager,
    /// Executor.
    executor: ReleaseExecutor,
    /// Detector.
    detector: super::detector::ReleaseDetector,
    /// Active release branches from config.
    active_branches: Vec<String>,
    /// Blocker label name.
    blocker_label: String,
}

impl ReleaseManager {
    /// Create a new release manager from configuration.
    pub fn new(
        github: GitHubClient,
        task_controller: TaskController,
        release_config: &crate::config::ReleaseConfig,
        blocker_label: String,
    ) -> Self {
        let active_branches = release_config
            .active_branches
            .clone()
            .unwrap_or_default();

        let voting_window_days = release_config.voting_window_days.unwrap_or(2);
        let stale_threshold_days = release_config.stale_threshold_days.unwrap_or(7);
        let voting_window = voting_window_days.max(0) as u32;
        let stale_threshold = stale_threshold_days.max(0) as u32;

        let approval_category = release_config
            .approval_discussion_category
            .as_deref()
            .unwrap_or("Announcements")
            .to_string();

        // Extract strings before moving github
        let github_owner = github.owner().to_string();
        let github_repo = github.repo().to_string();
        let github_token = github.auth().token().to_string();

        let detector = super::detector::ReleaseDetector::new(
            github.clone(),
            crate::config::ReleaseConfig {
                approval_discussion_category: Some(approval_category.clone()),
                active_branches: Some(active_branches.clone()),
                voting_window_days: Some(voting_window_days),
                stale_threshold_days: Some(stale_threshold_days),
            },
            blocker_label.clone(),
        );

        Self {
            github: github.clone(),
            task_controller,
            approval_manager: ReleaseProposalManager::new(
                GitHubClient::new(
                    github_owner.clone(),
                    github_repo.clone(),
                    GitHubAuth::new_with_default_api(&github_token),
                ),
                approval_category,
                voting_window,
                stale_threshold,
            ),
            executor: ReleaseExecutor::new(
                GitHubClient::new(
                    github_owner,
                    github_repo,
                    GitHubAuth::new_with_default_api(&github_token),
                ),
                true,
            ),
            detector,
            active_branches,
            blocker_label,
        }
    }

    /// Run the release manager cycle.
    ///
    /// This is the main entry point called by the scheduler on each run.
    pub async fn run(&mut self, state: &mut ReleaseState) -> Result<ReleaseRunResult> {
        tracing::info!("Starting release manager run");
        let start = Utc::now();
        let mut result = ReleaseRunResult::default();

        // 1. Detect release candidates
        let detection_result = self
            .detector
            .detect_candidates()
            .await?;

        for candidate in &detection_result.candidates {
            tracing::info!(
                "Release candidate: {} from {} ({} PRs, {} blockers, CI: {}, milestone: {})",
                candidate.version,
                match &candidate.source {
                    ReleaseSource::Main => "main",
                    ReleaseSource::Branch(b) => b.as_str(),
                },
                candidate.pr_count,
                candidate.blockers.len(),
                candidate.ci_green,
                candidate.milestone_set,
            );
        }
        result.candidates_found = detection_result.candidates.len();
        result.prs_checked = detection_result.prs_checked;

        // 2. For each candidate, check if we should propose a release
        let candidates: Vec<_> = self.active_branches.clone();

        for candidate in &detection_result.candidates {
            // Skip if CI is red
            if !candidate.ci_green {
                tracing::warn!(
                    "Skipping release {} from {}: CI is not green",
                    candidate.version,
                    candidate.source
                );
                result.ci_red_skips += 1;
                continue;
            }

            // Skip if no milestone is set
            if !candidate.milestone_set {
                tracing::info!(
                    "Skipping release {} from {}: no milestone set",
                    candidate.version,
                    candidate.source
                );
                result.no_milestone_skips += 1;
                continue;
            }

            // Skip if no PRs since last release
            if candidate.pr_count == 0 {
                tracing::info!(
                    "Skipping release {} from {}: no PRs since last release",
                    candidate.version,
                    candidate.source
                );
                result.no_prs_skips += 1;
                continue;
            }

            // Check if we already have a pending approval for this version
            if state.pending_approvals.contains_key(&candidate.version) {
                tracing::info!(
                    "Release {} already has a pending approval, skipping proposal",
                    candidate.version
                );
                result.pending_skips += 1;
                continue;
            }

            // Check if this is a new candidate we haven't seen
            if state.last_processed.as_deref() == Some(&candidate.version) {
                tracing::info!(
                    "Release {} was already processed, skipping",
                    candidate.version
                );
                result.already_processed_skips += 1;
                continue;
            }

            // Create a release proposal discussion
            let discussion_body = self.approval_manager.format_proposal_body(
                &candidate.version,
                &candidate.source.to_string(),
                candidate.pr_count,
                &candidate
                    .blockers
                    .iter()
                    .map(|b| format!("- Issue #{}: {} (reason: {})", b.issue_number, b.title, b.reason))
                    .collect::<Vec<_>>(),
                &[], // Issues list would be populated from milestone issues
                &[], // Breaking changes would be detected from diffs
                None, // Migration notes
            );

            let title = format!("[Release Proposal] {}", candidate.version);

            match self
                .approval_manager
                .create_proposal_discussion(&title, &discussion_body)
                .await
            {
                Ok((disc_num, disc_id)) => {
                    result.proposals_created += 1;
                    state.pending_approvals.insert(
                        candidate.version.clone(),
                        PendingApproval::new(
                            candidate.version.clone(),
                            disc_num,
                            disc_id,
                            candidate.source.to_string(),
                        ),
                    );
                    tracing::info!(
                        "Created release proposal discussion #{} for {}",
                        disc_num,
                        candidate.version
                    );
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to create release proposal for {}: {}",
                        candidate.version,
                        e
                    );
                    result.proposal_errors.push(e.to_string());
                }
            }

            // Update last processed
            state.last_processed = Some(candidate.version.clone());
        }

        // 3. Check pending approvals
        if !state.pending_approvals.is_empty() {
            let approval_entries: Vec<_> = self
                .check_pending_approvals(state)
                .await?
                .into_iter()
                .collect();

            result.approvals_checked = state.pending_approvals.len();

            // 4. Execute approved releases
            for (version, approval_result) in approval_entries {
                match approval_result {
                    ApprovalResult::Approved => {
                        if let Some(approval) = state.pending_approvals.remove(&version) {
                            // File a release task first
                            if let Some(candidate) = detection_result
                                .candidates
                                .iter()
                                .find(|c| c.version == version)
                            {
                                let task_request = self.executor
                                    .file_release_task(&version, &approval.source, candidate.pr_count);
                                if let Err(e) = self
                                    .task_controller
                                    .file_children("", vec![task_request])
                                    .await
                                {
                                    tracing::warn!("Failed to file release task: {}", e);
                                }
                            }

                            // Execute the release
                            let notification_body = self.approval_manager
                                .format_release_notification(&version, "main", &version, None);

                            match self
                                .executor
                                .execute(&version, "main", approval.discussion_number, &notification_body)
                                .await
                            {
                                Ok(exec_result) => {
                                    result.executions_succeeded += 1;
                                    result.released_versions.push(version.clone());
                                    if let Some(ref url) = exec_result.release_url {
                                        tracing::info!(
                                            "Release {} successful: {}",
                                            version,
                                            url
                                        );
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Release execution failed for {}: {}",
                                        version,
                                        e
                                    );
                                    result.execution_errors.push(e.to_string());
                                    // Re-add to pending for retry
                                    state.pending_approvals.insert(
                                        version.clone(),
                                        PendingApproval::new(
                                            version,
                                            approval.discussion_number,
                                            approval.discussion_id,
                                            approval.source,
                                        ),
                                    );
                                }
                            }
                        }
                    }
                    ApprovalResult::Rejected => {
                        // Mark rejection and remove from pending
                        tracing::info!(
                            "Release {} rejected by human",
                            version
                        );
                        state.pending_approvals.remove(&version);
                        result.rejections += 1;
                    }
                    _ => {
                        // Still pending or other status, do nothing
                    }
                }
            }

            // 5. Handle stale discussions
            self.handle_stale_discussions(state).await?;
        }

        result.elapsed_ms = (Utc::now() - start).num_milliseconds() as u64;
        state.last_run = Some(Utc::now());

        tracing::info!(
            "Release run complete: {} candidates, {} proposals, {} executions, {} stale",
            result.candidates_found,
            result.proposals_created,
            result.executions_succeeded,
            result.stale_handled,
        );

        Ok(result)
    }

    /// Check status of all pending approvals.
    async fn check_pending_approvals(
        &mut self,
        state: &mut ReleaseState,
    ) -> Result<HashMap<String, ApprovalResult>> {
        let mut results = HashMap::new();

        for (version, approval) in &state.pending_approvals {
            match self
                .approval_manager
                .check_approval(approval.discussion_number)
                .await
            {
                Ok(result) => {
                    if result != ApprovalResult::Pending {
                        results.insert(version.clone(), result);
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

    /// Handle stale discussions (send reminders or close).
    async fn handle_stale_discussions(&mut self, state: &mut ReleaseState) -> Result<()> {
        let now = Utc::now();
        let voting_window = Duration::days(self.approval_manager.voting_window_days() as i64);
        let stale_threshold = Duration::days(self.approval_manager.stale_threshold_days() as i64);

        // Collect stale version IDs first to avoid double mutable borrow
        let stale_ids: Vec<String> = state
            .pending_approvals
            .iter()
            .filter(|(_, approval)| {
                let age = now - approval.created_at;
                age > stale_threshold
            })
            .map(|(version, _)| version.clone())
            .collect();

        for version in stale_ids {
            tracing::info!(
                "Release {} proposal is stale, closing",
                version
            );
            if let Some(approval) = state.pending_approvals.remove(&version) {
                if let Err(e) = self.close_discussion(approval.discussion_number).await {
                    tracing::warn!("Failed to close discussion #{}: {}", approval.discussion_number, e);
                }
                if let Err(e) = self.file_revisit_task(&approval).await {
                    tracing::warn!("Failed to file revisit task for {}: {}", version, e);
                }
                tracing::info!("Stale release proposal for {} handled", version);
            }
        }

        // Collect versions needing reminders
        let to_remind: Vec<String> = state
            .pending_approvals
            .iter()
            .filter(|(_, approval)| {
                let age = now - approval.created_at;
                age > voting_window && approval.last_reminder_at.is_none()
            })
            .map(|(version, _)| version.clone())
            .collect();

        for version in to_remind {
            if let Some(approval) = state.pending_approvals.get(&version) {
                tracing::info!(
                    "Posting voting reminder for release {}",
                    version
                );
                let reminder_body = format!(
                    "## Release Reminder\n\nThe release proposal for **{}** has been waiting for approval.\n\n- **{}** → **{}**\n\nPlease react with 👍 to approve or 👎 to reject.\n\nThis discussion will be closed as stale if there is no response.",
                    version,
                    approval.source,
                    version
                );

                if let Err(e) = self
                    .post_discussion_comment(approval.discussion_number, &reminder_body)
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
        use crate::github::models::Discussion;

        let discussions = self
            .github
            .get_discussions(None, Some(100), None)
            .await?;

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
            struct Vars {
                id: String,
            }

            #[derive(serde::Deserialize)]
            struct UpdateResult {
                #[serde(rename = "updateDiscussion")]
                update_discussion: DiscClose,
            }

            #[derive(serde::Deserialize)]
            struct DiscClose {
                discussion: DiscId,
            }

            #[derive(serde::Deserialize)]
            struct DiscId {
                id: String,
            }

            let variables = Vars {
                id: disc.id.clone(),
            };

            let _: Option<crate::github::models::GraphQLResponse<UpdateResult>> = self
                .github
                .graphql(mutation, Some(variables))
                .await
                .ok();
        }

        Ok(())
    }

    /// File a revisit task for a stale release.
    async fn file_revisit_task(&self, approval: &PendingApproval) -> Result<()> {
        let request = crate::backlog::controller::CreateChildRequest {
            title: format!(
                "Revisit: release {} (stale proposal)",
                approval.version
            ),
            description: Some(format!(
                r#"Plan: plans/release-management-plan.md

The release proposal for **{}** was closed as stale.

SOURCE: {}
APPROVAL DISCUSSION: #{} (may be closed)

WHAT TO DO
Re-evaluate whether this release is still needed. If so, manually
trigger a new release proposal or close this task.
"#,
                approval.version,
                approval.source,
                approval.discussion_number,
            )),
            task_type: Some(task_type::CHORE.to_string()),
            rodgers_type: Some("release".to_string()),
            rodgers_labels: None,
            acceptance_criteria: Some(
                "- [ ] Release is evaluated and either proposed again or explicitly closed".to_string(),
            ),
            priority: Some(3), // Lower priority for stales
        };

        self.task_controller
            .file_children("", vec![request])
            .await?;

        Ok(())
    }

    /// Post a comment on a discussion.
    async fn post_discussion_comment(&mut self, discussion_number: i32, body: &str) -> Result<()> {
        use serde_json::json;

        let url = format!(
            "{}/repos/{}/{}/discussions/{}/comments",
            self.github.auth().api_url(),
            self.github.owner(),
            self.github.repo(),
            discussion_number
        );

        let request = self
            .github
            .client()
            .post(&url)
            .headers(self.github.auth().auth_headers())
            .json(&json!({ "body": body }));

        let response = request.send().await?;
        let status = response.status();
        if !status.is_success() {
            let text = response.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: text,
            });
        }

        Ok(())
    }

    /// Get the configured voting window in days.
    pub fn voting_window_days(&self) -> u32 {
        self.approval_manager.voting_window_days()
    }

    /// Get the configured stale threshold in days.
    pub fn stale_threshold_days(&self) -> u32 {
        self.approval_manager.stale_threshold_days()
    }
}

/// Result of a single release manager run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReleaseRunResult {
    /// Release candidates found in this run.
    pub candidates_found: usize,
    /// PRs evaluated during detection.
    pub prs_checked: usize,
    /// Release proposals created.
    pub proposals_created: usize,
    /// Errors creating proposals.
    #[serde(default)]
    pub proposal_errors: Vec<String>,
    /// Pending approvals checked.
    pub approvals_checked: usize,
    /// Successful executions.
    pub executions_succeeded: usize,
    /// Failed executions.
    #[serde(default)]
    pub execution_errors: Vec<String>,
    /// Rejected releases.
    pub rejections: usize,
    /// Stale discussions handled.
    pub stale_handled: usize,
    /// Versions that were released.
    #[serde(default)]
    pub released_versions: Vec<String>,
    /// Skips because CI was red.
    pub ci_red_skips: usize,
    /// Skips because no milestone was set.
    pub no_milestone_skips: usize,
    /// Skips because no PRs since last release.
    pub no_prs_skips: usize,
    /// Skips because already has a pending approval.
    pub pending_skips: usize,
    /// Skips because already processed.
    pub already_processed_skips: usize,
    /// Time elapsed in milliseconds.
    pub elapsed_ms: u64,
}

impl ReleaseRunResult {
    /// Whether any action was taken.
    pub fn took_action(&self) -> bool {
        self.proposals_created > 0
            || self.executions_succeeded > 0
            || self.rejections > 0
            || self.stale_handled > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_release_state_default() {
        let state = ReleaseState::default();
        assert!(state.last_run.is_none());
        assert!(state.last_processed.is_none());
        assert!(state.pending_approvals.is_empty());
    }

    #[test]
    fn test_pending_approval_new() {
        let approval = PendingApproval::new(
            "1.0.0".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "main".to_string(),
        );

        assert_eq!(approval.version, "1.0.0");
        assert_eq!(approval.discussion_number, 42);
        assert_eq!(approval.source, "main");
        assert!(approval.last_reminder_at.is_none());
    }

    #[test]
    fn test_pending_approval_into_approval() {
        let pending = PendingApproval::new(
            "1.0.0".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "main".to_string(),
        );

        let approval = pending.into_approval();
        assert_eq!(approval.version, "1.0.0");
        assert_eq!(approval.discussion_number, 42);
        assert_eq!(approval.source, "main");
        assert!(approval.is_pending());
    }

    #[test]
    fn test_release_run_result_took_action() {
        let mut result = ReleaseRunResult::default();
        assert!(!result.took_action());

        result.proposals_created = 1;
        assert!(result.took_action());

        result.proposals_created = 0;
        result.executions_succeeded = 1;
        assert!(result.took_action());

        result.executions_succeeded = 0;
        result.rejections = 1;
        assert!(result.took_action());

        result.rejections = 0;
        result.stale_handled = 1;
        assert!(result.took_action());

        result.stale_handled = 0;
        assert!(!result.took_action());
    }
}
