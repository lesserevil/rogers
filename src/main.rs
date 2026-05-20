//! Rodgers CLI entry point.

use clap::Parser;
use tracing::info;

use rogers::{init::check_and_suggest_templates, Result};

pub mod cli;

fn main() {
    // Initialize tracing with basic config
    tracing_subscriber::fmt::init();

    let cli = cli::Cli::parse();

    match run(cli) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(e.exit_code());
        }
    }
}

fn run(cli: cli::Cli) -> Result<()> {
    match cli.command {
        cli::Commands::Init {
            repo,
            fix: _,
            json: _,
            github_token: _,
        } => {
            info!(repo = repo, "Running init check");
            
            let auto_suggest = true;
            
            let result = check_and_suggest_templates(&repo, vec![], auto_suggest);
            
            if result.bead_filed {
                println!("Init check complete for {}", repo);
                println!("Templates missing - bead filed with suggested templates:");
                println!();
                println!("Title: {}", result.bead_title());
                println!("Type: {}", result.bead_type_label());
                println!();
                println!("Body:\n{}", result.bead_body.unwrap());
            } else {
                println!("Init check complete for {}. Templates are complete.", repo);
            }
        }
        cli::Commands::Doctor { .. } => {
            println!("Doctor command not yet implemented");
        }
    }
    
    Ok(())
}