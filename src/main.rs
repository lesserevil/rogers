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
            let _client = rogers::github::GitHubClient::new(&github_token.unwrap_or_default());

            // For now, just verify the client works
            // Full init implementation will come in follow-up beads
            println!("Repository: {}/{}", owner, repo_name);
            println!("Client initialized successfully");
            if fix {
                println!("Fix mode: enabled");
            }
            Ok(())
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
