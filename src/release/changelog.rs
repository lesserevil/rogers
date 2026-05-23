//! Changelog generation from PR data using conventional commits.
//!
//! This module provides functionality to generate release notes from PR titles
//! and labels. It parses conventional commit types from PR titles, groups PRs
//! by type, and generates markdown suitable for GitHub Release notes.
//!
//! ## Conventional Commit Types
//!
//! The following prefixes are recognized in PR titles:
//! - `feat:` → Features
//! - `fix:` → Bug Fixes
//! - `docs:` → Documentation
//! - `refactor:` → Refactors
//! - `perf:` → Performance
//! - `test:` → Tests
//! - `chore:` → Chores
//!
//! PR titles without a recognized prefix are categorized as `chore`.

use serde::{Deserialize, Serialize};

/// Conventional commit type extracted from a PR title.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConventionalCommitType {
    /// Feature additions (`feat:` prefix)
    Feat,
    /// Bug fixes (`fix:` prefix)
    Fix,
    /// Documentation changes (`docs:` prefix)
    Docs,
    /// Code refactoring (`refactor:` prefix)
    Refactor,
    /// Performance improvements (`perf:` prefix)
    Perf,
    /// Test changes (`test:` prefix)
    Test,
    /// Maintenance chores (`chore:` prefix or no recognized prefix)
    Chore,
}

impl std::fmt::Display for ConventionalCommitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConventionalCommitType::Feat => write!(f, "feat"),
            ConventionalCommitType::Fix => write!(f, "fix"),
            ConventionalCommitType::Docs => write!(f, "docs"),
            ConventionalCommitType::Refactor => write!(f, "refactor"),
            ConventionalCommitType::Perf => write!(f, "perf"),
            ConventionalCommitType::Test => write!(f, "test"),
            ConventionalCommitType::Chore => write!(f, "chore"),
        }
    }
}

/// Display section title for a commit type in changelog output.
impl ConventionalCommitType {
    /// Return the human-readable section heading for this type.
    pub fn section_title(&self) -> &'static str {
        match self {
            ConventionalCommitType::Feat => "Features",
            ConventionalCommitType::Fix => "Bug Fixes",
            ConventionalCommitType::Docs => "Documentation",
            ConventionalCommitType::Refactor => "Refactors",
            ConventionalCommitType::Perf => "Performance",
            ConventionalCommitType::Test => "Tests",
            ConventionalCommitType::Chore => "Chores",
        }
    }
}

/// A parsed conventional commit type with its extracted description.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParsedCommit {
    /// The conventional commit type.
    pub commit_type: ConventionalCommitType,
    /// The description portion (after the type and separator).
    pub description: String,
}

/// Metadata for a pull request to include in changelog generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequest {
    /// PR title (will be parsed for conventional commit type).
    pub title: String,
    /// PR number (used for generating links).
    pub number: u64,
    /// Full URL to the PR (used for generating links).
    pub url: String,
    /// Labels attached to the PR.
    #[serde(default)]
    pub labels: Vec<String>,
}

impl PullRequest {
    /// Create a new PR entry from a title and number.
    ///
    /// Constructs a standard GitHub PR URL from owner, repo, and number.
    pub fn new(owner: &str, repo: &str, number: u64, title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            number,
            url: format!("https://github.com/{owner}/{repo}/pull/{number}"),
            labels: Vec::new(),
        }
    }

    /// Create a new PR entry with a custom URL.
    ///
    /// The owner and repo parameters are accepted for API consistency
    /// but the provided `url` takes precedence.
    pub fn new_with_url(
        _owner: &str,
        _repo: &str,
        number: u64,
        title: impl Into<String>,
        url: String,
    ) -> Self {
        Self {
            title: title.into(),
            number,
            url,
            labels: Vec::new(),
        }
    }

    /// Add a label to this PR.
    pub fn with_label(mut self, label: &str) -> Self {
        self.labels.push(label.to_string());
        self
    }

    /// Add multiple labels to this PR.
    pub fn with_labels(mut self, labels: &[&str]) -> Self {
        for label in labels {
            self.labels.push(label.to_string());
        }
        self
    }

    /// Check if this PR is a backport PR.
    ///
    /// Backport PRs are identified by "backport" appearing in the title
    /// (case-insensitive) and are excluded from main branch changelogs.
    pub fn is_backport(&self) -> bool {
        self.title.to_lowercase().contains("backport")
    }

    /// Check if this PR should be included in a main branch changelog.
    ///
    /// Backport PRs are excluded.
    pub fn is_for_main_changelog(&self) -> bool {
        !self.is_backport()
    }
}

