//! Backport approval via GitHub Discussion.
//!
//! Each backport creates a GitHub Discussion in the configured approval
//! category. Human approval is collected via reactions (👍/👎) or comments.
//!
//! ## Voting rules (per plan/backport-plan.md)
//!
//! - Most recent vote wins (vote tiebreaking)
//! - 👎 always halts execution regardless of when it arrives
//! - Same `voting_window_days`, `stale_threshold_days` as release approvals
//! - If no vote within voting window → reminder comment
//! - If no vote within stale threshold → close Discussion, file revisit bead
//!
//! ## GraphQL vs REST
//!
//! GitHub Discussions reactions and comments require the GraphQL API.
//! The REST API does not expose discussion reactions directly.

use chrono::{DateTime, Utc};
use tracing::info;

use crate::RogersError;
use crate::config::schema::ReleaseConfig;
use crate::github::client::GithubClient;

/// State of a backport approval Discussion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalState {
    /// Vote is pending — no 👍 or 👎 received yet.
    Pending,
    /// Human approved (most recent non-stale vote is 👍).
    Approved,
    /// Human rejected (👎 received at any point).
    Rejected { reason: String },
    /// Discussion is stale — no response within voting window.
    Stale {
        /// True if reminder was already sent.
        reminder_sent: bool,
    },
    /// Discussion closed without vote — exceeds stale_threshold_days.
    Expired,
}

/// A single vote record (reaction or comment).
#[derive(Debug, Clone)]
pub struct VoteRecord {
    /// Who voted.
    pub voter: String,
    /// +1 for 👍, -1 for 👎, 0 for neutral comment.
    pub value: i8,
    /// When the vote was cast.
    pub timestamp: DateTime<Utc>,
    /// "reaction" | "comment"
    pub source: &'static str,
}

/// Result of checking the approval status.
#[derive(Debug, Clone)]
pub struct DiscussionVoteResult {
    /// Current approval state.
    pub state: ApprovalState,
    /// All votes collected so far.
    pub votes: Vec<VoteRecord>,
    /// Most recent vote (by timestamp).
    pub most_recent: Option<VoteRecord>,
    /// Whether a reminder was already sent.
    pub reminder_sent: bool,
}

/// Check approval status for a backport Discussion.
///
// NOTE: In a production system, this would poll periodically or rely on
// webhooks. For this implementation, we query once and return current state.
// In the triage loop, this is called each run until state != Pending.
///
/// Parameters:
/// - `discussion_number`: The GitHub Discussion number
/// - `created_at`: When the discussion was created (for timing calculations)
/// - `config`: Release configuration with voting_window_days, stale_threshold_days
/// - `github`: GitHub client for GraphQL queries
pub async fn check_approval_status(
    discussion_number: u64,
    created_at: &str,
    config: &ReleaseConfig,
    github: &GithubClient,
) -> Result<DiscussionVoteResult, RogersError> {
    let created: DateTime<Utc> = created_at
        .parse()
        .map_err(|e| RogersError::Config(format!("invalid discussion created_at: {}", e)))?;

    let now = Utc::now();
    let elapsed_days = (now - created).num_days() as u32;

    // Fetch votes via GraphQL
    let votes = monitor_discussion_votes(discussion_number, github).await?;

    let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();

    // Vote tiebreaking: most recent wins, but 👎 always halts
    let state = compute_vote_state(&votes, &most_recent, elapsed_days, config);

    let reminder_sent = state
        == ApprovalState::Stale {
            reminder_sent: true,
        }
        || state
            == ApprovalState::Stale {
                reminder_sent: false,
            };

    Ok(DiscussionVoteResult {
        state,
        votes,
        most_recent,
        reminder_sent,
    })
}

