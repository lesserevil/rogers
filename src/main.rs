use rogers::cli::Cli;
use rogers::error::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        rogers::cli::Commands::Init {
            repo,
            fix,
            json: _,
            github_token,
        } => {
            let (owner, repo_name) = parse_repo(&repo)?;
            let token = github_token.unwrap_or_default();
            let client = rogers::github::GitHubClient::new(&token);

            // Run init (async via tokio block_on since main is sync).
            let rt = tokio::runtime::Runtime::new().map_err(|e| {
                rogers::error::RogersError::Config(format!("Failed to create runtime: {}", e))
            })?;

            let result = rt
                .block_on(async { rogers::init::run_init(&owner, &repo_name, fix, &client).await });

            match result {
                Ok(_) => {
                    println!("Init check complete.");
                    if fix {
                        println!("Fix mode: completed");
                    }
                    Ok(())
                }
                Err(e) => {
                    println!("Init check failed: {}", e);
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

fn parse_repo(repo: &str) -> Result<(String, String)> {
    let parts: Vec<&str> = repo.splitn(2, '/').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(rogers::error::RogersError::Config(format!(
            "Invalid repository format '{}'. Expected owner/repo",
            repo
        )));
    }
    Ok((parts[0].to_string(), parts[1].to_string()))
}
