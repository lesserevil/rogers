//! Backport candidate detection.
//!
//! Inspects merged PRs and determines which ones are backport candidates:
//!  - Bug fix: commit linked to a GitHub issue labeled `bug`
//!  - Security patch: GH Advisory match, `security` label, or CVE pattern
//!  - `backport-me` label: human explicitly requested backport
//!
//! Each detected candidate returns a [`BackportCandidate`] with enough context
//! to allow the manager to file backport beads for all active release branches.
//!
//! Security patch detection is delegated to [`super::security`].

use regex::Regex;
use std::sync::OnceLock;
use tracing::{debug, info, warn};

use super::security::{self, SecuritySignal};
use crate::Config;
use crate::github::client::{GithubClient, MergedPr};

static BACKPORT_ME_PATTERN: OnceLock<Regex> = OnceLock::new();

fn backport_me_pattern() -> &'static Regex {
    BACKPORT_ME_PATTERN
        .get_or_init(|| Regex::new(r"(?i)backport[- ]?me").expect("hardcoded regex is valid"))
}

/// A merge to main that is a candidate for backporting.
#[derive(Debug, Clone)]
pub struct BackportCandidate {
    /// The merged PR that triggered detection.
    pub pr: MergedPr,
    /// Classification of why this is a candidate.
    pub reason: BackportReason,
    /// Priority: 1 = security (highest), 2 = normal.
    pub priority: u8,
}

/// Why this PR is flagged as a backport candidate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BackportReason {
    /// The linked issue is labeled `bug`.
    BugFix,
    /// GH Advisory, security label, or CVE pattern detected.
    SecurityPatch,
    /// GitHub issue has the `backport-me` label (or close variants).
    BackportMeLabel,
}

impl BackportCandidate {
    #[allow(dead_code)]
    pub(crate) fn new(pr: MergedPr, reason: BackportReason) -> Self {
        let priority = if reason == BackportReason::SecurityPatch {
            1
        } else {
            2
        };
        Self {
            pr,
            reason,
            priority,
        }
    }
}

/// Scan merged PRs and return those that are backport candidates.
///
/// This function is idempotent: calling it twice in the same run with the same
/// input returns the same result (no state is mutated).
pub async fn detect_candidates(
    merged_prs: Vec<MergedPr>,
    github: &GithubClient,
    config: &Config,
) -> Result<Vec<BackportCandidate>, crate::RogersError> {
    let mut candidates = Vec::new();

    for pr in merged_prs {
        match classify_pr(&pr, github, config).await {
            Ok(Some(candidate)) => {
                info!(
                    "Backport candidate detected: PR #{} — {:?} (priority={})",
                    candidate.pr.number, candidate.reason, candidate.priority
                );
                candidates.push(candidate);
            }
            Ok(None) => {
                debug!("PR #{} is not a backport candidate", pr.number);
            }
            Err(e) => {
                warn!("Error classifying PR #{}: {}; skipping", pr.number, e);
            }
        }
    }

    Ok(candidates)
}

/// Determine whether a single merged PR is a backport candidate.
///
/// The logic:
/// 1. Security patch detection (highest priority, priority=1)
///    - GH Advisory linked to issue
///    - `security` label on the issue
///    - CVE pattern in PR title or body OR linked issue body
/// 2. Bug fix detection (priority=2)
///    - Issue linked to PR is labeled `bug`
/// 3. Explicit backport request (priority=2)
///    - Issue labeled with `backport-me` pattern
///
/// Security takes priority over bug-label detection.
async fn classify_pr(
    pr: &MergedPr,
    github: &GithubClient,
    config: &Config,
) -> Result<Option<BackportCandidate>, crate::RogersError> {
    let security_label = &config.rogation.security_label;

    // Signal 1: Security detection (highest priority, returns priority=1)
    if let Some(reason) = detect_security_patch(pr, github, security_label).await? {
        return Ok(Some(BackportCandidate::new(pr.clone(), reason)));
    }

    // Signal 2: Bug fix detection
    if let Some(candidate) = detect_bug_fix(pr, github).await? {
        return Ok(Some(candidate));
    }

    // Signal 3: backport-me label detection
    if is_backport_me(pr, github).await? {
        return Ok(Some(BackportCandidate::new(
            pr.clone(),
            BackportReason::BackportMeLabel,
        )));
    }

    Ok(None)
}

/// Detect if the PR is a security patch.
///
/// Delegates to [`super::security::detect_security_signals`] which checks all
/// three signals: GH Advisory match, security label, and CVE pattern.
async fn detect_security_patch(
    pr: &MergedPr,
    github: &GithubClient,
    security_label: &str,
) -> Result<Option<BackportReason>, crate::RogersError> {
    // Extract linked issue number from PR body for security checks
    let body = pr.body.as_deref().unwrap_or("");
    let issue_number =
        issue_ref_capture(body).and_then(|caps| caps.get(1).and_then(|m| m.as_str().parse().ok()));

    // Use the dedicated security module to detect signals
    let signals = security::detect_security_signals(
        github,
        security_label,
        &pr.title,
        pr.body.as_deref(),
        issue_number,
    )
    .await?;

    if !signals.is_empty() {
        for signal in &signals {
            match signal {
                SecuritySignal::GHAdvisory { ghsa_id } => {
                    info!(
                        "PR #{} has GH Advisory '{}'; flagging as security patch",
                        pr.number, ghsa_id
                    );
                }
                SecuritySignal::SecurityLabel { label_name } => {
                    info!(
                        "PR #{} has security label '{}'; flagging as security patch",
                        pr.number, label_name
                    );
                }
                SecuritySignal::CvePattern { cve_id } => {
                    info!(
                        "PR #{} contains CVE '{}'; flagging as security patch",
                        pr.number, cve_id
                    );
                }
            }
        }
        return Ok(Some(BackportReason::SecurityPatch));
    }

    Ok(None)
}

