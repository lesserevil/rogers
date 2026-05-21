//! Security patch detection module.
//!
//! Detects security patches via three independent signals (any one triggers
//! a security classification):
//!
//! 1. **GH Advisory match** — The repository has a GitHub Security Advisory
//!    (GHSA) associated with the issue or commit.
//! 2. **Security label** — The issue has a configurable security label
//!    (default: `"security"`, configurable via `rogation.security_label`).
//! 3. **CVE pattern** — The commit message or issue body matches
//!    `CVE-\d{4}-\d{4,}` (e.g., `CVE-2024-12345`).
//!
//! When any signal is detected, the caller should file a backport bead
//! with `priority=1` (highest) so it is backported to ALL active branches.

use regex::Regex;
use std::sync::OnceLock;
use tracing::info;

use crate::github::client::{GithubAdvisory, GithubClient};

/// A security signal that was detected.
///
/// Each variant corresponds to one of the three detection methods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecuritySignal {
    /// A GitHub Security Advisory (GHSA) was found for this issue/commit.
    GHAdvisory { ghsa_id: String },
    /// The issue has the configured security label.
    SecurityLabel { label_name: String },
    /// A CVE identifier was found in the commit message or issue body.
    CvePattern { cve_id: String },
}

impl SecuritySignal {
    /// Returns a human-readable description of this signal.
    pub fn description(&self) -> &str {
        match self {
            Self::GHAdvisory { ghsa_id } => ghsa_id,
            Self::SecurityLabel { label_name } => label_name,
            Self::CvePattern { cve_id } => cve_id,
        }
    }
}

/// CVE identifier pattern as documented in the plan.
/// Matches strings like "CVE-2024-12345" (4-digit year, 4+ digit ID).
static CVE_RE: OnceLock<Regex> = OnceLock::new();

pub fn cve_re() -> &'static Regex {
    CVE_RE.get_or_init(|| Regex::new(r"CVE-\d{4}-\d{4,}").expect("hardcoded regex is valid"))
}

/// All active signals detected for a given context.
///
/// Returns an empty vec if no security signals were found.
/// The caller should treat any non-empty result as a security patch
/// requiring priority=1 backport beads.
pub async fn detect_security_signals(
    gh_client: &GithubClient,
    security_label: &str,
    pr_title: &str,
    pr_body: Option<&str>,
    issue_number: Option<u64>,
) -> Result<Vec<SecuritySignal>, crate::RogersError> {
    let mut signals = Vec::new();

    // Signal 1: CVE pattern in PR title or body
    for text in [pr_title, pr_body.unwrap_or_default()] {
        for cap in cve_re().find_iter(text) {
            signals.push(SecuritySignal::CvePattern {
                cve_id: cap.as_str().to_string(),
            });
        }
    }

    // Signal 2: Security label check on the linked issue
    if let Some(body) = pr_body {
        if let Some(issue_num) = extract_linked_issue(body) {
            // Best-effort: label fetch failure is not fatal
            let labels = match gh_client.issue_labels(issue_num).await {
                Ok(labels) => labels,
                Err(_) => Vec::new(),
            };
            for label in labels {
                if label.name.eq_ignore_ascii_case(security_label) {
                    signals.push(SecuritySignal::SecurityLabel {
                        label_name: label.name.clone(),
                    });
                }
            }
        }
    }

    // Signal 3: GH Advisory match
    // Check if any advisory is linked to this issue/commit.
    // GHAs are fetched and matched by CVE ID if the PR/body references one.
    let advisories = match fetch_repository_advisories(gh_client).await {
        Ok(a) => a,
        Err(_) => Vec::new(), // Best-effort: advisory fetch failure is not fatal
    };

    let cve_ids: Vec<String> = signals
        .iter()
        .filter_map(|s| match s {
            SecuritySignal::CvePattern { cve_id } => Some(cve_id.clone()),
            _ => None,
        })
        .collect();

    for advisory in &advisories {
        // If we have a CVE reference and this advisory matches
        if let Some(ref advisory_cve) = advisory.cve_id {
            if cve_ids.iter().any(|c| c == advisory_cve) {
                signals.push(SecuritySignal::GHAdvisory {
                    ghsa_id: advisory.ghsa_id.clone(),
                });
                break;
            }
        } else if let Some(ref advisory_summary) = advisory.summary {
            // Also match by summary text if it contains a CVE pattern
            for cve_id in &cve_ids {
                if advisory_summary.contains(cve_id) {
                    signals.push(SecuritySignal::GHAdvisory {
                        ghsa_id: advisory.ghsa_id.clone(),
                    });
                    break;
                }
            }
        }

        // If no CVE reference was found yet but we have GHAs,
        // check issue body/advisory connection
        if signals
            .iter()
            .all(|s| !matches!(s, SecuritySignal::GHAdvisory { .. }))
        {
            if let Some(ref _issue_num) = issue_number {
                if let Some(body) = pr_body {
                    if body.contains(&format!("GHSA-{}", &advisory.ghsa_id)) {
                        signals.push(SecuritySignal::GHAdvisory {
                            ghsa_id: advisory.ghsa_id.clone(),
                        });
                        break;
                    }
                }
            }
        }
    }

    if !signals.is_empty() {
        for signal in &signals {
            info!(
                "Security signal detected: {:?} for PR title: {:?}",
                signal,
                signal.description()
            );
        }
    }

    Ok(signals)
}

