//! Configuration schema for Rodgers.
//!
//! Loads and validates `config.yaml` (host-level) and optionally merges
//! `rogers.yaml` from the managed repository's default branch. Repo-level
//! overrides host-level for any overlapping keys.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::OnceLock;

use crate::RogersError;

// ---------------------------------------------------------------------------
// Top-level config
// ---------------------------------------------------------------------------

/// Root configuration document.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Config {
    pub github: GithubConfig,
    pub scheduler: SchedulerConfig,
    pub beads: BeadsConfig,
    pub llm: LlmConfig,
    pub triage: TriageConfig,
    pub release: ReleaseConfig,
    pub rogation: RogationConfig,
    pub log_level: LogLevel,

    /// Slack channel ID for error notifications (optional).
    #[serde(rename = "error_channel")]
    pub error_channel: Option<String>,
}

impl Config {
    /// Load `config.yaml` from the given path.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, RogersError> {
        let contents = std::fs::read_to_string(path.as_ref())?;
        let expanded = expand_env_vars(&contents);
        serde_yaml::from_str(&expanded).map_err(|e| RogersError::Config(e.to_string()))
    }

    /// Merge a repo-level `rogers.yaml` document on top of this config.
    /// Repo values take precedence.
    pub fn merge_repo_config(&mut self, repo: RepoConfig) {
        if let Some(v) = repo.github {
            if let Some(owner) = v.owner {
                self.github.owner = owner;
            }
            if let Some(repo_name) = v.repo {
                self.github.repo = repo_name;
            }
            if let Some(api_url) = v.api_url {
                self.github.api_url = api_url;
            }
        }

        if let Some(scheduling) = repo.scheduler {
            if let Some(enabled) = scheduling.enabled {
                self.scheduler.enabled = enabled;
            }
            if let Some(interval) = scheduling.interval_minutes {
                self.scheduler.interval_minutes = interval;
            }
        }

        if let Some(ref triage) = repo.triage {
            if let Some(ref assignees) = triage.assignees {
                if !assignees.is_empty() {
                    self.triage.assignees = assignees.clone();
                }
            }
        }

        if let Some(release_cfg) = repo.release {
            if let Some(active) = release_cfg.active_branches {
                self.release.active_branches = active;
            }
            if let Some(cat) = release_cfg.approval_discussion_category {
                self.release.approval_discussion_category = cat;
            }
            if let Some(window) = release_cfg.voting_window_days {
                self.release.voting_window_days = window;
            }
        }

        if let Some(project) = repo.project {
            if let Some(ref ignore_labels) = project.ignore_labels {
                if !ignore_labels.is_empty() {
                    self.rogation.ignore_labels = ignore_labels.clone();
                }
            }
            if let Some(ref labels_never_bot_managed) = project.labels_never_bot_managed {
                if !labels_never_bot_managed.is_empty() {
                    self.rogation.labels_never_bot_managed = labels_never_bot_managed.clone();
                }
            }
            if let Some(ref custom_type_names) = project.custom_type_names {
                if !custom_type_names.is_empty() {
                    self.rogation.custom_type_names = custom_type_names.clone();
                }
            }
            if let Some(security_label) = project.security_label {
                self.rogation.security_label = security_label;
            }
            if let Some(format_cfg) = project.format {
                self.rogation.format = Some(format_cfg);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Env-var injection
// ---------------------------------------------------------------------------

fn expand_env_vars(contents: &str) -> String {
    let re =
        regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("hardcoded regex is valid");
    re.replace_all(contents, |caps: &regex::Captures| {
        std::env::var(&caps[1]).unwrap_or_default()
    })
    .to_string()
}

// ---------------------------------------------------------------------------
// Sub-configs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GithubConfig {
    pub owner: String,
    pub repo: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default = "default_github_api_url")]
    pub api_url: String,
}

