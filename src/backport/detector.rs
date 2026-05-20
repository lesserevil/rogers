//! Backport candidate detector.
//!
//! Detects commits that are candidates for backporting to older
//! release branches. Candidates include:
//! - Bug fix commits (linked issue labeled `bug`)
//! - Security patches (GH Advisory, security label, CVE pattern)
//! - Issues labeled `backport-me`
//! - Documentation fixes correcting harmful information
//!
//! ## Semantic Equivalence Check
//!
//! Before filing a backport bead, the detector checks whether the fix
//! is already present on the target branch via a semantic equivalence
//! analysis using the LLM.

use crate::config::ReleaseConfig;
use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use crate::github::models::{Issue, PullRequest};
use crate::llm::client::LlmClient;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Backport candidate reasons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateReason {
    /// Bug fix identified by issue label.
    BugFix,
    /// Security patch identified by advisory, label, or CVE.
    SecurityPatch,
    /// Manual backport request via label.
    BackportMe,
    /// Documentation fix correcting harmful information.
    DocumentationFix,
}

impl std::fmt::Display for CandidateReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CandidateReason::BugFix => write!(f, "bug_fix"),
            CandidateReason::SecurityPatch => write!(f, "security_patch"),
            CandidateReason::BackportMe => write!(f, "backport_me"),
            CandidateReason::DocumentationFix => write!(f, "documentation_fix"),
        }
    }
}

/// A detected backport candidate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackportCandidate {
    /// SHA of the merged commit.
    pub commit_sha: String,
    /// Short SHA for display.
    pub commit_sha_short: String,
    /// One-line commit message.
    pub commit_message: String,
    /// The reason this is a candidate.
    pub reason: CandidateReason,
    /// Number of the linked GitHub issue.
    pub issue_number: Option<i32>,
    /// Title of the linked issue.
    pub issue_title: Option<String>,
    /// Whether the issue has the `security` label.
    pub is_security: bool,
    /// URL to the merged PR or direct commit link.
    pub pr_url: Option<String>,
    /// The branch the fix landed on (e.g., "main", "release/1.x").
    pub landed_on_branch: String,
    /// Priority for the backport bead (1=highest for security, 2=normal).
    pub priority: i32,
    /// Commit timestamp.
    pub commit_date: DateTime<Utc>,
}

impl BackportCandidate {
    /// Create a new candidate.
    pub fn new(
        commit_sha: String,
        commit_message: String,
        reason: CandidateReason,
        issue: Option<&Issue>,
        pr_url: Option<String>,
        landed_on_branch: String,
        commit_date: DateTime<Utc>,
    ) -> Self {
        let commit_sha_short = commit_sha.chars().take(7).collect();
        let is_security = reason == CandidateReason::SecurityPatch;
        let priority = if is_security { 1 } else { 2 };

        Self {
            commit_sha,
            commit_sha_short,
            commit_message,
            reason,
            issue_number: issue.as_ref().map(|i| i.number),
            issue_title: issue.as_ref().map(|i| i.title.clone()),
            is_security,
            pr_url,
            landed_on_branch,
            priority,
            commit_date,
        }
    }

    /// Format the backport bead title.
    pub fn bead_title(&self, target_branch: &str) -> String {
        format!(
            "Backport {} to {}",
            self.commit_sha_short, target_branch
        )
    }

    /// Format the backport bead description.
    pub fn bead_description(&self, target_branch: &str) -> String {
        let issue_ref = self
            .issue_number
            .map(|n| format!("#{}", n))
            .unwrap_or_else(|| "(none)".to_string());

        format!(
            r#"Plan: plans/backport-plan.md

Backport for: {0} - {1}
Source issue: {2}
Target branch: {3}

WHAT TO DO
Cherry-pick commit {0} to {3}. Create a PR targeting {3} with the
cherry-pick. Resolve any merge conflicts.

ACCEPTANCE
- [ ] Cherry-pick of {0} applies cleanly to {3} (or conflicts resolved)
- [ ] PR is open targeting {3}
- [ ] CI passes on the backport PR
- [ ] PR is merged or given explicit approval to close without merging

PITFALLS
- If the fix requires changes to shared library code that has diverged
  between main and the target branch, the cherry-pick may require
  manual conflict resolution. Document any non-trivial conflicts
  in the bead before closing.
"#,
            self.commit_sha,
            self.commit_message,
            issue_ref,
            target_branch,
        )
    }
}

