//! Backport bead creation.
//!
//! Provides the [`BackportBead`] builder for constructing well-formed backport
//! beads that match the plan specification exactly.
//!
//! Bead shape per plan/ backport-plan.md:
//!   title: "Backport #{sha_short} to {branch_name}"
//!   type:  chore
//!   tag:   rodgers:type=backport
//!   priority: 1 for security, 2 otherwise
//!   --deps discovered-from:{source_issue}
//!
//! Description sections: Plan, commit SHA, message, source issue,
//! target branch, WHAT TO DO, ACCEPTANCE, PITFALLS

use crate::backport::detector::BackportCandidate;

/// A fully-built backport bead ready for submission.
///
/// Constructed via [`BackportBead::build`].
#[derive(Debug, Clone)]
pub struct BackportBead {
    /// Bead title.
    pub title: String,
    /// Full bead description.
    pub description: String,
    /// Bead type (always "chore").
    pub bead_type: &'static str,
    /// Tag string (e.g. "rodgers:type=backport").
    pub tag: String,
    /// Priority: 1 = security (highest), 2 = normal.
    pub priority: u8,
    /// Acceptance criteria text.
    pub acceptance: String,
    /// Dependency specifier for discovered-from linking (e.g. "discovered-from:#42").
    pub discovered_from: Option<String>,
    /// External reference for the source PR (e.g. "gh-123").
    pub external_ref: Option<String>,
}

impl BackportBead {
    /// Build a `BackportBead` from a backport candidate and target branch.
    pub fn build(candidate: &BackportCandidate, target_branch: &str) -> Self {
        let pr = &candidate.pr;
        let sha = pr.merge_commit_sha.as_deref().unwrap_or("unknown");
        let sha_short = &sha[..sha.len().min(7)];

        // Extract source issue number from PR body (e.g., "Closes #123")
        let source_issue = extract_issue_ref(pr.body.as_deref().unwrap_or(""));
        let external_ref = Some(format!("gh-{}", pr.number));

        let title = format!("Backport #{sha_short} to {target_branch}");

        let description = format_description(
            sha,
            sha_short,
            &pr.title,
            pr.number,
            source_issue.as_deref(),
            target_branch,
        );

        let tag = "rodgers:type=backport".to_string();

        let priority = candidate.priority;

        let acceptance = format!(
            "Backport #{sha} to {branch} is merged or explicitly closed without merging",
            sha = sha,
            branch = target_branch
        );

        let discovered_from = source_issue.map(|issue| format!("discovered-from:#{}", issue));

        Self {
            title,
            description,
            bead_type: "chore",
            tag,
            priority,
            acceptance,
            discovered_from,
            external_ref,
        }
    }

    /// Return the priority formatted as a string (bd expects string like "1" or "2").
    pub fn priority_str(&self) -> String {
        self.priority.to_string()
    }

    /// Return the deps argument list for bd create (without the `--deps` prefix).
    /// Returns `Some` only when a discovered-from dependency is present.
    #[allow(dead_code)]
    pub fn deps_arg(&self) -> Option<String> {
        self.discovered_from.clone()
    }
}

/// Build a backport-bean description that matches the plan example exactly.
fn format_description(
    sha: &str,
    _sha_short: &str,
    pr_title: &str,
    pr_number: u64,
    source_issue: Option<&str>,
    target_branch: &str,
) -> String {
    let source_display = source_issue
        .map(|s| format!("Issue #{}", s))
        .unwrap_or_else(|| format!("PR #{}", pr_number));

    format!(
        "Plan: plans/backport-plan.md\n\n\
Backport for: #{sha} — \"{pr_title}\"\n\
Source issue: {source_display}\n\
Target branch: {target_branch}\n\n\
WHAT TO DO\n\
Cherry-pick commit #{sha} to {target_branch}. Create a PR targeting\n\
{target_branch} with the cherry-pick. Resolve any merge conflicts.\n\n\
ACCEPTANCE\n\
- [ ] Cherry-pick of #{sha} applies cleanly to {target_branch} (or conflicts resolved)\n\
- [ ] PR is open targeting {target_branch}\n\
- [ ] CI passes on the backport PR\n\
- [ ] PR is merged or given explicit approval to close without merging\n\n\
PITFALLS\n\
- If the fix requires changes to shared library code that has diverged\n\
  between main and {target_branch}, the cherry-pick may require\n\
  manual conflict resolution. Document any non-trivial conflicts\n\
  in this bead before closing.",
        sha = sha,
        pr_title = pr_title,
        source_display = source_display,
        target_branch = target_branch,
    )
}

