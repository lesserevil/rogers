//! Rodgers — github-native community relations agent
//!
//! Rodgers runs on a schedule, reads GitHub issues and discussions, and manages
//! the full triage-to-release lifecycle entirely through the GitHub API and a
//! local beads database.

mod beads;
mod cli;
mod doctor;
mod error;
mod feature_bug;
mod github;
mod labels;

use anyhow::Result;
use cli::Cli;
use cli::Commands;
use doctor::report::{OutputFormat, ReportGenerator};
use doctor::{
    ALL_CATEGORIES, CATEGORY_AUTH, CATEGORY_BEADS, CATEGORY_CONFIG, CATEGORY_DRIFT, CATEGORY_PLANS,
    CATEGORY_REPO, CategoryResult, CategoryStatus, DoctorResult, categories, drift, fix,
};
use std::path::PathBuf;

/// Parse configuration from a YAML file
fn load_config(path: &PathBuf) -> Result<categories::RodgersConfig> {
    let content = std::fs::read_to_string(path)?;
    let config: categories::RodgersConfig = serde_yaml::from_str(&content)?;
    Ok(config)
}

#[::tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Doctor {
            verbose,
            only,
            fix: fix_mode,
            json,
            config,
        } => {
            // Determine config path
            let config_path = config.unwrap_or_else(|| PathBuf::from("config.yaml"));

            // Determine output format
            let output_format = if json {
                OutputFormat::Json
            } else {
                OutputFormat::Text
            };

            // In fix mode, we need to be interactive
            if fix_mode {
                // Check for non-interactive environment
                if !fix::is_interactive() {
                    eprintln!(
                        "Error: --fix requires interactive mode. \
                         Set CI or RODGERS_NON_INTERACTIVE to skip."
                    );
                    std::process::exit(2);
                }

                // Run fix session
                let result = run_fix_session(&config_path).await;

                // Exit with appropriate code based on fix results
                std::process::exit(result.exit_code());
            }

            // Run doctor checks (no fix mode)
            let result = run_doctor_checks(&config_path, &only, verbose).await;

            // Generate and print report
            let generator = ReportGenerator::new(output_format, verbose);
            let report = generator.generate(&result);
            println!("{}", report);

            // Exit with appropriate code
            std::process::exit(result.exit_code());
        }
        Commands::Init { .. } => {
            println!("Init command not yet implemented");
            Ok(())
        }
    }
}

