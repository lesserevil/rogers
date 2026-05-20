//! Rodgers CLI entry point.

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "rogers",
    about = "Rodgers — github-native community relations agent",
    long_about = None,
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Audit a GitHub repository for readiness to be managed by Rodgers.
    Init {
        /// Target repository in owner/repo format.
        #[arg(long, value_name = "OWNER/REPO")]
        repo: String,

        /// Apply automated fixes where possible.
        #[arg(long, short = 'f')]
        fix: bool,

        /// Output JSON instead of human-readable text.
        #[arg(long, short = 'j')]
        json: bool,

        /// Repository admin token override (for applying settings that require admin).
        /// If not provided, reads from GITHUB_TOKEN env var.
        #[arg(long, visible_alias = "token")]
        github_token: Option<String>,
    },

    /// Audit an existing Rodgers installation for configuration problems and state drift.
    Doctor {
        /// Show detailed output including all drift events.
        #[arg(long, short = 'v')]
        verbose: bool,

        /// Limit to specific health check categories: config, auth, beads, plans, repo, drift.
        #[arg(long, short = 'o', value_delimiter = ',')]
        only: Vec<String>,

        /// Attempt to fix drift (interactive — prompts for confirmation per event).
        #[arg(long, short = 'f')]
        fix: bool,

        /// Output JSON instead of human-readable text.
        #[arg(long, short = 'j')]
        json: bool,

        /// Path to config.yaml (defaults to ./config.yaml).
        #[arg(long)]
        config: Option<std::path::PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();

    // TODO: Implement command handlers
    match cli.command {
        Commands::Init {
            repo,
            fix,
            json,
            github_token,
        } => {
            println!("Init command called for repo: {}", repo);
            if fix {
                println!("Running with fixes enabled");
            }
            if json {
                println!("JSON output requested");
            }
        }
        Commands::Doctor {
            verbose,
            only,
            fix,
            json,
            config,
        } => {
            println!("Doctor command called");
            if verbose {
                println!("Verbose mode");
            }
            if fix {
                println!("Running with fixes enabled");
            }
            if json {
                println!("JSON output requested");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parses_init() {
        let args = vec!["rogers", "init", "--repo", "owner/repo"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Init { repo, .. } => {
                assert_eq!(repo, "owner/repo");
            }
            _ => panic!("Expected Init command"),
        }
    }

    #[test]
    fn test_cli_parses_doctor() {
        let args = vec!["rogers", "doctor", "--verbose"];
        let cli = Cli::try_parse_from(args).unwrap();

        match cli.command {
            Commands::Doctor { verbose, .. } => {
                assert!(verbose);
            }
            _ => panic!("Expected Doctor command"),
        }
    }
}
