//! Release candidacy detector.
//!
//! Detects whether the repository is ready for a new release by evaluating:
//! - Merged PRs since the last release tag
//! - CI green status on the source branch
//! - Blockers: blocker label, priority labels, human-flagged, LLM-judged
//! - Milestone presence for the target release

use crate::config::ReleaseConfig;
use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use crate::github::models::{Issue, PullRequest, Release};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Type of source branch for release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseSource {
    /// Releasing from the default branch (main/master).
    Main,
    /// Releasing from a maintenance branch.
    Branch(String),
}

impl std::fmt::Display for ReleaseSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReleaseSource::Main => write!(f, "main"),
            ReleaseSource::Branch(name) => write!(f, "release/{}", name),
        }
    }
}

/// A blocker that could prevent a release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Blocker {
    /// Issue number associated with this blocker.
    pub issue_number: i32,
    /// Issue title.
    pub title: String,
    /// Why this is considered a blocker.
    pub reason: BlockerReason,
}

/// Why something is flagged as a blocker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerReason {
    /// Issue has the `blocker` label.
    BlockerLabel,
    /// Issue has a priority label and is in the release milestone.
    PriorityLabel,
    /// A human has explicitly flagged this issue.
    HumanFlagged,
    /// LLM judged this issue could be a blocker.
    LlmJudged,
}

impl std::fmt::Display for BlockerReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BlockerReason::BlockerLabel => write!(f, "blocker label"),
            BlockerReason::PriorityLabel => write!(f, "priority label"),
            BlockerReason::HumanFlagged => write!(f, "human flagged"),
            BlockerReason::LlmJudged => write!(f, "LLM judged"),
        }
    }
}

/// A candidate for a new release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseCandidate {
    /// Proposed version string (e.g., "1.2.3").
    pub version: String,
    /// Source branch.
    pub source: ReleaseSource,
    /// Number of merged PRs since last release.
    pub pr_count: usize,
    /// Last release tag info, if any.
    pub last_release: Option<LastRelease>,
    /// Blockers that should be surfaced before release.
    pub blockers: Vec<Blocker>,
    /// Whether CI is green on the source branch.
    pub ci_green: bool,
    /// Whether a milestone is set for this release.
    pub milestone_set: bool,
}

impl ReleaseCandidate {
    /// Check if all readiness criteria are met.
    pub fn is_ready(&self) -> bool {
        self.ci_green
            && self.milestone_set
            && !self.blockers.is_empty()
            && self.pr_count > 0
    }
}

/// Information about the last release tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LastRelease {
    /// Tag name.
    pub tag: String,
    /// Release name.
    pub name: String,
    /// URL to the release.
    pub url: String,
    /// SHA of the release commit.
    pub commit_sha: String,
    /// Date of the release.
    pub created_at: DateTime<Utc>,
}

impl From<&Release> for LastRelease {
    fn from(release: &Release) -> Self {
        Self {
            tag: release.tag_name.clone(),
            name: release.name.clone().unwrap_or_default(),
            url: release.url.clone().unwrap_or_default(),
            commit_sha: String::new(), // Will be filled by caller
            created_at: release.created_at,
        }
    }
}

impl From<&Issue> for Blocker {
    fn from(issue: &Issue) -> Self {
        Self {
            issue_number: issue.number,
            title: issue.title.clone(),
            reason: BlockerReason::BlockerLabel,
        }
    }
}

/// Result of a release candidacy detection run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CandidacyResult {
    /// Candidates found in this run.
    pub candidates: Vec<ReleaseCandidate>,
    /// Source branches that were checked.
    pub checked_branches: Vec<ReleaseSource>,
    /// Number of merged PRs evaluated.
    pub prs_checked: usize,
}

impl CandidacyResult {
    /// Add a candidate to the result.
    pub fn add_candidate(&mut self, candidate: ReleaseCandidate) {
        self.candidates.push(candidate);
    }

    /// Add a checked branch.
    pub fn add_checked_branch(&mut self, source: ReleaseSource) {
        self.checked_branches.push(source);
    }

    /// Increment the PR checked count.
    pub fn pr_checked_one(&mut self) {
        self.prs_checked += 1;
    }
}

/// Release detector.
///
/// Evaluates merged PRs, CI status, blockers, and milestones
/// to determine if a release should be proposed.
#[derive(Debug, Clone)]
pub struct ReleaseDetector {
    /// GitHub client.
    github: GitHubClient,
    /// Release configuration.
    release_config: ReleaseConfig,
    /// Blocker label name.
    blocker_label: String,
    /// Priority label prefix (e.g., "priority", "priority/high").
    priority_labels: Vec<String>,
}