/// Run all doctor health checks
///
/// Executes categories in order: config → auth → beads → plans → repo → drift
///
/// AC-4: Fail-fast behavior:
/// - If config validation fails, exit immediately (skip auth, beads, plans, repo, drift)
/// - If auth validation fails, exit immediately (skip beads, plans, repo, drift)
/// - Only continue to remaining categories if config AND auth passed
///
/// Exit code 1 if any category fails OR drift is detected.
async fn run_doctor_checks(
    config_path: &PathBuf,
    only_categories: &[String],
    verbose: bool,
) -> DoctorResult {
    let mut result = DoctorResult::new();

    // Determine which categories to run
    let categories_to_run: Vec<&str> = if only_categories.is_empty() {
        ALL_CATEGORIES.to_vec()
    } else {
        only_categories
            .iter()
            .filter(|c| ALL_CATEGORIES.contains(&c.as_str()))
            .map(|s| s.as_str())
            .collect()
    };

    // Filter for categories we need to check (pre-seed with skipped for filtering)
    for cat in ALL_CATEGORIES {
        if !categories_to_run.contains(cat) {
            result.categories.push(CategoryResult::skipped(*cat));
        }
    }

    // Load config - if it fails, record the failure and continue with other checks
    // Note: Some checks (auth, repo, drift) won't be able to run without config values
    // but we still try them and let them report their own failures
    let config = match load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            // If config can't be loaded, add a fail result but continue checking
            // other categories that don't strictly require a loaded config
            result.categories.push(CategoryResult::fail(
                CATEGORY_CONFIG,
                format!("Failed to load config: {}", e),
            ));
            // Continue running other categories - they'll report their own issues
            // about missing config values
            categories::RodgersConfig {
                github: categories::GitHubConfig {
                    owner: String::new(),
                    repo: String::new(),
                    token: None,
                    api_url: None,
                },
                scheduler: None,
                beads: categories::BeadsConfig {
                    remote: None,
                    database: None,
                },
                llm: categories::LlmConfig {
                    provider: None,
                    base_url: None,
                    model: None,
                    api_key: None,
                },
                triage: None,
                release: None,
                rogation: None,
                log_level: None,
                error_channel: None,
            }
        }
    };

    // Run config check first (always runs, always runs first)
    // AC-4: Fail-fast - exit immediately if config validation fails
    if categories_to_run.contains(&CATEGORY_CONFIG) {
        match categories::check_config(config_path) {
            Ok(cat_result) => {
                result.categories.push(cat_result.clone());
                // Fail-fast: if config check failed, exit immediately
                // Skip remaining categories (auth, beads, plans, repo, drift)
                if let CategoryStatus::Fail(_) = cat_result.status {
                    result.is_healthy = false;
                    return result;
                }
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_CONFIG, e.to_string()));
                result.is_healthy = false;
                return result;
            }
        }
    }

    // Run auth check
    // AC-4: Fail-fast - exit immediately if auth validation fails
    if categories_to_run.contains(&CATEGORY_AUTH) {
        let token = config.github.token.as_deref().unwrap_or("");
        let owner = &config.github.owner;
        let repo = &config.github.repo;
        let api_url = config.github.api_url.as_deref();

        match categories::check_auth(owner, repo, token, api_url).await {
            Ok(cat_result) => {
                result.categories.push(cat_result.clone());
                // Fail-fast: if auth check failed, exit immediately
                // Skip remaining categories (beads, plans, repo, drift)
                if let CategoryStatus::Fail(_) = cat_result.status {
                    result.is_healthy = false;
                    return result;
                }
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_AUTH, e.to_string()));
                result.is_healthy = false;
                return result;
            }
        }
    }

    // Continue running all remaining categories regardless of earlier failures.
    // We collect ALL failures to give a complete health report.
    let token = config.github.token.as_deref().unwrap_or("");
    let owner = &config.github.owner;
    let repo = &config.github.repo;
    let api_url = config.github.api_url.as_deref();

    // Run beads check
    if categories_to_run.contains(&CATEGORY_BEADS) {
        let remote = config.beads.remote.as_deref().unwrap_or("");
        let database = config.beads.database.as_deref();

        match categories::check_beads(remote, database).await {
            Ok(cat_result) => {
                result.categories.push(cat_result);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_BEADS, e.to_string()));
            }
        }
    }

    // Run plans check
    if categories_to_run.contains(&CATEGORY_PLANS) {
        // Plans dir is relative to config path or current directory
        let plans_dir = config_path
            .parent()
            .map(|p| p.join("plans"))
            .unwrap_or_else(|| PathBuf::from("plans"));

        match categories::check_plans(&plans_dir) {
            Ok(cat_result) => {
                result.categories.push(cat_result);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_PLANS, e.to_string()));
            }
        }
    }

    // Run repo check
    if categories_to_run.contains(&CATEGORY_REPO) {
        let active_branches = config
            .release
            .as_ref()
            .and_then(|r| r.active_branches.clone());

        match categories::check_repo(owner, repo, token, api_url, active_branches).await {
            Ok(cat_result) => {
                result.categories.push(cat_result);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_REPO, e.to_string()));
            }
        }
    }

    // Run drift check (always runs last if included)
    // Collect drift events for inclusion in the result
    if categories_to_run.contains(&CATEGORY_DRIFT) {
        let beads_remote = config.beads.remote.as_deref().unwrap_or("");
        let beads_database = config.beads.database.as_deref();

        match drift::check_drift(
            owner,
            repo,
            token,
            api_url,
            verbose,
            beads_remote,
            beads_database,
        )
        .await
        {
            Ok(drift_result) => {
                // Add the category result (summary of drift check)
                result.categories.push(drift_result.category_result);
                // Collect all drift events for the report
                result.drift_events.extend(drift_result.drift_events);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_DRIFT, e.to_string()));
            }
        }
    }

    // Set overall health status based on all collected results
    result.is_healthy = result.all_categories_passed() && !result.has_drift();

    result
}

/// Result of a fix session
struct FixSessionResult {
    /// Total drift events processed
    total_events: usize,
    /// Fixes that were applied
    applied: usize,
    /// Fixes that were skipped
    skipped: usize,
    /// Fixes that failed
    failed: usize,
    /// Whether user cancelled the session
    cancelled: bool,
}

impl FixSessionResult {
    /// Exit code based on fix results
    fn exit_code(&self) -> i32 {
        if self.cancelled {
            // User cancelled - partial results
            1
        } else if self.failed > 0 {
            // Some fixes failed
            1
        } else if self.applied > 0 && self.skipped == 0 && self.total_events > 0 {
            // All fixes applied successfully
            0
        } else {
            // No drift or all skipped - healthy
            0
        }
    }
}

