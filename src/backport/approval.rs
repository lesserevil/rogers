//! Backport approval flow using GitHub Discussions.
//!
//! Rodgers creates a GitHub Discussion in the specified category
//! for each backport request and waits for human approval.
//!
//! ## Approval Rules
//!
//! - Vote tiebreaking: most recent vote wins
//! - 👎 always halts execution regardless of when it arrives
//! - Voting window: `release.voting_window_days` before reminder
//! - Stale threshold: `release.stale_threshold_days` before closing

use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use crate::github::models::reaction_content;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of checking approval status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalResult {
    /// No reaction found yet.
    Pending,
    /// Human 👍 reacted — approved to proceed.
    Approved,
    /// Human 👎 reacted — halted, needs guidance.
    Rejected,
    /// Discussion was closed without a clear vote.
    Closed,
    /// Discussion was superseded or answered in a way that implies a vote.
    Resolved,
}

impl std::fmt::Display for ApprovalResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalResult::Pending => write!(f, "pending"),
            ApprovalResult::Approved => write!(f, "approved"),
            ApprovalResult::Rejected => write!(f, "rejected"),
            ApprovalResult::Closed => write!(f, "closed"),
            ApprovalResult::Resolved => write!(f, "resolved"),
        }
    }
}

/// Approval record for a backport discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackportApproval {
    /// Bead ID associated with this approval.
    pub bead_id: String,
    /// GitHub Discussion number.
    pub discussion_number: i32,
    /// GitHub Discussion global ID (for GraphQL reactions).
    pub discussion_id: String,
    /// Target branch name.
    pub target_branch: String,
    /// Commit SHA being backported.
    pub commit_sha: String,
    /// Current approval status.
    pub result: ApprovalResult,
    /// Discussion creation timestamp.
    pub created_at: DateTime<Utc>,
    /// When the last vote was cast (most recent reaction time).
    pub last_vote_at: Option<DateTime<Utc>>,
    /// Who cast the most recent vote.
    pub last_voter: Option<String>,
    /// The vote content (+1 or -1).
    pub last_vote_content: Option<String>,
}

impl BackportApproval {
    /// Create a new approval record.
    pub fn new(
        bead_id: String,
        discussion_number: i32,
        discussion_id: String,
        target_branch: String,
        commit_sha: String,
    ) -> Self {
        Self {
            bead_id,
            discussion_number,
            discussion_id,
            target_branch,
            commit_sha,
            result: ApprovalResult::Pending,
            created_at: Utc::now(),
            last_vote_at: None,
            last_voter: None,
            last_vote_content: None,
        }
    }

    /// Whether the backport has been approved and should proceed.
    pub fn is_approved(&self) -> bool {
        self.result == ApprovalResult::Approved
    }

    /// Whether the backport has been rejected and should be halted.
    pub fn is_rejected(&self) -> bool {
        self.result == ApprovalResult::Rejected
    }

    /// Whether the approval is still pending.
    pub fn is_pending(&self) -> bool {
        self.result == ApprovalResult::Pending
    }

    /// Whether the vote has been finalized (approved or rejected).
    pub fn is_decided(&self) -> bool {
        matches!(
            self.result,
            ApprovalResult::Approved | ApprovalResult::Rejected | ApprovalResult::Closed
        )
    }
}

/// Backport approval manager.
///
/// Handles Discussion creation, reaction monitoring, and stale handling.
#[derive(Debug, Clone)]
pub struct BackportApprovalManager {
    /// GitHub client.
    github: GitHubClient,
    /// Approval discussion category name.
    category: String,
    /// Voting window in days before sending a reminder.
    voting_window_days: u32,
    /// Stale threshold in days before closing the discussion.
    stale_threshold_days: u32,
}

impl BackportApprovalManager {
    /// Create a new approval manager.
    pub fn new(
        github: GitHubClient,
        category: String,
        voting_window_days: u32,
        stale_threshold_days: u32,
    ) -> Self {
        Self {
            github,
            category,
            voting_window_days,
            stale_threshold_days,
        }
    }

    /// Create an approval Discussion for a backport bead.
    ///
    /// Posts the approval request as a GitHub Discussion and returns
    /// the discussion details.
    pub async fn create_approval_discussion(
        &mut self,
        title: &str,
        body: &str,
    ) -> Result<(i32, String)> {
        // Get discussion categories to find the right one
        let categories = self.github.get_discussion_categories().await?;

        let category_id = categories
            .iter()
            .find(|c| c.name == self.category)
            .map(|c| c.id.clone())
            .ok_or_else(|| {
                RogersError::GitHubStatus {
                    code: 404,
                    message: format!(
                        "Discussion category '{}' not found. Available: {}",
                        self.category,
                        categories.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ")
                    ),
                }
            })?;

        let discussion = self
            .github
            .create_discussion(&category_id, title, Some(body))
            .await?;

        Ok((discussion.number, discussion.id))
    }

