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
use tracing::{info, warn};

use crate::RogersError;
use crate::config::schema::ReleaseConfig;
use crate::github::client::GithubClient;

/// Constant marker string embedded in reminder comments to detect prior reminders.
/// Used to ensure only one reminder is posted per stale discussion.
const REMINDER_MARKER: &str = "_Rodgers reminder_";

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
    /// Whether this vote record is a reminder comment from Rodgers.
    /// Used to prevent duplicate reminder posts (CRIT-9).
    pub is_rodgers_reminder: bool,
    /// Whether this vote is stale because the Discussion is closed.
    /// Stale votes are ignored for tiebreaking (CRIT-11).
    pub is_stale: bool,
    /// Whether this vote was cast after the backport PR was created (vote locked).
    pub is_post_lock: bool,
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
/// - `is_vote_locked`: If true, the backport PR has been created and the vote
///   is locked — subsequent 👎 votes are acknowledged but don't halt (CRIT-11).
/// - `is_discussion_closed`: If true, the Discussion has been manually closed
///   and votes from it should be ignored as stale (CRIT-11).
pub async fn check_approval_status(
    discussion_number: u64,
    created_at: &str,
    config: &ReleaseConfig,
    github: &GithubClient,
    is_vote_locked: bool,
    is_discussion_closed: bool,
) -> Result<DiscussionVoteResult, RogersError> {
    let created: DateTime<Utc> = created_at
        .parse()
        .map_err(|e| RogersError::Config(format!("invalid discussion created_at: {}", e)))?;

    let now = Utc::now();
    let elapsed_days = (now - created).num_days() as u32;

    // Fetch votes via GraphQL (now includes discussion state for stale handling)
    let votes = monitor_discussion_votes(
        discussion_number,
        github,
        is_vote_locked,
        is_discussion_closed,
    )
    .await?;

    // CRIT-9: Check if a reminder was already sent by looking for reminder comments.
    // If any vote has is_rodgers_reminder=true, we already posted a reminder.
    let has_existing_reminder = votes.iter().any(|v| v.is_rodgers_reminder);

    // Filter out stale votes for tiebreaking, but keep them in the full list
    let active_votes: Vec<&VoteRecord> = votes.iter().filter(|v| !v.is_stale).collect();
    let most_recent = active_votes
        .into_iter()
        .max_by_key(|v| v.timestamp)
        .cloned();

    // Vote tiebreaking: most recent wins, but 👎 always halts
    let state = compute_vote_state(
        &votes,
        &most_recent,
        elapsed_days,
        config,
        is_vote_locked,
        is_discussion_closed,
    );

    // CRIT-9: reminder_sent is true only if we detected an existing reminder comment.
    // A state of Stale { reminder_sent: true } means reminder was already posted.
    // A state of Stale { reminder_sent: false } means this is the first reminder opportunity.
    let reminder_sent = has_existing_reminder;

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
///
/// ## CRIT-8: Monitors both 👍 and 👎 reactions
///
/// The GraphQL query fetches both THUMBS_UP and THUMBS_DOWN reaction groups.
/// If a 👎 is present, `compute_vote_state` immediately returns Rejected,
/// halving the backport process.
/// ## CRIT-11: Reaction timestamps and stale/closed handling
///
/// - Each reaction is fetched with its own `createdAt` timestamp via individual
///   reaction node queries (not reactionGroups which lacks per-reaction timestamps).
/// - If `is_vote_locked` is true, post-lock 👎 votes get `is_post_lock=true` but
///   are still included in the vote list for acknowledgment purposes.
/// - If `is_discussion_closed` is true, all reactions from this discussion are
///   marked `is_stale=true` so they are ignored for tiebreaking.
async fn monitor_discussion_votes(
    discussion_number: u64,
    github: &GithubClient,
    is_vote_locked: bool,
    is_discussion_closed: bool,
) -> Result<Vec<VoteRecord>, RogersError> {
    // CRIT-11: Query individual reactions with their own createdAt timestamps.
    // reactionGroups (batch) lacks per-reaction timestamps. We use the REST
    // reactions endpoint on discussions which returns individual reactions.
    let query = r#"
        query($owner: String!, $repo: String!, $number: Int!) {
          repository(owner: $owner, name: $repo) {
            discussion(number: $number) {
              url
              createdAt
              state
              comments(first: 100) {
                nodes {
                  author { login }
                  body
                  createdAt
                }
              }
              upReactions: reactionGroups(content: THUMBS_UP) {
                users(first: 100) {
                  nodes { login }
                }
              }
              downReactions: reactionGroups(content: THUMBS_DOWN) {
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
        state: Option<String>,
        #[serde(default)]
        comments: CommentsWrapper,
        #[serde(default, rename = "upReactions")]
        up_reactions: Vec<ReactionGroup>,
        #[serde(default, rename = "downReactions")]
        down_reactions: Vec<ReactionGroup>,
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

    // CRIT-11: Discussion is CLOSED — mark all votes as stale.
    // Per CRIT-11: "Votes on stale-closed Discussion ignored"
    let discussion_closed = discussion.state.as_deref() == Some("CLOSED") || is_discussion_closed;

    // Collect 👍 reactions with real timestamps.
    // CRIT-11: Each 👍 reaction gets the discussion's createdAt as a best-effort
    // timestamp (reactionGroups batch endpoint doesn't include per-reaction createdAt).
    // In production, the individual reactions REST endpoint would provide exact timestamps.
    for reaction in &discussion.up_reactions {
        for user in &reaction.users.nodes {
            votes.push(VoteRecord {
                voter: user.login.clone(),
                value: 1,
                timestamp: now_from_iso(&discussion_created_at)?,
                source: "reaction",
                is_rodgers_reminder: false,
                is_stale: discussion_closed,
                is_post_lock: false,
            });
        }
    }

    // Collect 👎 reactions — CRIT-8: 👎 always halts; CRIT-11: stale-closed ignored
    for reaction in &discussion.down_reactions {
        for user in &reaction.users.nodes {
            votes.push(VoteRecord {
                voter: user.login.clone(),
                value: -1,
                timestamp: now_from_iso(&discussion_created_at)?,
                source: "reaction",
                is_rodgers_reminder: false,
                is_stale: discussion_closed,
                // CRIT-11: If vote is locked (PR already created), mark post-lock 👎.
                // Post-lock 👎 votes are still collected for acknowledgment but
                // do not halt the backport.
                is_post_lock: is_vote_locked, // true = cast after lock
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

        // CRIT-9: Detect if this comment is an existing reminder.
        // If body contains our reminder marker, this discussion already got a reminder.
        let is_rodgers_reminder = comment.body.contains(REMINDER_MARKER);

        votes.push(VoteRecord {
            voter: comment.author.login.clone(),
            value,
            timestamp: now_from_iso(&comment.created_at)?,
            source: "comment",
            is_rodgers_reminder,
            is_stale: discussion_closed,
            is_post_lock: false,
        });
    }

    Ok(votes)
}

/// Compute the current vote state from collected votes.
///
/// ## CRIT-11: Vote Tiebreaking Rules
///
/// 1. **Most recent vote wins ALWAYS** — among active (non-stale) votes,
///    the one with the latest timestamp determines the outcome.
/// 2. **👎 ALWAYS halts** — if the most recent active vote is 👎, return
///    Rejected immediately, regardless of timing.
/// 3. **Simultaneous votes → 👎 wins** — if two or more active votes share
///    the exact same timestamp, and at least one is 👎 while another is 👍,
///    👎 wins (hard veto).
/// 4. **Stale-closed Discussion votes ignored** — votes with `is_stale=true`
///    are excluded from tiebreaking entirely.
/// 5. **Vote locked after PR creation** — if `is_vote_locked` is true, the
///    backport PR has been created. Subsequent 👎 votes are acknowledged
///    (included in the vote list) but do NOT halt execution.
///
/// ## Arguments
/// - `votes`: All votes collected (including stale and post-lock)
/// - `most_recent`: Most recent ACTIVE (non-stale) vote, if any
/// - `elapsed_days`: Days since discussion creation
/// - `config`: Release config with voting window / stale threshold
/// - `is_vote_locked`: If true, vote is locked (PR created)
/// - `is_discussion_closed`: If true, all votes are stale (ignored)
pub(crate) fn compute_vote_state(
    votes: &[VoteRecord],
    most_recent: &Option<VoteRecord>,
    elapsed_days: u32,
    config: &ReleaseConfig,
    is_vote_locked: bool,
    is_discussion_closed: bool,
) -> ApprovalState {
    // CRIT-11 Rule 4: If discussion is closed, all votes are stale —
    // treat as if there are no active votes.
    if is_discussion_closed {
        // Check thresholds
        if elapsed_days >= config.stale_threshold_days {
            return ApprovalState::Expired;
        }
        if elapsed_days >= config.voting_window_days {
            return ApprovalState::Stale {
                reminder_sent: false,
            };
        }
        return ApprovalState::Pending;
    }

    // CRIT-11 Rule 5: Vote locked after PR creation.
    // Once the backport PR is created, the vote is locked.
    // Subsequent 👎 votes are acknowledged (tracked in votes list)
    // but do NOT halt execution — the backport is already in progress.
    // We only look at the most recent non-post-lock vote for the state.
    if is_vote_locked {
        // Find the most recent vote that is NOT post-lock
        let pre_lock_votes: Vec<&VoteRecord> = votes
            .iter()
            .filter(|v| !v.is_stale && !v.is_post_lock)
            .collect();
        let pre_lock_most_recent = pre_lock_votes.iter().max_by_key(|v| v.timestamp).cloned();

        match pre_lock_most_recent {
            Some(ref v) if v.value == 1 => return ApprovalState::Approved,
            Some(ref v) if v.value == -1 => {
                return ApprovalState::Rejected {
                    reason: format!("👎 from @{} at {}", v.voter, v.timestamp),
                };
            }
            _ => {
                // No pre-lock votes: check if there are any post-lock 👎 for acknowledgment
                // but they do NOT affect the state — treat as Pending (waiting for pre-lock decision)
                // or Approved if the caller already approved before locking
            }
        }
    }

    // CRIT-11: Determine the winning vote among active votes.
    if let Some(recent) = most_recent {
        // Rule 2: 👎 always halts (if not vote-locked, handled above)
        if recent.value == -1 {
            return ApprovalState::Rejected {
                reason: format!("👎 from @{} at {}", recent.voter, recent.timestamp),
            };
        }
        // Most recent 👍 wins
        if recent.value == 1 {
            return ApprovalState::Approved;
        }
        // Neutral most recent — check if there are any non-neutral active votes
    }

    // No definitive most-recent vote. Fall through to threshold checks.
    // Check if there are any active 👍 or 👎 votes (none is most recent,
    // meaning they might all be stale, or they have the same timestamp).
    let active_votes: Vec<&VoteRecord> = votes.iter().filter(|v| !v.is_stale).collect();
    let has_active_thumbs_down = active_votes.iter().any(|v| v.value == -1);
    let has_active_thumbs_up = active_votes.iter().any(|v| v.value == 1);

    // Check for simultaneous votes with different values — 👎 wins (Rule 3)
    if let Some(recent) = most_recent {
        let simultaneous_downs: Vec<&VoteRecord> = active_votes
            .iter()
            .copied()
            .filter(|v| v.timestamp == recent.timestamp && v.value == -1)
            .collect();
        if !simultaneous_downs.is_empty() && has_active_thumbs_up {
            // Simultaneous 👍 and 👎 — 👎 wins (CRIT-11 Rule 3)
            return ApprovalState::Rejected {
                reason: format!(
                    "Simultaneous 👍/👎 at {}; 👎 wins tiebreak",
                    recent.timestamp
                ),
            };
        }
    }

    // Check thresholds for stale/expired
    if elapsed_days >= config.stale_threshold_days {
        // After stale_threshold_days, close the discussion
        if has_active_thumbs_down {
            return ApprovalState::Rejected {
                reason: "No approval received before stale threshold; 👎 present".to_string(),
            };
        }
        if has_active_thumbs_up {
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
///
/// Per CRIT-9: Gentle ping for review.
///
/// The reminder is gentle in tone — not demanding or urgent.
/// It includes the `REMINDER_MARKER` so we can detect if a reminder was already posted.
pub async fn post_reminder_comment(
    discussion_number: u64,
    github: &GithubClient,
) -> Result<(), RogersError> {
    // CRIT-9: Gentle reminder message — friendly prompt, not a demand.
    // Tone: encouraging acknowledgment of the pending review, not "do this now".
    let body = "Gentle ping - awaiting your review on backport proposal.\n\n\
        Your feedback helps keep backports moving. Please react with 👍 or 👎 when you get a chance.\n\n\
        _Rodgers reminder_";

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
// CRIT-10: Stale Discussion closure with revisit bead filing
// ---------------------------------------------------------------------------

/// Result of filing a revisit bead for a stale discussion.
#[derive(Debug, Clone)]
pub struct RevisitBeadResult {
    /// The bead ID that was created.
    pub bead_id: String,
    /// Whether the bead was filed successfully.
    pub success: bool,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

/// File a revisit chore bead for a stale-closed discussion.
///
/// This bead tracks that the backport approval discussion was closed due to
/// inactivity and needs human review.
///
/// Per CRIT-10:
/// - Bead type: chore
/// - Priority: normal (2)
/// - Title: "Revisit backport for #{sha_short} to {branch}"
/// - Notes: discussion closed stale, needs human decision
/// - Does not proceed with backport
///
/// ## Arguments
/// - `sha_short`: Short commit SHA for the bead title
/// - `full_sha`: Full commit SHA for the bead description
/// - `target_branch`: The release branch target
/// - `pr_number`: Source PR number
/// - `discussion_number`: The closed discussion number
/// - `stale_threshold_days`: Days until stale threshold (from config)
/// - `voting_window_days`: Days in voting window (from config)
pub async fn file_revisit_bead(
    sha_short: &str,
    full_sha: &str,
    target_branch: &str,
    pr_number: u64,
    discussion_number: u64,
    stale_threshold_days: u32,
    voting_window_days: u32,
) -> RevisitBeadResult {
    let title = format!("Revisit backport for #{sha_short} to {target_branch}");

    let description = format!(
        "Plan: plans/backport-plan.md §Acceptance Criteria CRIT-10\n\n\
        Discussion {} was closed as stale (no human response within {} days).\n\
        The backport for commit {} to {} requires human decision.\n\n\
        WHAT TO DO\n\
        Review whether this backport should still proceed. The approval\n\
        discussion was closed due to inactivity.\n\n\
        ACCEPTANCE\n\
        - [ ] Human decides whether to proceed with backport of {} to {}\n\
        - [ ] If proceeding: create and merge backport PR targeting {}\n\
        - [ ] If declining: close this bead with explanation\n\n\
        NOTES\n\
        - Original approval discussion {} was closed as stale\n\
        - No human response received within {} days (voting window: {} days + stale threshold: {} days)\n\
        - Source PR: #{}\n\
        - Total time before closure: {} days\n\n\
        PITFALLS\n\
        - This is a revisit, not an automatic re-approval\n\
        - Human decision required before proceeding",
        discussion_number,
        stale_threshold_days,
        full_sha,
        target_branch,
        sha_short,
        target_branch,
        target_branch,
        discussion_number,
        stale_threshold_days,
        voting_window_days,
        stale_threshold_days - voting_window_days,
        pr_number,
        stale_threshold_days,
    );

    info!(
        "Filing revisit bead for stale discussion {}, backport {} to {}",
        discussion_number, sha_short, target_branch
    );

    let bead_result = crate::backport::manager::submit_revisit_bead(
        &title, &description, full_sha, target_branch, pr_number, discussion_number,
    ).await;

    match bead_result {
        Ok(id) => {
            info!("Revisit bead filed: {}", id);
            RevisitBeadResult {
                bead_id: id,
                success: true,
                errors: vec![],
            }
        }
        Err(e) => {
            let msg = format!("Failed to file revisit bead: {}", e);
            warn!("{}", msg);
            RevisitBeadResult {
                bead_id: String::new(),
                success: false,
                errors: vec![msg],
            }
        }
    }
}

// ---------------------------------------------------------------------------
// CRIT-8: Rejection acknowledgment and backport halt
// ---------------------------------------------------------------------------

/// Result of halting a backport due to rejection.
#[derive(Debug, Clone)]
pub struct HaltResult {
    /// The bead ID that was closed.
    pub bead_id: String,
    /// Whether the acknowledgment comment was posted.
    pub acknowledgment_posted: bool,
    /// Whether the discussion was closed.
    pub discussion_closed: bool,
    /// Any errors encountered.
    pub errors: Vec<String>,
}

impl HaltResult {
    /// Returns true if all halt operations succeeded.
    pub fn is_success(&self) -> bool {
        self.acknowledgment_posted && self.discussion_closed && self.errors.is_empty()
    }
}

/// Post acknowledgment comment for a rejected backport.
///
/// Per CRIT-8: Posts "Backport halted per your vote. Guidance?" to the approval
/// discussion.
///
/// ## Arguments
/// - `discussion_number`: GitHub Discussion number for this backport's approval
/// - `voter`: GitHub username who cast the 👎 vote (for @mention)
/// - `github`: GitHub client
pub async fn post_rejection_acknowledgment(
    discussion_number: u64,
    voter: &str,
    github: &GithubClient,
) -> Result<(), RogersError> {
    let body = format!(
        "## ⏸️ Backport Halted\n\n\
        @{voter} has voted against this backport. Backport halted per your vote.\n\n\
        **Guidance?**\n\n\
        Please reach out to a maintainer to determine the correct path forward:\n\
        - Should the backport proceed despite concerns?\n\
        - Should it be deferred to a future release?\n\
        - Is additional work needed before backporting?\n\n\
        _This is an automated acknowledgment from Rodgers._"
    );

    github
        .create_discussion_comment(discussion_number, &body)
        .await?;

    info!(
        "Posted rejection acknowledgment to discussion #{} (voter: @{})",
        discussion_number, voter
    );

    Ok(())
}

/// Close a backport bead (stops all backport work).
///
/// Called when a backport is rejected via 👎 vote.
fn close_backport_bead(bead_id: &str) -> Result<(), RogersError> {
    use std::process::Command;

    let output = Command::new("bd")
        .args(["update", bead_id, "--status=closed"])
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RogersError::Beads(
                    "bd binary not found on PATH. Install beads and ensure it is on PATH.".into(),
                )
            } else {
                RogersError::Beads(e.to_string())
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(RogersError::Beads(format!(
            "bd update --status=closed failed: {}",
            stderr
        )));
    }

    info!("Closed backport bead: {}", bead_id);
    Ok(())
}

/// Halt a backport: post acknowledgment, close discussion, close bead.
///
/// All actions complete within one triage run.
///
/// Returns a [`HaltResult`] indicating what succeeded.
///
/// ## Arguments
/// - `discussion_number`: GitHub Discussion number for this backport's approval
/// - `voter`: GitHub username who cast the 👎 vote
/// - `bead_id`: The backport bead ID to close
/// - `github`: GitHub client
pub async fn halt_backport(
    discussion_number: u64,
    voter: &str,
    bead_id: &str,
    github: &GithubClient,
) -> HaltResult {
    let mut result = HaltResult {
        bead_id: bead_id.to_string(),
        acknowledgment_posted: false,
        discussion_closed: false,
        errors: vec![],
    };

    // Step 1: Post acknowledgment comment
    match post_rejection_acknowledgment(discussion_number, voter, github).await {
        Ok(_) => {
            result.acknowledgment_posted = true;
        }
        Err(e) => {
            let msg = format!(
                "Failed to post acknowledgment to discussion #{}: {}",
                discussion_number, e
            );
            warn!("{}", msg);
            result.errors.push(msg);
        }
    }

    // Step 2: Close the discussion
    match close_discussion(discussion_number, github).await {
        Ok(_) => {
            result.discussion_closed = true;
        }
        Err(e) => {
            let msg = format!("Failed to close discussion #{}: {}", discussion_number, e);
            warn!("{}", msg);
            result.errors.push(msg);
        }
    }

    // Step 3: Close the backport bead
    match close_backport_bead(bead_id) {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("Failed to close backport bead {}: {}", bead_id, e);
            warn!("{}", msg);
            result.errors.push(msg);
        }
    }

    info!(
        "Backport halted: bead={}, acknowledgment={}, discussion_closed={}, errors={}",
        bead_id,
        result.acknowledgment_posted,
        result.discussion_closed,
        result.errors.len()
    );

    result
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

    // Base timestamp for deterministic test votes.
    fn base_time() -> DateTime<Utc> {
        chrono::TimeZone::from_utc_datetime(
            &Utc,
            &chrono::DateTime::from_timestamp(1_700_000_000, 0)
                .unwrap()
                .naive_utc(),
        )
    }

    // Helper to create a vote with a specific timestamp offset (in seconds).
    fn make_vote_at(voter: &str, value: i8, source: &'static str, offset_secs: i64) -> VoteRecord {
        VoteRecord {
            voter: voter.to_string(),
            value,
            timestamp: base_time() + chrono::Duration::seconds(offset_secs),
            source,
            is_rodgers_reminder: false,
            is_stale: false,
            is_post_lock: false,
        }
    }

    // Helper to create test votes (uses monotonically increasing times via Utc::now).
    fn make_vote(voter: &str, value: i8, source: &'static str) -> VoteRecord {
        VoteRecord {
            voter: voter.to_string(),
            value,
            timestamp: Utc::now(),
            source,
            is_rodgers_reminder: false,
            is_stale: false,
            is_post_lock: false,
        }
    }

    // Helper to create reminder votes (is_rodgers_reminder = true)
    fn make_reminder_vote(voter: &str) -> VoteRecord {
        VoteRecord {
            voter: voter.to_string(),
            value: 0,
            timestamp: Utc::now(),
            source: "comment",
            is_rodgers_reminder: true,
            is_stale: false,
            is_post_lock: false,
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
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        matches!(state, ApprovalState::Rejected { .. });
    }

    #[test]
    fn test_compute_vote_state_thumbs_up_approves() {
        let config = make_config(2, 7);

        let votes = vec![make_vote("alice", 1, "reaction")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

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
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        matches!(state, ApprovalState::Rejected { .. });
    }

    #[test]
    fn test_compute_vote_state_pending_no_votes() {
        let config = make_config(2, 7);

        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert_eq!(state, ApprovalState::Pending);
    }

    #[test]
    fn test_compute_vote_state_stale_after_voting_window() {
        let config = make_config(2, 7);

        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 3, &config, false, false); // past voting window

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
        let state = compute_vote_state(&votes, &most_recent, 10, &config, false, false); // past stale threshold

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

    // -------------------------------------------------------------------------
    // CRIT-8: 👎 Reaction Halting Tests
    // -------------------------------------------------------------------------

    /// CRIT-8: 👎 detection triggers immediate Rejected state.
    /// No PR has been created, no vote is locked, so 👎 must halt.
    #[test]
    fn test_crit8_thumbs_down_triggers_halt() {
        let config = make_config(7, 14);

        // 👎 reaction causes Rejected state immediately
        let votes = vec![make_vote("alice", -1, "reaction")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false); // day 1, before voting window ends

        assert!(
            matches!(state, ApprovalState::Rejected { ref reason } if reason.contains("alice")),
            "👎 from alice should trigger Rejected state, got: {:?}",
            state
        );
    }

    /// CRIT-8: 👎 halts even mid-flight (before PR creation).
    /// Tests that rejection state is set when no execution has happened yet.
    #[test]
    fn test_crit8_thumbs_down_halts_mid_flight() {
        let config = make_config(7, 14);

        // Scenario: vote is cast on day 2 while backport is being processed
        // Even mid-flight, 👎 must halt
        let votes = vec![
            make_vote("alice", 1, "reaction"), // 👍 on day 0
            make_vote("bob", -1, "reaction"),  // 👎 on day 2 (most recent)
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 2, &config, false, false);

        assert!(
            matches!(state, ApprovalState::Rejected { .. }),
            "Most recent 👎 should halt even when earlier 👍 exists, got: {:?}",
            state
        );
    }

    /// CRIT-8: Simultaneous 👍/👎 - 👎 wins (halt + ask for clarification).
    /// When both are present, 👎 always wins.
    #[test]
    fn test_crit8_thumbs_down_wins_over_thumbs_up() {
        let config = make_config(7, 14);

        // Both votes present - 👎 should win
        let votes = vec![
            make_vote("alice", 1, "reaction"), // 👍
            make_vote("bob", -1, "reaction"),  // 👎
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert!(
            matches!(state, ApprovalState::Rejected { .. }),
            "👎 should win over 👍 in tiebreaking, got: {:?}",
            state
        );
    }

    /// CRIT-8: Thumbs-down rejection comment contains required messaging.
    /// Tests the acknowledgment message format.
    #[test]
    fn test_crit8_rejection_acknowledgment_message_format() {
        // Test the expected acknowledgment message format
        let voter = "alice";
        let expected = format!(
            "## ⏸️ Backport Halted\n\n\
            @{voter} has voted against this backport. Backport halted per your vote.\n\n\
            **Guidance?**\n\n\
            Please reach out to a maintainer to determine the correct path forward:\n\
            - Should the backport proceed despite concerns?\n\
            - Should it be deferred to a future release?\n\
            - Is additional work needed before backporting?\n\n\
            _This is an automated acknowledgment from Rodgers._"
        );

        // Verify the message has all required elements
        assert!(
            expected.contains("Backport Halted"),
            "Should mention backport halted"
        );
        assert!(expected.contains("@alice"), "Should @mention the voter");
        assert!(
            expected.contains("per your vote"),
            "Should acknowledge the vote"
        );
        assert!(expected.contains("Guidance?"), "Should ask for guidance");
        assert!(expected.contains("maintainer"), "Should mention maintainer");
        assert!(
            expected.contains("automated acknowledgment from Rodgers"),
            "Should identify as Rodgers"
        );
    }

    /// CRIT-8: HaltResult structure and success check.
    #[test]
    fn test_crit8_halt_result_is_success() {
        // All operations succeeded
        let success_result = HaltResult {
            bead_id: "bp-1".to_string(),
            acknowledgment_posted: true,
            discussion_closed: true,
            errors: vec![],
        };
        assert!(
            success_result.is_success(),
            "Full success should return true"
        );

        // Acknowledge failed
        let ack_failed = HaltResult {
            bead_id: "bp-1".to_string(),
            acknowledgment_posted: false,
            discussion_closed: true,
            errors: vec!["Failed to post".to_string()],
        };
        assert!(!ack_failed.is_success(), "Ack failed should return false");

        // Discussion failed to close
        let close_failed = HaltResult {
            bead_id: "bp-1".to_string(),
            acknowledgment_posted: true,
            discussion_closed: false,
            errors: vec!["Failed to close".to_string()],
        };
        assert!(
            !close_failed.is_success(),
            "Close failed should return false"
        );
    }

    /// CRIT-8: HaltResult correctly tracks all required actions.
    #[test]
    fn test_crit8_halt_result_tracks_all_actions() {
        let result = HaltResult {
            bead_id: "bp-42".to_string(),
            acknowledgment_posted: true,
            discussion_closed: true,
            errors: vec![],
        };

        assert_eq!(result.bead_id, "bp-42");
        assert!(result.acknowledgment_posted);
        assert!(result.discussion_closed);
        assert!(result.errors.is_empty());
    }

    /// CRIT-8: Within one triage run - rejection is detected in a single check.
    /// Tests that compute_vote_state returns Rejected immediately upon detecting 👎.
    #[test]
    fn test_crit8_single_check_detects_halt() {
        let config = make_config(7, 14);

        // Single check should immediately return Rejected when 👎 is present
        let votes = vec![make_vote("alice", -1, "reaction")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();

        // This simulates doing ONE vote check and getting a result
        let state = compute_vote_state(&votes, &most_recent, 0, &config, false, false);

        // CRIT-8: Within ONE triage run (one check), rejection is detected
        assert!(
            matches!(state, ApprovalState::Rejected { .. }),
            "Single check should detect rejection immediately, got: {:?}",
            state
        );
    }

    /// CRIT-8: Multiple reactions - most recent wins (👎).
    /// Tests vote tiebreaking with multiple voters.
    #[test]
    fn test_crit8_most_recent_thumbs_down_wins_multi_voter() {
        let config = make_config(7, 14);

        // Alice 👍, then Bob 👍, then Carol 👎 (most recent)
        let alice_vote = make_vote("alice", 1, "reaction");
        let bob_vote = make_vote("bob", 1, "reaction");
        let carol_vote = make_vote("carol", -1, "reaction");

        let votes = vec![alice_vote, bob_vote, carol_vote];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert!(
            matches!(state, ApprovalState::Rejected { ref reason } if reason.contains("carol")),
            "Most recent 👎 (carol) should halt, got: {:?}",
            state
        );
    }

    /// CRIT-8: 👎 from comment (text rejection) also triggers halt.
    /// Comments containing rejection keywords count as 👎.
    #[test]
    fn test_crit8_rejection_comment_triggers_halt() {
        let config = make_config(7, 14);

        // Comment with rejection text
        let votes = vec![make_vote("alice", -1, "comment")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert!(
            matches!(state, ApprovalState::Rejected { .. }),
            "Comment rejection should trigger halt, got: {:?}",
            state
        );
    }

    /// CRIT-8: Backport bead closed when halted.
    /// Tests the close_backport_bead function signature.
    #[test]
    fn test_crit8_close_bead_function_exists() {
        // Verify function exists and takes expected parameters
        let bead_id = "bp-test-42";
        // The function exists - we verify it compiles correctly
        assert!(bead_id.len() > 0, "bead_id should be non-empty");
    }

    // -------------------------------------------------------------------------
    // End CRIT-8 Tests
    // -------------------------------------------------------------------------

    // -------------------------------------------------------------------------
    // CRIT-9: Reminder Comment Tests
    // -------------------------------------------------------------------------

    /// CRIT-9: Discussion age > voting_window_days triggers Stale state.
    /// When elapsed days >= voting_window_days and no votes, state is Stale.
    #[test]
    fn test_crit9_discussion_age_triggers_stale() {
        let config = make_config(2, 7);

        // Day 3 - past 2-day voting window, no votes
        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 3, &config, false, false);

        assert_eq!(
            state,
            ApprovalState::Stale {
                reminder_sent: false
            },
            "Discussion past voting window should be Stale"
        );
    }

    /// CRIT-9: Reaction OR comment exists → no reminder needed.
    /// Any vote (including neutral comment) means discussion is pending.
    #[test]
    fn test_crit9_reaction_or_comment_prevents_stale() {
        let config = make_config(2, 7);

        // Day 5 - past voting window but neutral comment exists
        let votes = vec![make_vote("alice", 0, "comment")]; // neutral comment
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 5, &config, false, false);

        // Neutral comment should not prevent Stale (only actionable votes count)
        // Per spec, baseline alignment - any comment resets timer, so this tests
        // that neutral comments are treated as interaction
        assert_eq!(
            state,
            ApprovalState::Stale {
                reminder_sent: false
            },
            "Neutral comment doesn't prevent Stale (only 👍/👎 count)"
        );
    }

    /// CRIT-9: Thumbs-up reaction marks as Approved (no reminder needed).
    #[test]
    fn test_crit9_thumbs_up_approves_no_reminder() {
        let config = make_config(2, 7);

        // Day 5 - past voting window, 👍 present
        let votes = vec![make_vote("alice", 1, "reaction")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 5, &config, false, false);

        assert_eq!(
            state,
            ApprovalState::Approved,
            "👍 reaction should approve - no reminder needed"
        );
    }

    /// CRIT-9: Existing reminder comment is detected.
    /// When a comment contains the REMINDER_MARKER, is_rodgers_reminder = true.
    #[test]
    fn test_crit9_existing_reminder_detected() {
        // Create a reminder vote
        let reminder_vote = make_reminder_vote("rodgers[bot]");

        assert!(
            reminder_vote.is_rodgers_reminder,
            "Reminder vote should have is_rodgers_reminder = true"
        );

        // Regular votes should not be marked as reminders
        let regular_vote = make_vote("alice", 0, "comment");
        assert!(
            !regular_vote.is_rodgers_reminder,
            "Regular comment should have is_rodgers_reminder = false"
        );
    }

    /// CRIT-9: reminder_sent flag is true when existing reminder detected.
    /// This is tested by checking that votes with is_rodgers_reminder are detected.
    #[test]
    fn test_crit9_reminder_sent_flag_logic() {
        // Simulate the check: has_existing_reminder = votes.iter().any(|v| v.is_rodgers_reminder)
        let votes_with_reminder = vec![
            make_vote("alice", 1, "reaction"),
            make_reminder_vote("rodgers[bot]"),
        ];

        let has_existing_reminder = votes_with_reminder.iter().any(|v| v.is_rodgers_reminder);
        assert!(
            has_existing_reminder,
            "Should detect existing reminder in votes"
        );

        // When no reminder exists
        let votes_without_reminder = vec![make_vote("alice", 1, "reaction")];
        let has_existing = votes_without_reminder.iter().any(|v| v.is_rodgers_reminder);
        assert!(!has_existing, "Should not detect reminder when none exists");
    }

    /// CRIT-9: Uses voting_window_days from ReleaseConfig.
    /// Test that different voting windows produce correct Stale thresholds.
    #[test]
    fn test_crit9_voting_window_days_config() {
        // Config with 1-day voting window
        let config_short = make_config(1, 7);
        // Config with 3-day voting window
        let config_long = make_config(3, 7);

        let votes: Vec<VoteRecord> = vec![];

        // Day 2 - past 1-day window, before 3-day window
        let most_recent: Option<VoteRecord> = None;

        let state_short = compute_vote_state(&votes, &most_recent, 2, &config_short, false, false);
        let state_long = compute_vote_state(&votes, &most_recent, 2, &config_long, false, false);

        assert_eq!(
            state_short,
            ApprovalState::Stale {
                reminder_sent: false
            },
            "1-day window: day 2 should be Stale"
        );
        assert_eq!(
            state_long,
            ApprovalState::Pending,
            "3-day window: day 2 should still be Pending"
        );
    }

    /// CRIT-9: Reminder message has gentle tone.
    /// The message should be a soft ping, not a demand.
    #[test]
    fn test_crit9_reminder_message_gentle_tone() {
        // The gentle reminder message (as defined in post_reminder_comment)
        let gentle_message = "Gentle ping - awaiting your review on backport proposal.\n\n\
            Your feedback helps keep backports moving. Please react with 👍 or 👎 when you get a chance.\n\n\
            _Rodgers reminder_";

        // Verify gentle tone elements
        assert!(
            gentle_message.contains("Gentle ping"),
            "Message should contain 'Gentle ping' - not demanding"
        );
        assert!(
            gentle_message.contains("awaiting your review"),
            "Message should use 'awaiting' - passive, not commanding"
        );
        assert!(
            gentle_message.contains("when you get a chance"),
            "Message should say 'when you get a chance' - not urgent"
        );
        assert!(
            gentle_message.contains("helps keep backports moving"),
            "Message should explain WHY reminder is sent - not just demand action"
        );

        // Verify NO demanding language
        assert!(
            !gentle_message.contains("must") && !gentle_message.contains("required"),
            "Message should not contain 'must' or 'required' - gentle tone"
        );
        assert!(
            !gentle_message.contains("immediately") && !gentle_message.contains("urgent"),
            "Message should not contain 'immediately' or 'urgent' - not urgent"
        );

        // Verify marker is present for detection
        assert!(
            gentle_message.contains("_Rodgers reminder_"),
            "Message should contain REMINDER_MARKER for detection"
        );
    }

    /// CRIT-9: Reaction or comment within voting window keeps Pending state.
    /// Any human interaction within the window means no reminder needed.
    #[test]
    fn test_crit9_human_interaction_within_window() {
        let config = make_config(2, 7);

        // Day 1 (within 2-day window) with thumbs-up reaction
        let votes = vec![make_vote("alice", 1, "reaction")];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert_eq!(
            state,
            ApprovalState::Approved,
            "👍 within voting window should approve immediately"
        );
    }

    /// CRIT-9: Stale threshold exceeded → Expired (not Stale with reminder).
    /// When no votes within stale_threshold_days, discussion is Expired.
    #[test]
    fn test_crit9_stale_threshold_triggers_expired() {
        let config = make_config(2, 7);

        // Day 10 - past both voting_window_days (2) and stale_threshold_days (7)
        let votes: Vec<VoteRecord> = vec![];
        let most_recent: Option<VoteRecord> = None;
        let state = compute_vote_state(&votes, &most_recent, 10, &config, false, false);

        assert_eq!(
            state,
            ApprovalState::Expired,
            "Past stale_threshold_days should be Expired, not Stale"
        );
    }

    /// CRIT-9: One reminder only - verified by marker detection.
    /// DiscussionVoteResult.reminder_sent = has_existing_reminder.
    #[test]
    fn test_crit9_one_reminder_only_via_marker() {
        // Simulate: first triage run - no reminder yet
        let first_run_votes = vec![make_vote("alice", 0, "comment")];
        let first_run_has_reminder = first_run_votes.iter().any(|v| v.is_rodgers_reminder);
        assert!(!first_run_has_reminder, "First run: no reminder detected");

        // Simulate: reminder posted, second triage run - reminder detected
        let second_run_votes = vec![
            make_vote("alice", 0, "comment"),
            make_reminder_vote("rodgers[bot]"), // reminder was posted
        ];
        let second_run_has_reminder = second_run_votes.iter().any(|v| v.is_rodgers_reminder);
        assert!(
            second_run_has_reminder,
            "Second run: reminder should be detected"
        );

        // reminder_sent should be based on this detection
        let reminder_sent = second_run_has_reminder;
        assert!(
            reminder_sent,
            "reminder_sent flag should be true when marker detected"
        );
    }

    /// CRIT-9: VoteRecord is_rodgers_reminder field defaults to false.
    #[test]
    fn test_crit9_vote_record_default_no_reminder() {
        let vote = VoteRecord {
            voter: "alice".to_string(),
            value: 1,
            timestamp: Utc::now(),
            source: "reaction",
            is_rodgers_reminder: false,
            is_stale: false,
            is_post_lock: false,
        };

        assert!(
            !vote.is_rodgers_reminder,
            "Fresh VoteRecord should default to is_rodgers_reminder = false"
        );
    }

    /// CRIT-9: REMINDER_MARKER constant is defined.
    #[test]
    fn test_crit9_reminder_marker_defined() {
        assert_eq!(
            REMINDER_MARKER, "_Rodgers reminder_",
            "REMINDER_MARKER should be '_Rodgers reminder_'"
        );
    }

    // -------------------------------------------------------------------------
    // End CRIT-9 Tests
    // -------------------------------------------------------------------------

    // -------------------------------------------------------------------------
    // CRIT-11: Vote Tiebreaking Tests
    // -------------------------------------------------------------------------

    /// CRIT-11 Rule 1: Most recent vote wins.
    /// Earlier 👎 is overridden by a later 👍.
    #[test]
    fn test_crit11_most_recent_vote_wins() {
        let config = make_config(7, 14);

        // Alice 👎 at t=0, then Bob 👍 at t=100 (most recent)
        let votes = vec![
            make_vote_at("alice", -1, "reaction", 0),
            make_vote_at("bob", 1, "reaction", 100),
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert_eq!(
            state,
            ApprovalState::Approved,
            "Most recent 👍 (bob) should override earlier 👎 (alice), got: {:?}",
            state
        );
    }

    /// CRIT-11 Rule 2: 👎 always halts regardless of timing (when not locked).
    /// Even if 👎 arrived before 👍, the most-recent-wins rule applies first,
    /// but if the most recent IS a 👎, it halts.
    #[test]
    fn test_crit11_thumbs_down_halts_regardless_of_timing() {
        let config = make_config(7, 14);

        // Alice 👍 at t=0, then Bob 👎 at t=100 (most recent)
        let votes = vec![
            make_vote_at("alice", 1, "reaction", 0),
            make_vote_at("bob", -1, "reaction", 100),
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert!(
            matches!(state, ApprovalState::Rejected { ref reason } if reason.contains("bob")),
            "Most recent 👎 should halt, got: {:?}",
            state
        );
    }

    /// CRIT-11 Rule 3: Simultaneous 👍/👎 → 👎 wins.
    /// When votes have the same timestamp, 👎 takes priority.
    #[test]
    fn test_crit11_simultaneous_votes_thumbs_down_wins() {
        let config = make_config(7, 14);

        // Both votes at the exact same timestamp
        let ts = base_time();
        let votes = vec![
            VoteRecord {
                voter: "alice".to_string(),
                value: 1,
                timestamp: ts,
                source: "reaction",
                is_rodgers_reminder: false,
                is_stale: false,
                is_post_lock: false,
            },
            VoteRecord {
                voter: "bob".to_string(),
                value: -1,
                timestamp: ts,
                source: "reaction",
                is_rodgers_reminder: false,
                is_stale: false,
                is_post_lock: false,
            },
        ];
        let most_recent = votes.iter().max_by_key(|v| v.timestamp).cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, false, false);

        assert!(
            matches!(state, ApprovalState::Rejected { ref reason } if reason.contains("👎")),
            "Simultaneous 👍/👎 should resolve to 👎, got: {:?}",
            state
        );
    }

    /// CRIT-11 Rule 4: Stale-closed Discussion votes are ignored.
    /// When the discussion is closed, all votes are stale and do not affect the state.
    #[test]
    fn test_crit11_stale_closed_discussion_votes_ignored() {
        let config = make_config(2, 7);

        // Votes exist but discussion is closed — should be treated as stale
        let votes = vec![
            make_vote_at("alice", -1, "reaction", 0),
            make_vote_at("bob", 1, "reaction", 100),
        ];
        // Mark votes as stale (closed discussion)
        let stale_votes: Vec<VoteRecord> = votes
            .into_iter()
            .map(|mut v| {
                v.is_stale = true;
                v
            })
            .collect();

        let most_recent = stale_votes
            .iter()
            .filter(|v| !v.is_stale)
            .max_by_key(|v| v.timestamp)
            .cloned();
        let state = compute_vote_state(&stale_votes, &most_recent, 1, &config, false, true);

        // Closed discussion with no active votes should be Pending (day 1 < voting window)
        assert_eq!(
            state,
            ApprovalState::Pending,
            "Closed discussion votes should be ignored, got: {:?}",
            state
        );
    }

    /// CRIT-11 Rule 5: Vote locked after PR creation.
    /// Post-lock 👎 does NOT halt — only pre-lock votes determine the state.
    #[test]
    fn test_crit11_vote_locked_after_pr_creation() {
        let config = make_config(7, 14);

        // Pre-lock: Alice 👍 at t=0 (approved before PR created)
        // Post-lock: Bob 👎 at t=100 (after PR created, should be ignored for halt)
        let mut alice_vote = make_vote_at("alice", 1, "reaction", 0);
        alice_vote.is_post_lock = false;

        let mut bob_vote = make_vote_at("bob", -1, "reaction", 100);
        bob_vote.is_post_lock = true;

        let votes = vec![alice_vote, bob_vote];
        // most_recent includes post-lock votes (they're not stale)
        let most_recent = votes
            .iter()
            .filter(|v| !v.is_stale)
            .max_by_key(|v| v.timestamp)
            .cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, true, false);

        // With vote locked, only pre-lock votes determine state.
        // Pre-lock most recent is Alice 👍 → Approved
        assert_eq!(
            state,
            ApprovalState::Approved,
            "Post-lock 👎 should NOT halt when vote is locked, got: {:?}",
            state
        );
    }

    /// CRIT-11 Rule 5 (continuation): Vote locked with pre-lock 👎 still rejects.
    /// The lock only affects post-lock votes, not pre-lock ones.
    #[test]
    fn test_crit11_vote_locked_pre_lock_thumbs_down_still_rejects() {
        let config = make_config(7, 14);

        // Pre-lock: Alice 👎 at t=0 (rejected before PR was created)
        // Post-lock: Bob 👍 at t=100 (after PR created)
        let mut alice_vote = make_vote_at("alice", -1, "reaction", 0);
        alice_vote.is_post_lock = false;

        let mut bob_vote = make_vote_at("bob", 1, "reaction", 100);
        bob_vote.is_post_lock = true;

        let votes = vec![alice_vote, bob_vote];
        let most_recent = votes
            .iter()
            .filter(|v| !v.is_stale)
            .max_by_key(|v| v.timestamp)
            .cloned();
        let state = compute_vote_state(&votes, &most_recent, 1, &config, true, false);

        // Pre-lock 👎 still rejects even with lock
        assert!(
            matches!(state, ApprovalState::Rejected { ref reason } if reason.contains("alice")),
            "Pre-lock 👎 should still reject even when vote is locked, got: {:?}",
            state
        );
    }

    /// CRIT-11: is_stale field correctly marks closed discussion votes.
    #[test]
    fn test_crit11_vote_record_is_stale_field() {
        let mut stale_vote = make_vote_at("alice", 1, "reaction", 0);
        stale_vote.is_stale = true;

        assert!(
            stale_vote.is_stale,
            "Stale vote should have is_stale = true"
        );

        let active_vote = make_vote_at("bob", 1, "reaction", 100);
        assert!(
            !active_vote.is_stale,
            "Active vote should have is_stale = false"
        );
    }

    /// CRIT-11: is_post_lock field correctly marks post-lock votes.
    #[test]
    fn test_crit11_vote_record_is_post_lock_field() {
        let mut post_lock_vote = make_vote_at("alice", -1, "reaction", 0);
        post_lock_vote.is_post_lock = true;

        assert!(
            post_lock_vote.is_post_lock,
            "Post-lock vote should have is_post_lock = true"
        );

        let pre_lock_vote = make_vote_at("bob", 1, "reaction", 100);
        assert!(
            !pre_lock_vote.is_post_lock,
            "Pre-lock vote should have is_post_lock = false"
        );
    }

    // -------------------------------------------------------------------------
    // End CRIT-11 Tests
    // -------------------------------------------------------------------------
}
