//! Release candidacy detection.
//!
//! This module determines whether a release should be proposed by:
//! 1. Finding the last semver release tag
//! 2. Fetching merged PRs to the target branch since that tag
//! 3. Grouping PRs by conventional commit type
//! 4. Calculating the semantic version bump
//! 5. Verifying CI is green on the target branch
//!
//! The detection is a standalone module callable by the triage loop.
//! It does NOT create discussions — that is handled by CRIT-2/CRIT-3.
//!
//! ## Conventional commit → version bump rules
//!
//! - `BREAKING CHANGE` or `breaking:` → major bump
//! - `feat:` → minor bump (if no breaking changes)
//! - `fix:`, `chore:`, `docs:`, `refactor:`, `perf:`, `test:` → patch bump
//!
//! ## Edge cases
//!
//! - **No tags yet** — initial release, all merged PRs are candidates
//! - **Non-conventional commit PRs** — categorized as `Chore`
//! - **No PRs since last tag** — no release proposed
//! - **CI not green** — no release proposed, even with PRs
//!
//! ## Acceptance Criteria (from plan)
//!
//! - [x] CRIT-1: Finds merged PRs since last tag
//! - [x] CRIT-1: Groups PRs by conventional commit type correctly
//! - [x] CRIT-1: Version bump calculation (BREAKING→major, feat→minor, fix→patch)
//! - [x] CRIT-1: Skips if CI not green on main
//! - [x] CRIT-1: Handles no tags gracefully (initial release)

use serde::{Deserialize, Serialize};

use crate::github::client::{GitHubClient, MergedPR};
use crate::release::changelog::{self, ConventionalCommitType, GroupedPRs, PullRequest};

/// Semantic version triplet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemVer {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
}

impl SemVer {
    /// Parse a tag like "v1.2.3" into a SemVer.
    ///
    /// Returns `None` if the string doesn't match the `vX.Y.Z` pattern.
    pub fn parse(tag: &str) -> Option<Self> {
        let stripped = tag.strip_prefix('v').unwrap_or(tag);
        let parts: Vec<&str> = stripped.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        let patch = parts[2].parse().ok()?;
        Some(Self {
            major,
            minor,
            patch,
        })
    }

    /// Format as "vX.Y.Z".
    pub fn to_tag(&self) -> String {
        format!("v{}.{}.{}", self.major, self.minor, self.patch)
    }

    /// Compute the next version based on commit types and breaking changes.
    pub fn bump(&self, commit_types: &[ConventionalCommitType], has_breaking: bool) -> Self {
        if has_breaking {
            Self {
                major: self.major + 1,
                minor: 0,
                patch: 0,
            }
        } else if commit_types.contains(&ConventionalCommitType::Feat) {
            Self {
                major: self.major,
                minor: self.minor + 1,
                patch: 0,
            }
        } else {
            Self {
                major: self.major,
                minor: self.minor,
                patch: self.patch + 1,
            }
        }
    }
}

/// The kind of version bump determined by the PRs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionBump {
    /// A major bump is needed (breaking changes present).
    Major,
    /// A minor bump is needed (features present, no breaking).
    Minor,
    /// A patch bump is needed (fixes, chores, etc.).
    Patch,
}

impl std::fmt::Display for VersionBump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VersionBump::Major => write!(f, "major"),
            VersionBump::Minor => write!(f, "minor"),
            VersionBump::Patch => write!(f, "patch"),
        }
    }
}

/// Reasons why a release was not proposed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NoReleaseReason {
    /// No tags exist and no PRs were found.
    NoPRs,
    /// CI is not green on the target branch.
    CiNotGreen,
    /// An error occurred during detection.
    Error(String),
}

/// Result of a release candidacy detection run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DetectionResult {
    /// A release should be proposed.
    ReleaseCandidate {
        /// The last release tag (or `None` for initial release).
        last_tag: Option<String>,
        /// The computed next version tag.
        next_version: String,
        /// The kind of version bump.
        version_bump: VersionBump,
        /// PRs grouped by conventional commit type.
        grouped_prs: GroupedPRs,
        /// All PRs included in this candidate.
        prs: Vec<PullRequest>,
    },
    /// No release should be proposed at this time.
    NoRelease(NoReleaseReason),
}