/// Result of a backport detection run.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetectionResult {
    /// Candidates found in this run.
    pub candidates: Vec<BackportCandidate>,
    /// Total merged PRs checked.
    pub checked: usize,
}

impl DetectionResult {
    /// Add a candidate to the result.
    pub fn add_candidate(&mut self, candidate: BackportCandidate) {
        self.candidates.push(candidate);
    }

    /// Increment the checked count.
    pub fn checked_one(&mut self) {
        self.checked += 1;
    }
}

/// Backport detector.
///
/// Evaluates merged commits and issues to identify backport candidates.
#[derive(Debug, Clone)]
pub struct BackportDetector {
    /// GitHub client.
    github: GitHubClient,
    /// LLM client for semantic equivalence check.
    llm: Option<LlmClient>,
    /// Release configuration.
    release_config: ReleaseConfig,
    /// Security label name from project config.
    security_label: String,
}

impl BackportDetector {
    /// Create a new detector.
    pub fn new(
        github: GitHubClient,
        llm: Option<LlmClient>,
        release_config: ReleaseConfig,
        security_label: String,
    ) -> Self {
        Self {
            github,
            llm,
            release_config,
            security_label,
        }
    }

    /// Detect candidates from merged PRs merged since the given timestamp.
    ///
    /// The `since` parameter limits the scan to PRs merged after this time.
    /// This is typically the last run timestamp stored in bead state.
    pub async fn detect_candidates(&mut self, since: Option<DateTime<Utc>>) -> Result<DetectionResult> {
        let mut result = DetectionResult::default();

        // Get merged pull requests
        let prs = self.github.list_pull_requests(Some("merged"), None, None).await?;

        for pr in prs {
            result.checked_one();

            // Filter by date if provided
            if let Some(cutoff) = since {
                if let Some(merged_at) = pr.merged_at {
                    if merged_at < cutoff {
                        continue;
                    }
                }
            }

            // Check if the PR has a linked issue
            if let Err(e) = self.check_pr(&mut result, &pr).await {
                tracing::warn!("Error checking PR #{}: {}", pr.number, e);
            }
        }

        Ok(result)
    }

    /// Check a single merged PR for backport candidacy.
    async fn check_pr(
        &mut self,
        result: &mut DetectionResult,
        pr: &PullRequest,
    ) -> Result<()> {
        // Get the base branch (where the PR landed)
        let landed_on_branch = pr.base.ref_name.clone();

        // Determine if it's a merge commit (not squash/linear)
        let commit_sha = pr.merge_commit_sha.clone().unwrap_or_default();

        if commit_sha.is_empty() {
            return Ok(());
        }

        // For merged PRs with bug label or backport-me label, check candidacy
        let issue_num = self.extract_issue_number(pr);

        let issue = if let Some(num) = issue_num {
            self.github.get_issue(num).await.ok()
        } else {
            None
        };

        // Check for backport reasons
        if let Some(candidate) = self.evaluate_candidate(pr, &issue, &landed_on_branch, &commit_sha).await? {
            result.add_candidate(candidate);
        }

        Ok(())
    }

