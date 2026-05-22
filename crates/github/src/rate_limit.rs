//! GitHub API rate limit handling with exponential backoff.
//!
//! This module implements rate limit detection, warning generation,
//! and automatic retry with exponential backoff.

use std::time::Duration;

use crate::models::RateLimitResponse;
use rogers_core::error::{Result, RogersError};

/// Default warning threshold for remaining API calls.
pub const DEFAULT_WARNING_THRESHOLD: i32 = 100;

/// Default maximum retry attempts.
pub const DEFAULT_MAX_RETRIES: u32 = 3;

/// Default base delay for exponential backoff (in seconds).
pub const DEFAULT_BASE_DELAY_SECS: u64 = 1;

/// Default maximum delay for exponential backoff (in seconds).
pub const DEFAULT_MAX_DELAY_SECS: u64 = 60;

/// Rate limit handler for GitHub API requests.
#[derive(Debug, Clone)]
pub struct RateLimitHandler {
    /// Current remaining API calls (updated from responses).
    remaining: i32,
    /// Unix timestamp when the rate limit resets.
    reset_at: i64,
    /// Threshold below which to warn about low remaining calls.
    warning_threshold: i32,
    /// Maximum number of retry attempts.
    max_retries: u32,
    /// Base delay for exponential backoff.
    base_delay: Duration,
    /// Maximum delay for exponential backoff.
    max_delay: Duration,
    /// Whether to respect the retry_after header.
    respect_retry_after: bool,
}

impl RateLimitHandler {
    /// Create a new RateLimitHandler with default settings.
    pub fn new() -> Self {
        Self {
            remaining: 5000, // Default GitHub API rate limit
            reset_at: 0,
            warning_threshold: DEFAULT_WARNING_THRESHOLD,
            max_retries: DEFAULT_MAX_RETRIES,
            base_delay: Duration::from_secs(DEFAULT_BASE_DELAY_SECS),
            max_delay: Duration::from_secs(DEFAULT_MAX_DELAY_SECS),
            respect_retry_after: true,
        }
    }

    /// Create a RateLimitHandler with custom settings.
    pub fn with_config(
        warning_threshold: i32,
        max_retries: u32,
        base_delay_secs: u64,
        max_delay_secs: u64,
    ) -> Self {
        Self {
            remaining: 5000,
            reset_at: 0,
            warning_threshold,
            max_retries,
            base_delay: Duration::from_secs(base_delay_secs),
            max_delay: Duration::from_secs(max_delay_secs),
            respect_retry_after: true,
        }
    }

    /// Get the current remaining API calls.
    pub fn remaining(&self) -> i32 {
        self.remaining
    }

    /// Get the reset timestamp.
    pub fn reset_at(&self) -> i64 {
        self.reset_at
    }

    /// Get the maximum retry attempts.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Check if we should warn about low remaining calls.
    pub fn should_warn(&self) -> bool {
        self.remaining < self.warning_threshold && self.remaining > 0
    }

    /// Get the warning message if applicable.
    pub fn get_warning_message(&self) -> Option<String> {
        if self.should_warn() {
            Some(format!(
                "GitHub API rate limit low: {} remaining out of 5000. Will reset at {}.",
                self.remaining,
                format_reset_time(self.reset_at)
            ))
        } else {
            None
        }
    }

    /// Update rate limit info from a response.
    pub fn update_from_response(&mut self, response: &RateLimitResponse) {
        self.remaining = response.resources.core.remaining;
        self.reset_at = response.resources.core.reset;
    }

    /// Update remaining count directly (for when rate limit headers are present).
    pub fn update_from_headers(&mut self, remaining: i32, reset: i64) {
        self.remaining = remaining;
        self.reset_at = reset;
    }

    /// Check if we've exceeded retries.
    pub fn should_retry(&self, attempt: u32) -> bool {
        attempt < self.max_retries
    }