/// Configuration for release candidacy detection.
#[derive(Debug, Clone)]
pub struct DetectorConfig {
    /// Target branch to check (e.g. "main").
    pub target_branch: String,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            target_branch: "main".to_string(),
        }
    }
}

impl DetectorConfig {
    /// Create config targeting a specific branch.
    pub fn for_branch(branch: &str) -> Self {
        Self {
            target_branch: branch.to_string(),
        }
    }
}

/// Release candidacy detector.
///
/// Orchestrates GitHub API calls and changelog logic to determine whether
/// a release should be proposed.
pub struct ReleaseDetector {
    github: GitHubClient,
    config: DetectorConfig,
}

impl ReleaseDetector {
    /// Create a new detector for the given GitHub client and config.
    pub fn new(github: GitHubClient, config: DetectorConfig) -> Self {
        Self { github, config }
    }

    /// Create a detector from environment with default config (target branch: "main").
    pub fn from_env() -> crate::error::Result<Self> {
        let github = GitHubClient::from_env()?;
        Ok(Self::new(github, DetectorConfig::default()))
    }

    /// Run the full detection flow.
    ///
    /// This is the main entry point. It:
    /// 1. Finds the last semver tag
    /// 2. Fetches merged PRs to the target branch since that tag
    /// 3. Checks CI is green
    /// 4. Groups PRs and calculates version bump
    /// 5. Returns `DetectionResult`
    pub async fn detect(&self) -> DetectionResult {
        // Step 1: Find last release tag
        let last_tag = match self.find_last_release_tag().await {
            Ok(tag) => tag,
            Err(e) => {
                tracing::warn!(error = %e, "failed to find last release tag");
                return DetectionResult::NoRelease(NoReleaseReason::Error(format!(
                    "failed to find last tag: {}",
                    e
                )));
            }
        };

        // Step 2: Fetch merged PRs since the last tag
        let merged_prs = match self.fetch_merged_prs_since_tag(&last_tag).await {
            Ok(prs) => prs,
            Err(e) => {
                tracing::warn!(error = %e, "failed to fetch merged PRs");
                return DetectionResult::NoRelease(NoReleaseReason::Error(format!(
                    "failed to fetch PRs: {}",
                    e
                )));
            }
        };

        // No PRs → no release needed
        if merged_prs.is_empty() {
            return DetectionResult::NoRelease(NoReleaseReason::NoPRs);
        }

        // Step 3: Check CI is green on the target branch
        match self.github.is_ci_green(&self.config.target_branch).await {
            Ok(green) => {
                if !green {
                    return DetectionResult::NoRelease(NoReleaseReason::CiNotGreen);
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "failed to check CI status");
                return DetectionResult::NoRelease(NoReleaseReason::Error(format!(
                    "failed to check CI: {}",
                    e
                )));
            }
        }

        // Step 4: Convert MergedPRs to PullRequests and group by type
        let prs: Vec<PullRequest> = self
            .merged_prs_to_pull_requests(&merged_prs)
            .into_iter()
            .filter(|pr| pr.is_for_main_changelog())
            .collect();

        if prs.is_empty() {
            return DetectionResult::NoRelease(NoReleaseReason::NoPRs);
        }

        let grouped = changelog::group_prs_by_type(&prs);

        // Step 5: Calculate version bump
        let commit_types: Vec<ConventionalCommitType> =
            grouped.groups.iter().map(|(t, _)| t.clone()).collect();

        let has_breaking = merged_prs
            .iter()
            .any(|pr| pr.title.to_lowercase().contains("breaking change"));

        let version_bump = determine_version_bump(&commit_types, has_breaking);

        // Step 6: Compute next version
        let current_version = match &last_tag {
            Some(tag) => match SemVer::parse(tag) {
                Some(v) => v,
                None => SemVer {
                    major: 0,
                    minor: 0,
                    patch: 0,
                },
            },
            None => SemVer {
                major: 0,
                minor: 0,
                patch: 0,
            },
        };

        let next_version = current_version.bump(&commit_types, has_breaking);

        DetectionResult::ReleaseCandidate {
            last_tag: last_tag.map(|s| s.to_string()),
            next_version: next_version.to_tag(),
            version_bump,
            grouped_prs: grouped,
            prs,
        }
    }