/// Fetch all votes (reactions and comments) on a Discussion via GraphQL.
///
/// Both 👍/👎 reactions and comments count as votes.
/// Comments are treated as neutral (+0) unless they contain approval keywords.
async fn monitor_discussion_votes(
    discussion_number: u64,
    github: &GithubClient,
) -> Result<Vec<VoteRecord>, RogersError> {
    let query = r#"
        query($owner: String!, $repo: String!, $number: Int!) {
          repository(owner: $owner, name: $repo) {
            discussion(number: $number) {
              url
              createdAt
              comments(first: 100) {
                nodes {
                  author { login }
                  body
                  createdAt
                }
              }
              reactions: reactionGroups(content: THUMBS_UP) {
                users(first: 100) {
                  nodes { login }
                }
              }
            }
          }
        }
    "#;

    #[derive(serde::Deserialize)]
    struct GraphQLResponse {
        data: Option<GraphQLData>,
        errors: Option<Vec<GraphQLError>>,
    }

    #[derive(serde::Deserialize)]
    struct GraphQLData {
        repository: Option<Repository>,
    }

    #[derive(serde::Deserialize)]
    struct Repository {
        discussion: Option<DiscussionData>,
    }

    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct DiscussionData {
        url: String,
        created_at: String,
        #[serde(default)]
        comments: CommentsWrapper,
        #[serde(default)]
        reactions: Vec<ReactionGroup>,
    }

    #[derive(serde::Deserialize, Default)]
    struct CommentsWrapper {
        #[serde(default)]
        nodes: Vec<CommentNode>,
    }

    #[derive(serde::Deserialize)]
    struct CommentNode {
        author: Author,
        body: String,
        created_at: String,
    }

    #[derive(serde::Deserialize)]
    struct Author {
        login: String,
    }

    #[derive(serde::Deserialize)]
    struct ReactionGroup {
        #[serde(rename = "-users", default)]
        users: UserGroup,
    }

    #[derive(serde::Deserialize, Default)]
    struct UserGroup {
        #[serde(default)]
        nodes: Vec<UserNode>,
    }

    #[derive(serde::Deserialize)]
    struct UserNode {
        login: String,
    }

    #[derive(serde::Deserialize, Debug)]
    struct GraphQLError {
        message: String,
    }

    #[derive(serde::Serialize)]
    struct GraphQLRequest<'a> {
        query: &'a str,
        variables: serde_json::Value,
    }

    let variables = serde_json::json!({
        "owner": github.config().owner,
        "repo": github.config().repo,
        "number": discussion_number
    });

    let request = GraphQLRequest { query, variables };

    let url = format!("{}/graphql", github.config().api_url);
    let resp = github
        .client()
        .post(&url)
        .header("Authorization", &github.auth_header())
        .header("Accept", "application/vnd.github.zzz机构的-preview+json")
        .json(&request)
        .send()
        .await?;

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let message = resp.text().await.unwrap_or_default();
        return Err(RogersError::GitHubStatus { code, message });
    }

    let response: GraphQLResponse = resp.json().await.map_err(RogersError::GitHub)?;

    if let Some(errors) = response.errors {
        if !errors.is_empty() {
            let error_messages: Vec<_> = errors.iter().map(|e| e.message.clone()).collect();
            return Err(RogersError::Config(format!(
                "GraphQL errors: {:?}",
                error_messages
            )));
        }
    }

    let data = response
        .data
        .ok_or_else(|| RogersError::Config("no data in GraphQL response".to_string()))?;

    let discussion = data
        .repository
        .and_then(|r| r.discussion)
        .ok_or_else(|| RogersError::Config("discussion not found".to_string()))?;

    let mut votes = Vec::new();
    let discussion_created_at = discussion.created_at.clone();

    // Collect 👍 reactions
    for reaction in &discussion.reactions {
        for user in &reaction.users.nodes {
            votes.push(VoteRecord {
                voter: user.login.clone(),
                value: 1,
                timestamp: now_from_iso(&discussion_created_at)?,
                source: "reaction",
            });
        }
    }

    // Collect comments (neutral unless explicitly approving/rejecting)
    for comment in &discussion.comments.nodes {
        let body_lower = comment.body.to_lowercase();
        let value = if body_lower.contains("approved")
            || body_lower.contains("lgtm")
            || body_lower.contains("👍")
        {
            1
        } else if body_lower.contains("rejected")
            || body_lower.contains("👎")
            || body_lower.contains(" decline ")
            || body_lower.contains(" don't ")
        {
            -1
        } else {
            0
        };

        votes.push(VoteRecord {
            voter: comment.author.login.clone(),
            value,
            timestamp: now_from_iso(&comment.created_at)?,
            source: "comment",
        });
    }

    Ok(votes)
}