    /// Calculate the delay for a given retry attempt.
    ///
    /// Uses exponential backoff: base_delay * 2^attempt, capped at max_delay.
    /// If respect_retry_after is enabled, this can be overridden by a specific delay.
    pub fn calculate_delay(&self, attempt: u32, retry_after_secs: Option<u64>) -> Duration {
        // If we have a specific retry_after, use it (with cap)
        if self.respect_retry_after {
            if let Some(secs) = retry_after_secs {
                let requested = Duration::from_secs(secs);
                if requested < self.max_delay {
                    return requested;
                }
                return self.max_delay;
            }
        }

        // Exponential backoff: base_delay * 2^attempt
        let exponential = self.base_delay * 2u32.pow(attempt);
        exponential.min(self.max_delay)
    }

    /// Calculate delay based on reset time.
    ///
    /// If the reset time is in the future, returns the delay until reset.
    /// Otherwise, falls back to exponential backoff.
    pub fn calculate_delay_until_reset(&self, attempt: u32) -> Duration {
        // If we know when the reset is, use that
        if self.reset_at > 0 {
            let now = chrono::Utc::now().timestamp();
            // reset_at and now are both i64, calculate difference
            let time_until_reset_secs = self.reset_at.saturating_sub(now);

            // If we're facing rate limit and reset is imminent, wait for it
            if time_until_reset_secs > 0 {
                let duration = Duration::from_secs(time_until_reset_secs as u64);
                if duration < self.max_delay {
                    return duration;
                }
            }
        }

        // Otherwise use exponential backoff
        self.calculate_delay(attempt, None)
    }

    /// Get the number of seconds until rate limit reset.
    pub fn seconds_until_reset(&self) -> u64 {
        let now = chrono::Utc::now().timestamp();
        if self.reset_at > now {
            (self.reset_at - now) as u64
        } else {
            0
        }
    }

    /// Check if we're currently rate limited.
    pub fn is_rate_limited(&self) -> bool {
        self.remaining == 0
    }
}

impl Default for RateLimitHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Format a Unix timestamp as a human-readable reset time.
fn format_reset_time(timestamp: i64) -> String {
    if timestamp == 0 {
        return "unknown".to_string();
    }

    use chrono::{DateTime, TimeZone, Utc};
    let dt: DateTime<Utc> = Utc.timestamp_opt(timestamp, 0).unwrap();
    dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()
}

/// Result of a retry operation.
#[derive(Debug)]
pub struct RetryResult<T> {
    /// The successful result, if any.
    pub value: Option<T>,
    /// The error if all retries failed.
    pub error: Option<RogersError>,
    /// Number of attempts made.
    pub attempts: u32,
    /// Whether we gave up due to rate limiting.
    pub rate_limited: bool,
}

impl<T> RetryResult<T> {
    /// Returns true if the operation succeeded.
    pub fn is_success(&self) -> bool {
        self.value.is_some()
    }

    /// Returns true if the operation failed.
    pub fn is_failure(&self) -> bool {
        self.error.is_some()
    }
}

/// Execute a request with automatic rate limit handling.
///
/// This function wraps a request function with rate limit detection,
/// automatic retry with exponential backoff, and proper error handling.
///
/// # Arguments
/// * `request_fn` - The request function to execute.
/// * `handler` - The rate limit handler.
/// * `retry_after_header` - Optional retry-after header value from the response.
pub async fn execute_with_rate_limit<T>(
    request_fn: impl Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<T>>>>,
    handler: &RateLimitHandler,
    retry_after_header: Option<u64>,
) -> Result<T> {
    let mut attempts = 0u32;
    let mut last_error = None;

    loop {
        attempts += 1;

        match request_fn().await {
            Ok(result) => return Ok(result),
            Err(e) => {
                // Check if this is a rate limit error
                if is_rate_limit_error(&e) {
                    // Check if we should retry
                    if handler.should_retry(attempts) {
                        // Calculate delay
                        let delay = handler.calculate_delay(attempts, retry_after_header);

                        tokio::time::sleep(delay).await;
                        continue;
                    } else {
                        last_error = Some(e);
                        break;
                    }
                } else {
                    // Non-rate-limit error, don't retry
                    return Err(e);
                }
            }
        }
    }

    Err(last_error.unwrap_or_else(|| RogersError::GitHubStatus {
        code: 429,
        message: "Max retries exceeded due to rate limiting".to_string(),
    }))
}

