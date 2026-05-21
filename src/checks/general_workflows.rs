//! General Workflows audit check.
//!
//! Verifies that `.github/workflows/` contains at least one workflow with
//! a `pull_request` or `pull_request_target` trigger targeting a common
//! default branch (main, master, develop, etc.).
//!
//! - Warn if no CI workflow found for PRs targeting the default branch
//! - Info if a CI workflow exists and appears active

use crate::checks::{CheckResult, InitCheck};
use crate::error::Result;
use crate::github::GitHubClient;

/// Check for CI workflows that run on pull requests to the main branch.
pub struct GeneralWorkflowsCheck;

/// Common default branch names to look for in PR triggers.
const DEFAULT_BRANCHES: &[&str] = &["main", "master", "develop"];

/// PR trigger keywords that indicate CI on pull requests.
const PR_TRIGGER_KEYWORDS: &[&str] = &["pull_request", "pull_request_target"];

impl InitCheck for GeneralWorkflowsCheck {
    fn name(&self) -> &'static str {
        "general_workflows"
    }

    async fn check(
        &self,
        github: &GitHubClient,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<CheckResult>> {
        // Step 1: List all workflows via the GitHub Actions API.
        let workflows = github.list_workflows(owner, repo).await?;

        // Filter to only workflow files (ends with .yml or .yaml).
        let workflow_files: Vec<_> = workflows
            .iter()
            .filter(|w| w.path.ends_with(".yml") || w.path.ends_with(".yaml"))
            .collect();

        if workflow_files.is_empty() {
            // No workflow files at all → Warn.
            return Ok(vec![CheckResult::warn(format!(
                "No GitHub Actions workflow files found in {}/{} — no CI will run on pull requests",
                owner, repo
            ))]);
        }

        // Step 2: Fetch and analyze each workflow file.
        let mut has_pr_trigger = false;
        let mut pr_workflow_names = Vec::new();
        let mut other_names = Vec::new();

        for workflow in &workflow_files {
            match github
                .get_file_contents(owner, repo, &workflow.path, github.default_ref())
                .await
            {
                Ok(contents) => {
                    if let Some(triggers) = find_pr_triggers(&contents) {
                        has_pr_trigger = true;
                        pr_workflow_names.push((workflow.name.clone(), triggers));
                    } else {
                        other_names.push(workflow.name.clone());
                    }
                }
                Err(e) => {
                    // If we can't read a workflow file, log a warning but continue.
                    tracing::warn!("Failed to read workflow '{}': {}", workflow.path, e);
                }
            }
        }

        // Step 3: Determine severity based on findings.
        if !has_pr_trigger {
            // No PR trigger found → Warn.
            let mut desc = format!(
                "No CI workflow found for pull requests targeting default branch in {}/{}",
                owner, repo
            );
            if !workflow_files.is_empty() {
                desc.push_str(&format!(
                    "\n\nFound {} workflow(s) but none trigger on pull requests: {}",
                    workflow_files.len(),
                    workflow_files
                        .iter()
                        .map(|w| w.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            Ok(vec![CheckResult::warn(desc)])
        } else {
            // PR trigger found → Info.
            let mut desc = format!(
                "CI workflow(s) found for pull requests in {}/{}: {}",
                owner,
                repo,
                pr_workflow_names
                    .iter()
                    .map(|(name, triggers)| {
                        format!("{} (triggers: {})", name, triggers.join(", "))
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !other_names.is_empty() {
                desc.push_str(&format!(
                    "\n\nAdditional workflow(s): {}",
                    other_names.join(", ")
                ));
            }
            Ok(vec![CheckResult::info(desc)])
        }
    }
}

/// Find PR triggers in a workflow file content.
///
/// Returns a vector of trigger keywords found (e.g., `["pull_request"]`).
fn find_pr_triggers(content: &str) -> Option<Vec<String>> {
    let mut found_triggers = Vec::new();

    for trigger_keyword in PR_TRIGGER_KEYWORDS {
        if has_matching_pr_trigger(content, trigger_keyword) {
            found_triggers.push(trigger_keyword.to_string());
        }
    }

    if found_triggers.is_empty() {
        None
    } else {
        Some(found_triggers)
    }
}

/// Parse a YAML value for `branches` into a list of branch name strings.
/// Handles both inline `[main, master]` and block `- main` formats.
fn parse_branch_values(content_after_colon: &str) -> Vec<String> {
    let mut branches = Vec::new();
    let trimmed = content_after_colon.trim();

    if trimmed.starts_with('[') {
        // Inline array: [main, master, develop]
        if let Some(end_bracket) = trimmed.find(']') {
            let inner = &trimmed[1..end_bracket];
            for item in inner.split(',') {
                let branch = item.trim().trim_matches('"').trim_matches('\'').trim();
                if !branch.is_empty() {
                    branches.push(branch.to_string());
                }
            }
        }
    } else {
        // Block array: lines starting with "- "
        for line in content_after_colon.lines() {
            let trimmed_line = line.trim();
            if let Some(branch_value) = trimmed_line.strip_prefix("- ") {
                let branch = branch_value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .trim();
                if !branch.is_empty() {
                    branches.push(branch.to_string());
                }
            } else if !trimmed_line.is_empty() && !trimmed_line.starts_with('#') && !branches.is_empty() {
                // Non-list, non-empty, non-comment line after list items — end of list
                break;
            }
        }
    }

    branches
}

/// Check if a trigger has a `branches` filter with default branch names.
/// The `section` parameter should be the YAML text under the trigger keyword.
fn has_matching_branches_in_section(section: &str) -> bool {
    // Search for `branches:` key (not `branches_ignore` or `branches_ignored`).
    let mut remaining = section;

    loop {
        let idx = match remaining.find("branches:") {
            Some(i) => i,
            None => return false,
        };

        let after_key = &remaining[idx + 9..]; // After "branches:"
        // Make sure it's not `branches_something` (like branches_ignore)
        if after_key.starts_with(|c: char| c.is_alphanumeric() || c == '_') {
            // Skip past this and search for next `branches:`
            remaining = &remaining[idx + 9..];
            continue;
        }

        // Found a valid `branches:` key. Parse the values.
        let branches = parse_branch_values(after_key);

        for branch in &branches {
            if branch == "*"
                || DEFAULT_BRANCHES
                    .iter()
                    .any(|default| branch.eq_ignore_ascii_case(default))
            {
                return true;
            }
        }

        // Continue searching for more `branches:` keys in remaining text
        remaining = after_key;
    }
}

/// Check if a trigger section has no `branches` key (runs on all PRs).
fn section_has_no_branches_key(section: &str) -> bool {
    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed == "branches:"
            || (trimmed.starts_with("branches:")
                && !trimmed.starts_with("branches_ignore")
                && !trimmed.starts_with("branches_ignored"))
        {
            return false;
        }
    }
    true
}

/// Check if a trigger keyword in the content has matching PR branch configuration.
fn has_matching_pr_trigger(content: &str, trigger_keyword: &str) -> bool {
    let content_lower = content.to_lowercase();

    // Check if the trigger keyword exists at all.
    let keyword_with_colon = format!("{}:", trigger_keyword);
    let keyword_in_content = content_lower.contains(&keyword_with_colon)
        || content_lower.lines().any(|line| line.trim() == trigger_keyword);

    if !keyword_in_content {
        return false;
    }

    // Extract the YAML section under this trigger keyword.
    let section = extract_trigger_section(&content_lower, trigger_keyword);

    // Check if it has branches filter matching our defaults.
    if has_matching_branches_in_section(&section) {
        return true;
    }

    // Check if no branches filter at all (runs on all branches) — still counts.
    if section_has_no_branches_key(&section) {
        return true;
    }

    false
}

/// Extract the YAML section under a trigger keyword, stopping at the next top-level key
/// or the end of the `on:` block.
fn extract_trigger_section(content: &str, trigger_keyword: &str) -> String {
    let keyword_with_colon = format!("{}:", trigger_keyword);
    let trigger_pos = match content.find(&keyword_with_colon) {
        Some(pos) => pos + keyword_with_colon.len(),
        None => return String::new(),
    };

    let after_trigger = &content[trigger_pos..];

    // Walk through lines to find the end of this section.
    // A section ends when we hit a non-indented line that looks like a YAML key.
    let mut result = String::new();

    for line in after_trigger.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            result.push_str(line);
            result.push('\n');
            continue;
        }

        // Check if this is a top-level key (not indented, looks like `key:`)
        // This indicates the end of the current trigger section.
        if !line.starts_with(' ') && !line.starts_with('\t') {
            // Top-level key — end of this trigger section.
            break;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checks::{Fixability, Severity};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn make_client(server: &MockServer) -> GitHubClient {
        GitHubClient::new("").with_base_url(&server.uri())
    }

    fn base64_encode(input: &str) -> String {
        let encode_table: [u8; 64] =
            *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut result = String::new();
        let bytes = input.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let b0 = bytes[i] as u32;
            let b1 = if i + 1 < bytes.len() {
                bytes[i + 1] as u32
            } else {
                0
            };
            let b2 = if i + 2 < bytes.len() {
                bytes[i + 2] as u32
            } else {
                0
            };

            result.push(encode_table[(b0 >> 2) as usize] as char);
            result.push(
                encode_table[((b0 & 0x03) << 4) as usize + ((b1 >> 4) & 0x0F) as usize] as char,
            );

            if i + 1 < bytes.len() {
                result.push(
                    encode_table[((b1 & 0x0F) << 2) as usize + ((b2 >> 6) & 0x03) as usize] as char,
                );
            } else {
                result.push('=');
            }

            if i + 2 < bytes.len() {
                result.push(encode_table[b2 as usize & 0x3F] as char);
            } else {
                result.push('=');
            }

            i += 3;
        }

        result
    }

    const OWNER: &str = "test-owner";
    const REPO: &str = "test-repo";

    // ─── Integration tests using mock server ───────────────────────────

    /// Test: no workflows at all → Warn.
    #[tokio::test]
    async fn test_no_workflows_returns_warn() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 0,
                "workflows": []
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0]
            .description
            .contains("No GitHub Actions workflow files found"));
        assert_eq!(results[0].fixability, Fixability::Manual);
    }

    /// Test: workflows exist but none have PR triggers → Warn.
    #[tokio::test]
    async fn test_no_pr_triggers_returns_warn() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Release",
                    "path": ".github/workflows/release.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/release.yml",
                    "badge_url": "https://github.com/test/test/workflows/Release/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0]
            .description
            .contains("No CI workflow found for pull requests"));
        assert!(results[0].description.contains("Release"));
    }

    /// Test: CI workflow with PR trigger on main → Info.
    #[tokio::test]
    async fn test_pr_trigger_on_main_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("CI workflow"));
        assert!(results[0].description.contains("pull requests"));
        assert!(results[0].description.contains("CI"));
        assert_eq!(results[0].fixability, Fixability::NotApplicable);
    }

    /// Test: PR trigger on master → Info.
    #[tokio::test]
    async fn test_pr_trigger_on_master_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: [master]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: PR trigger on develop → Info.
    #[tokio::test]
    async fn test_pr_trigger_on_develop_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: [develop]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: pull_request_target with branches: [main] → Info.
    #[tokio::test]
    async fn test_pull_request_target_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request_target:\n    branches: [main]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("pull_request_target"));
    }

    /// Test: multiple workflows, one with PR trigger → Info.
    #[tokio::test]
    async fn test_multiple_workflows_with_pr_trigger_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 3,
                "workflows": [
                    {
                        "id": 1,
                        "name": "CI",
                        "path": ".github/workflows/ci.yml",
                        "state": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z",
                        "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                        "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                        "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                    },
                    {
                        "id": 2,
                        "name": "Release",
                        "path": ".github/workflows/release.yml",
                        "state": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z",
                        "url": "https://api.github.com/repos/test/test/actions/workflows/2",
                        "html_url": "https://github.com/test/test/blob/main/.github/workflows/release.yml",
                        "badge_url": "https://github.com/test/test/workflows/Release/badge.svg"
                    },
                    {
                        "id": 3,
                        "name": "Lint",
                        "path": ".github/workflows/lint.yml",
                        "state": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z",
                        "url": "https://api.github.com/repos/test/test/actions/workflows/3",
                        "html_url": "https://github.com/test/test/blob/main/.github/workflows/lint.yml",
                        "badge_url": "https://github.com/test/test/workflows/Lint/badge.svg"
                    }
                ]
            })))
            .mount(&server)
            .await;

        // CI has PR trigger.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: [main]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        // Release has push trigger on tags.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        // Lint has only push on branches.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/lint.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Lint\non:\n  push:\n    branches: [main]\njobs:\n  lint:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("CI"));
        assert!(results[0].description.contains("Release"));
        assert!(results[0].description.contains("Lint"));
    }

    /// Test: workflow with branches: [*] (wildcard) → Info.
    #[tokio::test]
    async fn test_pr_trigger_wildcard_branch_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: ['*']\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: PR trigger on non-default branch only → Warn.
    #[tokio::test]
    async fn test_pr_trigger_on_non_default_branch_returns_warn() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: [feature, staging]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        // PR on feature/staging only, not main/master/develop → Warn
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0]
            .description
            .contains("No CI workflow found for pull requests"));
    }

    /// Test: PR trigger with branches-ignore instead of branches → Info.
    #[tokio::test]
    async fn test_pr_trigger_with_branches_ignore_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches-ignore:\n      - main\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        // branches-ignore without branches means it runs on all branches except those ignored
        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        // Has branches-ignore but no branches — runs on all branches except ignored
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: workflow file not readable still allows other files to be checked.
    #[tokio::test]
    async fn test_unreadable_workflow_skipped() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 2,
                "workflows": [
                    {
                        "id": 1,
                        "name": "CI",
                        "path": ".github/workflows/ci.yml",
                        "state": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z",
                        "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                        "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                        "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                    },
                    {
                        "id": 2,
                        "name": "Release",
                        "path": ".github/workflows/release.yml",
                        "state": "active",
                        "created_at": "2024-01-01T00:00:00Z",
                        "updated_at": "2024-01-02T00:00:00Z",
                        "url": "https://api.github.com/repos/test/test/actions/workflows/2",
                        "html_url": "https://github.com/test/test/blob/main/.github/workflows/release.yml",
                        "badge_url": "https://github.com/test/test/workflows/Release/badge.svg"
                    }
                ]
            })))
            .mount(&server)
            .await;

        // CI workflow is readable but has no PR trigger.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode("name: CI\non:\n  push:\n    branches: [main]\n"),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        // Release workflow is NOT readable (404).
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yml",
            ))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "message": "Not Found"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        // Should return Warn since only readable file has no PR trigger.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0]
            .description
            .contains("No CI workflow found for pull requests"));
    }

    /// Test: `.yaml` extension workflows are recognized.
    #[tokio::test]
    async fn test_yaml_extension_recognized() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yaml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yaml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yaml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: [main]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: check name returns correct string.
    #[tokio::test]
    async fn test_check_name() {
        let check = GeneralWorkflowsCheck;
        assert_eq!(check.name(), "general_workflows");
    }

    /// Test: PR trigger with multiple branches including main → Info.
    #[tokio::test]
    async fn test_pr_trigger_multiple_branches_with_main_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches:\n      - main\n      - master\n      - develop\n      - 'release/*'\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: PR trigger with YAML multiline format → Info.
    #[tokio::test]
    async fn test_pr_trigger_multiline_branch_format_returns_info() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        // Multi-line branch format with main listed.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches:\n      - main\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: PR trigger with branches but none match default branches → Warn.
    #[tokio::test]
    async fn test_pr_trigger_branches_no_match_returns_warn() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "CI",
                    "path": ".github/workflows/ci.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/ci.yml",
                    "badge_url": "https://github.com/test/test/workflows/CI/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/ci.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: CI\non:\n  pull_request:\n    branches: [feature, staging, hotfix]\njobs:\n  test:\n    runs-on: ubuntu-latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = GeneralWorkflowsCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0]
            .description
            .contains("No CI workflow found for pull requests"));
    }

    // ─── Unit tests for detection helpers ──────────────────────────────

    #[test]
    fn test_find_pr_triggers_pull_request_main() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_some());
        let triggers = triggers.unwrap();
        assert!(triggers.contains(&"pull_request".to_string()));
    }

    #[test]
    fn test_find_pr_triggers_pull_request_master() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches: [master]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_some());
        let triggers = triggers.unwrap();
        assert!(triggers.contains(&"pull_request".to_string()));
    }

    #[test]
    fn test_find_pr_triggers_pull_request_develop() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches: [develop]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_some());
        let triggers = triggers.unwrap();
        assert!(triggers.contains(&"pull_request".to_string()));
    }

    #[test]
    fn test_find_pr_triggers_pull_request_wildcard() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches: ['*']
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_some());
    }

    #[test]
    fn test_find_pr_triggers_pull_request_target() {
        let yaml = r#"
name: CI
on:
  pull_request_target:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_some());
        let triggers = triggers.unwrap();
        assert!(triggers.contains(&"pull_request_target".to_string()));
    }

    #[test]
    fn test_find_pr_triggers_push_only() {
        let yaml = r#"
name: CI
on:
  push:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_none());
    }

    #[test]
    fn test_find_pr_triggers_release_only() {
        let yaml = r#"
name: Release
on:
  push:
    tags:
      - 'v*'
jobs:
  release:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_none());
    }

    #[test]
    fn test_find_pr_triggers_pr_on_non_default_branch() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches: [feature, staging]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_none());
    }

    #[test]
    fn test_find_pr_triggers_pr_with_branches_ignore() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches-ignore:
      - main
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        // branches-ignore without branches means it runs on all branches
        assert!(triggers.is_some());
    }

    #[test]
    fn test_find_pr_triggers_empty() {
        let yaml = r#"name: Empty
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_none());
    }

    #[test]
    fn test_find_pr_triggers_both_triggers() {
        let yaml = r#"
name: CI
on:
  pull_request:
    branches: [main]
  pull_request_target:
    branches: [develop]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        let triggers = find_pr_triggers(&yaml);
        assert!(triggers.is_some());
        let triggers = triggers.unwrap();
        assert!(triggers.contains(&"pull_request".to_string()));
        assert!(triggers.contains(&"pull_request_target".to_string()));
    }

    #[test]
    fn test_parse_branch_values_inline() {
        let values = parse_branch_values(" [main, master, develop]");
        assert_eq!(values, vec!["main", "master", "develop"]);
    }

    #[test]
    fn test_parse_branch_values_inline_wildcard() {
        let values = parse_branch_values(" ['*']");
        assert_eq!(values, vec!["*"]);
    }

    #[test]
    fn test_parse_branch_values_block() {
        let values = parse_branch_values("\n    - main\n    - master\n    - develop\n");
        assert_eq!(values, vec!["main", "master", "develop"]);
    }

    #[test]
    fn test_parse_branch_values_quoted() {
        let values = parse_branch_values(" ['main', 'master']");
        assert_eq!(values, vec!["main", "master"]);
    }

    #[test]
    fn test_parse_branch_values_single_inline() {
        let values = parse_branch_values(" [main]");
        assert_eq!(values, vec!["main"]);
    }

    #[test]
    fn test_parse_branch_values_single_block() {
        let values = parse_branch_values("\n      - main\n");
        assert_eq!(values, vec!["main"]);
    }

    #[test]
    fn test_extract_trigger_section_stops_at_jobs() {
        let content = "pull_request:\n  branches: [main]\njobs:\n  test:\n    runs-on: ubuntu-latest\n";
        let section = extract_trigger_section(&content.to_lowercase(), "pull_request");
        assert!(section.contains("branches"));
        assert!(!section.contains("jobs"));
    }

    #[test]
    fn test_extract_trigger_section_with_both_triggers() {
        let content = "pull_request:\n  branches: [main]\npull_request_target:\n  branches: [develop]\njobs:\n  test:\n    runs-on: ubuntu-latest\n";
        let section = extract_trigger_section(&content.to_lowercase(), "pull_request");
        assert!(section.contains("branches"));
        assert!(section.contains("main"));
        // Section should NOT include pull_request_target (next trigger)
        assert!(!section.contains("pull_request_target"));
    }

    #[test]
    fn test_section_has_no_branches_key_true() {
        let section = "  paths:\n    - '*.rs'\n  types:\n    - opened\n";
        assert!(section_has_no_branches_key(section));
    }

    #[test]
    fn test_section_has_no_branches_key_false() {
        let section = "  branches: [main]\n  paths:\n    - '*.rs'\n";
        assert!(!section_has_no_branches_key(section));
    }

    #[test]
    fn test_section_has_no_branches_key_ignore_not_branches() {
        let section = "  branches-ignore:\n    - main\n";
        assert!(section_has_no_branches_key(section));
    }

    #[test]
    fn test_has_matching_branches_in_section_inline_main() {
        let section = "  branches: [main]\n";
        assert!(has_matching_branches_in_section(section));
    }

    #[test]
    fn test_has_matching_branches_in_section_block_develop() {
        let section = "  branches:\n    - develop\n";
        assert!(has_matching_branches_in_section(section));
    }

    #[test]
    fn test_has_matching_branches_in_section_no_match() {
        let section = "  branches: [feature, staging]\n";
        assert!(!has_matching_branches_in_section(section));
    }

    #[test]
    fn test_has_matching_branches_in_section_wildcard() {
        let section = "  branches: ['*']\n";
        assert!(has_matching_branches_in_section(section));
    }

    #[test]
    fn test_has_matching_pr_trigger_push_only() {
        let content = "push:\n  branches: [main]\n";
        assert!(!has_matching_pr_trigger(&content.to_lowercase(), "pull_request"));
    }

    #[test]
    fn test_has_matching_pr_trigger_with_branches() {
        let content = "pull_request:\n  branches: [main]\n";
        assert!(has_matching_pr_trigger(&content.to_lowercase(), "pull_request"));
    }

    #[test]
    fn test_has_matching_pr_trigger_with_paths_only() {
        let content = "pull_request:\n  paths:\n    - '*.rs'\n";
        assert!(has_matching_pr_trigger(&content.to_lowercase(), "pull_request"));
    }
}