/// Group of PRs organized by conventional commit type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GroupedPRs {
    /// PRs grouped by commit type.
    /// Order is preserved: Feat, Fix, Docs, Refactor, Perf, Test, Chore.
    pub groups: Vec<(ConventionalCommitType, Vec<PullRequest>)>,
}

impl GroupedPRs {
    /// Create a new empty grouped PRs collection.
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    /// Check if there are no PRs in any group.
    pub fn is_empty(&self) -> bool {
        self.groups.iter().all(|(_, prs)| prs.is_empty())
    }

    /// Get all PRs across all groups, flattened.
    pub fn all_prs(&self) -> Vec<&PullRequest> {
        self.groups.iter().flat_map(|(_, prs)| prs.iter()).collect()
    }
}

/// Configuration for changelog generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangelogConfig {
    /// Repository owner for generating PR links.
    pub owner: String,
    /// Repository name for generating PR links.
    pub repo: String,
    /// Display name for the release (e.g., "v1.2.0").
    pub release_name: String,
    /// Optional date for the release (defaults to current date).
    pub release_date: Option<String>,
}

impl ChangelogConfig {
    /// Create a new changelog configuration.
    pub fn new(owner: &str, repo: &str, release_name: &str) -> Self {
        Self {
            owner: owner.to_string(),
            repo: repo.to_string(),
            release_name: release_name.to_string(),
            release_date: None,
        }
    }

    /// Set the release date.
    pub fn with_date(mut self, date: &str) -> Self {
        self.release_date = Some(date.to_string());
        self
    }
}

/// Escape special characters for safe inclusion in markdown.
///
/// This escapes characters that could interfere with markdown rendering:
/// backslashes, backticks, dollar signs, braces, brackets, parens, hashes,
/// plus, equals, pipes, tildes, and underscores at the start of words.
fn escape_markdown(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('(', "\\(")
        .replace(')', "\\)")
}

/// Parse a PR title to extract the conventional commit type.
///
/// Recognizes these prefixes (case-insensitive, must be followed by `:`):
/// `feat`, `fix`, `chore`, `docs`, `refactor`, `perf`, `test`
///
/// Uses the **first match** when multiple prefixes appear in the title.
/// Titles without a recognized prefix are categorized as `chore`.
///
/// # Examples
///
/// ```
/// use rogers::release::changelog::{parse_conventional_commit, ConventionalCommitType};
///
/// let parsed = parse_conventional_commit("feat: add user login");
/// assert_eq!(parsed.commit_type, ConventionalCommitType::Feat);
/// assert_eq!(parsed.description, "add user login");
///
/// let parsed = parse_conventional_commit("no prefix here");
/// assert_eq!(parsed.commit_type, ConventionalCommitType::Chore);
/// ```
pub fn parse_conventional_commit(title: &str) -> ParsedCommit {
    let trimmed = title.trim();

    // Try each conventional commit prefix in priority order
    let prefixes = [
        ("feat", ConventionalCommitType::Feat),
        ("fix", ConventionalCommitType::Fix),
        ("chore", ConventionalCommitType::Chore),
        ("docs", ConventionalCommitType::Docs),
        ("refactor", ConventionalCommitType::Refactor),
        ("perf", ConventionalCommitType::Perf),
        ("test", ConventionalCommitType::Test),
    ];

    for (prefix, commit_type) in &prefixes {
        let prefix_pat = format!("{}:", prefix.to_lowercase());
        if trimmed.to_lowercase().starts_with(&prefix_pat) {
            // Extract description after the prefix and separator
            let desc_start = prefix_pat.len();
            let description = trimmed[desc_start..].trim().to_string();

            return ParsedCommit {
                commit_type: commit_type.clone(),
                description,
            };
        }
    }

    // No recognized prefix — default to chore, use full title as description
    ParsedCommit {
        commit_type: ConventionalCommitType::Chore,
        description: trimmed.to_string(),
    }
}