/// Check if an error indicates rate limiting.
pub fn is_rate_limit_error(error: &RogersError) -> bool {
    match error {
        RogersError::GitHubStatus { code, .. } => *code == 429,
        _ => false,
    }
}

/// Check if an error indicates an authentication error (should fail fast).
pub fn is_auth_error(error: &RogersError) -> bool {
    match error {
        RogersError::GitHubStatus { code, .. } => *code == 401 || *code == 403,
        RogersError::Auth(_) => true,
        _ => false,
    }
}

/// Check if an error indicates a not-found error (handle gracefully).
pub fn is_not_found_error(error: &RogersError) -> bool {
    match error {
        RogersError::GitHubStatus { code, .. } => *code == 404,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{RateLimitResource, RateLimitResponse, Resources};

    #[test]
    fn test_new_handler() {
        let handler = RateLimitHandler::new();
        assert_eq!(handler.remaining(), 5000);
        assert_eq!(handler.warning_threshold, DEFAULT_WARNING_THRESHOLD);
        assert_eq!(handler.max_retries, DEFAULT_MAX_RETRIES);
    }

    #[test]
    fn test_should_warn() {
        let mut handler = RateLimitHandler::new();

        // At threshold, should not warn
        handler.remaining = 100;
        assert!(!handler.should_warn());

        // Below threshold, should warn
        handler.remaining = 99;
        assert!(handler.should_warn());

        // At zero (rate limited), should not warn (different concern)
        handler.remaining = 0;
        assert!(!handler.should_warn());
    }

    #[test]
    fn test_get_warning_message() {
        let mut handler = RateLimitHandler::new();
        handler.remaining = 50;
        handler.reset_at = chrono::Utc::now().timestamp() + 3600;

        let warning = handler.get_warning_message();
        assert!(warning.is_some());
        let msg = warning.unwrap();
        assert!(msg.contains("50"));
        assert!(msg.contains("remaining"));
    }

    #[test]
    fn test_update_from_response() {
        let mut handler = RateLimitHandler::new();
        let response = RateLimitResponse {
            resources: Resources {
                core: RateLimitResource {
                    limit: 5000,
                    remaining: 4500,
                    reset: 1234567890,
                    used: 500,
                    resource: Some("core".to_string()),
                },
                search: RateLimitResource {
                    limit: 30,
                    remaining: 30,
                    reset: 1234567890,
                    used: 0,
                    resource: Some("search".to_string()),
                },
                graphql: RateLimitResource {
                    limit: 5000,
                    remaining: 4999,
                    reset: 1234567890,
                    used: 1,
                    resource: Some("graphql".to_string()),
                },
            },
            rate: RateLimitResource {
                limit: 5000,
                remaining: 4500,
                reset: 1234567890,
                used: 500,
                resource: None,
            },
        };

        handler.update_from_response(&response);
        assert_eq!(handler.remaining(), 4500);
        assert_eq!(handler.reset_at, 1234567890);
    }

    #[test]
    fn test_should_retry() {
        let handler = RateLimitHandler::with_config(100, 3, 1, 60);

        assert!(handler.should_retry(0));
        assert!(handler.should_retry(1));
        assert!(handler.should_retry(2));
        assert!(!handler.should_retry(3));
        assert!(!handler.should_retry(4));
    }

    #[test]
    fn test_calculate_delay_exponential() {
        let handler = RateLimitHandler::with_config(100, 3, 1, 60);

        // Attempt 0: base_delay * 2^0 = 1s
        let delay0 = handler.calculate_delay(0, None);
        assert_eq!(delay0, Duration::from_secs(1));

        // Attempt 1: base_delay * 2^1 = 2s
        let delay1 = handler.calculate_delay(1, None);
        assert_eq!(delay1, Duration::from_secs(2));

        // Attempt 2: base_delay * 2^2 = 4s
        let delay2 = handler.calculate_delay(2, None);
        assert_eq!(delay2, Duration::from_secs(4));

        // Attempt 3: base_delay * 2^3 = 8s
        let delay3 = handler.calculate_delay(3, None);
        assert_eq!(delay3, Duration::from_secs(8));
    }

    #[test]
    fn test_calculate_delay_with_retry_after() {
        let handler = RateLimitHandler::with_config(100, 3, 1, 60);

        // retry_after should take precedence
        let delay = handler.calculate_delay(0, Some(5));
        assert_eq!(delay, Duration::from_secs(5));
    }

    #[test]
    fn test_calculate_delay_respects_max() {
        let handler = RateLimitHandler::with_config(100, 3, 1, 30);

        // Should cap at max_delay (30s)
        let delay = handler.calculate_delay(10, None);
        assert_eq!(delay, Duration::from_secs(30));
    }

    #[test]
    fn test_is_rate_limited() {
        let mut handler = RateLimitHandler::new();

        handler.remaining = 100;
        assert!(!handler.is_rate_limited());

        handler.remaining = 0;
        assert!(handler.is_rate_limited());
    }

    #[test]
    fn test_is_rate_limit_error() {
        let rate_limit_error = RogersError::GitHubStatus {
            code: 429,
            message: "rate limited".to_string(),
        };
        assert!(is_rate_limit_error(&rate_limit_error));

        let other_error = RogersError::GitHubStatus {
            code: 500,
            message: "server error".to_string(),
        };
        assert!(!is_rate_limit_error(&other_error));

        let auth_error = RogersError::Auth("token expired".to_string());
        assert!(!is_rate_limit_error(&auth_error));
    }

    #[test]
    fn test_is_auth_error() {
        let auth_error_401 = RogersError::GitHubStatus {
            code: 401,
            message: "unauthorized".to_string(),
        };
        assert!(is_auth_error(&auth_error_401));

        let auth_error_403 = RogersError::GitHubStatus {
            code: 403,
            message: "forbidden".to_string(),
        };
        assert!(is_auth_error(&auth_error_403));

        let other_error = RogersError::GitHubStatus {
            code: 404,
            message: "not found".to_string(),
        };
        assert!(!is_auth_error(&other_error));
    }

    #[test]
    fn test_is_not_found_error() {
        let not_found_error = RogersError::GitHubStatus {
            code: 404,
            message: "not found".to_string(),
        };
        assert!(is_not_found_error(&not_found_error));

        let other_error = RogersError::GitHubStatus {
            code: 500,
            message: "server error".to_string(),
        };
        assert!(!is_not_found_error(&other_error));
    }

    #[test]
    fn test_retry_result() {
        // Success case
        let success: RetryResult<i32> = RetryResult {
            value: Some(42),
            error: None,
            attempts: 1,
            rate_limited: false,
        };
        assert!(success.is_success());
        assert!(!success.is_failure());

        // Failure case
        let failure: RetryResult<i32> = RetryResult {
            value: None,
            error: Some(RogersError::GitHubStatus {
                code: 429,
                message: "rate limited".to_string(),
            }),
            attempts: 3,
            rate_limited: true,
        };
        assert!(!failure.is_success());
        assert!(failure.is_failure());
    }

    #[test]
    fn test_format_reset_time() {
        // Zero gives unknown
        assert_eq!(format_reset_time(0), "unknown");

        // Valid timestamp
        let result = format_reset_time(1704067200);
        assert!(result.contains("2024"));
    }
}