/// Detect if the merged PR's linked issue is labeled `bug`.
async fn detect_bug_fix(
    pr: &MergedPr,
    github: &GithubClient,
) -> Result<Option<BackportCandidate>, crate::RogersError> {
    let body = pr.body.as_deref().unwrap_or("");

    let Some(caps) = issue_ref_capture(body) else {
        debug!(
            "PR #{} body contains no issue reference; not classifying as bug fix",
            pr.number
        );
        return Ok(None);
    };

    let issue_num: u64 = caps[1].parse().unwrap_or(0);
    if issue_num == 0 {
        return Ok(None);
    }

    let labels = github.issue_labels(issue_num).await.unwrap_or_default();
    for label in &labels {
        if label.name.eq_ignore_ascii_case("bug") {
            info!(
                "PR #{} linked to bug issue #{}; flagging as bug fix",
                pr.number, issue_num
            );
            return Ok(Some(BackportCandidate::new(
                pr.clone(),
                BackportReason::BugFix,
            )));
        }
    }

    Ok(None)
}

/// Returns true if the PR's linked issue has a `backport-me` label.
async fn is_backport_me(pr: &MergedPr, github: &GithubClient) -> Result<bool, crate::RogersError> {
    let body = pr.body.as_deref().unwrap_or("");
    let Some(caps) = issue_ref_capture(body) else {
        return Ok(false);
    };
    let issue_num: u64 = caps[1].parse().unwrap_or(0);
    if issue_num == 0 {
        return Ok(false);
    }
    let labels = github.issue_labels(issue_num).await.unwrap_or_default();
    for label in &labels {
        if backport_me_pattern().is_match(&label.name) {
            info!(
                "PR #{} linked to issue #{} with backport-me label '{}'",
                pr.number, issue_num, label.name
            );
            return Ok(true);
        }
    }
    Ok(false)
}

/// Match "closes #N", "fixes #N", "resolves #N", or bare "#N" in text.
fn issue_ref_capture<'a>(text: &'a str) -> Option<regex::Captures<'a>> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r"(?:(?:closes?|fixes?|resolves?|[Rr]eferences?)\s+)?#(\d+)")
            .expect("hardcoded regex is valid")
    });
    re.captures(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::client::{GithubLabel, GithubUser};

    fn make_pr(number: u64, title: &str, body: Option<&str>, labels: Vec<&str>) -> MergedPr {
        MergedPr {
            number,
            title: title.to_string(),
            body: body.map(String::from),
            merged_at: Some("2024-01-01T00:00:00Z".to_string()),
            merge_commit_sha: Some("abc123def456".to_string()),
            user: GithubUser {
                login: "test-user".to_string(),
                user_type: "User".to_string(),
            },
            labels: labels
                .into_iter()
                .map(|n| GithubLabel {
                    name: n.to_string(),
                    color: "fefefe".to_string(),
                })
                .collect(),
            state: "closed".to_string(),
        }
    }

    #[test]
    fn test_backport_me_pattern() {
        let re = backport_me_pattern();
        assert!(re.is_match("backport-me"));
        assert!(re.is_match("backport me"));
        assert!(re.is_match("Backport-Me"));
        assert!(!re.is_match("backport-to-release"));
    }

    #[test]
    fn test_issue_ref_capture() {
        assert_eq!(&issue_ref_capture("Closes #12345").unwrap()[1], "12345");
        assert_eq!(&issue_ref_capture("fixes #1").unwrap()[1], "1");
        assert_eq!(&issue_ref_capture("Resolves #999").unwrap()[1], "999");
        assert_eq!(&issue_ref_capture("See #42").unwrap()[1], "42");
        assert!(issue_ref_capture("No issue here").is_none());
    }

    #[test]
    fn test_priority_security_is_highest() {
        let pr = make_pr(1, "Fix security vuln", None, vec!["security"]);
        let c = BackportCandidate::new(pr, BackportReason::SecurityPatch);
        assert_eq!(c.priority, 1);

        let pr2 = make_pr(2, "Fix bug", Some("Closes #10"), vec![]);
        let c2 = BackportCandidate::new(pr2, BackportReason::BugFix);
        assert_eq!(c2.priority, 2);
    }

    #[test]
    fn test_backport_me_reason() {
        let pr = make_pr(3, "Update README", Some("Closes #77"), vec![]);
        let c = BackportCandidate::new(pr, BackportReason::BackportMeLabel);
        assert_eq!(c.reason, BackportReason::BackportMeLabel);
        assert_eq!(c.priority, 2);
    }
}