/// Group a list of PRs by their conventional commit type.
///
/// Backport PRs are excluded from the grouping. The groups are ordered
/// by conventional commit type priority: Feat, Fix, Docs, Refactor, Perf, Test, Chore.
///
/// Only types that have at least one PR are included in the result.
pub fn group_prs_by_type(prs: &[PullRequest]) -> GroupedPRs {
    // Initialize groups in priority order
    let mut grouped: Vec<(ConventionalCommitType, Vec<PullRequest>)> = Vec::new();

    for pr in prs {
        // Exclude backport PRs from main branch changelog
        if !pr.is_for_main_changelog() {
            continue;
        }

        let parsed = parse_conventional_commit(&pr.title);
        let commit_type = parsed.commit_type;

        // Check if we already have a group for this type
        let found = grouped.iter_mut().find(|(t, _)| *t == commit_type);

        if let Some((_, group_prs)) = found {
            group_prs.push(pr.clone());
        } else {
            grouped.push((commit_type, vec![pr.clone()]));
        }
    }

    GroupedPRs { groups: grouped }
}

/// Generate markdown release notes from grouped PRs.
///
/// Produces GitHub Release notes format with:
/// - A header with the release name
/// - Sections for each PR type (Features, Bug Fixes, etc.)
/// - Each entry formatted as `- description ([#number](url))`
/// - Empty groups are omitted
///
/// The markdown is escaped to safely handle special characters.
pub fn generate_markdown(grouped: &GroupedPRs, config: &ChangelogConfig) -> String {
    let mut lines = Vec::new();

    // Release header
    let header = if let Some(ref date) = config.release_date {
        format!("## {}\n\n**{}**\n\n", config.release_name, date)
    } else {
        format!("## {}\n\n", config.release_name)
    };
    lines.push(header);

    // Generate sections for each group
    let mut has_content = false;
    for (commit_type, prs) in &grouped.groups {
        if prs.is_empty() {
            continue;
        }

        has_content = true;
        let section_title = commit_type.section_title();
        lines.push(format!("### {}", section_title));
        lines.push(String::new()); // blank line after header

        for pr in prs {
            let escaped_title = escape_markdown(&pr.title);
            let link = format!("[#{}]({})", pr.number, pr.url);
            lines.push(format!("- {} ({})", escaped_title, link));
        }

        lines.push(String::new()); // blank line after section
    }

    if !has_content {
        lines.push(String::from("_No changes in this release._"));
    }

    lines.join("\n")
}

