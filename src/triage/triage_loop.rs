//! Triage loop — runs on every scheduler tick.
//!
//! On each run, Rodgers:
//!  1. Fetches merged PRs since the last run
//!  2. Identifies backport candidates (bug/security/backport-me merges)
//!  3. Determines active release branches from config
//!  4. Delegates to the backport manager to create backport beads
//!
//! All steps complete within ONE triage run - no multi-run state machine
//! is needed for the initial detection phase.

use tracing::{info, warn};

use crate::Config;
use crate::github::client::GithubClient;
use crate::triage::state::LastRunState;

/// Runs one triage cycle, scanning for merged PRs and triggering backport detection.
pub async fn run_triage(
    config: &Config,
    github: &GithubClient,
    state: &LastRunState,
) -> Result<TriageResult, crate::RogersError> {
    info!("Starting triage run");

    // Step 1: Fetch merged PRs since last run
    let since = state.last_run_timestamp();
    let merged_prs = github.merged_prs_since(since).await?;

    if merged_prs.is_empty() {
        info!("No merged PRs since {}", since);
        return Ok(TriageResult::default());
    }

    info!("Found {} newly-merged PRs", merged_prs.len());

    // Step 2: Identify backport candidates from merged PRs
    let merged_prs_count = merged_prs.len();
    let candidates =
        crate::backport::detector::detect_candidates(merged_prs, github, config).await?;

    if candidates.is_empty() {
        info!("No backport candidates detected in this run");
        return Ok(TriageResult::default());
    }

    info!(
        "Detected {} backport candidates: {:?}",
        candidates.len(),
        candidates.iter().map(|c| c.pr.number).collect::<Vec<_>>()
    );

    // Step 3: Read active release branches from config
    let active_branches = resolve_active_branches(config, github).await?;

    if active_branches.is_empty() {
        warn!(
            "No active release branches configured; no backports will be created. \
            Set release.active_branches in config.yaml."
        );
    }

    // Step 4: Delegate to backport manager (passing github client + discussion category
    // for creating approval discussions per backport bead)
    let discussion_category = &config.release.approval_discussion_category;
    let backport_results = crate::backport::manager::process_candidates(
        &candidates,
        &active_branches,
        github,
        discussion_category,
    )
    .await?;

    let result = TriageResult {
        merged_prs_count,
        candidates_detected: candidates.len(),
        active_branches_found: active_branches.len(),
        backport_results,
    };

    info!(
        "Triage run complete: {} candidates across {} active branches",
        result.candidates_detected, result.active_branches_found
    );

    Ok(result)
}

/// Resolve active release branches from config, skipping any that don't exist.
async fn resolve_active_branches(
    config: &Config,
    github: &GithubClient,
) -> Result<Vec<String>, crate::RogersError> {
    let configured = &config.release.active_branches;
    let mut active: Vec<String> = Vec::with_capacity(configured.len());

    for branch in configured {
        match github.branch_exists(branch).await {
            Ok(true) => {
                info!("Active release branch confirmed: {}", branch);
                active.push(branch.clone());
            }
            Ok(false) => {
                warn!(
                    "Configured active branch '{}' does not exist; skipping",
                    branch
                );
            }
            Err(e) => {
                warn!(
                    "Could not check branch '{}' existence: {}; skipping",
                    branch, e
                );
            }
        }
    }

    Ok(active)
}

// ---------------------------------------------------------------------------
// Results and state
// ---------------------------------------------------------------------------

/// Outcome of a single triage run.
#[derive(Debug, Clone, Default)]
pub struct TriageResult {
    /// Number of PRs merged since the last run.
    pub merged_prs_count: usize,
    /// Number of backport candidates detected.
    pub candidates_detected: usize,
    /// Number of active release branches resolved.
    pub active_branches_found: usize,
    /// Individual backport processing results.
    pub backport_results: Vec<crate::backport::manager::BackportResult>,
}