fn default_github_api_url() -> String {
    "https://api.github.com".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SchedulerConfig {
    #[serde(default = "default_interval")]
    pub interval_minutes: u32,
    #[serde(default = "default_scheduler_enabled")]
    pub enabled: bool,
}

fn default_interval() -> u32 {
    15
}

fn default_scheduler_enabled() -> bool {
    true
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_minutes: default_interval(),
            enabled: default_scheduler_enabled(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BeadsConfig {
    /// Dolt remote URL for bead storage.
    #[serde(default)]
    pub remote: Option<String>,
    /// Dolt database name.
    #[serde(default = "default_database")]
    pub database: String,
}

fn default_database() -> String {
    "message.hibernate".to_string()
}

impl Default for BeadsConfig {
    fn default() -> Self {
        Self {
            remote: None,
            database: default_database(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct LlmConfig {
    #[serde(default = "default_llm_provider")]
    pub provider: String,
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
}

fn default_llm_provider() -> String {
    "openai".to_string()
}

fn default_llm_base_url() -> String {
    "https://api.openai.com/v1".to_string()
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            provider: default_llm_provider(),
            base_url: default_llm_base_url(),
            model: String::new(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TriageConfig {
    #[serde(default)]
    pub default_labels: Vec<String>,
    #[serde(default)]
    pub bot_labels: Vec<String>,
    #[serde(default = "default_close_labels")]
    pub close_labels: Vec<String>,
    #[serde(default)]
    pub assignees: Vec<String>,
}

fn default_close_labels() -> Vec<String> {
    vec![
        "wontfix".to_string(),
        "duplicate".to_string(),
        "not planned".to_string(),
    ]
}

impl Default for TriageConfig {
    fn default() -> Self {
        Self {
            default_labels: vec![
                "bug".to_string(),
                "enhancement".to_string(),
                "question".to_string(),
            ],
            bot_labels: Vec::new(),
            close_labels: default_close_labels(),
            assignees: Vec::new(),
        }
    }
}

/// Release management configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ReleaseConfig {
    /// GitHub Discussion category for Rodgers' release and backport proposals.
    #[serde(default = "default_discussion_category")]
    pub approval_discussion_category: String,

    /// Release branches Rodgers tracks for backport evaluation.
    /// E.g., `["release/1.x", "release/2.x"]`. Main is always implicit.
    #[serde(default)]
    pub active_branches: Vec<String>,

    /// Days Rodgers waits before nudging a stale release proposal.
    #[serde(default = "default_voting_window")]
    pub voting_window_days: u32,

    /// Days before Rodgers closes a stale release proposal.
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold_days: u32,
}

fn default_discussion_category() -> String {
    "Announcements".to_string()
}

fn default_voting_window() -> u32 {
    2
}

fn default_stale_threshold() -> u32 {
    7
}

impl Default for ReleaseConfig {
    fn default() -> Self {
        Self {
            approval_discussion_category: default_discussion_category(),
            active_branches: Vec::new(),
            voting_window_days: default_voting_window(),
            stale_threshold_days: default_stale_threshold(),
        }
    }
}

/// Rogation (repo-level) configuration mirroring `rogers.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RogationConfig {
    /// Labels that suppress Rodgers' processing entirely.
    #[serde(default)]
    pub ignore_labels: Vec<String>,

    /// Labels Rodgers will never manage — humans own these exclusively.
    #[serde(default)]
    pub labels_never_bot_managed: Vec<String>,

    /// Project-specific bead type aliases.
    #[serde(default)]
    pub custom_type_names: std::collections::HashMap<String, String>,

    /// Project-specific bead description format.
    #[serde(default)]
    pub format: Option<String>,

    /// Explicit path to per-project agent instructions.
    #[serde(default)]
    pub agent_file: Option<String>,

    /// Path to per-project issue templates.
    #[serde(default)]
    pub template_dir: Option<String>,

    /// Label Rodgers checks when detecting security patches.
    #[serde(default = "default_security_label")]
    pub security_label: String,
}

fn default_security_label() -> String {
    "security".to_string()
}

impl Default for RogationConfig {
    fn default() -> Self {
        Self {
            ignore_labels: Vec::new(),
            labels_never_bot_managed: Vec::new(),
            custom_type_names: std::collections::HashMap::new(),
            format: None,
            agent_file: None,
            template_dir: None,
            security_label: default_security_label(),
        }
    }
}

// ---------------------------------------------------------------------------
// Repo-level (rogers.yaml) config — subset that can be overridden
// ---------------------------------------------------------------------------

/// Subset of Config that can be overridden from `rogers.yaml` in the managed repo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepoConfig {
    #[serde(default)]
    pub github: Option<RepoGithubConfig>,
    #[serde(default)]
    pub scheduler: Option<RepoSchedulerConfig>,
    #[serde(default)]
    pub triage: Option<RepoTriageConfig>,
    #[serde(default)]
    pub release: Option<RepoReleaseConfig>,
    #[serde(default)]
    pub project: Option<RepoProjectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepoGithubConfig {
    pub owner: Option<String>,
    pub repo: Option<String>,
    pub api_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepoSchedulerConfig {
    pub enabled: Option<bool>,
    pub interval_minutes: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepoTriageConfig {
    pub assignees: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepoReleaseConfig {
    pub active_branches: Option<Vec<String>>,
    pub approval_discussion_category: Option<String>,
    /// Override release.voting_window_days from the managed repo's rogerg.yaml.
    /// Days Rodgers waits before nudging a stale release proposal.
    pub voting_window_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct RepoProjectConfig {
    pub ignore_labels: Option<Vec<String>>,
    pub labels_never_bot_managed: Option<Vec<String>>,
    pub custom_type_names: Option<std::collections::HashMap<String, String>>,
    pub format: Option<String>,
    pub agent_file: Option<String>,
    pub template_dir: Option<String>,
    pub security_label: Option<String>,
}

// ---------------------------------------------------------------------------
// Log level
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "debug"),
            LogLevel::Info => write!(f, "info"),
            LogLevel::Warn => write!(f, "warn"),
            LogLevel::Error => write!(f, "error"),
        }
    }
}

// ---------------------------------------------------------------------------
// Global config singleton (loaded once per process)
// ---------------------------------------------------------------------------

static CONFIG: OnceLock<Config> = OnceLock::new();

/// Load and store the global config. Safe to call multiple times.
pub fn init_config(path: impl AsRef<Path>) -> Result<&'static Config, RogersError> {
    let config = Config::load(path)?;
    CONFIG
        .set(config)
        .map_err(|_| RogersError::Config("config already initialized".to_string()))?;
    Ok(CONFIG.get().unwrap())
}

/// Returns the globally loaded config, if initialized.
pub fn config() -> Option<&'static Config> {
    CONFIG.get()
}
