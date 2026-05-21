//! Release notes generation from PR titles and labels.
//!
//! This module provides functionality for generating changelog/release notes
//! from PR data using conventional commits. It is the core engine for
//! CRIT-3 of the release management plan.
//!
//! ## Architecture
//!
//! ```mermaid
//! flowchart LR
//!     A[PR Data] --> B[Parse Commit Type]
//!     B --> C[Group by Type]
//!     C --> D[Generate Markdown]
//!     D --> E[GitHub Release Notes]
//! ```
//!
//! ## Usage
//!
//! ```
//! use rogers::release::{
//!     changelog::{PullRequest, ChangelogConfig, generate_release_notes},
//! };
//!
//! let prs = vec![
//!     PullRequest::new("myorg", "myrepo", 1, "feat: add user login"),
//!     PullRequest::new("myorg", "myrepo", 2, "fix: resolve crash"),
//! ];
//!
//! let config = ChangelogConfig::new("myorg", "myrepo", "v1.0.0");
//! let notes = generate_release_notes(&prs, &config);
//! ```
//!
//! ## Conventional Commit Support
//!
//! Recognized types: `feat`, `fix`, `chore`, `docs`, `refactor`, `perf`, `test`
//! PRs without a recognized prefix are categorized as `chore`.

pub mod changelog;

pub use changelog::{
    ChangelogConfig, ConventionalCommitType, GroupedPRs, ParsedCommit, PullRequest,
    generate_markdown, generate_release_notes, group_prs_by_type, parse_conventional_commit,
};
