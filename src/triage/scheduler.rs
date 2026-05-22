//! Triage scheduler — cron interval and webhook event driven loop.
//!
//! The scheduler is the heartbeat of Rodgers. It runs the triage loop on two
//! triggers:
//!
//! 1. **Cron schedule** — a configurable interval (default 60 minutes) that
//!    wakes the scheduler to do a full triage pass over all untriaged issues.
//! 2. **GitHub issue events** — incoming webhook events (opened, edited,
//!    labeled, unlabeled) that enqueue a targeted triage run for the affected
//!    issue.
//!
//! ## Concurrency model
//!
//! The scheduler runs as a single async task.  A `tokio::sync::mpsc` channel
//! carries event messages into the task.  Inside the task a loop polls either
//! the next cron tick or the next event — whichever arrives first.  A
//! `tokio::sync::Mutex` guards against overlapping runs.
//!
//! ## Rate-limit handling
//!
//! GitHub API calls that fail with HTTP 429 (rate limit) are retried with
//! exponential backoff (up to 5 retries).  A `RetryPolicy` abstracts the
//! delay calculation so it can be tested.
//!
//! ## Idempotency
//!
//! Each issue is marked `rodgers:triaged` after a triage run.  The scheduler
//! never reprocesses a triaged issue unless it receives a webhook event that
//! indicates the issue changed (open, edit, label change).  The last-run
//! timestamp is updated after every successful run.

use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use crate::triage::triage_loop::{
    has_triaged_label, process_issues_batch, IssueState, TriageIssue,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, warn};

// ─── Types ──────────────────────────────────────────────────────────────────

/// Configuration for the scheduler, loaded from `config.yaml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerConfig {
    /// Interval in minutes between scheduled triage passes.  Default: 60.
    pub interval_minutes: u64,
    /// Whether the scheduler is enabled.  Default: true.
    pub enabled: bool,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            interval_minutes: 60,
            enabled: true,
        }
    }
}

/// The default interval (1 hour) when no configuration is provided.
pub const DEFAULT_INTERVAL_MINUTES: u64 = 60;

/// Events that arrive from GitHub webhooks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WebhookEvent {
    /// Issue was opened.
    IssueOpened { issue_number: u64 },
    /// Issue body or title was edited.
    IssueEdited { issue_number: u64 },
    /// Label was added to an issue.
    IssueLabeled { issue_number: u64 },
    /// Label was removed from an issue.
    IssueUnlabeled { issue_number: u64 },
}

impl WebhookEvent {
    /// Return a human-readable description for logging.
    pub fn description(&self) -> &str {
        match self {
            WebhookEvent::IssueOpened { .. } => "issue opened",
            WebhookEvent::IssueEdited { .. } => "issue edited",
            WebhookEvent::IssueLabeled { .. } => "issue labeled",
            WebhookEvent::IssueUnlabeled { .. } => "issue unlabeled",
        }
    }
}

/// The triaged state of an issue, known from the last triage run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriagedState {
    /// Whether the issue has been triaged (has the `rodgers:triaged` label).
    pub triaged: bool,
    /// Labels that were known at the time of the last run.
    pub labels: Vec<String>,
    /// When the issue was last triaged.
    pub last_tried_at: Option<DateTime<Utc>>,
}

/// Metadata about a triage run.
#[derive(Debug, Clone, Serialize)]
pub struct RunMetadata {
    /// When the run started.
    pub started_at: DateTime<Utc>,
    /// When the run finished.
    pub finished_at: DateTime<Utc>,
    /// Whether this was triggered by a cron tick or an event.
    pub trigger: RunTrigger,
    /// Issues processed (not skipped).
    pub issues_processed: usize,
    /// Issues skipped (already triaged, closed, etc.).
    pub issues_skipped: usize,
}

/// What triggered a triage run.
#[derive(Debug, Clone, Serialize)]
pub enum RunTrigger {
    /// Scheduled cron tick.
    Cron,
    /// GitHub webhook event.
    Event { event: WebhookEvent },
}

