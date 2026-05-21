//! Release proposal and approval management.
//!
//! Handles creation of Release Proposal Discussions and evaluation of
//! human reactions (👍/👎) to determine if a release should proceed.
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

/// Result of checking release approval status.
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
    /// Discussion was answered (implies a decision).
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

/// An approval record for a release discussion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseApproval {
    /// Version being released.
    pub version: String,
    /// GitHub Discussion number.
    pub discussion_number: i32,
    /// GitHub Discussion global ID (for GraphQL reactions).
    pub discussion_id: String,
    /// Source branch of the release.
    pub source: String,
    /// Current approval status.
    pub result: ApprovalResult,
    /// Discussion creation timestamp.
    pub created_at: DateTime<Utc>,
    /// When the last vote was cast.
    pub last_vote_at: Option<DateTime<Utc>>,
    /// Who cast the most recent vote.
    pub last_voter: Option<String>,
    /// The vote content (+1 or -1).
    pub last_vote_content: Option<String>,
}

impl ReleaseApproval {
    /// Create a new approval record.
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
            result: ApprovalResult::Pending,
            created_at: Utc::now(),
            last_vote_at: None,
            last_voter: None,
            last_vote_content: None,
        }
    }

    /// Whether the release has been approved.
    pub fn is_approved(&self) -> bool {
        self.result == ApprovalResult::Approved
    }

    /// Whether the release has been rejected.
    pub fn is_rejected(&self) -> bool {
        self.result == ApprovalResult::Rejected
    }

    /// Whether the approval is still pending.
    pub fn is_pending(&self) -> bool {
        self.result == ApprovalResult::Pending
    }

    /// Whether the vote has been finalized.
    pub fn is_decided(&self) -> bool {
        matches!(
            self.result,
            ApprovalResult::Approved
                | ApprovalResult::Rejected
                | ApprovalResult::Closed
                | ApprovalResult::Resolved
        )
    }
}

/// Release proposal manager.
///
/// Handles Discussion creation, reaction monitoring, and stale handling.
#[derive(Debug, Clone)]
pub struct ReleaseProposalManager {
    /// GitHub client.
    github: GitHubClient,
    /// Approval discussion category name.
    category: String,
    /// Voting window in days before sending a reminder.
    voting_window_days: u32,
    /// Stale threshold in days before closing the discussion.
    stale_threshold_days: u32,
}

impl ReleaseProposalManager {
    /// Create a new proposal manager.
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

