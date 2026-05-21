use rogers::cli::Cli;
use rogers::error::{Result, RogersError};

fn main() {
    if let Err(e) = run() {
        let exit_code = e.exit_code();
        eprintln!("Error: {}", e);

        // For auth and repo errors, provide actionable guidance.
        match &e {
            RogersError::Auth(_msg) | RogersError::Config(_msg) => {
                eprintln!("\nPlease check your configuration and try again.");
            }
            RogersError::RepoNotFound => {
                eprintln!("\nRepository not found or not accessible.");
                eprintln!("Verify the owner/repo format: 'rogers init --repo owner/repo'");
            }
            RogersError::GitHubStatus { code, message } => {
                if *code == 401 {
                    eprintln!(
                        "\nAuthentication failed. Provide a valid token with --github-token or set the GITHUB_TOKEN environment variable."
                    );
                } else if *code == 404 {
                    eprintln!("\nRepository not found or not accessible.");
                    eprintln!("Verify the owner/repo format: 'rogers init --repo owner/repo'");
                }
                eprintln!("GitHub API error: {}", message);
            }
            _ => {
                eprintln!("\nPlease check your configuration and try again.");
            }
        }

        std::process::exit(exit_code);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        rogers::cli::Commands::Init {
            repo,
            fix,
            json,
            github_token,
        } => {
            // Validate repo format early (before any API calls).
            let (owner, repo_name) = parse_repo(&repo)?;

            // Validate GITHUB_TOKEN early — provide clear error if missing.
            let token = resolve_github_token(&github_token)?;

            // Build client with resolved token.
            let client = rogers::github::GitHubClient::new(&token);

            // Run the full audit via an async runtime (main is synchronous).
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                RogersError::Config(format!("Failed to create async runtime: {}", e))
            })?;

            let result = rt.block_on(async {
                rogers::init::run_all_checks(&owner, &repo_name, &client).await
            });

            match result {
                Ok(check_results) => {
                    // Format and output the report.
                    if json {
                        let report = rogers::init::report::ReportFormatter::format_json(
                            &format!("{}/{}", owner, repo_name),
                            &check_results,
                            fix,
                        );
                        println!("{}", report);
                    } else {
                        let report = rogers::init::report::ReportFormatter::format_text(
                            &format!("{}/{}", owner, repo_name),
                            &check_results,
                            fix,
                        );
                        print!("{}", report);
                    }

                    // Determine exit code based on findings.
                    let has_blockers = check_results
                        .iter()
                        .any(|r| r.severity == rogers::checks::Severity::Blocker);

                    if has_blockers {
                        std::process::exit(1);
                    } else {
                        std::process::exit(0);
                    }
                }
                Err(e) => {
                    // Re-raise the error to the outer match where exit_code is applied.
                    Err(e)
                }
            }
        }
        rogers::cli::Commands::Doctor {
            verbose,
            only,
            fix,
            json: _,
            config,
        } => {
            println!("Doctor check (placeholder — not yet implemented)");
            if verbose {
                println!("Verbose mode: enabled");
            }
            if !only.is_empty() {
                println!("Categories: {}", only.join(", "));
            }
            if fix {
                println!("Fix mode: enabled");
            }
            if let Some(cfg) = config {
                println!("Config: {:?}", cfg);
            }
            Ok(())
        }
    }
}

/// Resolve the GitHub token from CLI flag or environment variable.
///
/// Returns an error if no token is available — the caller should exit
/// with code 3 (auth/repo error) for a clear, actionable message.
fn resolve_github_token(cli_token: &Option<String>) -> Result<String> {
    let token = match cli_token {
        Some(t) if !t.is_empty() => t.clone(),
        Some(_) => String::new(),
        None => std::env::var("GITHUB_TOKEN").unwrap_or_default(),
    };

    if token.is_empty() {
        Err(RogersError::Auth(
            "No GitHub token provided. Set the GITHUB_TOKEN environment variable or pass --github-token on the command line.".to_string(),
        ))
    } else {
        Ok(token)
    }
}

/// Parse a repository string in owner/repo format.
///
/// Returns an error with exit code 2 (config) if the format is invalid.
fn parse_repo(repo: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo.splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(RogersError::Config(format!(
            "Invalid repository format '{}'. Expected owner/repo",
            repo
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_repo_valid() {
        let (owner, repo) = parse_repo("owner/repo").unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(repo, "repo");
    }

    #[test]
    fn test_parse_repo_with_slash_in_name() {
        let (owner, repo) = parse_repo("rust-lang/rust").unwrap();
        assert_eq!(owner, "rust-lang");
        assert_eq!(repo, "rust");
    }

    #[test]
    fn test_parse_repo_no_slash() {
        let result = parse_repo("owner");
        assert!(result.is_err());
        match result.unwrap_err() {
            RogersError::Config(msg) => {
                assert!(msg.contains("Invalid repository format"));
            }
            _ => panic!("Expected Config error"),
        }
    }

    #[test]
    fn test_parse_repo_empty_parts() {
        let result = parse_repo("/repo");
        assert!(result.is_err());

        let result = parse_repo("owner/");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repo_empty_string() {
        let result = parse_repo("");
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_github_token_from_cli() {
        let token = resolve_github_token(&Some("ghp_test123".to_string())).unwrap();
        assert_eq!(token, "ghp_test123");
    }

    #[test]
    fn test_resolve_github_token_empty_cli_falls_through() {
        let result = resolve_github_token(&Some("".to_string()));
        // Should fail because empty string → no env var set (in test)
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_github_token_none_no_env() {
        // Save current value if any
        let saved = std::env::var("GITHUB_TOKEN").ok();
        // Clear it
        unsafe { std::env::remove_var("GITHUB_TOKEN") }

        let result = resolve_github_token(&None);

        // Restore
        if let Some(val) = saved {
            unsafe { std::env::set_var("GITHUB_TOKEN", val) };
        }

        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_github_token_from_env() {
        unsafe { std::env::set_var("GITHUB_TOKEN", "ghp_from_env") }

        let token = resolve_github_token(&None).unwrap();
        assert_eq!(token, "ghp_from_env");

        unsafe { std::env::remove_var("GITHUB_TOKEN") }
    }

    #[test]
    fn test_resolve_github_token_cli_overrides_env() {
        unsafe { std::env::set_var("GITHUB_TOKEN", "ghp_from_env") }

        let token = resolve_github_token(&Some("ghp_from_cli".to_string())).unwrap();
        assert_eq!(token, "ghp_from_cli");

        unsafe { std::env::remove_var("GITHUB_TOKEN") }
    }

    #[test]
    fn test_error_exit_codes() {
        use rogers::error::RogersError as E;
        assert_eq!(E::Config("test".into()).exit_code(), 2);
        assert_eq!(E::Auth("test".into()).exit_code(), 3);
        assert_eq!(E::RepoNotFound.exit_code(), 3);
        assert_eq!(
            E::GitHubStatus {
                code: 401,
                message: "bad".into()
            }
            .exit_code(),
            3
        );
    }
}
