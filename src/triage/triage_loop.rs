//! Triage loop for processing GitHub issues.
//!
//! The triage loop is the core workflow that processes new issues and:
//! - Checks if they were filed using an issue template
//! - Posts reformat offers for non-conforming issues within one triage run
//! - Ensures one offer only (tracks via label)
//!
//! ## One Triage Run
//!
//! "One triage run" refers to a single scheduler tick. The goal is to provide
//! fast feedback to requestors — their non-conforming issue receives a reformat
//! offer comment immediately upon detection.

// Note: This module defines the types and logic for the triage loop.
// GitHub API interactions would be implemented in a separate module once
// the GitHub API client is available.

use serde::{Deserialize, Serialize};

/// GitHub issue state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IssueState {
    Open,
    Closed,
}

/// A GitHub issue reference.
///
/// Minimal representation of an issue for triage processing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubIssue {
    /// The issue number.
    pub number: u64,
    /// The issue title.
    pub title: String,
    /// The issue body text.
    pub body: String,
    /// Current state of the issue.
    pub state: IssueState,
    /// Label names applied to the issue.
    pub labels: Vec<String>,
    /// Username of the issue author.
    pub author: String,
}

impl GitHubIssue {
    /// Create a new GitHub issue reference.
    pub fn new(
        number: u64,
        title: String,
        body: String,
        state: IssueState,
        labels: Vec<String>,
        author: String,
    ) -> Self {
        Self {
            number,
            title,
            body,
            state,
            labels,
            author,
        }
    }

    /// Check if the issue is open.
    pub fn is_open(&self) -> bool {
        self.state == IssueState::Open
    }

    /// Check if the issue has a specific label.
    pub fn has_label(&self, label: &str) -> bool {
        self.labels.contains(&label.to_string())
    }

    /// Get the author mention for comments.
    pub fn author_mention(&self) -> String {
        format!("@{}", self.author)
    }
}

/// Result of processing a single issue through triage.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageResult {
    /// Whether the issue was processed successfully.
    pub success: bool,
    /// Whether a reformat offer was posted.
    pub offer_posted: bool,
    /// Optional error message if processing failed.
    pub error: Option<String>,
}

impl TriageResult {
    /// Create a successful result with no offer posted.
    pub fn success() -> Self {
        Self {
            success: true,
            offer_posted: false,
            error: None,
        }
    }

    /// Create a successful result with an offer posted.
    pub fn offer_posted() -> Self {
        Self {
            success: true,
            offer_posted: true,
            error: None,
        }
    }

    /// Create a failure result.
    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            success: false,
            offer_posted: false,
            error: Some(message.into()),
        }
    }
}

/// The triage loop configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriageConfig {
    /// Whether to enable reformat offer posting.
    pub reformat_enabled: bool,
    /// The label added when reformat offer has been sent.
    pub offer_sent_label: &'static str,
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            reformat_enabled: true,
            offer_sent_label: crate::triage::reformat_offer::OFFER_SENT_LABEL,
        }
    }
}

/// Process a single issue through the triage loop.
///
/// This function:
/// 1. Checks if the issue is open
/// 2. Checks if the issue already has the reformat offer sent label
/// 3. Checks for template conformance
/// 4. If non-conforming and no offer sent, returns `needs_offer = true`
///
/// # Arguments
///
/// * `issue` - The issue to process
/// * `config` - The triage configuration
///
/// # Returns
///
/// A `TriageResult` indicating what action should be taken
pub fn process_issue(issue: &GitHubIssue, config: &TriageConfig) -> TriageResult {
    // Step 1: Check if issue is open
    if !issue.is_open() {
        return TriageResult::success();
    }

    // Step 2: Check if reformat offer already sent
    if issue.has_label(config.offer_sent_label) {
        return TriageResult::success();
    }

    // Step 3: Check conformance
    let conformance = crate::templates::check_conformance(&issue.body);
    if conformance.is_conforming {
        return TriageResult::success();
    }

    // Step 4: Non-conforming issue - offer to reformat
    if config.reformat_enabled {
        TriageResult::offer_posted()
    } else {
        TriageResult::success()
    }
}

/// Process a batch of issues through triage.
///
/// # Arguments
///
/// * `issues` - The issues to process
/// * `config` - The triage configuration
///
/// # Returns
///
/// A vector of `TriageResult` for each issue
pub fn process_issues(issues: &[GitHubIssue], config: &TriageConfig) -> Vec<TriageResult> {
    issues
        .iter()
        .map(|issue| process_issue(issue, config))
        .collect()
}

