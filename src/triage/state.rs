//! Triage state — persisted between runs to track what has already been processed.
//!
//! Rodgers stores a simple timestamp of the last successful triage run so that on
//! the next run it can fetch "all PRs merged since last time" via the GitHub API.

use chrono::Utc;
use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Persistent state saved between triage runs.
#[derive(Debug, Clone)]
pub struct LastRunState {
    /// ISO 8601 timestamp of the last successful triage run.
    pub last_run: String,
    path: std::path::PathBuf,
}

impl LastRunState {
    /// Load from disk, creating a sensible default if missing.
    pub fn load(path: impl AsRef<Path>) -> Self {
        let path = path.as_ref().to_path_buf();
        if let Ok(contents) = fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str::<LastRunJson>(&contents) {
                info!("Loaded triage state from {:?}", path);
                return Self {
                    last_run: state.last_run,
                    path,
                };
            }
        }
        // Default: process everything merged in the last hour to avoid a big backlog on first run.
        let one_hour_ago = Utc::now()
            .checked_sub_signed(chrono::Duration::hours(1))
            .map(|t| t.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let default = Self {
            last_run: one_hour_ago,
            path,
        };
        info!(
            "No triage state found; defaulting last-run to {}",
            default.last_run
        );
        default
    }

    /// Returns the ISO 8601 timestamp string for use in GitHub API queries.
    pub fn last_run_timestamp(&self) -> &str {
        &self.last_run
    }

    /// Persist the current time as the new last-run timestamp.
    pub fn mark_complete(&mut self) {
        self.last_run = Utc::now().to_rfc3339();
        if let Err(e) = self.save() {
            warn!("Failed to save triage state: {}", e);
        }
    }

    fn save(&self) -> Result<(), std::io::Error> {
        let json = LastRunJson {
            last_run: self.last_run.clone(),
        };
        let contents = serde_json::to_string_pretty(&json)?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&self.path, contents)
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct LastRunJson {
    last_run: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_last_run_default() {
        let tmp = TempDir::new().unwrap();
        let state = LastRunState::load(tmp.path().join("state.json"));
        // Should default to roughly one hour ago — parseable as RFC3339
        assert!(chrono::DateTime::parse_from_rfc3339(&state.last_run).is_ok());
    }

    #[test]
    fn test_mark_complete() {
        let tmp = TempDir::new().unwrap();
        let mut state = LastRunState::load(tmp.path().join("state.json"));
        let original = state.last_run.clone();
        std::thread::sleep(std::time::Duration::from_millis(10));
        state.mark_complete();
        assert_ne!(original, state.last_run);
        assert!(chrono::DateTime::parse_from_rfc3339(&state.last_run).is_ok());
    }
}
