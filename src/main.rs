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
async fn run_doctor_checks(
    config_path: &PathBuf,
    only_categories: &[String],
    verbose: bool,
) -> DoctorResult {
    let mut result = DoctorResult::new();
    let config = match load_config(config_path) {
        Ok(c) => c,
        Err(e) => {
            // If config can't be loaded, add a fail result and return
            result.categories.push(CategoryResult::fail(
                "config",
                format!("Failed to load config: {}", e),
            ));
            return result;
        }
    };

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

    // Check if config passed - if not, fail fast
    if result.any_category_failed() {
        // If config fails, skip remaining categories
        return result;
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

    // Check if auth passed - if not, fail fast
    if result.any_category_failed() {
        // If auth fails, skip remaining categories
        return result;
    }

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
    if categories_to_run.contains(&CATEGORY_DRIFT) {
        match drift::check_drift(owner, repo, token, api_url, verbose).await {
            Ok(cat_result) => {
                // Collect drift events from the drift result
                // Note: In a real implementation, we'd pass actual drift events back
                result.categories.push(cat_result);
            }
            Err(e) => {
                result
                    .categories
                    .push(CategoryResult::fail(CATEGORY_DRIFT, e.to_string()));
            }
        }
    }

    // Set overall health status
    result.is_healthy = result.all_categories_passed() && !result.has_drift();

    result
}