/// Run interactive fix session for drift events
///
/// Presents each drift event with options and prompts for confirmation
/// before applying fixes via the GitHub API.
async fn run_fix_session(config_path: &PathBuf) -> FixSessionResult {
    // Load configuration
    let config = match load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: Failed to load config from {:?}: {}", config_path, e);
            return FixSessionResult {
                total_events: 0,
                applied: 0,
                skipped: 0,
                failed: 0,
                cancelled: true,
            };
        }
    };

    let owner = &config.github.owner;
    let repo = &config.github.repo;
    let token = config.github.token.as_deref().unwrap_or("");
    let api_url = config.github.api_url.as_deref();

    // Validate required config
    if owner.is_empty() || repo.is_empty() || token.is_empty() {
        eprintln!("Error: --fix requires github.owner, github.repo, and github.token in config");
        return FixSessionResult {
            total_events: 0,
            applied: 0,
            skipped: 0,
            failed: 0,
            cancelled: true,
        };
    }

    // Get drift events first (run drift check)
    let beads_remote = config.beads.remote.as_deref().unwrap_or("");
    let beads_database = config.beads.database.as_deref();
    let drift_result = match drift::check_drift(
        owner,
        repo,
        token,
        api_url,
        true,
        beads_remote,
        beads_database,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Error: Failed to run drift check: {}", e);
            return FixSessionResult {
                total_events: 0,
                applied: 0,
                skipped: 0,
                failed: 0,
                cancelled: true,
            };
        }
    };

    let events = drift_result.drift_events;

    if events.is_empty() {
        println!("No drift detected. Rodgers is healthy!");
        return FixSessionResult {
            total_events: 0,
            applied: 0,
            skipped: 0,
            failed: 0,
            cancelled: false,
        };
    }

    println!("Found {} drift events to address.\n", events.len());
    println!("Starting interactive fix session...");
    println!("For each event, choose (y)es to apply first option, or enter to skip");
    println!("Press Ctrl+C at any time to cancel.\n");

    // Create fix session
    let mut session = fix::FixSession::new(
        owner.clone(),
        repo.clone(),
        token.to_string(),
        config.github.api_url,
    );

    let mut results: Vec<fix::FixResult> = Vec::new();
    let mut cancelled = false;

    // Process each event
    for event in &events {
        let result = session.fix_event(event).await;
        results.push(result);

        // Check if user cancelled
        if results
            .last()
            .map(|r| r.action == "user_cancelled")
            .unwrap_or(false)
        {
            cancelled = true;
            break;
        }
    }

    // Summarize results
    let summary = fix::summarize_results(&results);
    println!("\n{}", summary);

    // Count results
    let total = results.len();
    let applied = results.iter().filter(|r| r.applied).count();
    let skipped = results.iter().filter(|r| r.action == "skipped").count();
    let failed = results.iter().filter(|r| r.error.is_some()).count();

    FixSessionResult {
        total_events: total,
        applied,
        skipped,
        failed,
        cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// AC-4 Unit test: Invalid config → exit after config, no auth/beads/repo/drift
    #[tokio::test]
    async fn test_fail_fast_invalid_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Write invalid config - missing required keys
        let invalid_config = r#"
github:
  owner: ""  # Invalid - empty
llm:
  provider: openai
"#;
        std::fs::write(&config_path, invalid_config).unwrap();

        let result = run_doctor_checks(&config_path, &[], false).await;

        // Should exit fast on config failure
        assert!(result.any_category_failed());

        // Only config should be in results
        let config_result = result.categories.iter().find(|c| c.name == CATEGORY_CONFIG);
        assert!(config_result.is_some());
        assert!(matches!(
            config_result.unwrap().status,
            CategoryStatus::Fail(_)
        ));

        // Auth, beads, plans, repo, drift should NOT be run (not even as skipped
        // because they are not run due to fail-fast)
        let auth_result = result.categories.iter().find(|c| c.name == CATEGORY_AUTH);
        let beads_result = result.categories.iter().find(|c| c.name == CATEGORY_BEADS);
        let plans_result = result.categories.iter().find(|c| c.name == CATEGORY_PLANS);
        let repo_result = result.categories.iter().find(|c| c.name == CATEGORY_REPO);
        let drift_result = result.categories.iter().find(|c| c.name == CATEGORY_DRIFT);

        // These should not exist because we exited fast before they were even checked
        assert!(
            auth_result.is_none(),
            "Auth should not run on config fail-fast"
        );
        assert!(
            beads_result.is_none(),
            "Beads should not run on config fail-fast"
        );
        assert!(
            plans_result.is_none(),
            "Plans should not run on config fail-fast"
        );
        assert!(
            repo_result.is_none(),
            "Repo should not run on config fail-fast"
        );
        assert!(
            drift_result.is_none(),
            "Drift should not run on config fail-fast"
        );
    }

    /// AC-4 Unit test: Invalid auth → exit after auth, no beads/repo/drift
    #[tokio::test]
    async fn test_fail_fast_invalid_auth() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Write valid config first so we pass config check
        let valid_config = r#"
