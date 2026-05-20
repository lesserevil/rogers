//! GitHub authentication module.
//!
//! Handles Personal Access Token (PAT) authentication and scope validation.

use serde::{Deserialize, Serialize};

/// Required GitHub token scopes for Rodgers operation.
pub const REQUIRED_SCOPES: &[&str] = &["repo", "read:org"];

/// Authentication configuration for GitHub API.
#[derive(Debug, Clone)]
pub struct GitHubAuth {
    /// Personal Access Token (PAT)
    token: String,
    /// API base URL (e.g., https://api.github.com or GitHub Enterprise URL)
    api_url: String,
}

impl GitHubAuth {
    /// Create a new GitHubAuth from a token and optional API URL.
    ///
    /// # Arguments
    /// * `token` - GitHub Personal Access Token
    /// * `api_url` - Optional API base URL (e.g., GitHub Enterprise URL)
    pub fn new(token: impl Into<String>, api_url: &str) -> Self {
        Self {
            token: token.into(),
            api_url: if api_url.is_empty() {
                "https://api.github.com".to_string()
            } else {
                api_url.to_string()
            },
        }
    }

    /// Create a GitHubAuth with default API URL.
    pub fn new_with_default_api(token: impl Into<String>) -> Self {
        Self::new(token, "")
    }

    /// Get the token value.
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Get the API base URL.
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Validate that the token has the required scopes.
    ///
    /// Note: GitHub's API doesn't provide a direct "scopes check" endpoint for PATs.
    /// This function validates the token format and presence but cannot verify
    /// actual scopes without making an authenticated request.
    ///
    /// Returns a Result indicating whether the token appears valid.
    pub fn validate_token(&self) -> Result<(), AuthError> {
        // Check token is not empty
        if self.token.trim().is_empty() {
            return Err(AuthError::EmptyToken);
        }

        // GitHub PATs are typically in format: ghp_*, gho_*, ghu_*, ghs_*, ghr_*
        // Fine-grained tokens start with gho_ or ghp_
        // Classic tokens: ghp_ (personal access tokens)
        // OAuth: gho_ (OAuth access tokens)
        // GitHub App: ghu_ (GitHub App user-to-server tokens)
        // Installation: ghs_ (GitHub App server-to-server tokens)
        // Refresh: ghr_ (GitHub App refresh tokens)
        let valid_prefixes = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"];
        let has_valid_prefix = valid_prefixes
            .iter()
            .any(|prefix| self.token.starts_with(prefix));

        // Also accept any bearer token (for GitHub Enterprise with different formats)
        if !has_valid_prefix {
            // Check if it looks like a bearer token (alphanumeric string of reasonable length)
            if self.token.len() >= 20 && self.token.chars().all(|c| c.is_alphanumeric()) {
                // Treat as valid bearer token
                return Ok(());
            }
            return Err(AuthError::InvalidTokenFormat);
        }

        Ok(())
    }

    /// Create an Authorization header value.
    pub fn auth_header(&self) -> String {
        format!("Bearer {}", self.token)
    }

    /// Create a reqwest header map with authentication.
    pub fn auth_headers(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::AUTHORIZATION,
            self.auth_header()
                .parse()
                .expect("valid authorization header"),
        );
        headers
    }
}

impl From<&crate::config::GitHubConfig> for GitHubAuth {
    fn from(config: &crate::config::GitHubConfig) -> Self {
        GitHubAuth::new(&config.token, config.api_url.as_ref())
    }
}