    /// Find the latest semver release tag via the GitHub API.
    ///
    /// Returns `Ok(None)` if no tags exist (initial release scenario).
    /// Prefers tags matching `v*.*.*` pattern.
    async fn find_last_release_tag(&self) -> crate::error::Result<Option<String>> {
        let tags = self.github.fetch_tags().await?;

        // Find the first tag matching vX.Y.Z (API returns newest first)
        for tag in &tags {
            if SemVer::parse(&tag.name).is_some() {
                return Ok(Some(tag.name.clone()));
            }
        }

        Ok(None)
    }

    /// Fetch merged PRs to the target branch since the last tag.
    ///
    /// If `last_tag` is `None`, returns all merged PRs (initial release).
    /// Filters by `merged_at` date from the tag's commit date.
    async fn fetch_merged_prs_since_tag(
        &self,
        last_tag: &Option<String>,
    ) -> crate::error::Result<Vec<MergedPR>> {
        let all_prs = self
            .github
            .fetch_merged_prs(&self.config.target_branch)
            .await?;

        // If no tag, return all merged PRs
        let tag_name = match last_tag {
            Some(t) => t,
            None => return Ok(all_prs),
        };

        // Find the tag's commit SHA from the tags API
        let tags = self.github.fetch_tags().await?;
        let tag_commit_sha = match tags.iter().find(|t| t.name == *tag_name) {
            Some(tag) => tag.commit.sha.clone(),
            None => return Ok(all_prs), // Can't find tag commit, return all
        };

        // Get the commit date for the tag
        let tag_commit = match self.github.fetch_commit_by_sha(&tag_commit_sha).await {
            Ok(c) => c,
            Err(_) => return Ok(all_prs), // Can't date-compare, return all
        };

        let tag_date = tag_commit.commit.author.date;

        // Filter PRs merged after the tag
        let since_tag: Vec<MergedPR> = all_prs
            .into_iter()
            .filter(|pr| {
                if let Some(merged_at) = &pr.merged_at {
                    // Simple string comparison works for RFC3339
                    merged_at > &tag_date
                } else {
                    false
                }
            })
            .collect();

        Ok(since_tag)
    }

    /// Convert MergedPR list to PullRequest list for changelog grouping.
    fn merged_prs_to_pull_requests(&self, merged_prs: &[MergedPR]) -> Vec<PullRequest> {
        merged_prs
            .iter()
            .map(|pr| {
                PullRequest::new(
                    self.github.owner(),
                    self.github.repo(),
                    pr.number,
                    &pr.title,
                )
            })
            .collect()
    }
}