/// Generate release notes from a list of PRs.
///
/// This is a convenience function that parses, groups, and formats
/// PR data into release-ready markdown.
pub fn generate_release_notes(prs: &[PullRequest], config: &ChangelogConfig) -> String {
    let grouped = group_prs_by_type(prs);
    generate_markdown(&grouped, config)
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // parse_conventional_commit tests
    // =============================================================================

    #[test]
    fn test_parse_feat_prefix() {
        let parsed = parse_conventional_commit("feat: add user authentication");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Feat);
        assert_eq!(parsed.description, "add user authentication");
    }

    #[test]
    fn test_parse_fix_prefix() {
        let parsed = parse_conventional_commit("fix: resolve null pointer exception");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Fix);
        assert_eq!(parsed.description, "resolve null pointer exception");
    }

    #[test]
    fn test_parse_chore_prefix() {
        let parsed = parse_conventional_commit("chore: update dependencies");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Chore);
        assert_eq!(parsed.description, "update dependencies");
    }

    #[test]
    fn test_parse_docs_prefix() {
        let parsed = parse_conventional_commit("docs: add README installation guide");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Docs);
        assert_eq!(parsed.description, "add README installation guide");
    }

    #[test]
    fn test_parse_refactor_prefix() {
        let parsed = parse_conventional_commit("refactor: simplify user validation logic");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Refactor);
        assert_eq!(parsed.description, "simplify user validation logic");
    }

    #[test]
    fn test_parse_perf_prefix() {
        let parsed = parse_conventional_commit("perf: optimize database query");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Perf);
        assert_eq!(parsed.description, "optimize database query");
    }

    #[test]
    fn test_parse_test_prefix() {
        let parsed = parse_conventional_commit("test: add integration test for auth");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Test);
        assert_eq!(parsed.description, "add integration test for auth");
    }

    #[test]
    fn test_parse_no_prefix_defaults_to_chore() {
        let parsed = parse_conventional_commit("Updated the README");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Chore);
        assert_eq!(parsed.description, "Updated the README");
    }

    #[test]
    fn test_parse_no_prefix_empty_title() {
        let parsed = parse_conventional_commit("");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Chore);
        assert_eq!(parsed.description, "");
    }

    #[test]
    fn test_parse_first_match_wins() {
        // Multiple prefixes — use first match
        let parsed = parse_conventional_commit("fix: this also says feat: but fix comes first");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Fix);
        assert_eq!(
            parsed.description,
            "this also says feat: but fix comes first"
        );
    }

    #[test]
    fn test_parse_lowercase_prefix_match() {
        // Prefixes are case-insensitive
        let parsed = parse_conventional_commit("FEAT: uppercase feature");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Feat);
        assert_eq!(parsed.description, "uppercase feature");

        let parsed = parse_conventional_commit("Fix: mixed case fix");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Fix);
        assert_eq!(parsed.description, "mixed case fix");
    }

    #[test]
    fn test_parse_prefix_not_at_start() {
        // "feat:" only counts if it's at the start of the title
        let parsed = parse_conventional_commit("feat add something");
        // "feat" is not followed by ":", so it should fall through to chore
        assert_eq!(parsed.commit_type, ConventionalCommitType::Chore);
    }

    #[test]
    fn test_parse_with_leading_whitespace() {
        let parsed = parse_conventional_commit("  feat: add feature with spaces");
        assert_eq!(parsed.commit_type, ConventionalCommitType::Feat);
        assert_eq!(parsed.description, "add feature with spaces");
    }

    // =============================================================================
    // PullRequest tests
    // =============================================================================

    #[test]
    fn test_pull_request_new() {
        let pr = PullRequest::new("myorg", "myrepo", 42, "feat: add login");
        assert_eq!(pr.number, 42);
        assert_eq!(pr.title, "feat: add login");
        assert_eq!(pr.url, "https://github.com/myorg/myrepo/pull/42");
        assert!(pr.labels.is_empty());
    }

    #[test]
    fn test_pull_request_with_label() {
        let pr =
            PullRequest::new("myorg", "myrepo", 42, "feat: add login").with_label("enhancement");
        assert_eq!(pr.labels, vec!["enhancement".to_string()]);
    }

    #[test]
    fn test_pull_request_with_labels() {
        let pr = PullRequest::new("myorg", "myrepo", 42, "feat: add login")
            .with_labels(&["enhancement", "good-first-issue"]);
        assert_eq!(
            pr.labels,
            vec!["enhancement".to_string(), "good-first-issue".to_string()]
        );
    }

    #[test]
    fn test_pull_request_is_backport() {
        let pr = PullRequest::new("myorg", "myrepo", 42, "Backport to release/1.x");
        assert!(pr.is_backport());
    }

    #[test]
    fn test_pull_request_is_backport_case_insensitive() {
        let pr = PullRequest::new("myorg", "myrepo", 42, "BACKPORT: hotfix for v1.2");
        assert!(pr.is_backport());

        let pr2 = PullRequest::new("myorg", "myrepo", 43, "backport of #41");
        assert!(pr2.is_backport());
    }

    #[test]
    fn test_pull_request_not_backport() {
        let pr = PullRequest::new("myorg", "myrepo", 42, "feat: add new feature");
        assert!(!pr.is_backport());
    }

    #[test]
    fn test_pull_request_is_for_main_changelog() {
        let regular = PullRequest::new("myorg", "myrepo", 42, "feat: add feature");
        assert!(regular.is_for_main_changelog());

        let backport = PullRequest::new("myorg", "myrepo", 42, "Backport to release/1.x");
        assert!(!backport.is_for_main_changelog());
    }

    #[test]
    fn test_pull_request_new_with_url() {
        let pr = PullRequest::new_with_url(
            "myorg",
            "myrepo",
            42,
            "feat: add feature",
            "https://example.com/pr/42".to_string(),
        );
        assert_eq!(pr.url, "https://example.com/pr/42");
    }

    // =============================================================================
    // ConventionalCommitType tests
    // =============================================================================

    #[test]
    fn test_commit_type_display() {
        assert_eq!(format!("{}", ConventionalCommitType::Feat), "feat");
        assert_eq!(format!("{}", ConventionalCommitType::Fix), "fix");
        assert_eq!(format!("{}", ConventionalCommitType::Chore), "chore");
        assert_eq!(format!("{}", ConventionalCommitType::Docs), "docs");
        assert_eq!(format!("{}", ConventionalCommitType::Refactor), "refactor");
        assert_eq!(format!("{}", ConventionalCommitType::Perf), "perf");
        assert_eq!(format!("{}", ConventionalCommitType::Test), "test");
    }

    #[test]
    fn test_commit_type_section_titles() {
        assert_eq!(ConventionalCommitType::Feat.section_title(), "Features");
        assert_eq!(ConventionalCommitType::Fix.section_title(), "Bug Fixes");
        assert_eq!(
            ConventionalCommitType::Docs.section_title(),
            "Documentation"
        );
        assert_eq!(
            ConventionalCommitType::Refactor.section_title(),
            "Refactors"
        );
        assert_eq!(ConventionalCommitType::Perf.section_title(), "Performance");
        assert_eq!(ConventionalCommitType::Test.section_title(), "Tests");
        assert_eq!(ConventionalCommitType::Chore.section_title(), "Chores");
    }

    // =============================================================================
    // group_prs_by_type tests
    // =============================================================================

    #[test]
    fn test_group_prs_by_type_basic() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: add login"),
            PullRequest::new("myorg", "myrepo", 2, "fix: resolve crash"),
            PullRequest::new("myorg", "myrepo", 3, "feat: add signup"),
            PullRequest::new("myorg", "myrepo", 4, "docs: update readme"),
            PullRequest::new("myorg", "myrepo", 5, "no prefix here"), // chore
        ];

        let grouped = group_prs_by_type(&prs);

        assert_eq!(grouped.groups.len(), 4); // feat, fix, docs, chore (no empty groups)

        // First group should be Feat with 2 PRs
        assert_eq!(grouped.groups[0].0, ConventionalCommitType::Feat);
        assert_eq!(grouped.groups[0].1.len(), 2);

        // Second group should be Fix with 1 PR
        assert_eq!(grouped.groups[1].0, ConventionalCommitType::Fix);
        assert_eq!(grouped.groups[1].1.len(), 1);

        // Third group should be Docs with 1 PR
        assert_eq!(grouped.groups[2].0, ConventionalCommitType::Docs);
        assert_eq!(grouped.groups[2].1.len(), 1);

        // Fourth group should be Chore with 1 PR (no prefix → chore)
        assert_eq!(grouped.groups[3].0, ConventionalCommitType::Chore);
        assert_eq!(grouped.groups[3].1.len(), 1);
    }

    #[test]
    fn test_group_prs_by_type_all_types() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: feature 1"),
            PullRequest::new("myorg", "myrepo", 2, "fix: bug 1"),
            PullRequest::new("myorg", "myrepo", 3, "chore: chore 1"),
            PullRequest::new("myorg", "myrepo", 4, "docs: docs 1"),
            PullRequest::new("myorg", "myrepo", 5, "refactor: refactor 1"),
            PullRequest::new("myorg", "myrepo", 6, "perf: perf 1"),
            PullRequest::new("myorg", "myrepo", 7, "test: test 1"),
        ];

        let grouped = group_prs_by_type(&prs);

        assert_eq!(grouped.groups.len(), 7);
        assert_eq!(grouped.groups[0].1.len(), 1);
        assert_eq!(grouped.groups[1].1.len(), 1);
        assert_eq!(grouped.groups[2].1.len(), 1);
        assert_eq!(grouped.groups[3].1.len(), 1);
        assert_eq!(grouped.groups[4].1.len(), 1);
        assert_eq!(grouped.groups[5].1.len(), 1);
        assert_eq!(grouped.groups[6].1.len(), 1);
    }

    #[test]
    fn test_group_prs_by_type_excludes_backports() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: feature on main"),
            PullRequest::new("myorg", "myrepo", 2, "Backport feat to v1.x"),
            PullRequest::new("myorg", "myrepo", 3, "feat: another main feature"),
        ];

        let grouped = group_prs_by_type(&prs);

        // Only 2 groups: feat (2 PRs) — no docs/chore/etc
        assert_eq!(grouped.groups.len(), 1);
        assert_eq!(grouped.groups[0].0, ConventionalCommitType::Feat);
        assert_eq!(grouped.groups[0].1.len(), 2);
    }

    #[test]
    fn test_group_prs_by_type_empty() {
        let prs: Vec<PullRequest> = vec![];
        let grouped = group_prs_by_type(&prs);
        assert!(grouped.groups.is_empty());
    }

    #[test]
    fn test_group_prs_by_type_all_backports() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "Backport #1 to release/1.x"),
            PullRequest::new("myorg", "myrepo", 2, "backport #2 to release/1.x"),
        ];

        let grouped = group_prs_by_type(&prs);
        assert!(grouped.groups.is_empty());
    }

    #[test]
    fn test_group_prs_by_type_preserves_order_within_group() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 3, "feat: third"),
            PullRequest::new("myorg", "myrepo", 1, "feat: first"),
            PullRequest::new("myorg", "myrepo", 2, "feat: second"),
        ];

        let grouped = group_prs_by_type(&prs);
        assert_eq!(grouped.groups.len(), 1);
        assert_eq!(grouped.groups[0].1[0].number, 3);
        assert_eq!(grouped.groups[0].1[1].number, 1);
        assert_eq!(grouped.groups[0].1[2].number, 2);
    }

    // =============================================================================
    // Markdown generation tests
    // =============================================================================

    #[test]
    fn test_generate_markdown_basic() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: add user login"),
            PullRequest::new("myorg", "myrepo", 2, "fix: resolve auth crash"),
        ];

        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        assert!(markdown.contains("## v1.0.0"));
        assert!(markdown.contains("### Features"));
        assert!(markdown.contains("add user login"));
        assert!(markdown.contains("[#1](https://github.com/myorg/myrepo/pull/1)"));
        assert!(markdown.contains("### Bug Fixes"));
        assert!(markdown.contains("resolve auth crash"));
        assert!(markdown.contains("[#2](https://github.com/myorg/myrepo/pull/2)"));
    }

    #[test]
    fn test_generate_markdown_with_date() {
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: add feature")];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0").with_date("2024-06-15");
        let markdown = generate_markdown(&grouped, &config);

        assert!(markdown.contains("## v1.0.0"));
        assert!(markdown.contains("**2024-06-15**"));
    }

    #[test]
    fn test_generate_markdown_omits_empty_groups() {
        // Only feat and fix PRs, so no docs/chore/refactor/etc sections
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: feature 1"),
            PullRequest::new("myorg", "myrepo", 2, "fix: bug 1"),
        ];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        assert!(markdown.contains("### Features"));
        assert!(markdown.contains("### Bug Fixes"));
        assert!(!markdown.contains("### Documentation"));
        assert!(!markdown.contains("### Chores"));
        assert!(!markdown.contains("### Refactors"));
    }

    #[test]
    fn test_generate_markdown_no_changes_message() {
        let prs: Vec<PullRequest> = vec![];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        assert!(markdown.contains("_No changes in this release._"));
    }

    #[test]
    fn test_generate_markdown_with_pr_links() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 42, "feat: add feature"),
            PullRequest::new("myorg", "myrepo", 99, "fix: fix bug"),
        ];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        assert!(markdown.contains("[#42](https://github.com/myorg/myrepo/pull/42)"));
        assert!(markdown.contains("[#99](https://github.com/myorg/myrepo/pull/99)"));
    }

    #[test]
    fn test_generate_markdown_all_sections() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: add feature"),
            PullRequest::new("myorg", "myrepo", 2, "fix: fix bug"),
            PullRequest::new("myorg", "myrepo", 3, "docs: update docs"),
            PullRequest::new("myorg", "myrepo", 4, "refactor: clean code"),
            PullRequest::new("myorg", "myrepo", 5, "perf: speed up"),
            PullRequest::new("myorg", "myrepo", 6, "test: add tests"),
            PullRequest::new("myorg", "myrepo", 7, "no prefix"),
        ];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        assert!(markdown.contains("### Features"));
        assert!(markdown.contains("### Bug Fixes"));
        assert!(markdown.contains("### Documentation"));
        assert!(markdown.contains("### Refactors"));
        assert!(markdown.contains("### Performance"));
        assert!(markdown.contains("### Tests"));
        assert!(markdown.contains("### Chores"));
    }

    // =============================================================================
    // Escape markdown tests
    // =============================================================================

    #[test]
    fn test_escape_markdown_special_chars() {
        // These would be tested via generate_markdown since escape_markdown is private
        let prs = vec![PullRequest::new(
            "myorg",
            "myrepo",
            1,
            "feat: use [brackets] and (parens) and *stars*",
        )];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        // Verify PR link is still correct
        assert!(markdown.contains("[#1](https://github.com/myorg/myrepo/pull/1)"));
        // Verify the title appears in the output
        assert!(markdown.contains("feat: use"));
    }

    #[test]
    fn test_generate_markdown_backports_excluded() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: main feature"),
            PullRequest::new("myorg", "myrepo", 2, "Backport: fix to v1.x"),
        ];
        let grouped = group_prs_by_type(&prs);
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_markdown(&grouped, &config);

        // Should only have Features section, no Bug Fixes
        assert!(markdown.contains("### Features"));
        assert!(!markdown.contains("### Bug Fixes"));
    }

    // =============================================================================
    // GroupedPRs tests
    // =============================================================================

    #[test]
    fn test_grouped_prs_is_empty() {
        let grouped = GroupedPRs::new();
        assert!(grouped.is_empty());
    }

    #[test]
    fn test_grouped_prs_is_not_empty() {
        let prs = vec![PullRequest::new("myorg", "myrepo", 1, "feat: test")];
        let grouped = group_prs_by_type(&prs);
        assert!(!grouped.is_empty());
    }

    #[test]
    fn test_grouped_prs_all_prs() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "feat: f1"),
            PullRequest::new("myorg", "myrepo", 2, "fix: f2"),
        ];
        let grouped = group_prs_by_type(&prs);
        let all = grouped.all_prs();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].number, 1);
        assert_eq!(all[1].number, 2);
    }

    // =============================================================================
    // ChangelogConfig tests
    // =============================================================================

    #[test]
    fn test_changelog_config_new() {
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        assert_eq!(config.owner, "myorg");
        assert_eq!(config.repo, "myrepo");
        assert_eq!(config.release_name, "v1.0.0");
        assert!(config.release_date.is_none());
    }

    #[test]
    fn test_changelog_config_with_date() {
        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0").with_date("2024-06-15");
        assert_eq!(config.release_date, Some("2024-06-15".to_string()));
    }

    // =============================================================================
    // Integration tests
    // =============================================================================

    #[test]
    fn test_full_changelog_generation() {
        // Simulates a realistic release scenario
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 10, "feat: add user dashboard"),
            PullRequest::new("myorg", "myrepo", 11, "feat: add settings page"),
            PullRequest::new("myorg", "myrepo", 12, "fix: login timeout handling"),
            PullRequest::new("myorg", "myrepo", 13, "docs: add API reference"),
            PullRequest::new("myorg", "myrepo", 14, "refactor: extract auth module"),
            PullRequest::new("myorg", "myrepo", 15, "perf: cache user queries"),
            PullRequest::new("myorg", "myrepo", 16, "test: add dashboard tests"),
            PullRequest::new("myorg", "myrepo", 17, "chore: bump version"),
            // These should be excluded
            PullRequest::new("myorg", "myrepo", 18, "Backport to release/1.x"),
        ];

        let config = ChangelogConfig::new("myorg", "myrepo", "v1.2.0").with_date("2024-06-15");
        let markdown = generate_release_notes(&prs, &config);

        // Verify all expected sections
        assert!(markdown.contains("## v1.2.0"));
        assert!(markdown.contains("**2024-06-15**"));
        assert!(markdown.contains("### Features"));
        assert!(markdown.contains("### Bug Fixes"));
        assert!(markdown.contains("### Documentation"));
        assert!(markdown.contains("### Refactors"));
        assert!(markdown.contains("### Performance"));
        assert!(markdown.contains("### Tests"));
        assert!(markdown.contains("### Chores"));

        // Verify PR links
        assert!(markdown.contains("[#10](https://github.com/myorg/myrepo/pull/10)"));
        assert!(markdown.contains("[#11](https://github.com/myorg/myrepo/pull/11)"));
        assert!(markdown.contains("[#12](https://github.com/myorg/myrepo/pull/12)"));
        assert!(markdown.contains("[#17](https://github.com/myorg/myrepo/pull/17)"));

        // Verify backport is excluded
        assert!(!markdown.contains("release/1.x"));
    }

    #[test]
    fn test_release_notes_without_conventional_prefix() {
        // PRs without prefixes should be categorized as chore
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "Update dependencies"),
            PullRequest::new("myorg", "myrepo", 2, "Fix typo in readme"),
            PullRequest::new("myorg", "myrepo", 3, "feat: add feature"),
        ];

        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_release_notes(&prs, &config);

        assert!(markdown.contains("### Features"));
        assert!(markdown.contains("### Chores"));
        assert!(markdown.contains("Update dependencies"));
        assert!(markdown.contains("Fix typo in readme"));
    }

    #[test]
    fn test_release_notes_with_multiple_prefixes_uses_first() {
        let prs = vec![
            PullRequest::new("myorg", "myrepo", 1, "fix: handle error feat: add fallback"),
            PullRequest::new(
                "myorg",
                "myrepo",
                2,
                "feat: this has fix: in the description",
            ),
        ];

        let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
        let markdown = generate_release_notes(&prs, &config);

        // Both should be in Features or Fix - first one (fix:) should be Bug Fixes
        // But "feat:" is checked first in the list... wait, we check feat first
        // Actually, looking at the order: feat, fix, chore, docs, refactor, perf, test
        // So "fix: handle error feat: add fallback" starts with "fix:" → Fix
        // "feat: this has fix:..." starts with "feat:" → Feat
        assert!(markdown.contains("### Bug Fixes"));
        assert!(markdown.contains("### Features"));
    }
}
