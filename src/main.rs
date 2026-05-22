//! Rodgers — GitHub-native community relations agent.

use clap::Parser;
use std::path::{Path, PathBuf};

use rogers::config::validation::load_and_validate_config;
use rogers::{Result, RogersError};
use rogers::cli::{Cli, Commands};

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(e.exit_code());
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            repo,
            fix,
            json,
            github_token,
        } => {
            cmd_init(&repo, fix, json, github_token.as_deref())
        }
        Commands::Doctor {
            verbose,
            only,
            fix,
            json,
            config,
        } => cmd_doctor(verbose, only, fix, json, config.as_deref()),
    }
}

fn load_config(
    config_path: Option<&PathBuf>,
) -> Result<(rogers::config::schema::Config, rogers::config::validation::ValidationResult)> {
    let path = config_path
        .map(|p| p.as_path())
        .unwrap_or_else(|| Path::new("config.yaml"));

    let (config, validation_result) = load_and_validate_config(&path)?;

    for warning in &validation_result.warnings {
        eprintln!("Warning: {}", warning);
    }

    Ok((config, validation_result))
}

fn cmd_init(repo: &str, fix: bool, json: bool, github_token: Option<&str>) -> Result<()> {
    if json {
        println!(
            r#"{{"command": "init", "repo": "{}", "fix": {}}}"#,
            repo, fix
        );
    } else {
        println!("Initializing Rodgers for repository: {}", repo);
        if fix {
            println!("Auto-fix mode enabled");
        }
        if let Some(token) = github_token {
            println!("Using provided GitHub token");
        } else {
            println!("Using GITHUB_TOKEN from environment");
        }
    }

    Err(RogersError::Config(
        "init command not yet implemented".to_string(),
    ))
}

fn cmd_doctor(
    verbose: bool,
    only: Vec<String>,
    fix: bool,
    json: bool,
    config_path: Option<&Path>,
) -> Result<()> {
    let path = config_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| PathBuf::from("config.yaml"));
    let (_config, validation_result) = load_and_validate_config(&path)?;

    if json {
        println!(
            r#"{{"command": "doctor", "verbose": {}, "fix": {}, "warnings": {}}}"#,
            verbose,
            fix,
            validation_result.warnings.len()
        );
    } else {
        println!("Running Rodgers health check...");
        if verbose {
            println!("Verbose mode enabled");
        }
        if !only.is_empty() {
            println!("Limiting to checks: {}", only.join(", "));
        }
        if fix {
            println!("Auto-fix mode enabled (interactive)");
        }
        if validation_result.has_warnings() {
            println!(
                "Configuration warnings: {}",
                validation_result.warnings.len()
            );
            for warning in &validation_result.warnings {
                println!("  - {}", warning);
            }
        } else {
            println!("Configuration: OK (no warnings)");
        }
    }

    Err(RogersError::Config(
        "doctor command not yet implemented".to_string(),
    ))
}