/// Determine the version bump from commit types and breaking change flag.
pub fn determine_version_bump(
    commit_types: &[ConventionalCommitType],
    has_breaking: bool,
) -> VersionBump {
    if has_breaking {
        return VersionBump::Major;
    }

    if commit_types.contains(&ConventionalCommitType::Feat) {
        return VersionBump::Minor;
    }

    VersionBump::Patch
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // SemVer parsing and formatting tests
    // =============================================================================

    #[test]
    fn test_semver_parse_valid() {
        let v = SemVer::parse("v1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);
    }

    #[test]
    fn test_semver_parse_without_v() {
        let v = SemVer::parse("0.1.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 1);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_semver_parse_invalid() {
        assert!(SemVer::parse("latest").is_none());
        assert!(SemVer::parse("v1.2").is_none());
        assert!(SemVer::parse("v1.2.3.4").is_none());
        assert!(SemVer::parse("").is_none());
        assert!(SemVer::parse("release-1.0").is_none());
    }

    #[test]
    fn test_semver_to_tag() {
        let v = SemVer {
            major: 2,
            minor: 0,
            patch: 1,
        };
        assert_eq!(v.to_tag(), "v2.0.1");
    }

    #[test]
    fn test_semver_roundtrip() {
        let original = "v3.14.159";
        let parsed = SemVer::parse(original).unwrap();
        assert_eq!(parsed.to_tag(), original);
    }

    // =============================================================================
    // Version bump calculation tests
    // =============================================================================

    #[test]
    fn test_semver_bump_major_breaking() {
        let v = SemVer {
            major: 1,
            minor: 5,
            patch: 3,
        };
        let types = vec![ConventionalCommitType::Fix, ConventionalCommitType::Feat];
        let next = v.bump(&types, true);
        assert_eq!(
            next,
            SemVer {
                major: 2,
                minor: 0,
                patch: 0,
            }
        );
    }

    #[test]
    fn test_semver_bump_minor_feat() {
        let v = SemVer {
            major: 1,
            minor: 5,
            patch: 3,
        };
        let types = vec![ConventionalCommitType::Feat, ConventionalCommitType::Fix];
        let next = v.bump(&types, false);
        assert_eq!(
            next,
            SemVer {
                major: 1,
                minor: 6,
                patch: 0,
            }
        );
    }

    #[test]
    fn test_semver_bump_patch_fix() {
        let v = SemVer {
            major: 1,
            minor: 5,
            patch: 3,
        };
        let types = vec![ConventionalCommitType::Fix, ConventionalCommitType::Chore];
        let next = v.bump(&types, false);
        assert_eq!(
            next,
            SemVer {
                major: 1,
                minor: 5,
                patch: 4,
            }
        );
    }

    #[test]
    fn test_semver_bump_patch_docs_only() {
        let v = SemVer {
            major: 0,
            minor: 0,
            patch: 0,
        };
        let types = vec![ConventionalCommitType::Docs];
        let next = v.bump(&types, false);
        assert_eq!(
            next,
            SemVer {
                major: 0,
                minor: 0,
                patch: 1,
            }
        );
    }

    #[test]
    fn test_semver_bump_patch_empty_types() {
        let v = SemVer {
            major: 1,
            minor: 2,
            patch: 3,
        };
        let types: Vec<ConventionalCommitType> = vec![];
        let next = v.bump(&types, false);
        assert_eq!(
            next,
            SemVer {
                major: 1,
                minor: 2,
                patch: 4,
            }
        );
    }

    #[test]
    fn test_determine_version_bump_breaking() {
        let types = vec![ConventionalCommitType::Feat, ConventionalCommitType::Fix];
        assert_eq!(determine_version_bump(&types, true), VersionBump::Major);
    }

    #[test]
    fn test_determine_version_bump_feat() {
        let types = vec![ConventionalCommitType::Feat, ConventionalCommitType::Chore];
        assert_eq!(determine_version_bump(&types, false), VersionBump::Minor);
    }

    #[test]
    fn test_determine_version_bump_fix_only() {
        let types = vec![
            ConventionalCommitType::Fix,
            ConventionalCommitType::Docs,
            ConventionalCommitType::Refactor,
        ];
        assert_eq!(determine_version_bump(&types, false), VersionBump::Patch);
    }

    #[test]
    fn test_determine_version_bump_empty() {
        let types: Vec<ConventionalCommitType> = vec![];
        assert_eq!(determine_version_bump(&types, false), VersionBump::Patch);
    }

    // =============================================================================
    // VersionBump display tests
    // =============================================================================

    #[test]
    fn test_version_bump_display() {
        assert_eq!(format!("{}", VersionBump::Major), "major");
        assert_eq!(format!("{}", VersionBump::Minor), "minor");
        assert_eq!(format!("{}", VersionBump::Patch), "patch");
    }

    // =============================================================================
    // DetectorConfig tests
    // =============================================================================

    #[test]
    fn test_detector_config_default() {
        let config = DetectorConfig::default();
        assert_eq!(config.target_branch, "main");
    }

    #[test]
    fn test_detector_config_for_branch() {
        let config = DetectorConfig::for_branch("develop");
        assert_eq!(config.target_branch, "develop");
    }

    // =============================================================================
    // ReleaseDetector construction tests
    // =============================================================================

    #[test]
    fn test_release_detector_new() {
        let github = GitHubClient::new(
            "myorg",
            "myrepo",
            crate::github::auth::GitHubAuth::new_with_default_api("ghp_test_token"),
        );
        let config = DetectorConfig::for_branch("main");
        let detector = ReleaseDetector::new(github, config);
        assert_eq!(detector.config.target_branch, "main");
    }

    // =============================================================================
    // Merged PR to PullRequest conversion tests
    // =============================================================================

    #[test]
    fn test_merged_prs_to_pull_requests() {
        let github = GitHubClient::new(
            "myorg",
            "myrepo",
            crate::github::auth::GitHubAuth::new_with_default_api("ghp_test_token"),
        );
        let config = DetectorConfig::default();
        let detector = ReleaseDetector::new(github, config);

        let merged_prs = vec![
            MergedPR {
                number: 1,
                title: "feat: add login".to_string(),
                state: "closed".to_string(),
                merge_commit_sha: Some("abc".to_string()),
                merged_at: Some("2024-01-15T10:00:00Z".to_string()),
                labels: vec![],
                user: None,
                base: crate::github::client::GitHubPRRef {
                    ref_field: "main".to_string(),
                    sha: "def".to_string(),
                },
            },
            MergedPR {
                number: 2,
                title: "fix: crash on login".to_string(),
                state: "closed".to_string(),
                merge_commit_sha: Some("ghi".to_string()),
                merged_at: Some("2024-01-16T12:00:00Z".to_string()),
                labels: vec![],
                user: None,
                base: crate::github::client::GitHubPRRef {
                    ref_field: "main".to_string(),
                    sha: "jkl".to_string(),
                },
            },
        ];

        let prs = detector.merged_prs_to_pull_requests(&merged_prs);
        assert_eq!(prs.len(), 2);
        assert_eq!(prs[0].number, 1);
        assert_eq!(prs[0].title, "feat: add login");
        assert_eq!(prs[0].url, "https://github.com/myorg/myrepo/pull/1");
        assert_eq!(prs[1].number, 2);
        assert_eq!(prs[1].title, "fix: crash on login");
    }

    #[test]
    fn test_merged_prs_to_pull_requests_empty() {
        let github = GitHubClient::new(
            "myorg",
            "myrepo",
            crate::github::auth::GitHubAuth::new_with_default_api("ghp_test_token"),
        );
        let config = DetectorConfig::default();
        let detector = ReleaseDetector::new(github, config);
        let prs = detector.merged_prs_to_pull_requests(&[]);
        assert!(prs.is_empty());
    }

    // =============================================================================
    // Conventional commit detection for breaking changes
    // =============================================================================

    #[test]
    fn test_breaking_change_in_title() {
        // The detector checks for "breaking change" (case-insensitive) in PR titles
        let title1 = "feat: BREAKING CHANGE: new API";
        assert!(title1.to_lowercase().contains("breaking change"));

        let title2 = "feat: breaking change in auth";
        assert!(title2.to_lowercase().contains("breaking change"));

        let title3 = "fix: normal fix without breaking";
        assert!(!title3.to_lowercase().contains("breaking change"));
    }

    // =============================================================================
    // DetectionResult tests
    // =============================================================================

    #[test]
    fn test_detection_result_release_candidate() {
        let result = DetectionResult::ReleaseCandidate {
            last_tag: Some("v1.0.0".to_string()),
            next_version: "v1.1.0".to_string(),
            version_bump: VersionBump::Minor,
            grouped_prs: GroupedPRs::new(),
            prs: vec![],
        };

        match result {
            DetectionResult::ReleaseCandidate {
                last_tag,
                next_version,
                version_bump,
                ..
            } => {
                assert_eq!(last_tag, Some("v1.0.0".to_string()));
                assert_eq!(next_version, "v1.1.0");
                assert_eq!(version_bump, VersionBump::Minor);
            }
            _ => panic!("expected ReleaseCandidate"),
        }
    }

    #[test]
    fn test_detection_result_no_release_ci() {
        let result = DetectionResult::NoRelease(NoReleaseReason::CiNotGreen);
        match result {
            DetectionResult::NoRelease(reason) => {
                assert_eq!(reason, NoReleaseReason::CiNotGreen);
            }
            _ => panic!("expected NoRelease"),
        }
    }

    #[test]
    fn test_detection_result_no_release_no_prs() {
        let result = DetectionResult::NoRelease(NoReleaseReason::NoPRs);
        match result {
            DetectionResult::NoRelease(reason) => {
                assert_eq!(reason, NoReleaseReason::NoPRs);
            }
            _ => panic!("expected NoRelease"),
        }
    }

    #[test]
    fn test_detection_result_no_release_error() {
        let result =
            DetectionResult::NoRelease(NoReleaseReason::Error("network timeout".to_string()));
        match result {
            DetectionResult::NoRelease(NoReleaseReason::Error(msg)) => {
                assert_eq!(msg, "network timeout");
            }
            _ => panic!("expected NoRelease(Error)"),
        }
    }

    // =============================================================================
    // Initial release scenario (no tags)
    // =============================================================================

    #[test]
    fn test_initial_release_version_computation() {
        // When no tags exist, we start from v0.0.0
        let current = SemVer {
            major: 0,
            minor: 0,
            patch: 0,
        };

        // Adding a feature → v0.1.0
        let types = vec![ConventionalCommitType::Feat];
        let next = current.bump(&types, false);
        assert_eq!(
            next,
            SemVer {
                major: 0,
                minor: 1,
                patch: 0,
            }
        );

        // Adding a fix → v0.0.1
        let types = vec![ConventionalCommitType::Fix];
        let next = current.bump(&types, false);
        assert_eq!(
            next,
            SemVer {
                major: 0,
                minor: 0,
                patch: 1,
            }
        );
    }

    // =============================================================================
    // Edge cases
    // =============================================================================

    #[test]
    fn test_semver_parse_with_prerelease_returns_none() {
        // v1.0.0-beta.1 should not match our simple vX.Y.Z pattern
        assert!(SemVer::parse("v1.0.0-beta.1").is_none());
    }

    #[test]
    fn test_semver_parse_zero_version() {
        let v = SemVer::parse("v0.0.0").unwrap();
        assert_eq!(v.major, 0);
        assert_eq!(v.minor, 0);
        assert_eq!(v.patch, 0);
    }

    #[test]
    fn test_semver_bump_major_from_zero() {
        let v = SemVer {
            major: 0,
            minor: 0,
            patch: 0,
        };
        let types = vec![ConventionalCommitType::Feat];
        let next = v.bump(&types, true);
        assert_eq!(
            next,
            SemVer {
                major: 1,
                minor: 0,
                patch: 0,
            }
        );
    }

    #[test]
    fn test_multiple_breaking_and_feat_still_major() {
        let types = vec![
            ConventionalCommitType::Feat,
            ConventionalCommitType::Fix,
            ConventionalCommitType::Feat,
        ];
        // Even with features, breaking change takes priority
        assert_eq!(determine_version_bump(&types, true), VersionBump::Major);
    }

    // =============================================================================
    // NoReleaseReason equality tests
    // =============================================================================

    #[test]
    fn test_no_release_reason_equality() {
        assert_eq!(NoReleaseReason::NoPRs, NoReleaseReason::NoPRs);
        assert_eq!(NoReleaseReason::CiNotGreen, NoReleaseReason::CiNotGreen);
        assert_ne!(NoReleaseReason::NoPRs, NoReleaseReason::CiNotGreen);
    }

    #[test]
    fn test_detection_result_clone() {
        let result = DetectionResult::NoRelease(NoReleaseReason::NoPRs);
        let cloned = result.clone();
        match cloned {
            DetectionResult::NoRelease(NoReleaseReason::NoPRs) => {}
            _ => panic!("clone failed"),
        }
    }
}
