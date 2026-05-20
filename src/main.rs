//! Rodgers binary entry point.

pub mod cli;

use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

fn main() -> rogers::Result<()> {
    init_tracing();

    // Stub CLI dispatch — real clap-based dispatch in cli::Cli::parse()
    eprintln!("NOTE: This binary is stubbed. Run `cargo test` for the actual logic.");
    eprintln!("The library modules (backport, triage, config, github) build correctly.");
    Ok(())
}

fn init_tracing() {
    let filter = match EnvFilter::try_from_default_env() {
        Ok(f) => f,
        Err(_) => EnvFilter::new("info"),
    };
    let _ = tracing_subscriber::registry()
        .with(fmt::layer().with_target(true))
        .with(filter)
        .try_init();
}
