//! Rodgers — github-native community relations agent
//!
//! Rodgers runs on a schedule, reads GitHub issues and discussions, and manages
//! the full triage-to-release lifecycle entirely through the GitHub API and a
//! local beads database.

mod cli;
mod doctor;
mod error;
mod labels;

use anyhow::Result;
use cli::Cli;
use cli::Commands;
use doctor::report::{OutputFormat, ReportGenerator};
use doctor::{
    ALL_CATEGORIES, CATEGORY_AUTH, CATEGORY_BEADS, CATEGORY_CONFIG, CATEGORY_DRIFT, CATEGORY_PLANS,
    CATEGORY_REPO, CategoryResult, DoctorResult, categories, drift,
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
            fix: _,
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

            // Run doctor checks
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
/// Executes all categories and collects failures from each.
/// Does not fail-fast - collects ALL failures so the report lists all issues.
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
    if categories_to_run.contains(&CATEGORY_CONFIG) {
        match categories::check_config(config_path) {
            Ok(cat_result) => {
                result.categories.push(cat_result);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_CONFIG, e.to_string()));
            }
        }
    }

    // Run auth check
    if categories_to_run.contains(&CATEGORY_AUTH) {
        let token = config.github.token.as_deref().unwrap_or("");
        let owner = &config.github.owner;
        let repo = &config.github.repo;
        let api_url = config.github.api_url.as_deref();

        match categories::check_auth(owner, repo, token, api_url).await {
            Ok(cat_result) => {
                result.categories.push(cat_result);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_AUTH, e.to_string()));
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
        match drift::check_drift(owner, repo, token, api_url, verbose).await {
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