/// Check if a PR has any security signals.
///
/// Convenience wrapper that returns true if at least one signal was found.
pub async fn is_security_patch(
    gh_client: &GithubClient,
    security_label: &str,
    pr_title: &str,
    pr_body: Option<&str>,
    issue_number: Option<u64>,
) -> Result<bool, crate::RogersError> {
    let signals =
        detect_security_signals(gh_client, security_label, pr_title, pr_body, issue_number).await?;
    Ok(!signals.is_empty())
}

/// Fetch repository security advisories from the GitHub API.
async fn fetch_repository_advisories(
    gh_client: &GithubClient,
) -> Result<Vec<GithubAdvisory>, crate::RogersError> {
    gh_client.advisories().await
}

/// Extract a linked issue number from PR body text.
///
/// Matches patterns like "Closes #123", "Fixes #456", etc.
fn extract_linked_issue(body: &str) -> Option<u64> {
    static ISSUE_RE: OnceLock<Regex> = OnceLock::new();
    let re = ISSUE_RE.get_or_init(|| {
        Regex::new(r"(?i)(?:(?:closes?|fixes?|resolves?)|references?)\s+#(\d+)")
            .expect("hardcoded regex is valid")
    });
    re.captures(body)
        .and_then(|caps| caps.get(1))
        .and_then(|m| m.as_str().parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // CVE pattern tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_cve_pattern_matches_valid_cves() {
        assert!(cve_re().is_match("CVE-2024-12345"));
        assert!(cve_re().is_match("CVE-2023-999999999"));
        assert!(cve_re().is_match("Fixed CVE-2024-12345 in the code"));
        assert!(cve_re().is_match("See also CVE-2023-1234 and CVE-2024-5678"));
    }

    #[test]
    fn test_cve_pattern_rejects_invalid_cves() {
        assert!(!cve_re().is_match("CVE-24-12345")); // year too short
        assert!(!cve_re().is_match("CVE-2024-123")); // ID too short (3 digits, need 4+)
        assert!(!cve_re().is_match("CVE-2024-1")); // ID too short (1 digit)
        assert!(!cve_re().is_match("cve-2024-12345")); // lowercase (no case insensitive flag)
    }

    // -----------------------------------------------------------------------
    // Signal extraction tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_extract_linked_issue_closes() {
        assert_eq!(extract_linked_issue("Closes #12345"), Some(12345));
        assert_eq!(extract_linked_issue("Closes #1"), Some(1));
    }

    #[test]
    fn test_extract_linked_issue_fixes() {
        assert_eq!(extract_linked_issue("Fixes #42 the bug"), Some(42));
    }

    #[test]
    fn test_extract_linked_issue_resolves() {
        assert_eq!(
            extract_linked_issue("Resolves #999 - security issue"),
            Some(999)
        );
    }

    #[test]
    fn test_extract_linked_issue_references() {
        assert_eq!(extract_linked_issue("References #777"), Some(777));
    }

    #[test]
    fn test_extract_linked_issue_no_match() {
        assert_eq!(extract_linked_issue("No issue here"), None);
        assert_eq!(extract_linked_issue("Just a regular message"), None);
    }

    // -----------------------------------------------------------------------
    // Security signal detection tests (async — require live GitHub calls)
    // -----------------------------------------------------------------------

    /// CRIT-12: CVE pattern in PR title triggers security signal.
    #[tokio::test]
    async fn test_cve_in_title_triggers_signal() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let signals = detect_security_signals(
            &gh_client,
            "security",
            "Fix CVE-2024-12345 vulnerability",
            None,
            None,
        )
        .await
        .expect("should not error");
        let cve_signals: Vec<_> = signals
            .iter()
            .filter(|s| matches!(s, SecuritySignal::CvePattern { .. }))
            .collect();
        assert_eq!(cve_signals.len(), 1);
        assert!(
            matches!(&cve_signals[0], SecuritySignal::CvePattern { cve_id } if cve_id == "CVE-2024-12345")
        );
    }

    /// CRIT-12: CVE pattern in PR body triggers security signal.
    #[tokio::test]
    async fn test_cve_in_body_triggers_signal() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let signals = detect_security_signals(
            &gh_client,
            "security",
            "Security fix",
            Some("Closes #42 - fixed CVE-2023-99999"),
            None,
        )
        .await
        .expect("should not error");
        let cve_signals: Vec<_> = signals
            .iter()
            .filter(|s| matches!(s, SecuritySignal::CvePattern { .. }))
            .collect();
        assert_eq!(cve_signals.len(), 1);
    }

    /// CRIT-12: CVE pattern in PR title triggers security signal (multiple CVEs).
    #[tokio::test]
    async fn test_multiple_cves_in_title() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let signals = detect_security_signals(
            &gh_client,
            "security",
            "Fix CVE-2024-12345 and CVE-2024-67890",
            None,
            None,
        )
        .await
        .expect("should not error");
        let cve_signals: Vec<_> = signals
            .iter()
            .filter(|s| matches!(s, SecuritySignal::CvePattern { .. }))
            .collect();
        assert_eq!(cve_signals.len(), 2);
    }

    /// CRIT-12: No security signals when none of the patterns match.
    #[tokio::test]
    async fn test_no_security_signals_for_regular_bug() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let signals = detect_security_signals(
            &gh_client,
            "security",
            "Fix login crash bug",
            Some("Closes #42"),
            None,
        )
        .await
        .expect("should not error");
        assert!(
            signals.is_empty(),
            "Should detect no security signals for regular bug"
        );
    }

    /// CRIT-12: Security signal detection works with empty inputs.
    #[tokio::test]
    async fn test_empty_inputs_no_signals() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let signals = detect_security_signals(&gh_client, "security", "", None, None)
            .await
            .expect("should not error");
        assert!(signals.is_empty());
    }

    /// CRIT-12: Security signal detection with custom security label.
    #[tokio::test]
    async fn test_custom_security_label() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let signals = detect_security_signals(
            &gh_client,
            "security-critical", // custom label
            "Fix CVE-2024-12345",
            None,
            None,
        )
        .await
        .expect("should not error");
        // At minimum should detect the CVE pattern
        let has_cve = signals
            .iter()
            .any(|s| matches!(s, SecuritySignal::CvePattern { .. }));
        assert!(
            has_cve,
            "Should detect CVE pattern regardless of security label"
        );
    }

    /// CRIT-12: CVE pattern with various year formats.
    #[test]
    fn test_cve_pattern_various_years() {
        assert!(cve_re().is_match("CVE-2020-12345"));
        assert!(cve_re().is_match("CVE-2021-99999"));
        assert!(cve_re().is_match("CVE-2022-1234"));
        assert!(cve_re().is_match("CVE-2023-123456"));
        assert!(cve_re().is_match("CVE-2024-1234")); // minimum: 4-digit ID
        assert!(cve_re().is_match("CVE-2024-12345678")); // long ID
    }

    /// CRIT-12: CVE signal description returns the CVE ID.
    #[test]
    fn test_cve_signal_description() {
        let signal = SecuritySignal::CvePattern {
            cve_id: "CVE-2024-12345".to_string(),
        };
        assert_eq!(signal.description(), "CVE-2024-12345");
    }

    /// CRIT-12: GHAdvisory signal description returns the GHSA ID.
    #[test]
    fn test_ghadvisory_signal_description() {
        let signal = SecuritySignal::GHAdvisory {
            ghsa_id: "GHSA-abc1-2345-def6".to_string(),
        };
        assert_eq!(signal.description(), "GHSA-abc1-2345-def6");
    }

    /// CRIT-12: SecurityLabel signal description returns the label name.
    #[test]
    fn test_securitylabel_signal_description() {
        let signal = SecuritySignal::SecurityLabel {
            label_name: "security".to_string(),
        };
        assert_eq!(signal.description(), "security");
    }

    /// CRIT-12: `is_security_patch` returns true when CVE is present.
    #[tokio::test]
    async fn test_is_security_patch_with_cve() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let result = is_security_patch(&gh_client, "security", "Fix CVE-2024-12345", None, None)
            .await
            .expect("should not error");
        assert!(result);
    }

    /// CRIT-12: `is_security_patch` returns false when no signals.
    #[tokio::test]
    async fn test_is_security_patch_no_signals() {
        let gh_client = GithubClient::new(crate::config::schema::GithubConfig {
            owner: "test".to_string(),
            repo: "test".to_string(),
            api_url: "https://api.github.com".to_string(),
            token: None,
        });
        let result = is_security_patch(
            &gh_client,
            "security",
            "Fix typo in docs",
            Some("Closes #42"),
            None,
        )
        .await
        .expect("should not error");
        assert!(!result);
    }
}