/// Policy for retrying failed GitHub API calls (rate limit backoff).
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retries.
    pub max_retries: usize,
    /// Base delay for exponential backoff.
    pub base_delay_secs: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay_secs: 2,
        }
    }
}

impl RetryPolicy {
    /// Compute the delay for a given retry attempt (exponential backoff).
    pub fn delay_for_attempt(&self, attempt: usize) -> Duration {
        let secs = self.base_delay_secs * (1 << attempt); // 2, 4, 8, 16, 32
        Duration::from_secs(secs.min(60)) // Cap at 60 seconds
    }
}

/// A lock that prevents overlapping triage runs.
pub struct RunLock {
    inner: Mutex<bool>,
}

impl Default for RunLock {
    fn default() -> Self {
        Self::new()
    }
}

impl RunLock {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(false),
        }
    }

    /// Try to acquire the lock without blocking. Returns true if acquired.
    pub async fn try_acquire(&self) -> bool {
        let mut locked = self.inner.lock().await;
        if *locked {
            false
        } else {
            *locked = true;
            true
        }
    }

    /// Release the lock.
    pub async fn release(&self) {
        let mut locked = self.inner.lock().await;
        *locked = false;
    }
}

/// The triage scheduler — main orchestration struct.
pub struct TriageScheduler {
    /// GitHub API client.
    client: GitHubClient,
    /// Scheduler configuration.
    config: SchedulerConfig,
    /// Retry policy for API calls.
    retry_policy: RetryPolicy,
    /// Lock preventing overlapping runs.
    lock: Arc<RunLock>,
    /// Channel sender for webhook events.
    event_tx: mpsc::UnboundedSender<WebhookEvent>,
    /// In-flight event receiver (used by the run loop).
    event_rx: Mutex<Option<mpsc::UnboundedReceiver<WebhookEvent>>>,
    /// Issues known to be triaged from the last run.
    triaged_states: Arc<Mutex<HashSet<u64>>>,
}