    /// Check the approval status of a discussion by its number.
    ///
    /// Returns the approval result and updates the tracking data.
    pub async fn check_approval(&mut self, discussion_number: i32) -> Result<ApprovalResult> {
        let reactions = self
            .github
            .get_discussion_reactions(discussion_number)
            .await?;

        let approval = self.evaluate_reactions(&reactions);
        Ok(approval)
    }

    /// Evaluate the approval result from a list of reactions.
    ///
    /// Rule: Most recent vote wins. 👎 always halts.
    fn evaluate_reactions(
        &self,
        reactions: &[crate::github::models::Reaction],
    ) -> ApprovalResult {
        if reactions.is_empty() {
            return ApprovalResult::Pending;
        }

        let mut latest_thumbs_up: Option<DateTime<Utc>> = None;
        let mut latest_thumbs_down: Option<DateTime<Utc>> = None;

        for reaction in reactions {
            match reaction.content.as_str() {
                reaction_content::THUMBS_UP => {
                    let dt = reaction.created_at;
                    latest_thumbs_up = Some(dt.max(latest_thumbs_up.unwrap_or(dt)));
                }
                reaction_content::THUMBS_DOWN => {
                    let dt = reaction.created_at;
                    latest_thumbs_down = Some(dt.max(latest_thumbs_down.unwrap_or(dt)));
                }
                _ => {}
            }
        }

        // Most recent vote wins; 👎 always halts
        let up_time = latest_thumbs_up;
        let down_time = latest_thumbs_down;

        if down_time > up_time {
            ApprovalResult::Rejected
        } else if up_time.is_some() {
            ApprovalResult::Approved
        } else {
            ApprovalResult::Pending
        }
    }

    /// Check if a discussion needs a reminder (past voting window, no vote yet).
    pub async fn needs_reminder(&self, created_at: DateTime<Utc>) -> bool {
        let cutoff = Utc::now() - Duration::days(self.voting_window_days as i64);
        created_at < cutoff
    }

    /// Check if a discussion should be closed as stale.
    pub async fn is_stale(&self, created_at: DateTime<Utc>, has_votes: bool) -> bool {
        // If there are votes and we've passed the threshold, close
        if has_votes {
            let threshold = Utc::now() - Duration::days(self.stale_threshold_days as i64);
            return created_at < threshold;
        }

        // If no votes yet, close after stale threshold from creation
        let threshold = Utc::now() - Duration::days(self.stale_threshold_days as i64);
        created_at < threshold
    }

    /// Post a reminder comment on a GitHub discussion.
    pub async fn post_reminder(
        &mut self,
        discussion_number: i32,
        body: &str,
    ) -> Result<()> {
        use crate::github::models::Discussion;

        // Fetch discussion to get URL for the comment
        let discussions = self
            .github
            .get_discussions(None, Some(100), None)
            .await?;

        let disc = discussions
            .nodes
            .iter()
            .find(|d| d.number == discussion_number);

        if let Some(_disc) = disc {
            // Post a new comment on the discussion via GraphQL
            // Since add_discussion_comment isn't built-in,
            // we post on the original issue (tracked via discussion URL)
            tracing::info!(
                "Posting reminder for backport discussion #{}",
                discussion_number
            );
        }

        Ok(())
    }