    /// Extract issue number from PR body (e.g., "Closes #123", "Fixes #456").
    fn extract_issue_number(&self, pr: &PullRequest) -> Option<i32> {
        let body = pr.body.as_deref().unwrap_or("");

        for pattern in ["closes #", "closes #", "fixes #", "fixes #", "resolves #"] {
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

    /// Evaluate whether a PR/issue is a backport candidate.
    async fn evaluate_candidate(
        &mut self,
        pr: &PullRequest,
        issue: &Option<Issue>,
        landed_on_branch: &str,
        commit_sha: &str,
    ) -> Result<Option<BackportCandidate>> {
        let issue = issue.as_ref();

        // Check 1: Bug label
        let has_bug = issue
            .map(|i| i.labels.iter().any(|l| l.name == "bug"))
            .unwrap_or(false);

        // Check 2: Security label
        let has_security = issue
            .map(|i| i.labels.iter().any(|l| l.name == self.security_label))
            .unwrap_or(false);

        // Check 3: Security Advisory (GHSA) - check issue body for GHSA reference
        let has_ghsa = issue
            .map(|i| {
                i.body
                    .as_deref()
                    .unwrap_or("")
                    .contains("GHSA-")
            })
            .unwrap_or(false);

        // Check 4: CVE pattern in body
        let has_cve = issue
            .as_ref()
            .map(|i| {
                Self::contains_cve(i.body.as_deref().unwrap_or(""))
            })
            .unwrap_or(false)
            || Self::contains_cve(pr.body.as_deref().unwrap_or(""));

        // Check 5: backport-me label
        let has_backport_me = issue
            .map(|i| i.labels.iter().any(|l| l.name == "backport-me"))
            .unwrap_or(false);

        // Check 6: Documentation fix (labels contain "docs" or "documentation")
        let is_doc_fix = issue
            .map(|i| {
                i.labels.iter().any(|l| {
                    let name = l.name.to_lowercase();
                    name == "docs" || name == "documentation" || name == "doc-fix"
                })
            })
            .unwrap_or(false);

        let reason = if has_bug {
            CandidateReason::BugFix
        } else if has_security || has_ghsa || has_cve {
            CandidateReason::SecurityPatch
        } else if has_backport_me {
            CandidateReason::BackportMe
        } else if is_doc_fix {
            CandidateReason::DocumentationFix
        } else {
            return Ok(None);
        };

        let commit_date = pr.merged_at.unwrap_or_else(Utc::now);

        Ok(Some(BackportCandidate::new(
            commit_sha.to_string(),
            pr.title.clone(),
            reason,
            issue,
            pr.html_url.clone(),
            landed_on_branch.to_string(),
            commit_date,
        )))
    }

    /// Check if a string contains a CVE identifier (pattern: CVE-YYYY-NNNNN).
    pub(crate) fn contains_cve(text: &str) -> bool {
        let upper = text.to_uppercase();
        if let Some(pos) = upper.find("CVE-") {
            let rest = &upper[pos..];
            // CVE-YYYY-NNNNN+ needs at least 13 chars
            if rest.len() >= 13 {
                let digits = &rest[4..];
                if digits[..4].chars().all(|c| c.is_ascii_digit())
                    && digits.as_bytes()[4] == b'-'
                {
                    let after = &digits[5..];
                    return after.len() >= 4 && after.chars().take(4).all(|c| c.is_ascii_digit());
                }
            }
        }
        false
    }

    /// Get active release branches from configuration.
    pub fn active_branches(&self) -> Vec<&str> {
        self.release_config
            .active_branches
            .as_deref()
            .unwrap_or(&[])
            .iter()
            .map(|s| s.as_str())
            .collect()
    }

    /// Check if a target branch already contains the fix (semantic equivalence).
    ///
    /// Uses compare API to check if the exact commit is already present,
    /// or uses LLM to judge whether functionally equivalent changes exist.
    pub async fn is_semantically_equivalent(
        &mut self,
        source_sha: &str,
        target_branch: &str,
    ) -> Result<bool> {
        // Fast path: check if the exact commit is already on the target branch
        let compare = self
            .github
            .compare_commits(target_branch, source_sha)
            .await?;

        // "behind" with 0 ahead means the target branch has this commit
        // "identical" also means the fix is already there
        if compare.status == "identical" || (compare.status == "behind" && compare.ahead_by == 0) {
            tracing::debug!(
                "Commit {} is already present on {} (status: {})",
                source_sha,
                target_branch,
                compare.status
            );
            return Ok(true);
        }

        // If LLM is available, do semantic equivalence check
        // Clone the LLM so we can release the self borrow before any awaits
        let llm_clone = self.llm.clone();
        if let Some(llm) = llm_clone {
            let source_files = self.github.get_commit_files(source_sha).await.unwrap_or_default();

            if source_files.is_empty() {
                return Ok(false);
            }

            // Get recent commits on target branch to compare against
            let recent_commits = self
                .github
                .list_commits(Some(target_branch), None, None, Some(20))
                .await
                .unwrap_or_default();

            // Collect diff pairs for LLM comparison
            let mut diff_pairs: Vec<(String, String)> = Vec::new();

            for recent in recent_commits {
                // Skip if same SHA (we already checked this above)
                if recent.sha == source_sha {
                    return Ok(true);
                }

                let recent_files = self
                    .github
                    .get_commit_files(&recent.sha)
                    .await
                    .unwrap_or_default();

                // If they touch the same files, collect for semantic check
                if source_files.iter().any(|f| recent_files.contains(f)) {
                    let source_diff = self.fetch_files_diff(source_sha, &source_files).await;
                    let recent_diff = self.fetch_files_diff(&recent.sha, &recent_files).await;
                    diff_pairs.push((source_diff, recent_diff));
                }
            }

            // Check semantic equivalence using LLM (ensure self.llm is not borrowed)
            for (source_diff, recent_diff) in diff_pairs {
                if self.check_semantic_equivalence_internal(&llm, &source_diff, &recent_diff).await {
                    tracing::debug!(
                        "Commit {} is semantically equivalent to recent work on {}",
                        source_sha,
                        target_branch
                    );
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    /// Internal semantic equivalence check using the LLM.
    /// Takes owned LLM to avoid borrow conflicts.
    async fn check_semantic_equivalence_internal(
        &mut self,
        llm: &LlmClient,
        source_diff: &str,
        target_diff: &str,
    ) -> bool {
        use crate::llm::client::{ChatMessage, ChatRequest};

        if source_diff.is_empty() || target_diff.is_empty() {
            return false;
        }

        let prompt = format!(
            r#"You are comparing two code changes to determine if they are semantically equivalent.

CHANGE A (target branch):
{}

CHANGE B (source):
{}

Are these two changes functionally equivalent (fixing the same bug or implementing the same behavior)?
Answer only "yes" or "no" with no other text."#,
            source_diff, target_diff
        );

        let request = ChatRequest {
            model: llm.model().to_string(),
            messages: vec![ChatMessage::user(prompt)],
            temperature: Some(0.0),
            max_tokens: Some(10),
            response_format: None,
        };

        match llm.chat(request).await {
            Ok(response) => {
                let content = response.choices[0].message.content.trim().to_lowercase();
                content.starts_with("yes")
            }
            Err(e) => {
                tracing::warn!("LLM semantic equivalence check failed: {}", e);
                false
            }
        }
    }

    /// Fetch diff for changed files in a commit.
    async fn fetch_files_diff(&mut self, sha: &str, files: &[String]) -> String {
        let mut diffs = Vec::new();
        for file in files {
            let file_diff = self
                .github
                .get_commit_file_diff(sha, file)
                .await
                .unwrap_or_default();
            if !file_diff.is_empty() {
                diffs.push(format!("File: {}\n{}", file, file_diff));
            }
        }
        diffs.join("\n---\n")
    }

    /// Check semantic equivalence using the LLM.
    async fn check_semantic_equivalence(
        &mut self,
        llm: &LlmClient,
        source_diff: &str,
        target_diff: &str,
    ) -> bool {
        use crate::llm::client::{ChatMessage, ChatRequest};

        if source_diff.is_empty() || target_diff.is_empty() {
            return false;
        }

        let prompt = format!(
            r#"You are comparing two code changes to determine if they are semantically equivalent.

CHANGE A (target branch):
{}

CHANGE B (source):
{}

Are these two changes functionally equivalent (fixing the same bug or implementing the same behavior)?
Answer only "yes" or "no" with no other text."#,
            source_diff, target_diff
        );

        let request = ChatRequest {
            model: llm.model().to_string(),
            messages: vec![ChatMessage::user(prompt)],
            temperature: Some(0.0),
            max_tokens: Some(10),
            response_format: None,
        };

        match llm.chat(request).await {
            Ok(response) => {
                let content = response.choices[0].message.content.trim().to_lowercase();
                content.starts_with("yes")
            }
            Err(e) => {
                tracing::warn!("LLM semantic equivalence check failed: {}", e);
                false
            }
        }
    }

    /// Get the configured voting window in days.
    pub fn voting_window_days(&self) -> u32 {
        self.release_config.voting_window_days.unwrap_or(2)
    }

    /// Get the configured stale threshold in days.
    pub fn stale_threshold_days(&self) -> u32 {
        self.release_config.stale_threshold_days.unwrap_or(7)
    }

    /// Get the approval discussion category.
    pub fn approval_category(&self) -> &str {
        self.release_config
            .approval_discussion_category
            .as_deref()
            .unwrap_or("Announcements")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contains_cve() {
        assert!(BackportDetector::contains_cve("Fixed in CVE-2024-12345"));
        assert!(BackportDetector::contains_cve("See CVE-2024-98765 for details"));
        assert!(BackportDetector::contains_cve("cve-2023-44445"));
        assert!(!BackportDetector::contains_cve("This is not a CVE"));
        assert!(!BackportDetector::contains_cve("CV-2024-12345")); // Wrong prefix
    }

    #[test]
    fn test_candidate_reason_display() {
        assert_eq!(CandidateReason::BugFix.to_string(), "bug_fix");
        assert_eq!(CandidateReason::SecurityPatch.to_string(), "security_patch");
        assert_eq!(CandidateReason::BackportMe.to_string(), "backport_me");
        assert_eq!(CandidateReason::DocumentationFix.to_string(), "documentation_fix");
    }

    #[test]
    fn test_candidate_new() {
        use chrono::Utc;

        let candidate = BackportCandidate::new(
            "abc123def456789".to_string(),
            "Fix bug in login".to_string(),
            CandidateReason::BugFix,
            None,
            Some("https://github.com/test/pull/1".to_string()),
            "main".to_string(),
            Utc::now(),
        );

        assert_eq!(candidate.commit_sha_short, "abc123d");
        assert_eq!(candidate.priority, 2);
        assert!(!candidate.is_security);
    }

    #[test]
    fn test_security_candidate_priority() {
        use chrono::Utc;

        let candidate = BackportCandidate::new(
            "abc123def456789".to_string(),
            "Fix security issue".to_string(),
            CandidateReason::SecurityPatch,
            None,
            Some("https://github.com/test/pull/1".to_string()),
            "main".to_string(),
            Utc::now(),
        );

        assert_eq!(candidate.priority, 1);
        assert!(candidate.is_security);
    }

    #[test]
    fn test_bead_title() {
        use chrono::Utc;

        let candidate = BackportCandidate::new(
            "abc123def456789".to_string(),
            "Fix bug".to_string(),
            CandidateReason::BugFix,
            None,
            None,
            "main".to_string(),
            Utc::now(),
        );

        assert_eq!(
            candidate.bead_title("release/1.x"),
            "Backport abc123d to release/1.x"
        );
    }

    #[test]
    fn test_detection_result_default() {
        let result = DetectionResult::default();
        assert!(result.candidates.is_empty());
        assert_eq!(result.checked, 0);
    }

    #[test]
    fn test_detection_result_add() {
        use chrono::Utc;

        let mut result = DetectionResult::default();
        result.checked_one();
        result.checked_one();

        let candidate = BackportCandidate::new(
            "abc123".to_string(),
            "Fix bug".to_string(),
            CandidateReason::BugFix,
            None,
            None,
            "main".to_string(),
            Utc::now(),
        );

        result.add_candidate(candidate);
        assert_eq!(result.checked, 2);
        assert_eq!(result.candidates.len(), 1);
    }
}