/// Extract the issue number from text like "Closes #123" or "Fixes #456 xyz".
///
/// Returns the number as a string, or None if no issue reference is found.
fn extract_issue_ref(text: &str) -> Option<String> {
    // Same regex as detector::issue_ref_capture
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        regex::Regex::new(r"(?:(?:closes?|fixes?|resolves?|[Rr]eferences?)\s+)?#(\d+)")
            .expect("hardcoded regex is valid")
    });
    re.captures(text)
        .and_then(|caps| caps.get(1))
        .map(|m| m.as_str().to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backport::detector::BackportReason;
    use crate::github::client::{GithubUser, MergedPr};

    fn make_pr(number: u64, title: &str, body: Option<&str>) -> MergedPr {
        MergedPr {
            number,
            title: title.to_string(),
            body: body.map(String::from),
            merged_at: Some("2024-01-01T00:00:00Z".to_string()),
            merge_commit_sha: Some("abc123def456abc123".to_string()),
            user: GithubUser {
                login: "test-user".to_string(),
                user_type: "User".to_string(),
            },
            labels: vec![],
            state: "closed".to_string(),
        }
    }

    fn make_candidate(pr: MergedPr, reason: BackportReason) -> BackportCandidate {
        BackportCandidate::new(pr, reason)
    }

    #[test]
    fn test_backport_bead_title_format() {
        let pr = make_pr(42, "Fix login crash", Some("Closes #77"));
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/1.x");

        assert_eq!(bead.title, "Backport #abc123d to release/1.x");
    }

    #[test]
    fn test_backport_bead_type_is_chore() {
        let pr = make_pr(1, "Test", None);
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/2.x");

        assert_eq!(bead.bead_type, "chore");
    }

    #[test]
    fn test_backport_bead_tag() {
        let pr = make_pr(1, "Test", None);
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/3.x");

        assert_eq!(bead.tag, "rodgers:type=backport");
    }

    #[test]
    fn test_backport_bead_description_has_all_sections() {
        let pr = make_pr(42, "Fix critical bug", Some("Closes #99"));
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/1.x");

        let desc = &bead.description;
        assert!(desc.contains("Plan: plans/backport-plan.md"));
        assert!(desc.contains("abc123def456abc123"));
        assert!(desc.contains("Source issue: Issue #99"));
        assert!(desc.contains("Target branch: release/1.x"));
        assert!(desc.contains("WHAT TO DO"));
        assert!(desc.contains("Cherry-pick"));
        assert!(desc.contains("ACCEPTANCE"));
        assert!(desc.contains("- [ ] Cherry-pick"));
        assert!(desc.contains("- [ ] PR is open"));
        assert!(desc.contains("- [ ] CI passes"));
        assert!(desc.contains("- [ ] PR is merged"));
        assert!(desc.contains("PITFALLS"));
    }

    #[test]
    fn test_backport_bead_priority_security_is_1() {
        let pr = make_pr(1, "Fix CVE-2024-99999", None);
        let c = make_candidate(pr, BackportReason::SecurityPatch);
        let bead = BackportBead::build(&c, "release/1.x");

        assert_eq!(bead.priority, 1);
        assert_eq!(bead.priority_str(), "1");
    }

    #[test]
    fn test_backport_bead_priority_bug_is_2() {
        let pr = make_pr(2, "Fix bug", Some("Closes #10"));
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/2.x");

        assert_eq!(bead.priority, 2);
        assert_eq!(bead.priority_str(), "2");
    }

    #[test]
    fn test_backport_bead_priority_backport_me_is_2() {
        let pr = make_pr(3, "Update", Some("Closes #42"));
        let c = make_candidate(pr, BackportReason::BackportMeLabel);
        let bead = BackportBead::build(&c, "release/3.x");

        assert_eq!(bead.priority, 2);
    }

    #[test]
    fn test_backport_bead_discovered_from_contains_issue() {
        let pr = make_pr(42, "Fix bug", Some("Closes #77 some text"));
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/1.x");

        assert_eq!(
            bead.discovered_from,
            Some("discovered-from:#77".to_string())
        );
        assert_eq!(bead.deps_arg(), Some("discovered-from:#77".to_string()));
    }

    #[test]
    fn test_backport_bead_discovered_from_none_when_no_issue_in_body() {
        let pr = make_pr(42, "Fix bug", None);
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/1.x");

        // Falls back to PR number
        assert_eq!(bead.discovered_from, None);
    }

    #[test]
    fn test_backport_bead_external_ref() {
        let pr = make_pr(42, "Fix bug", Some("Closes #77"));
        let c = make_candidate(pr, BackportReason::BugFix);
        let bead = BackportBead::build(&c, "release/1.x");

        assert_eq!(bead.external_ref, Some("gh-42".to_string()));
    }

    #[test]
    fn test_priority_different_values() {
        let security_pr = make_pr(1, "CVE-2024-12345 is patched", None);
        let security_c = make_candidate(security_pr, BackportReason::SecurityPatch);
        let security_bead = BackportBead::build(&security_c, "release/1.x");
        assert_eq!(security_bead.priority, 1);

        let bug_pr = make_pr(2, "Bug fix", None);
        let bug_c = make_candidate(bug_pr, BackportReason::BugFix);
        let bug_bead = BackportBead::build(&bug_c, "release/1.x");
        assert_eq!(bug_bead.priority, 2);

        let backportme_pr = make_pr(3, "Force backport", None);
        let backportme_c = make_candidate(backportme_pr, BackportReason::BackportMeLabel);
        let backportme_bead = BackportBead::build(&backportme_c, "release/1.x");
        assert_eq!(backportme_bead.priority, 2);
    }
}
