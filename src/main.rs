//! Rodgers — github-native community relations agent.
//!
//! Rodgers runs on a schedule, reads GitHub issues, and manages the full
//! triage-to-release lifecycle through the GitHub API and beads database.

mod error;
mod github;
mod labels;

pub mod llm;
pub mod question_router;

fn main() {
    println!("Rogers - github-native community relations agent");
}
