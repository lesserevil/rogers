//! Configuration types for the GitHub crate.

use serde::{Deserialize, Serialize};

/// GitHub configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubConfig {
    pub owner: String,
    pub repo: String,
    pub token: String,
    pub api_url: Option<String>,
}