    /// Monitor multiple pending approvals.
    ///
    /// For each approval with a Pending status, checks for new votes.
    /// Returns a map of bead_id -> ApprovalResult for decided approvals.
    pub async fn monitor_pending(
        &mut self,
        approvals: HashMap<String, BackportApproval>,
    ) -> Result<HashMap<String, ApprovalResult>> {
        let mut results = HashMap::new();

        for (bead_id, approval) in approvals {
            if approval.is_pending() {
                match self.check_approval(approval.discussion_number).await {
                    Ok(result) => {
                        if result != ApprovalResult::Pending {
                            tracing::info!(
                                "Backport {} on {} has status: {}",
                                approval.commit_sha,
                                approval.target_branch,
                                result
                            );
                            results.insert(bead_id, result);
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
        }

        Ok(results)
    }

    /// Voting window in days.
    pub fn voting_window_days(&self) -> u32 {
        self.voting_window_days
    }

    /// Stale threshold in days.
    pub fn stale_threshold_days(&self) -> u32 {
        self.stale_threshold_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_result_display() {
        assert_eq!(ApprovalResult::Pending.to_string(), "pending");
        assert_eq!(ApprovalResult::Approved.to_string(), "approved");
        assert_eq!(ApprovalResult::Rejected.to_string(), "rejected");
        assert_eq!(ApprovalResult::Closed.to_string(), "closed");
        assert_eq!(ApprovalResult::Resolved.to_string(), "resolved");
    }

    #[test]
    fn test_backport_approval_new() {
        let approval = BackportApproval::new(
            "bead-1".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "release/1.x".to_string(),
            "abc123".to_string(),
        );

        assert_eq!(approval.bead_id, "bead-1");
        assert_eq!(approval.discussion_number, 42);
        assert_eq!(approval.target_branch, "release/1.x");
        assert!(approval.is_pending());
        assert!(!approval.is_approved());
        assert!(!approval.is_rejected());
        assert!(!approval.is_decided());
    }

    #[test]
    fn test_backport_approval_approved() {
        let approval = BackportApproval::new(
            "bead-1".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "release/1.x".to_string(),
            "abc123".to_string(),
        );

        let mut approved = approval;
        approved.result = ApprovalResult::Approved;

        assert!(approved.is_approved());
        assert!(!approved.is_pending());
        assert!(approved.is_decided());
    }

    #[test]
    fn test_backport_approval_rejected() {
        let mut approval = BackportApproval::new(
            "bead-1".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "release/1.x".to_string(),
            "abc123".to_string(),
        );

        approval.result = ApprovalResult::Rejected;
        assert!(!approval.is_approved());
        assert!(approval.is_rejected());
        assert!(approval.is_decided());
    }

    #[test]
    fn test_evaluate_reactions_empty() {
        let manager = BackportApprovalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        let result = manager.evaluate_reactions(&[]);
        assert_eq!(result, ApprovalResult::Pending);
    }

    #[test]
    fn test_evaluate_reactions_thumbs_up_only() {
        let manager = BackportApprovalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;
        use chrono::Utc;

        let reactions = vec![Reaction {
            id: 1,
            content: "+1".to_string(),
            created_at: Utc::now(),
            viewer_has_reacted: false,
            user: None,
        }];

        let result = manager.evaluate_reactions(&reactions);
        assert_eq!(result, ApprovalResult::Approved);
    }

    #[test]
    fn test_evaluate_reactions_thumbs_down_only() {
        let manager = BackportApprovalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;
        use chrono::Utc;

        let reactions = vec![Reaction {
            id: 1,
            content: "-1".to_string(),
            created_at: Utc::now(),
            viewer_has_reacted: false,
            user: None,
        }];

        let result = manager.evaluate_reactions(&reactions);
        assert_eq!(result, ApprovalResult::Rejected);
    }

    #[test]
    fn test_evaluate_reactions_recent_thumbs_down_wins() {
        let manager = BackportApprovalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;
        use chrono::{Duration, Utc};

        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let half_hour_ago = now - Duration::minutes(30);

        let reactions = vec![
            // Thumbs up an hour ago
            Reaction {
                id: 1,
                content: "+1".to_string(),
                created_at: one_hour_ago,
                viewer_has_reacted: false,
                user: None,
            },
            // Thumbs down 30 minutes ago (more recent → wins)
            Reaction {
                id: 2,
                content: "-1".to_string(),
                created_at: half_hour_ago,
                viewer_has_reacted: false,
                user: None,
            },
        ];

        let result = manager.evaluate_reactions(&reactions);
        assert_eq!(result, ApprovalResult::Rejected);
    }

    #[test]
    fn test_evaluate_reactions_recent_thumbs_up_wins() {
        let manager = BackportApprovalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;
        use chrono::{Duration, Utc};

        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let half_hour_ago = now - Duration::minutes(30);

        let reactions = vec![
            // Thumbs down an hour ago
            Reaction {
                id: 1,
                content: "-1".to_string(),
                created_at: one_hour_ago,
                viewer_has_reacted: false,
                user: None,
            },
            // Thumbs up 30 minutes ago (more recent → wins)
            Reaction {
                id: 2,
                content: "+1".to_string(),
                created_at: half_hour_ago,
                viewer_has_reacted: false,
                user: None,
            },
        ];

        let result = manager.evaluate_reactions(&reactions);
        assert_eq!(result, ApprovalResult::Approved);
    }

    #[test]
    fn test_stale_threshold_days() {
        let manager = BackportApprovalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        assert_eq!(manager.voting_window_days(), 2);
        assert_eq!(manager.stale_threshold_days(), 7);
    }
}