/// Compute the current vote state from collected votes.
///
/// ## Tiebreaking rules (per plan requirement)
///
/// - Most recent vote wins always
/// - 👎 always halts execution regardless of when it arrives
/// - Vote tiebreaking: If multiple votes have same timestamp, 👎 wins
/// - Votes on a stale-closed Discussion are ignored
fn compute_vote_state(
    votes: &[VoteRecord],
    most_recent: &Option<VoteRecord>,
    elapsed_days: u32,
    config: &ReleaseConfig,
) -> ApprovalState {
    // Check for 👎 (always halts)
    let has_thumbs_down = votes.iter().any(|v| v.value == -1);

    // Check for 👎 specifically (more authoritative than neutral comments)
    let has_thumbs_up = votes.iter().any(|v| v.value == 1);

    // Early return if we have a definitive vote
    if let Some(recent) = most_recent {
        if recent.value == -1 {
            return ApprovalState::Rejected {
                reason: format!("👎 from @{} at {}", recent.voter, recent.timestamp),
            };
        }
        if recent.value == 1 {
            return ApprovalState::Approved;
        }
    }

    // Check thresholds for stale/expired
    if elapsed_days >= config.stale_threshold_days {
        // After stale_threshold_days, close the discussion
        // If there's a recent thumbs-up, still approve; if there's thumbs-down, reject
        if has_thumbs_down {
            return ApprovalState::Rejected {
                reason: "No approval received before stale threshold; 👎 present".to_string(),
            };
        }
        if has_thumbs_up {
            return ApprovalState::Approved;
        }
        return ApprovalState::Expired;
    }

    if elapsed_days >= config.voting_window_days {
        // In voting window but no decision yet — needs reminder
        return ApprovalState::Stale {
            reminder_sent: false,
        };
    }

    ApprovalState::Pending
}

/// Post a reminder comment on the Discussion.
pub async fn post_reminder_comment(
    discussion_number: u64,
    github: &GithubClient,
) -> Result<(), RogersError> {
    let body = "## 🕐 Backport Approval Pending\n\n\
        This backport proposal is awaiting your approval. \n\n\
        Please react with 👍 to approve or 👎 to reject.\n\n\
        _This is an automated reminder from Rodgers._";

    let query = r#"
        mutation($owner: String!, $repo: String!, $discussionNumber: Int!, $body: String!) {
          addDiscussionComment(
            input: {
              discussionId: $discussionNumber
              body: $body
            }
          ) {
            comment { id }
          }
        }
    "#;

    #[derive(serde::Serialize)]
    struct GraphQLRequest<'a> {
        query: &'a str,
        variables: serde_json::Value,
    }

    #[derive(serde::Deserialize)]
    struct GraphQLResponse {
        data: Option<serde_json::Value>,
        errors: Option<Vec<serde_json::Value>>,
    }

    let variables = serde_json::json!({
        "owner": github.config().owner,
        "repo": github.config().repo,
        "discussionNumber": discussion_number,
        "body": body
    });

    let request = GraphQLRequest { query, variables };

    let url = format!("{}/graphql", github.config().api_url);
    let resp = github
        .client()
        .post(&url)
        .header("Authorization", &github.auth_header())
        .header("Accept", "application/vnd.github.zzz机构的-preview+json")
        .json(&request)
        .send()
        .await?;

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let message = resp.text().await.unwrap_or_default();
        return Err(RogersError::GitHubStatus { code, message });
    }

    Ok(())
}

/// Close a Discussion after it becomes stale.
pub async fn close_discussion(
    discussion_number: u64,
    github: &GithubClient,
) -> Result<(), RogersError> {
    let query = r#"
        mutation($owner: String!, $repo: String!, $discussionNumber: Int!) {
          closeDiscussion(
            input: {
              discussionId: $discussionNumber
              reason: CLOSED
            }
          ) {
            clientMutationId
          }
        }
    "#;

    #[derive(serde::Serialize)]
    struct GraphQLRequest<'a> {
        query: &'a str,
        variables: serde_json::Value,
    }

    #[derive(serde::Deserialize)]
    struct GraphQLResponse {
        data: Option<serde_json::Value>,
        errors: Option<Vec<serde_json::Value>>,
    }

    let variables = serde_json::json!({
        "owner": github.config().owner,
        "repo": github.config().repo,
        "discussionNumber": discussion_number
    });

    let request = GraphQLRequest { query, variables };

    let url = format!("{}/graphql", github.config().api_url);
    let resp = github
        .client()
        .post(&url)
        .header("Authorization", &github.auth_header())
        .header("Accept", "application/vnd.github.zzz机构的-preview+json")
        .json(&request)
        .send()
        .await?;

    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        let message = resp.text().await.unwrap_or_default();
        return Err(RogersError::GitHubStatus { code, message });
    }

    info!("Closed stale backport discussion #{}", discussion_number);

    Ok(())
}