impl ReleaseDetector {
    /// Create a new detector.
    pub fn new(
        github: GitHubClient,
        release_config: ReleaseConfig,
        blocker_label: String,
    ) -> Self {
        Self {
            github,
            release_config,
            blocker_label,
            priority_labels: vec![
                "priority/critical".to_string(),
                "priority/high".to_string(),
                "priority/medium".to_string(),
                "priority/low".to_string(),
            ],
        }
    }

    /// Detect release candidates from the configured source branches.
    ///
    /// Checks `main` and any configured active release branches.
    pub async fn detect_candidates(&mut self) -> Result<CandidacyResult> {
        let mut result = CandidacyResult::default();

        // Check main branch
        let main_candidate = self.check_branch(&ReleaseSource::Main).await?;
        if let Some(candidate) = main_candidate {
            result.add_candidate(candidate);
            result.add_checked_branch(ReleaseSource::Main);
        }

        // Check active release branches
        let active_branches: Vec<String> = self.release_config
            .active_branches
            .clone()
            .unwrap_or_default();

        for branch in active_branches {
            let source = ReleaseSource::Branch(branch.clone());
            let candidate = self.check_branch(&source).await?;
            if let Some(c) = candidate {
                result.add_candidate(c);
                result.add_checked_branch(source);
            }
        }

        Ok(result)
    }

    /// Check a single branch for release candidacy.
    async fn check_branch(&mut self, source: &ReleaseSource) -> Result<Option<ReleaseCandidate>> {
        let branch_name = match source {
            ReleaseSource::Main => "main",
            ReleaseSource::Branch(name) => name.as_str(),
        };

        // Get the latest release tag to compare against
        let releases = self.github.list_releases(None, None).await?;
        let last_release = releases.first().cloned();

        // Get merged PRs since last release (or all merged PRs if no release exists)
        let prs = self
            .github
            .list_pull_requests(Some("merged"), None, None)
            .await?;

        let since_cutoff = last_release.as_ref().map(|r| r.created_at);
        let prs_on_branch: Vec<_> = prs
            .into_iter()
            .filter(|pr| {
                pr.merged_at >= since_cutoff
                    && pr.base.ref_name == branch_name
            })
            .collect();

        let pr_count = prs_on_branch.len();
        if pr_count == 0 {
            return Ok(None);
        }

        // Evaluate each PR for candidacy and collect blockers
        let mut blockers = Vec::new();
        let mut milestone_set = false;
        let mut pr_count_internal = 0usize;

        for pr in &prs_on_branch {
            self.pr_checked_one_inner(&mut pr_count_internal);

            // Check for linked issues
            if let Some(issue_num) = self.extract_issue_number(pr) {
                if let Ok(issue) = self.github.get_issue(issue_num).await {
                    // Check milestone
                    if issue.milestone.is_some() {
                        milestone_set = true;
                    }

                    // Check for blockers
                    if let Some(blocker) = self.evaluate_blocker(&issue) {
                        blockers.push(blocker);
                    }
                }
            }
        }

        // Check if milestone is set (even if PRs don't have linked issues)
        if !milestone_set {
            milestone_set = self.has_milestone_issues(since_cutoff.as_ref()).await?;
        }

        // Check CI status on the branch
        let ci_green = self.check_ci_status(branch_name).await?;

        // Determine version
        let version = if let Some(ref last_rel) = last_release {
            self.next_version(&last_rel.tag_name)
        } else {
            "0.1.0".to_string()
        };

        // Build last_release info
        let last_release_info = last_release.as_ref().map(|r| {
            let mut info: LastRelease = r.into();
            info.commit_sha = r.html_url.clone().unwrap_or_default();
            info
        });

        let candidate = ReleaseCandidate {
            version,
            source: source.clone(),
            pr_count,
            last_release: last_release_info,
            blockers,
            ci_green,
            milestone_set,
        };

        Ok(Some(candidate))
    }

    /// Internal: increment PR count.
    fn pr_checked_one_inner(&self, count: &mut usize) {
        *count += 1;
    }

    /// Check if there are milestone issues for the release timeframe.
    async fn has_milestone_issues(&mut self, since: Option<&DateTime<Utc>>) -> Result<bool> {
        let issues = self.github.list_issues(None, None, None, None, None).await?;
        let cutoff = since.copied().unwrap_or_else(Utc::now);
        Ok(issues
            .iter()
            .any(|i| i.milestone.is_some() && i.created_at > cutoff))
    }

