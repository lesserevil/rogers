//! Git operations for Rodgers.
//!
//! This module provides local git operations needed for release management:
//! branch creation, tag creation, and pushing to remotes.
//!
//! ## Modules
//!
//! - `client` - Git client for local repository operations

pub mod client;

pub use client::{
    BranchAlreadyExists, GitClient, GitError, TagAlreadyExists, create_annotated_tag,
    create_release_branch, push_branch_and_tag,
};