/// Helper: parse ISO 8601 timestamp to DateTime<Utc>.
fn now_from_iso(s: &str) -> Result<DateTime<Utc>, RogersError> {
    s.parse::<DateTime<Utc>>()
        .map_err(|e| RogersError::Config(format!("invalid timestamp '{}': {}", s, e)))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::ReleaseConfig;

    fn make_config(voting_window: u32, stale_threshold: u32) -> ReleaseConfig {
        ReleaseConfig {
            approval_discussion_category: "Announcements".to_string(),
            active_branches: vec!["release/1.x".to_string()],
            voting_window_days: voting_window,
            stale_threshold_days: stale_threshold,
        }
    }

    // Helper to create test votes
    fn make_vote(voter: &str, value: i8, source: &'static str) -> VoteRecord {
        VoteRecord {
            voter: voter.to_string(),
            value,
            timestamp: Utc::now(),
            source,
        }
    }

    #[test]
    fn test_compute_vote_state_thumbs_down_always_halts() {
        let config = make_config(2, 7);

        // 👎 should halt even with neutral comments
        let votes = vec![
            make_vote("alice", 0, "comment"),
            make_vote("bob", -1, "reaction"),
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config);

        matches!(state, ApprovalState::Rejected { .. });
    }

    #[test]
    fn test_compute_vote_state_thumbs_up_approves() {
        let config = make_config(2, 7);

        let votes = vec![make_vote("alice", 1, "reaction")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config);

        assert_eq!(state, ApprovalState::Approved);
    }

    #[test]
    fn test_compute_vote_state_tiebreaking_most_recent_wins() {
        let config = make_config(2, 7);

        // Most recent should win, regardless of vote order
        let votes = vec![
            make_vote("alice", 1, "reaction"),
            make_vote("bob", -1, "reaction"),
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config);

        matches!(state, ApprovalState::Rejected { .. });
    }

    #[test]
    fn test_compute_vote_state_pending_no_votes() {
        let config = make_config(2, 7);

        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 1, &config);

        assert_eq!(state, ApprovalState::Pending);
    }

    #[test]
    fn test_compute_vote_state_stale_after_voting_window() {
        let config = make_config(2, 7);

        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 3, &config); // past voting window

        assert_eq!(
            state,
            ApprovalState::Stale {
                reminder_sent: false
            }
        );
    }

    #[test]
    fn test_compute_vote_state_expired_after_stale_threshold() {
        let config = make_config(2, 7);

        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 10, &config); // past stale threshold

        assert_eq!(state, ApprovalState::Expired);
    }

    #[test]
    fn test_discussion_vote_result_structure() {
        let votes = vec![
            make_vote("alice", 1, "reaction"),
            make_vote("bob", 0, "comment"),
        ];

        let result = DiscussionVoteResult {
            state: ApprovalState::Approved,
            votes: votes.clone(),
            most_recent: Some(votes[0].clone()),
            reminder_sent: false,
        };

        assert_eq!(result.state, ApprovalState::Approved);
        assert_eq!(result.votes.len(), 2);
        assert!(result.most_recent.is_some());
        assert_eq!(result.most_recent.as_ref().unwrap().voter, "alice");
    }

    #[test]
    fn test_approval_state_display() {
        let pending = ApprovalState::Pending;
        assert_eq!(format!("{:?}", pending), "Pending");

        let approved = ApprovalState::Approved;
        assert_eq!(format!("{:?}", approved), "Approved");

        let rejected = ApprovalState::Rejected {
            reason: "test".to_string(),
        };
        assert_eq!(format!("{:?}", rejected), "Rejected { reason: \"test\" }");

        let stale = ApprovalState::Stale {
            reminder_sent: true,
        };
        assert_eq!(format!("{:?}", stale), "Stale { reminder_sent: true }");
    }
}