    /// Create a release proposal discussion.
    ///
    /// Returns the discussion number and ID.
    pub async fn create_proposal_discussion(
        &mut self,
        title: &str,
        body: &str,
    ) -> Result<(i32, String)> {
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

    /// Check the approval status of a discussion.
    pub async fn check_approval(&mut self, discussion_number: i32) -> Result<ApprovalResult> {
        let reactions = self
            .github
            .get_discussion_reactions(discussion_number)
            .await?;

        let result = self.evaluate_reactions(&reactions);

        // Check if discussion is closed
        if result == ApprovalResult::Pending {
            // Check discussion state via GraphQL
            if self.is_discussion_closed(discussion_number).await? {
                return Ok(ApprovalResult::Closed);
            }
        }

        Ok(result)
    }

    /// Evaluate approval from reactions.
    ///
    /// Most recent vote wins. 👎 always halts.
    fn evaluate_reactions(
        &self,
        reactions: &[crate::github::models::Reaction],
    ) -> ApprovalResult {
        if reactions.is_empty() {
            return ApprovalResult::Pending;
        }

        let mut latest_up: Option<DateTime<Utc>> = None;
        let mut latest_down: Option<DateTime<Utc>> = None;

        for reaction in reactions {
            match reaction.content.as_str() {
                reaction_content::THUMBS_UP => {
                    let dt = reaction.created_at;
                    latest_up = Some(dt.max(latest_up.unwrap_or(dt)));
                }
                reaction_content::THUMBS_DOWN => {
                    let dt = reaction.created_at;
                    latest_down = Some(dt.max(latest_down.unwrap_or(dt)));
                }
                _ => {}
            }
        }

        let up_time = latest_up;
        let down_time = latest_down;

        if down_time > up_time {
            ApprovalResult::Rejected
        } else if up_time.is_some() {
            ApprovalResult::Approved
        } else {
            ApprovalResult::Pending
        }
    }

    /// Check if a discussion is closed via GraphQL.
    async fn is_discussion_closed(&mut self, discussion_number: i32) -> Result<bool> {
        use crate::github::models::GraphQLResponse;
        use serde::Serialize;

        let query = r#"
            query($owner: String!, $repo: String!, $number: Int!) {
                repository(owner: $owner, name: $repo) {
                    discussion(number: $number) {
                        state
                    }
                }
            }
        "#;

        #[derive(Serialize)]
        struct Vars {
            owner: String,
            repo: String,
            number: i32,
        }

        #[derive(serde::Deserialize)]
        struct RepoDisc {
            repository: DiscRepo,
        }

        #[derive(serde::Deserialize)]
        struct DiscRepo {
            discussion: Option<DiscState>,
        }

        #[derive(serde::Deserialize)]
        struct DiscState {
            state: String,
        }

        let variables = Vars {
            owner: self.github.owner().to_string(),
            repo: self.github.repo().to_string(),
            number: discussion_number,
        };

        let response: GraphQLResponse<RepoDisc> = self.github.graphql(query, Some(variables)).await?;

        Ok(response
            .data
            .and_then(|d| d.repository.discussion)
            .map(|d| d.state == "CLOSED")
            .unwrap_or(false))
    }

    /// Check if a discussion needs a reminder.
    pub async fn needs_reminder(&self, created_at: DateTime<Utc>) -> bool {
        let cutoff = Utc::now() - Duration::days(self.voting_window_days as i64);
        created_at < cutoff
    }

    /// Check if a discussion should be closed as stale.
    pub async fn is_stale(&self, created_at: DateTime<Utc>) -> bool {
        let threshold = Utc::now() - Duration::days(self.stale_threshold_days as i64);
        created_at < threshold
    }

    /// Format the body of a release proposal discussion.
    pub fn format_proposal_body(
        &self,
        version: &str,
        source: &str,
        pr_count: usize,
        blockers: &[String],
        issues: &[String],
        breaking_changes: &[String],
        migration_notes: Option<&str>,
    ) -> String {
        let blockers_section = if blockers.is_empty() {
            "None".to_string()
        } else {
            blockers.join("\n")
        };

        let issues_section = if issues.is_empty() {
            "None listed".to_string()
        } else {
            issues.join("\n")
        };

        let breaking_section = if breaking_changes.is_empty() {
            "None".to_string()
        } else {
            breaking_changes.join("\n")
        };

        let migration_section = migration_notes
            .map(|n| n.to_string())
            .unwrap_or_else(|| "None".to_string());

        format!(
            r#"## Release {version}

**Proposed by:** Rodgers
**Source:** {source}
**Commits since last release:** {pr_count} merged PRs

### Issues in this release

{issues}

### Breaking Changes

{breaking}

### Migration Notes

{migration}

### Potential Blockers

{blockers}

### Vote

React with 👍 to approve, 👎 to reject.  
Release will be cut within one triage run of approval unless vetoed.
"#,
            version = version,
            source = source,
            pr_count = pr_count,
            issues = issues_section,
            breaking = breaking_section,
            migration = migration_section,
            blockers = blockers_section,
        )
    }

    /// Format a post-release notification comment.
    pub fn format_release_notification(
        &self,
        version: &str,
        branch_name: &str,
        tag_name: &str,
        release_url: Option<&str>,
    ) -> String {
        format!(
            "Release {version} has been cut: branch `{branch_name}` created, tag `{tag_name}` created{release_note}.",
            release_note = release_url
                .map(|url| format!(", [link to release]({url})"))
                .unwrap_or_default()
        )
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
    fn test_release_approval_new() {
        let approval = ReleaseApproval::new(
            "1.0.0".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "main".to_string(),
        );

        assert_eq!(approval.version, "1.0.0");
        assert_eq!(approval.discussion_number, 42);
        assert_eq!(approval.source, "main");
        assert!(approval.is_pending());
        assert!(!approval.is_approved());
        assert!(!approval.is_rejected());
        assert!(!approval.is_decided());
    }

    #[test]
    fn test_release_approval_approved() {
        let mut approval = ReleaseApproval::new(
            "1.0.0".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "main".to_string(),
        );
        approval.result = ApprovalResult::Approved;

        assert!(approval.is_approved());
        assert!(!approval.is_pending());
        assert!(approval.is_decided());
    }

    #[test]
    fn test_release_approval_rejected() {
        let mut approval = ReleaseApproval::new(
            "1.0.0".to_string(),
            42,
            "gid://github/Discussion/123".to_string(),
            "main".to_string(),
        );
        approval.result = ApprovalResult::Rejected;

        assert!(approval.is_rejected());
        assert!(!approval.is_approved());
        assert!(approval.is_decided());
    }

    #[test]
    fn test_evaluate_reactions_empty() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        let result = manager.evaluate_reactions(&[]);
        assert_eq!(result, ApprovalResult::Pending);
    }

    #[test]
    fn test_evaluate_reactions_thumbs_up() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;

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
    fn test_evaluate_reactions_thumbs_down() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;

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
    fn test_evaluate_reactions_recent_down_wins() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;
        use chrono::Duration;

        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let half_hour_ago = now - Duration::minutes(30);

        let reactions = vec![
            Reaction {
                id: 1,
                content: "+1".to_string(),
                created_at: one_hour_ago,
                viewer_has_reacted: false,
                user: None,
            },
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
    fn test_evaluate_reactions_recent_up_wins() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        use crate::github::models::Reaction;
        use chrono::Duration;

        let now = Utc::now();
        let one_hour_ago = now - Duration::hours(1);
        let half_hour_ago = now - Duration::minutes(30);

        let reactions = vec![
            Reaction {
                id: 1,
                content: "-1".to_string(),
                created_at: one_hour_ago,
                viewer_has_reacted: false,
                user: None,
            },
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
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        assert_eq!(manager.voting_window_days(), 2);
        assert_eq!(manager.stale_threshold_days(), 7);
    }

    #[test]
    fn test_format_proposal_body() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        let body = manager.format_proposal_body(
            "1.0.0",
            "main",
            5,
            &["Issue #123: Critical bug".to_string()],
            &["#123".to_string(), "#124".to_string()],
            &[],
            Some("Upgrade Rust to 1.75"),
        );

        assert!(body.contains("Release 1.0.0"));
        assert!(body.contains("main"));
        assert!(body.contains("5 merged PRs"));
        assert!(body.contains("Issue #123"));
        assert!(body.contains("#123"));
        assert!(body.contains("Upgrade Rust to 1.75"));
        assert!(body.contains("👍"));
        assert!(body.contains("👎"));
    }

    #[test]
    fn test_format_proposal_body_empty() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        let body = manager.format_proposal_body(
            "0.1.0",
            "main",
            3,
            &[],
            &[],
            &[],
            None,
        );

        assert!(body.contains("Release 0.1.0"));
        assert!(body.contains("None"));
    }

    #[test]
    fn test_format_release_notification() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        let notification = manager.format_release_notification(
            "1.0.0",
            "release/1.0.0",
            "v1.0.0",
            Some("https://github.com/test/releases/tag/v1.0.0"),
        );

        assert!(notification.contains("Release 1.0.0"));
        assert!(notification.contains("release/1.0.0"));
        assert!(notification.contains("v1.0.0"));
        assert!(notification.contains("link to release"));
    }

    #[test]
    fn test_format_release_notification_no_url() {
        let manager = ReleaseProposalManager::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            "Announcements".to_string(),
            2,
            7,
        );

        let notification = manager.format_release_notification(
            "1.0.0",
            "release/1.0.0",
            "v1.0.0",
            None,
        );

        assert!(notification.contains("Release 1.0.0"));
        assert!(!notification.contains("link to release"));
    }
}
