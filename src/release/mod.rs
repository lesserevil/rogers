//! Release management module.
//!
//! This module handles the full release lifecycle: changelog generation,
//! GitHub Release creation via the API, and post-release notifications.
//!
//! ## Modules
//!
//! - `changelog` — Changelog generation from PR data using conventional commits.
//! - `github_release` — GitHub Release API integration for creating and updating releases.
//!
//! ## Release Flow
//!
//! ```mermaid
//! flowchart LR
//!     A[PR Data] --> B[Generate Changelog]
//!     B --> C[Create Release Config]
//!     C --> D[Create/Update GitHub Release]
//!     D --> E[Post Discussion Notification]
//! ```

pub mod changelog;
pub mod github_release;

pub use changelog::{
    ChangelogConfig, ConventionalCommitType, GroupedPRs, ParsedCommit, PullRequest,
    generate_markdown, generate_release_notes, group_prs_by_type, parse_conventional_commit,
};
pub use github_release::{
    ReleaseClient, ReleaseConfig, ReleaseInfo, build_release_config, release_notification_comment,
};