/// Authentication errors.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthError {
    /// Token is empty or missing
    #[serde(rename = "EMPTY_TOKEN")]
    EmptyToken,

    /// Token format is invalid for GitHub
    #[serde(rename = "INVALID_TOKEN_FORMAT")]
    InvalidTokenFormat,

    /// Token does not have required scopes
    #[serde(rename = "MISSING_REQUIRED_SCOPES")]
    MissingRequiredScopes { missing: Vec<String> },

    /// Authentication failed (401 response)
    #[serde(rename = "AUTH_FAILED")]
    AuthFailed { message: String },

    /// Token is expired or revoked
    #[serde(rename = "TOKEN_EXPIRED")]
    TokenExpired,

    /// Insufficient permissions (403 response)
    #[serde(rename = "INSUFFICIENT_PERMISSIONS")]
    InsufficientPermissions { required: Vec<String> },
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::EmptyToken => write!(f, "GitHub token is empty or missing"),
            AuthError::InvalidTokenFormat => {
                write!(
                    f,
                    "GitHub token format is invalid. Expected classic PAT (ghp_...), GitHub App token (ghu_/ghs_/ghr_...), or OAuth token (gho_...)"
                )
            }
            AuthError::MissingRequiredScopes { missing } => {
                write!(
                    f,
                    "GitHub token is missing required scopes: {}",
                    missing.join(", ")
                )
            }
            AuthError::AuthFailed { message } => {
                write!(f, "GitHub authentication failed: {}", message)
            }
            AuthError::TokenExpired => write!(f, "GitHub token has expired or been revoked"),
            AuthError::InsufficientPermissions { required } => {
                write!(
                    f,
                    "GitHub token has insufficient permissions. Required: {}",
                    required.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for AuthError {}

impl AuthError {
    /// Returns true if this error indicates the token is invalid and should fail fast.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            AuthError::EmptyToken
                | AuthError::InvalidTokenFormat
                | AuthError::AuthFailed { .. }
                | AuthError::TokenExpired
        )
    }

    /// Returns true if this error is retryable (insufficient permissions *might* be temporary).
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            AuthError::InsufficientPermissions { .. }
                | AuthError::MissingRequiredScopes { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_auth() {
        let auth = GitHubAuth::new("ghp_test123", None);
        assert_eq!(auth.token(), "ghp_test123");
        assert_eq!(auth.api_url(), "https://api.github.com");
    }

    #[test]
    fn test_new_auth_with_api_url() {
        let auth = GitHubAuth::new("ghp_test123", Some("https://github.example.com/api/v3"));
        assert_eq!(auth.api_url(), "https://github.example.com/api/v3");
    }

    #[test]
    fn test_auth_header() {
        let auth = GitHubAuth::new("ghp_test123", None);
        assert_eq!(auth.auth_header(), "Bearer ghp_test123");
    }

    #[test]
    fn test_auth_headers() {
        let auth = GitHubAuth::new("ghp_test123", None);
        let headers = auth.auth_headers();
        assert!(headers.contains_key(reqwest::header::AUTHORIZATION));
    }

    #[test]
    fn test_validate_empty_token() {
        let auth = GitHubAuth::new("", None);
        let result = auth.validate_token();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::EmptyToken));
    }

    #[test]
    fn test_validate_whitespace_token() {
        let auth = GitHubAuth::new("   ", None);
        let result = auth.validate_token();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::EmptyToken));
    }

    #[test]
    fn test_validate_valid_classic_pat() {
        let auth = GitHubAuth::new("ghp_abcdefghijklmnopqrstuvwxyz1234567890", None);
        assert!(auth.validate_token().is_ok());
    }

    #[test]
    fn test_validate_valid_oauth_token() {
        let auth = GitHubAuth::new("gho_abcdefghijklmnopqrstuvwxyz1234567890", None);
        assert!(auth.validate_token().is_ok());
    }

    #[test]
    fn test_validate_valid_github_app_user_token() {
        let auth = GitHubAuth::new("ghu_abcdefghijklmnopqrstuvwxyz1234567890", None);
        assert!(auth.validate_token().is_ok());
    }

    #[test]
    fn test_validate_valid_github_app_server_token() {
        let auth = GitHubAuth::new("ghs_abcdefghijklmnopqrstuvwxyz1234567890", None);
        assert!(auth.validate_token().is_ok());
    }

    #[test]
    fn test_validate_valid_github_app_refresh_token() {
        let auth = GitHubAuth::new("ghr_abcdefghijklmnopqrstuvwxyz1234567890", None);
        assert!(auth.validate_token().is_ok());
    }

    #[test]
    fn test_validate_invalid_short_token() {
        let auth = GitHubAuth::new("ghp_short", None);
        let result = auth.validate_token();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), AuthError::InvalidTokenFormat));
    }

    #[test]
    fn test_validate_generic_bearer_token() {
        // Accept any 20+ char alphanumeric token as a bearer token
        let auth = GitHubAuth::new("abcdefghijklmnopqrstuvwxyz1234567890", None);
        assert!(auth.validate_token().is_ok());
    }

    #[test]
    fn test_from_github_config() {
        use crate::config::GitHubConfig;

        let config = GitHubConfig {
            owner: "test-owner".to_string(),
            repo: "test-repo".to_string(),
            token: "ghp_test123".to_string(),
            api_url: Some("https://api.github.com".to_string()),
        };
        let auth = GitHubAuth::from(&config);
        assert_eq!(auth.token(), "ghp_test123");
        assert_eq!(auth.api_url(), "https://api.github.com");
    }

    #[test]
    fn test_auth_error_is_auth_error() {
        assert!(AuthError::EmptyToken.is_auth_error());
        assert!(AuthError::InvalidTokenFormat.is_auth_error());
        assert!(AuthError::TokenExpired.is_auth_error());
        assert!(AuthError::AuthFailed {
            message: "test".to_string()
        }
        .is_auth_error());

        // These are permission issues, not auth failures
        assert!(!AuthError::InsufficientPermissions {
            required: vec!["repo".to_string()]
        }
        .is_auth_error());
        assert!(!AuthError::MissingRequiredScopes {
            missing: vec!["repo".to_string()]
        }
        .is_auth_error());
    }

    #[test]
    fn test_auth_error_is_retryable() {
        assert!(AuthError::InsufficientPermissions {
            required: vec!["repo".to_string()]
        }
        .is_retryable());
        assert!(AuthError::MissingRequiredScopes {
            missing: vec!["repo".to_string()]
        }
        .is_retryable());

        // Auth errors are not retryable
        assert!(!AuthError::EmptyToken.is_retryable());
        assert!(!AuthError::InvalidTokenFormat.is_retryable());
        assert!(!AuthError::TokenExpired.is_retryable());
    }
}