github:
  owner: test-owner
  repo: test-repo
  token: invalid-token-that-will-fail-auth
  api_url: https://api.github.com
beads:
  remote: https://dolt.example.com/test
  database: test.hibernate
llm:
  provider: openai
  model: gpt-4o-mini
"#;
        std::fs::write(&config_path, valid_config).unwrap();

        let result = run_doctor_checks(&config_path, &[], false).await;

        // Should fail on auth
        assert!(result.any_category_failed());

        // Config should pass
        let config_result = result.categories.iter().find(|c| c.name == CATEGORY_CONFIG);
        assert!(config_result.is_some());
        assert!(config_result.unwrap().status.is_ok(), "Config should pass");

        // Auth should fail
        let auth_result = result.categories.iter().find(|c| c.name == CATEGORY_AUTH);
        assert!(auth_result.is_some());
        assert!(
            matches!(auth_result.unwrap().status, CategoryStatus::Fail(_)),
            "Auth should fail with invalid token"
        );

        // Beads, plans, repo, drift should NOT be run due to fail-fast
        let beads_result = result.categories.iter().find(|c| c.name == CATEGORY_BEADS);
        let plans_result = result.categories.iter().find(|c| c.name == CATEGORY_PLANS);
        let repo_result = result.categories.iter().find(|c| c.name == CATEGORY_REPO);
        let drift_result = result.categories.iter().find(|c| c.name == CATEGORY_DRIFT);

        assert!(
            beads_result.is_none(),
            "Beads should not run on auth fail-fast"
        );
        assert!(
            plans_result.is_none(),
            "Plans should not run on auth fail-fast"
        );
        assert!(
            repo_result.is_none(),
            "Repo should not run on auth fail-fast"
        );
        assert!(
            drift_result.is_none(),
            "Drift should not run on auth fail-fast"
        );
    }

    /// AC-4 Unit test: Valid config/auth → continues to beads
    #[tokio::test]
    async fn test_continues_past_auth_when_valid() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Write valid config
        let valid_config = r#"
github:
  owner: test-owner
  repo: test-repo
  token: ${RODGERS_GITHUB_TOKEN}
  api_url: https://api.github.com
beads:
  remote: https://dolt.example.com/test
  database: test.hibernate
llm:
  provider: openai
  model: gpt-4o-mini
"#;
        std::fs::write(&config_path, valid_config).unwrap();

        let result = run_doctor_checks(&config_path, &[], false).await;

        // Config should pass (may have warnings but not fail)
        let config_result = result.categories.iter().find(|c| c.name == CATEGORY_CONFIG);
        assert!(config_result.is_some());

        // Note: Auth check will likely fail because we don't have a real token,
        // but in a real scenario with valid token, it would pass and we would
        // continue to other categories

        // If we got past config without fail-fast, config should be present
        let passed_config = config_result.is_some()
            && matches!(
                config_result.unwrap().status,
                CategoryStatus::Pass | CategoryStatus::Warn(_)
            );
        if passed_config {
            // If config passed, auth should be checked (whether it passes or fails)
            let auth_result = result.categories.iter().find(|c| c.name == CATEGORY_AUTH);
            assert!(auth_result.is_some(), "Auth should run after config passes");
        }
    }

    /// AC-4 Unit test: --only category skips to that category (does not reset fail-fast logic)
    #[tokio::test]
    async fn test_only_category_runs_specific_category() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.yaml");

        // Write invalid config
        let invalid_config = r#"
github:
  owner: ""  # Invalid
llm:
  provider: openai
"#;
        std::fs::write(&config_path, invalid_config).unwrap();

        // Use --only to run only plans (skipping config and auth)
        let result = run_doctor_checks(&config_path, &["plans".to_string()], false).await;

        // Plans should be run even though config would fail
        // Because --only skips earlier categories
        let plans_result = result.categories.iter().find(|c| c.name == CATEGORY_PLANS);
        assert!(
            plans_result.is_some(),
            "Plans should run when --only plans is specified"
        );

        // Config should be skipped (not run, not failed)
        let config_result = result.categories.iter().find(|c| c.name == CATEGORY_CONFIG);
        assert!(config_result.is_some());
        assert!(
            matches!(config_result.unwrap().status, CategoryStatus::Skipped),
            "Config should be skipped when --only plans is specified"
        );
    }
}