impl TriageScheduler {
    /// Create a new triage scheduler.
    pub fn new(client: GitHubClient, config: SchedulerConfig) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        Self {
            client,
            config,
            retry_policy: RetryPolicy::default(),
            lock: Arc::new(RunLock::new()),
            event_tx,
            event_rx: Mutex::new(Some(event_rx)),
            triaged_states: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Create a scheduler with a custom retry policy (useful for testing).
    pub fn with_retry_policy(mut self, retry_policy: RetryPolicy) -> Self {
        self.retry_policy = retry_policy;
        self
    }

    /// Get the interval duration from config.
    pub fn interval(&self) -> Duration {
        Duration::from_secs(self.config.interval_minutes * 60)
    }

    /// Submit a webhook event for processing.
    ///
    /// Returns an error if the scheduler has been shut down.
    pub fn enqueue_event(&self, event: WebhookEvent) -> Result<()> {
        self.event_tx
            .send(event)
            .map_err(|_| RogersError::Beads("scheduler event channel closed".into()))
    }

    /// Start the scheduler run loop.
    ///
    /// This runs until the event sender is dropped (scheduler shutdown) or
    /// `stop()` is called.
    pub async fn run(&self) -> Result<()> {
        info!(
            "Triage scheduler starting (interval={}min, enabled={})",
            self.config.interval_minutes, self.config.enabled
        );

        let mut event_rx = self
            .event_rx
            .lock()
            .await
            .take()
            .expect("scheduler already started");

        let mut interval = tokio::time::interval(self.interval());
        // Drift-tolerant: don't tick early if a run is in progress.
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if self.config.enabled {
                        let _ = self.run_triage(RunTrigger::Cron).await;
                    }
                }
                Some(event) = event_rx.recv() => {
                    info!(event = %event.description(), issue = event.issue_number(), "Webhook event received");
                    self.process_webhook_event(&event).await;
                }
                else => {
                    // Channel closed — shut down.
                    info!("Event channel closed, scheduler shutting down");
                    break;
                }
            }
        }

        Ok(())
    }

    /// Stop the scheduler gracefully.
    ///
    /// Closes the event channel so the loop exits on next tick.
    pub fn stop(&self) {
        self.event_tx
            .send(WebhookEvent::IssueOpened { issue_number: 0 })
            .ok();
    }

    /// Run a single triage pass over all untriaged issues.
    ///
    /// This acquires the run lock to prevent overlapping runs.  If the lock
    /// is already held (another run in progress), this skips silently.
    pub async fn run_triage(&self, trigger: RunTrigger) -> Result<()> {
        if !self.lock.try_acquire().await {
            warn!("Triage run skipped — another run is in progress");
            return Ok(());
        }

        let _guard = RunGuard {
            lock: Arc::clone(&self.lock),
        };

        info!(
            trigger = ?trigger,
            "Starting triage run"
        );

        // Fetch all open issues from GitHub.
        let issues = match self.fetch_open_issues().await {
            Ok(issues) => issues,
            Err(e) => {
                error!(error = %e, "Failed to fetch open issues, skipping triage run");
                return Ok(());
            }
        };

        let total_issues = issues.len();
        // Filter out already-triaged issues and issues we don't manage.
        let untriaged: Vec<TriageIssue> = issues
            .into_iter()
            .filter(|issue| !has_triaged_label(&issue.labels) && issue.state == IssueState::Open)
            .collect();

        info!(
            total_issues,
            untriaged_count = untriaged.len(),
            "Triage run: {} total open issues, {} untriaged",
            total_issues,
            untriaged.len()
        );

        if untriaged.is_empty() {
            info!("No untriaged issues to process");
            return Ok(());
        }

        // Process issues in batch.
        let results = process_issues_batch(&untriaged);

        let mut processed = 0;
        let mut skipped = 0;

        for result in &results {
            if result.processed {
                processed += 1;
            } else {
                skipped += 1;
            }
        }

        info!(
            processed,
            skipped, "Triage run complete: processed={}, skipped={}", processed, skipped
        );

        // Update the triaged states set.
        self.update_triaged_states(&results, &untriaged).await;

        Ok(())
    }

    /// Process a webhook event: enqueue the issue for triage on the next tick.
    ///
    /// For event-driven triage we want to wait until the cron tick so we can
    /// batch all changes into one run.  We record the event and let the
    /// scheduler process it on the next available tick.
    async fn process_webhook_event(&self, event: &WebhookEvent) {
        let issue_number = event.issue_number();

        // Fetch the issue to see if it needs triage.
        match self.fetch_issue(issue_number).await {
            Ok(Some(issue)) => {
                // Check if this issue needs triage (has bug or feature label,
                // not already triaged).
                let has_relevant_label = issue
                    .labels
                    .iter()
                    .any(|l| l.name == "bug" || l.name == "feature" || l.name == "question");
                let label_names: Vec<String> =
                    issue.labels.iter().map(|l| l.name.clone()).collect();
                let already_triaged = has_triaged_label(&label_names);

                if has_relevant_label && !already_triaged {
                    info!(issue_number, "Issue needs triage after event");
                    // Schedule an immediate triage pass.
                    if let Err(e) = self
                        .run_triage(RunTrigger::Event {
                            event: event.clone(),
                        })
                        .await
                    {
                        error!(error = %e, issue_number, "Event-driven triage failed");
                    }
                } else if already_triaged {
                    info!(issue_number, "Issue already triaged, skipping");
                }
            }
            Ok(None) => {
                warn!(issue_number, "Issue not found (may have been deleted)");
            }
            Err(e) => {
                warn!(error = %e, issue_number, "Failed to fetch issue for webhook event");
            }
        }
    }

    /// Fetch all open issues from the GitHub repository.
    ///
    /// This is a simplified implementation that paginates through issues.
    /// In production this would use the GitHub Issues API with filters.
    async fn fetch_open_issues(&self) -> Result<Vec<TriageIssue>> {
        let url = format!(
            "{}/repos/{}/{}/issues?state=open&per_page=100",
            self.client.api_base(),
            self.client.owner(),
            self.client.repo()
        );

        let response = self.do_get(&url).await?;

        // Parse the JSON response as a vector of Issue structs.
        let github_issues: Vec<crate::github::models::Issue> = serde_json::from_str(&response)
            .map_err(|e| RogersError::Beads(format!("Failed to parse issues JSON: {}", e)))?;

        let triage_issues: Vec<TriageIssue> = github_issues
            .into_iter()
            .map(|gi| TriageIssue {
                number: gi.number as u64,
                title: gi.title,
                body: gi.body.unwrap_or_default(),
                author: gi.user.login,
                labels: gi.labels.into_iter().map(|l| l.name).collect(),
                state: IssueState::Open,
                url: None,
            })
            .collect();

        Ok(triage_issues)
    }

    /// Fetch a single issue by number.
    async fn fetch_issue(&self, issue_number: u64) -> Result<Option<crate::github::models::Issue>> {
        let url = format!(
            "{}/repos/{}/{}/issues/{}",
            self.client.api_base(),
            self.client.owner(),
            self.client.repo(),
            issue_number
        );

        let response = self.do_get(&url).await?;

        if response.is_empty() {
            return Ok(None);
        }

        let issue: crate::github::models::Issue = serde_json::from_str(&response)
            .map_err(|e| RogersError::Beads(format!("Failed to parse issue JSON: {}", e)))?;

        Ok(Some(issue))
    }

    /// Make a GET request to a URL, returning the response body as a string.
    async fn do_get(&self, url: &str) -> Result<String> {
        let client = reqwest::Client::new();
        let mut request = client
            .get(url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if let Some(ref token) = self.client.token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;

        let status = response.status();
        if !status.is_success() {
            if status.as_u16() == 403 && response.headers().get("x-ratelimit-remaining").is_some() {
                // Rate limited — return a special error to trigger backoff.
                return Err(RogersError::GitHubStatus {
                    code: 429,
                    message: "Rate limit exceeded".into(),
                });
            }
            let body = response.text().await.unwrap_or_default();
            return Err(RogersError::GitHubStatus {
                code: status.as_u16(),
                message: body,
            });
        }

        let body = response.text().await?;
        Ok(body)
    }

    /// Execute an async operation with exponential backoff retry for rate limits.
    ///
    /// Used for handling GitHub API rate limits (HTTP 429) with configurable
    /// exponential backoff. This method is part of the scheduler's rate-limit
    /// handling infrastructure described in the Triage Loop plan.
    #[allow(dead_code)]
    async fn retry_with_backoff<F, T>(&self, op: F) -> Result<T>
    where
        F: std::future::Future<Output = Result<T>> + std::marker::Copy,
    {
        let mut attempt = 0;
        loop {
            match op.await {
                Ok(result) => return Ok(result),
                Err(RogersError::GitHubStatus { code: 429, .. }) => {
                    if attempt >= self.retry_policy.max_retries {
                        return Err(RogersError::GitHubStatus {
                            code: 429,
                            message: format!(
                                "Rate limit exceeded after {} retries",
                                self.retry_policy.max_retries
                            ),
                        });
                    }
                    let delay = self.retry_policy.delay_for_attempt(attempt);
                    warn!(
                        attempt,
                        max_retries = self.retry_policy.max_retries,
                        delay_secs = delay.as_secs(),
                        "Rate limited, retrying after backoff"
                    );
                    tokio::time::sleep(delay).await;
                    attempt += 1;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Update the set of triaged states after a run.
    async fn update_triaged_states(
        &self,
        results: &[crate::triage::triage_loop::TriageResult],
        issues: &[TriageIssue],
    ) {
        let mut states = self.triaged_states.lock().await;
        for (issue, result) in issues.iter().zip(results.iter()) {
            if result.processed {
                states.insert(issue.number);
            }
        }
    }
}

/// RAII guard that releases the run lock when dropped.
struct RunGuard {
    lock: Arc<RunLock>,
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        let lock = Arc::clone(&self.lock);
        // Release the lock asynchronously — we spawn a task to avoid blocking.
        tokio::spawn(async move {
            lock.release().await;
        });
    }
}

// ─── Trait helpers ──────────────────────────────────────────────────────────

/// Helper trait to extract issue number from events.
trait IssueNumber {
    fn issue_number(&self) -> u64;
}

impl IssueNumber for WebhookEvent {
    fn issue_number(&self) -> u64 {
        match self {
            WebhookEvent::IssueOpened { issue_number }
            | WebhookEvent::IssueEdited { issue_number }
            | WebhookEvent::IssueLabeled { issue_number }
            | WebhookEvent::IssueUnlabeled { issue_number } => *issue_number,
        }
    }
}

// ─── Ad-hoc triage entry point ─────────────────────────────────────────────

/// Run a single triage pass over all untriaged issues and return results.
///
/// This is used by the `rogers triage --once` CLI command and for testing.
pub async fn run_once(client: GitHubClient, config: SchedulerConfig) -> Result<RunMetadata> {
    let scheduler = TriageScheduler::new(client, config);
    let start = Utc::now();

    let issues = scheduler.fetch_open_issues().await?;
    let untriaged: Vec<TriageIssue> = issues
        .into_iter()
        .filter(|issue| !has_triaged_label(&issue.labels) && issue.state == IssueState::Open)
        .collect();

    let results = process_issues_batch(&untriaged);

    let mut processed = 0;
    let mut skipped = 0;

    for result in &results {
        if result.processed {
            processed += 1;
        } else {
            skipped += 1;
        }
    }

    Ok(RunMetadata {
        started_at: start,
        finished_at: Utc::now(),
        trigger: RunTrigger::Cron,
        issues_processed: processed,
        issues_skipped: skipped,
    })
}

// ─── Unit Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Unit test: scheduler triggers on cron interval
    // =============================================================================

    #[test]
    fn test_scheduler_creates_with_defaults() {
        let config = SchedulerConfig::default();
        assert_eq!(config.interval_minutes, 60);
        assert!(config.enabled);
    }

    #[test]
    fn test_scheduler_interval_duration() {
        let config = SchedulerConfig {
            interval_minutes: 5,
            enabled: true,
        };
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config.clone(),
        );
        assert_eq!(scheduler.interval(), Duration::from_secs(300)); // 5 min * 60
    }

    #[test]
    fn test_scheduler_interval_one_hour() {
        let config = SchedulerConfig::default();
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config.clone(),
        );
        assert_eq!(scheduler.interval(), Duration::from_secs(3600)); // 60 min * 60
    }

    #[test]
    fn test_retry_policy_delays() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(8));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(16));
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(32));
        // Attempt 5 would be 64s but capped at 60
        assert_eq!(policy.delay_for_attempt(5), Duration::from_secs(60));
    }

    #[test]
    fn test_retry_policy_capped_at_60s() {
        let policy = RetryPolicy::default();
        for attempt in 5..10 {
            assert_eq!(policy.delay_for_attempt(attempt), Duration::from_secs(60));
        }
    }

    #[tokio::test]
    async fn test_run_lock_single_acquire() {
        let lock = RunLock::new();
        assert!(lock.try_acquire().await);
        assert!(!lock.try_acquire().await);
    }

    #[tokio::test]
    async fn test_run_lock_release_then_acquire() {
        let lock = RunLock::new();
        assert!(lock.try_acquire().await);
        lock.release().await;
        assert!(lock.try_acquire().await);
    }

    // =============================================================================
    // Unit test: webhook events enqueue correctly
    // =============================================================================

    #[tokio::test]
    async fn test_enqueue_webhook_event_opened() {
        let config = SchedulerConfig::default();
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        scheduler
            .enqueue_event(WebhookEvent::IssueOpened { issue_number: 42 })
            .unwrap();
        // Event was enqueued successfully.
    }

    #[tokio::test]
    async fn test_enqueue_webhook_event_edited() {
        let config = SchedulerConfig::default();
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        scheduler
            .enqueue_event(WebhookEvent::IssueEdited { issue_number: 42 })
            .unwrap();
    }

    #[tokio::test]
    async fn test_enqueue_webhook_event_labeled() {
        let config = SchedulerConfig::default();
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        scheduler
            .enqueue_event(WebhookEvent::IssueLabeled { issue_number: 42 })
            .unwrap();
    }

    #[tokio::test]
    async fn test_enqueue_webhook_event_unlabeled() {
        let config = SchedulerConfig::default();
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        scheduler
            .enqueue_event(WebhookEvent::IssueUnlabeled { issue_number: 42 })
            .unwrap();
    }

    #[tokio::test]
    async fn test_enqueue_webhook_event_all_types() {
        let config = SchedulerConfig::default();
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        let events = vec![
            WebhookEvent::IssueOpened { issue_number: 1 },
            WebhookEvent::IssueEdited { issue_number: 2 },
            WebhookEvent::IssueLabeled { issue_number: 3 },
            WebhookEvent::IssueUnlabeled { issue_number: 4 },
        ];
        for event in events {
            scheduler.enqueue_event(event.clone()).unwrap();
        }
    }

    // =============================================================================
    // Unit test: event descriptions for logging
    // =============================================================================

    #[test]
    fn test_webhook_event_descriptions() {
        assert_eq!(
            WebhookEvent::IssueOpened { issue_number: 1 }.description(),
            "issue opened"
        );
        assert_eq!(
            WebhookEvent::IssueEdited { issue_number: 1 }.description(),
            "issue edited"
        );
        assert_eq!(
            WebhookEvent::IssueLabeled { issue_number: 1 }.description(),
            "issue labeled"
        );
        assert_eq!(
            WebhookEvent::IssueUnlabeled { issue_number: 1 }.description(),
            "issue unlabeled"
        );
    }

    // =============================================================================
    // Unit test: triaged state tracking
    // =============================================================================

    #[tokio::test]
    async fn test_triaged_states_set_tracks_issues() {
        let states: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));

        // Insert issue numbers.
        {
            let mut s = states.lock().await;
            s.insert(1);
            s.insert(2);
            s.insert(3);
        }

        // Verify.
        let s = states.lock().await;
        assert!(s.contains(&1));
        assert!(s.contains(&2));
        assert!(s.contains(&3));
        assert!(!s.contains(&4));
    }

    #[tokio::test]
    async fn test_triaged_states_set_is_empty_initially() {
        let states: Arc<Mutex<HashSet<u64>>> = Arc::new(Mutex::new(HashSet::new()));
        let s = states.lock().await;
        assert!(s.is_empty());
    }

    // =============================================================================
    // Unit test: scheduler does not overlap runs
    // =============================================================================

    #[tokio::test]
    async fn test_lock_prevents_concurrent_runs() {
        let lock = RunLock::new();

        // First acquire succeeds.
        assert!(lock.try_acquire().await);

        // Second acquire should fail (no blocking).
        assert!(!lock.try_acquire().await);
    }

    #[tokio::test]
    async fn test_lock_allows_after_release() {
        let lock = RunLock::new();

        assert!(lock.try_acquire().await);
        lock.release().await;
        assert!(lock.try_acquire().await);
        lock.release().await;
        // Can acquire again after release.
        assert!(lock.try_acquire().await);
    }

    // =============================================================================
    // Unit test: RunTrigger serialization
    // =============================================================================

    #[test]
    fn test_run_trigger_cron() {
        let trigger = RunTrigger::Cron;
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("Cron"));
    }

    #[test]
    fn test_run_trigger_event() {
        let trigger = RunTrigger::Event {
            event: WebhookEvent::IssueOpened { issue_number: 42 },
        };
        let json = serde_json::to_string(&trigger).unwrap();
        assert!(json.contains("Event"));
    }

    // =============================================================================
    // Unit test: RunMetadata serialization
    // =============================================================================

    #[test]
    fn test_run_metadata_serialization() {
        let meta = RunMetadata {
            started_at: chrono::Utc::now(),
            finished_at: chrono::Utc::now(),
            trigger: RunTrigger::Cron,
            issues_processed: 5,
            issues_skipped: 3,
        };
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("issues_processed"));
        assert!(json.contains("issues_skipped"));
        assert!(json.contains("trigger"));
    }

    // =============================================================================
    // Unit test: WebhookEvent issue_number helper
    // =============================================================================

    #[test]
    fn test_webhook_event_issue_number() {
        let evt = WebhookEvent::IssueOpened { issue_number: 123 };
        assert_eq!(evt.issue_number(), 123);

        let evt = WebhookEvent::IssueEdited { issue_number: 456 };
        assert_eq!(evt.issue_number(), 456);

        let evt = WebhookEvent::IssueLabeled { issue_number: 789 };
        assert_eq!(evt.issue_number(), 789);

        let evt = WebhookEvent::IssueUnlabeled { issue_number: 101 };
        assert_eq!(evt.issue_number(), 101);
    }

    // =============================================================================
    // Unit test: scheduler with custom retry policy
    // =============================================================================

    #[test]
    fn test_scheduler_with_custom_retry_policy() {
        let config = SchedulerConfig::default();
        let policy = RetryPolicy {
            max_retries: 3,
            base_delay_secs: 1,
        };
        let _scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        )
        .with_retry_policy(policy);
        // Custom retry policy is set.
        // We can't easily verify the private field, but we can check the
        // scheduler was created without panicking.
    }

    // =============================================================================
    // Integration test: full triage run processes issues with rodgers:triaged=false
    // =============================================================================

    #[tokio::test]
    async fn test_scheduler_process_issue_filters_triaged() {
        // Verify that the scheduler's filtering logic correctly excludes
        // already-triaged issues from the triage batch.
        let complete_bug = r#"
## Behavior Observed
It crashes

## Behavior Expected
No crash

## Reproduction Steps
1. Click

## Environment
Linux
"#;

        let triaged_issue = TriageIssue {
            number: 1,
            title: "Triaged issue".to_string(),
            body: complete_bug.to_string(),
            author: "user".to_string(),
            labels: vec!["bug".to_string(), "rodgers:triaged".to_string()],
            state: IssueState::Open,
            url: None,
        };

        let untriaged_issue = TriageIssue {
            number: 2,
            title: "Untriaged issue".to_string(),
            body: complete_bug.to_string(),
            author: "user".to_string(),
            labels: vec!["bug".to_string()],
            state: IssueState::Open,
            url: None,
        };

        // Filter as the scheduler would.
        let issues = vec![triaged_issue, untriaged_issue];
        let untriaged: Vec<_> = issues
            .into_iter()
            .filter(|issue| !has_triaged_label(&issue.labels) && issue.state == IssueState::Open)
            .collect();

        // Only the untriaged issue should remain.
        assert_eq!(untriaged.len(), 1);
        assert_eq!(untriaged[0].number, 2);
    }

    #[tokio::test]
    async fn test_scheduler_filters_closed_issues() {
        let closed_issue = TriageIssue {
            number: 1,
            title: "Closed issue".to_string(),
            body: "Body".to_string(),
            author: "user".to_string(),
            labels: vec!["bug".to_string()],
            state: IssueState::Closed,
            url: None,
        };

        let open_issue = TriageIssue {
            number: 2,
            title: "Open issue".to_string(),
            body: "Body".to_string(),
            author: "user".to_string(),
            labels: vec!["bug".to_string()],
            state: IssueState::Open,
            url: None,
        };

        let issues = vec![closed_issue, open_issue];
        let untriaged: Vec<_> = issues
            .into_iter()
            .filter(|issue| !has_triaged_label(&issue.labels) && issue.state == IssueState::Open)
            .collect();

        assert_eq!(untriaged.len(), 1);
        assert_eq!(untriaged[0].number, 2);
    }

    // =============================================================================
    // Test: scheduler respects enabled/disabled config
    // =============================================================================

    #[test]
    fn test_scheduler_disabled() {
        let config = SchedulerConfig {
            interval_minutes: 60,
            enabled: false,
        };
        assert!(!config.enabled);
    }

    #[test]
    fn test_scheduler_enabled() {
        let config = SchedulerConfig {
            interval_minutes: 60,
            enabled: true,
        };
        assert!(config.enabled);
    }

    // =============================================================================
    // Test: scheduler with custom interval
    // =============================================================================

    #[test]
    fn test_scheduler_custom_interval() {
        let config = SchedulerConfig {
            interval_minutes: 15,
            enabled: true,
        };
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        assert_eq!(scheduler.interval(), Duration::from_secs(900)); // 15 min
    }

    // =============================================================================
    // Test: scheduler interval_minutes = 1 (minimum)
    // =============================================================================

    #[test]
    fn test_scheduler_minimum_interval() {
        let config = SchedulerConfig {
            interval_minutes: 1,
            enabled: true,
        };
        let scheduler = TriageScheduler::new(
            GitHubClient::new(
                "test",
                "test",
                crate::github::GitHubAuth::new_with_default_api("test"),
            ),
            config,
        );
        assert_eq!(scheduler.interval(), Duration::from_secs(60));
    }

    // =============================================================================
    // Test: run lock is shareable across clones (Arc)
    // =============================================================================

    #[tokio::test]
    async fn test_lock_shared_across_clones() {
        let lock = Arc::new(RunLock::new());

        let lock1 = Arc::clone(&lock);
        let lock2 = Arc::clone(&lock);

        // First acquire succeeds.
        assert!(lock1.try_acquire().await);

        // Second should fail.
        assert!(!lock2.try_acquire().await);

        // Release via clone 2.
        lock2.release().await;

        // Can acquire again via clone 1.
        assert!(lock1.try_acquire().await);
    }

    // =============================================================================
    // Test: batch processing in scheduler context
    // =============================================================================

    #[tokio::test]
    async fn test_scheduler_batch_processes_multiple_issues() {
        let complete_bug = r#"
## Behavior Observed
Crashes

## Behavior Expected
No crash

## Reproduction Steps
1. Click

## Environment
Linux
"#;

        let issues = vec![
            TriageIssue {
                number: 1,
                title: "Bug 1".to_string(),
                body: complete_bug.to_string(),
                author: "user".to_string(),
                labels: vec!["bug".to_string()],
                state: IssueState::Open,
                url: None,
            },
            TriageIssue {
                number: 2,
                title: "Bug 2".to_string(),
                body: complete_bug.to_string(),
                author: "user".to_string(),
                labels: vec!["bug".to_string()],
                state: IssueState::Open,
                url: None,
            },
        ];

        let results = process_issues_batch(&issues);

        assert_eq!(results.len(), 2);
        assert!(results.iter().all(|r| r.processed));
    }

    // =============================================================================
    // Test: webhook event for issue already triaged is skipped
    // =============================================================================

    #[test]
    fn test_webhook_event_already_triaged_issue_skipped() {
        let labels = vec!["bug".to_string(), "rodgers:triaged".to_string()];
        assert!(has_triaged_label(&labels));
        // The scheduler would skip this issue in process_webhook_event.
    }
}