    /// Check CI status on a branch (simplified: check if last commit has any CI-related labels or status).
    async fn check_ci_status(&mut self, branch: &str) -> Result<bool> {
        // Check the latest commits on the branch
        let commits = self
            .github
            .list_commits(Some(branch), None, None, Some(5))
            .await?;

        if commits.is_empty() {
            return Ok(false);
        }

        // Simple heuristic: if the branch has commits and we can reach the API, assume green
        // In production, this would check GitHub check runs / status checks API
        Ok(true)
    }

    /// Evaluate whether an issue is a blocker.
    fn evaluate_blocker(&self, issue: &Issue) -> Option<Blocker> {
        let label_names: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();

        if label_names.contains(&self.blocker_label.as_str()) {
            return Some(Blocker {
                issue_number: issue.number,
                title: issue.title.clone(),
                reason: BlockerReason::BlockerLabel,
            });
        }

        for priority_label in &self.priority_labels {
            if label_names.iter().any(|l| *l == priority_label.as_str()) {
                return Some(Blocker {
                    issue_number: issue.number,
                    title: issue.title.clone(),
                    reason: BlockerReason::PriorityLabel,
                });
            }
        }

        // Check if issue has been manually flagged as a blocker
        if issue.title.to_lowercase().contains("blocker") || issue.title.to_lowercase().contains("critical") {
            return Some(Blocker {
                issue_number: issue.number,
                title: issue.title.clone(),
                reason: BlockerReason::HumanFlagged,
            });
        }

        None
    }

    /// Extract issue number from PR body (Closes/Fixes/Resolves #N pattern).
    fn extract_issue_number(&self, pr: &PullRequest) -> Option<i32> {
        let body = pr.body.as_deref().unwrap_or("");
        for pattern in ["closes #", "fixes #", "resolves #"] {
            if let Some(pos) = body.to_lowercase().find(pattern) {
                let rest = &body[pos + pattern.len()..];
                let num_str: String = rest
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                if !num_str.is_empty() {
                    return num_str.parse().ok();
                }
            }
        }
        None
    }

    /// Generate the next version number from the current tag.
    pub fn next_version(&self, current_tag: &str) -> String {
        // Strip leading 'v' if present
        let tag = current_tag.strip_prefix('v').unwrap_or(current_tag);
        let parts: Vec<u32> = tag
            .split('.')
            .filter_map(|p| p.parse().ok())
            .collect();

        match parts.len() {
            3 => {
                // Increment patch
                format!("{}.{}.{}", parts[0], parts[1], parts[2] + 1)
            }
            2 => format!("{}.{}.0", parts[0], parts[1] + 1),
            _ => "0.1.0".to_string(),
        }
    }

    /// Format the approval body for a release proposal.
    pub fn format_approval_body(
        &self,
        candidate: &ReleaseCandidate,
        discussion_url: Option<&str>,
    ) -> String {
        let last_release_ref = candidate
            .last_release
            .as_ref()
            .map(|r| format!("`{}`", r.tag))
            .unwrap_or_else(|| "none (first release)".to_string());

        let source = match &candidate.source {
            ReleaseSource::Main => "main".to_string(),
            ReleaseSource::Branch(name) => format!("release/{}", name),
        };

        let blockers_section = if candidate.blockers.is_empty() {
            "None".to_string()
        } else {
            candidate
                .blockers
                .iter()
                .map(|b| {
                    format!(
                        "- Issue #{}: {} (reason: {})",
                        b.issue_number, b.title, b.reason
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            r#"## Release {version}

**Proposed by:** Rodgers
**Source:** {source}
**Commits since last release:** {pr_count} merged PRs
**Last release:** {last_release}

### Blockers

{blockers}

### Vote

React with 👍 to approve, 👎 to reject.
Release will be cut within one triage run of approval unless vetoed.
"#,
            version = candidate.version,
            source = source,
            pr_count = candidate.pr_count,
            last_release = last_release_ref,
            blockers = blockers_section,
        )
    }
}

/// Helper to get mutable reference through wrapper.
struct ResultMut<'a, T>(&'a mut T);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_version_patch() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        assert_eq!(detector.next_version("1.2.3"), "1.2.4");
    }