/// Count issues that need reformat offers in a batch.
///
/// # Arguments
///
/// * `issues` - The issues to process
/// * `config` - The triage configuration
///
/// # Returns
///
/// The number of issues that need reformat offers
pub fn count_needs_offer(issues: &[GitHubIssue], config: &TriageConfig) -> usize {
    process_issues(issues, config)
        .into_iter()
        .filter(|r| r.offer_posted)
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_issue(body: &str, labels: Vec<&str>, state: IssueState) -> GitHubIssue {
        GitHubIssue::new(
            1,
            "Test Issue".to_string(),
            body.to_string(),
            state,
            labels.into_iter().map(|s| s.to_string()).collect(),
            "testuser".to_string(),
        )
    }

    fn config() -> TriageConfig {
        TriageConfig::default()
    }

    #[test]
    fn test_issue_is_open() {
        let issue = make_issue("", vec![], IssueState::Open);
        assert!(issue.is_open());
    }

    #[test]
    fn test_issue_is_closed() {
        let issue = make_issue("", vec![], IssueState::Closed);
        assert!(!issue.is_open());
    }

    #[test]
    fn test_issue_has_label() {
        let issue = make_issue("", vec!["bug", "help"], IssueState::Open);
        assert!(issue.has_label("bug"));
        assert!(issue.has_label("help"));
        assert!(!issue.has_label("feature"));
    }

    #[test]
    fn test_issue_author_mention() {
        let issue = make_issue("", vec![], IssueState::Open);
        assert_eq!(issue.author_mention(), "@testuser");
    }

    #[test]
    fn test_process_conforming_issue() {
        let issue = make_issue(
            "Bug content\n<!-- template: bug_report -->",
            vec![],
            IssueState::Open,
        );
        let result = process_issue(&issue, &config());
        assert!(result.success);
        assert!(!result.offer_posted);
    }

    #[test]
    fn test_process_non_conforming_issue_needs_offer() {
        let issue = make_issue("Freeform issue without template", vec![], IssueState::Open);
        let result = process_issue(&issue, &config());
        assert!(result.success);
        assert!(result.offer_posted);
    }

    #[test]
    fn test_process_non_conforming_with_offer_sent_label() {
        let issue = make_issue(
            "Freeform issue without template",
            vec!["reformat-offer-sent"],
            IssueState::Open,
        );
        let result = process_issue(&issue, &config());
        assert!(result.success);
        assert!(!result.offer_posted);
    }

    #[test]
    fn test_process_closed_issue() {
        let issue = make_issue(
            "Freeform issue without template",
            vec![],
            IssueState::Closed,
        );
        let result = process_issue(&issue, &config());
        assert!(result.success);
        assert!(!result.offer_posted);
    }

    #[test]
    fn test_process_email_reply_issue() {
        let issue = make_issue(
            "GitHub Email Reply\nSome content here",
            vec![],
            IssueState::Open,
        );
        let result = process_issue(&issue, &config());
        // Email replies are non-conforming
        assert!(result.success);
        assert!(result.offer_posted);
    }

    #[test]
    fn test_process_with_reformat_disabled() {
        let issue = make_issue("Freeform issue without template", vec![], IssueState::Open);
        let mut cfg = TriageConfig::default();
        cfg.reformat_enabled = false;
        let result = process_issue(&issue, &cfg);
        assert!(result.success);
        assert!(!result.offer_posted);
    }

    #[test]
    fn test_process_issues_batch() {
        let issues = vec![
            make_issue("<!-- template: bug_report -->", vec![], IssueState::Open),
            make_issue("Freeform issue 1", vec![], IssueState::Open),
            make_issue("Freeform issue 2", vec![], IssueState::Open),
            make_issue(
                "<!-- template: feature_request -->",
                vec![],
                IssueState::Open,
            ),
        ];
        let results = process_issues(&issues, &config());
        assert_eq!(results.len(), 4);
        // 0 conforming + 2 non-conforming = 2 offers
        assert_eq!(results.iter().filter(|r| r.offer_posted).count(), 2);
    }

    #[test]
    fn test_count_needs_offer() {
        let issues = vec![
            make_issue("<!-- template: bug_report -->", vec![], IssueState::Open),
            make_issue("Freeform issue 1", vec![], IssueState::Open),
            make_issue("Freeform issue 2", vec![], IssueState::Open),
        ];
        assert_eq!(count_needs_offer(&issues, &config()), 2);
    }

    #[test]
    fn test_triage_result_success() {
        let result = TriageResult::success();
        assert!(result.success);
        assert!(!result.offer_posted);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_triage_result_offer_posted() {
        let result = TriageResult::offer_posted();
        assert!(result.success);
        assert!(result.offer_posted);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_triage_result_failure() {
        let result = TriageResult::failure("API error");
        assert!(!result.success);
        assert!(!result.offer_posted);
        assert_eq!(result.error, Some("API error".to_string()));
    }
}