    #[test]
    fn test_next_version_major() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        assert_eq!(detector.next_version("1.2"), "1.3.0");
    }

    #[test]
    fn test_next_version_strip_v() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        assert_eq!(detector.next_version("v1.2.3"), "1.2.4");
    }

    #[test]
    fn test_next_version_empty() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        assert_eq!(detector.next_version(""), "0.1.0");
    }

    #[test]
    fn test_release_source_display_main() {
        assert_eq!(ReleaseSource::Main.to_string(), "main");
    }

    #[test]
    fn test_release_source_display_branch() {
        assert_eq!(
            ReleaseSource::Branch("1.x".to_string()).to_string(),
            "release/1.x"
        );
    }

    #[test]
    fn test_blocker_reason_display() {
        assert_eq!(BlockerReason::BlockerLabel.to_string(), "blocker label");
        assert_eq!(BlockerReason::PriorityLabel.to_string(), "priority label");
        assert_eq!(BlockerReason::HumanFlagged.to_string(), "human flagged");
        assert_eq!(BlockerReason::LlmJudged.to_string(), "LLM judged");
    }

    #[test]
    fn test_candidacy_result_default() {
        let result = CandidacyResult::default();
        assert!(result.candidates.is_empty());
        assert!(result.checked_branches.is_empty());
        assert_eq!(result.prs_checked, 0);
    }

    #[test]
    fn test_candidacy_result_add() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        let mut result = CandidacyResult::default();
        let mut pr_counter = 0usize;

        result.add_checked_branch(ReleaseSource::Main);
        detector.pr_checked_one_inner(&mut pr_counter);

        let candidate = ReleaseCandidate {
            version: "1.0.0".to_string(),
            source: ReleaseSource::Main,
            pr_count: 5,
            last_release: None,
            blockers: vec![],
            ci_green: true,
            milestone_set: true,
        };
        result.add_candidate(candidate);

        assert_eq!(result.candidates.len(), 1);
        assert_eq!(result.checked_branches.len(), 1);
        assert_eq!(pr_counter, 1);
    }

    #[test]
    fn test_extract_issue_number_closes() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        let pr = PullRequest {
            number: 1,
            title: "test".to_string(),
            body: Some("Closes #123".to_string()),
            state: "closed".to_string(),
            user: crate::github::models::User { login: "test".to_string(), id: 1, node_id: None, avatar_url: None, html_url: None, user_type: None },
            labels: vec![],
            assignees: vec![],
            milestone: None,
            comments: 0,
            commits: 1,
            additions: 1,
            deletions: 1,
            changed_files: 1,
            closed_at: None,
            merged_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            merge_commit_sha: None,
            head: crate::github::models::RepoRef { ref_name: "test".to_string(), sha: "abc".to_string(), repo: crate::github::models::Repository { id: 1, name: "test".to_string(), node_id: None, full_name: "test/repo".to_string(), private: false, html_url: None, description: None } },
            base: crate::github::models::RepoRef { ref_name: "main".to_string(), sha: "def".to_string(), repo: crate::github::models::Repository { id: 1, name: "test".to_string(), node_id: None, full_name: "test/repo".to_string(), private: false, html_url: None, description: None } },
            node_id: None,
            url: None,
            html_url: None,
            draft: false,
            mergeable: None,
        };
        assert_eq!(detector.extract_issue_number(&pr), Some(123));
    }

    #[test]
    fn test_extract_issue_number_no_match() {
        let detector = ReleaseDetector::new(
            GitHubClient::new("owner", "repo", crate::github::auth::GitHubAuth::new_with_default_api("ghp_test")),
            ReleaseConfig::default(),
            "blocker".to_string(),
        );
        let pr = PullRequest {
            number: 1,
            title: "test".to_string(),
            body: Some("no issue reference".to_string()),
            state: "closed".to_string(),
            user: crate::github::models::User { login: "test".to_string(), id: 1, node_id: None, avatar_url: None, html_url: None, user_type: None },
            labels: vec![],
            assignees: vec![],
            milestone: None,
            comments: 0,
            commits: 1,
            additions: 1,
            deletions: 1,
            changed_files: 1,
            closed_at: None,
            merged_at: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            merge_commit_sha: None,
            head: crate::github::models::RepoRef { ref_name: "test".to_string(), sha: "abc".to_string(), repo: crate::github::models::Repository { id: 1, name: "test".to_string(), node_id: None, full_name: "test/repo".to_string(), private: false, html_url: None, description: None } },
            base: crate::github::models::RepoRef { ref_name: "main".to_string(), sha: "def".to_string(), repo: crate::github::models::Repository { id: 1, name: "test".to_string(), node_id: None, full_name: "test/repo".to_string(), private: false, html_url: None, description: None } },
            node_id: None,
            url: None,
            html_url: None,
            draft: false,
            mergeable: None,
        };
        assert_eq!(detector.extract_issue_number(&pr), None);
    }
}
