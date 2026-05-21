//! Release Workflow audit check.
//!
//! Verifies that `.github/workflows/` contains at least one workflow file
//! with a release-capable trigger (push with tag pattern or workflow_dispatch
//! with release inputs) and artifact upload step.
//!
//! - Blocker if no release-capable workflow found
//! - Warn if a release workflow exists but no artifact upload detected
//! - Info if a complete release workflow with artifacts is found

use crate::checks::{CheckResult, Fixability, InitCheck, Severity};
use crate::error::Result;
use crate::github::GitHubClient;

/// Check for release-capable GitHub Actions workflows.
pub struct ReleaseWorkflowCheck;

/// Tag patterns that indicate a release-triggered workflow.
const RELEASE_TAG_PATTERNS: &[&str] = &["v*", "*.*.*", "release-*", "release/*"];

/// Artifact upload patterns to search for in workflow YAML strings.
const ARTIFACT_PATTERNS: &[&str] = &[
    "uses: actions/upload-artifact",
    "uses: ./actions/upload-artifact",
    "gh release upload",
    "docker push",
    "aws s3 cp",
    "aws s3 sync",
    "ghr upload",
    "make publish",
    "cargo publish",
    "npm publish",
    "pip upload",
    "twine upload",
];

impl InitCheck for ReleaseWorkflowCheck {
    fn name(&self) -> &'static str {
        "release_workflow"
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
            // No workflow files at all → Blocker.
            return Ok(vec![CheckResult {
                severity: Severity::Blocker,
                description: format!(
                    "No GitHub Actions workflow files found in {}/{}",
                    owner, repo
                ),
                fixability: Fixability::Manual,
                fix_instructions: Some(format!(
                    "Create a release workflow at \
                     https://github.com/{owner}/{repo}/new/main/.github/workflows/release.yml\n\n\
                     Rodgers recommends the following template:\n\n\
                     ```yaml\n\
                     name: Release\n\n\
                     on:\n\
                       push:\n\
                         tags:\n\
                           - 'v*'\n\
                           - '*.*.*'\n\
                     \n\
                     jobs:\n\
                       release:\n\
                         runs-on: ubuntu-latest\n\
                         steps:\n\
                           - uses: actions/checkout@v4\n\
                           # Build your project here\n\
                           # - run: make build\n\
                           \n\
                           - name: Upload release artifact\n\
                             uses: actions/upload-artifact@v4\n\
                             with:\n\
                               name: release-artifacts\n\
                               path: dist/\n\
                           \n\
                           - name: Create GitHub Release\n\
                             uses: softprops/action-gh-release@v1\n\
                             with:\n\
                               files: dist/*\n\
                     ```\n\n\
                     See: https://docs.github.com/en/actions/use-cases-and-examples/publishing-packages/publishing-nodejs-packages\n\
                     "
                )),
            }]);
        }

        // Step 2: Fetch and analyze each workflow file.
        let mut has_release_trigger = false;
        let mut has_artifact_upload = false;
        let mut release_workflow_names = Vec::new();
        let mut non_release_names = Vec::new();

        for workflow in &workflow_files {
            match github
                .get_file_contents(owner, repo, &workflow.path, github.default_ref())
                .await
            {
                Ok(contents) => {
                    let content_lower = contents.to_lowercase();
                    let is_release = is_release_workflow(&content_lower);
                    let has_artifacts = has_artifact_upload_step(&content_lower);

                    if is_release {
                        has_release_trigger = true;
                        release_workflow_names.push(workflow.name.clone());
                        if has_artifacts {
                            has_artifact_upload = true;
                        }
                    } else {
                        non_release_names.push(workflow.name.clone());
                    }
                }
                Err(e) => {
                    // If we can't read a workflow file, log a warning but continue.
                    tracing::warn!("Failed to read workflow '{}': {}", workflow.path, e);
                }
            }
        }

        // Step 3: Determine severity based on findings.
        if !has_release_trigger {
            // No release-capable workflow found → Blocker.
            let mut desc = format!(
                "No release-capable GitHub Actions workflow found in {}/{}",
                owner, repo
            );
            if !workflow_files.is_empty() {
                desc.push_str(&format!(
                    "\n\nFound {} workflow(s) but none trigger on release tags: {}",
                    workflow_files.len(),
                    workflow_files
                        .iter()
                        .map(|w| w.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            Ok(vec![CheckResult {
                severity: Severity::Blocker,
                description: desc,
                fixability: Fixability::Manual,
                fix_instructions: Some(
                    "Add a release workflow to your repository. Rodgers recommends creating a \
                     file at `.github/workflows/release.yml` with a `push` trigger on tag patterns \
                     (v*, *.*.*, release-*) and an artifact upload step.\n\n\
                     See: https://docs.github.com/en/actions/use-cases-and-examples/publishing-packages/publishing-nodejs-packages\n\
                    ".to_string()),
            }])
        } else if !has_artifact_upload {
            // Release workflow exists but no artifact upload → Warn.
            let mut desc = format!(
                "Release workflow(s) found in {}/{} but no artifact upload step detected: {}",
                owner,
                repo,
                release_workflow_names.join(", ")
            );
            if !non_release_names.is_empty() {
                desc.push_str(&format!(
                    "\n\nNote: {} non-release workflow(s) also exist: {}",
                    non_release_names.len(),
                    non_release_names.join(", ")
                ));
            }
            Ok(vec![CheckResult {
                severity: Severity::Warn,
                description: desc,
                fixability: Fixability::Manual,
                fix_instructions: Some(
                    "Add an artifact upload step to your release workflow. Common approaches:\n\n\
                     1. Use `actions/upload-artifact@v4` to upload build outputs\n\
                     2. Use `gh release upload` in a job step to attach files to the release\n\
                     3. Use `docker push` to publish container images\n\
                     4. Use `aws s3 cp` to upload artifacts to S3\n\n\
                     See: https://docs.github.com/en/actions/use-cases-and-examples/publishing-packages/publishing-nodejs-packages\n\
                    ".to_string()),
            }])
        } else {
            // Complete release workflow with artifacts → Info.
            let mut desc = format!(
                "Release workflow found with artifact uploads in {}/{}: {}",
                owner,
                repo,
                release_workflow_names.join(", ")
            );
            if !non_release_names.is_empty() {
                desc.push_str(&format!(
                    "\n\nAdditional workflow(s): {}",
                    non_release_names.join(", ")
                ));
            }
            Ok(vec![CheckResult {
                severity: Severity::Info,
                description: desc,
                fixability: Fixability::NotApplicable,
                fix_instructions: None,
            }])
        }
    }
}

/// Check if a workflow file has a release-capable trigger.
///
/// Looks for:
/// - `push` with `tags` matching patterns like `v*`, `*.*.*`, `release-*`
/// - `workflow_dispatch` with release-related inputs
fn is_release_workflow(content: &str) -> bool {
    // Check for push trigger with tag patterns.
    if has_tag_push_trigger(content) {
        return true;
    }

    // Check for workflow_dispatch with release-related inputs.
    if has_release_dispatch(content) {
        return true;
    }

    false
}

/// Check if the workflow has a `push` trigger with tag patterns.
fn has_tag_push_trigger(content: &str) -> bool {
    // Look for `push:` section, then check for `tags:` within it.
    let push_idx = match content.find("push:") {
        Some(idx) => idx,
        None => return false,
    };

    // Get a reasonable chunk after the `push:` to find tags.
    let after_push = &content[push_idx..];

    // Check if there's a `tags:` section under push.
    let tags_idx = match after_push.find("tags:") {
        Some(idx) => idx,
        None => return false,
    };

    // Make sure it's under push, not some other key (e.g., `push_tags`).
    // Look at the line containing `tags:` and ensure `push` is the parent key.
    let line_start = after_push[..tags_idx]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);
    let preceding_line = &after_push[line_start..tags_idx].trim_end();

    // If there's only whitespace/newlines between push and tags, tags belongs to push.
    if !preceding_line.chars().any(|c| !c.is_whitespace()) {
        // Found tags under push. Now check if any tag pattern matches.
        return has_matching_tag_pattern(&after_push[tags_idx..]);
    }

    false
}

/// Check if tag patterns under `tags:` include release patterns.
fn has_matching_tag_pattern(after_tags: &str) -> bool {
    // Get a reasonable chunk after `tags:` (up to the next top-level key).
    let chunk = &after_tags[7..]; // Skip "tags:"

    for pattern in RELEASE_TAG_PATTERNS {
        // Use simple substring search since YAML tag patterns are globs.
        // e.g., the file will contain `- 'v*'` or `- v*`
        if chunk.contains(pattern) {
            return true;
        }
    }

    false
}

/// Check if the workflow has `workflow_dispatch` with release-related inputs.
fn has_release_dispatch(content: &str) -> bool {
    // Look for `workflow_dispatch` trigger.
    if !content.contains("workflow_dispatch") {
        return false;
    }

    // Check if there are release-related inputs (e.g., `release_version`, `target`, etc.).
    let release_input_patterns = ["release", "version", "publish"];

    // Get the chunk after `workflow_dispatch` and check for release inputs.
    let dispatch_idx = content.find("workflow_dispatch").unwrap();
    let after_dispatch = &content[dispatch_idx..];

    // Look for inputs: section and check for release-related keys.
    if let Some(inputs_idx) = after_dispatch.find("inputs:") {
        let after_inputs = &after_dispatch[inputs_idx..];
        for pattern in &release_input_patterns {
            if after_inputs.contains(&format!("{}:", pattern))
                || after_inputs.contains(&format!("- {}", pattern))
            {
                return true;
            }
        }
        // Also check for `target` as a key (not substring of staging etc.)
        if after_inputs.contains("target:") || after_inputs.contains("- target") {
            return true;
        }
        // Also check for `tag` as a key
        if after_inputs.contains("tag:") || after_inputs.contains("- tag") {
            return true;
        }
    }

    false
}

/// Check if a workflow file has an artifact upload step.
fn has_artifact_upload_step(content: &str) -> bool {
    let content_lower = content.to_lowercase();
    for pattern in ARTIFACT_PATTERNS {
        if content_lower.contains(&pattern.to_lowercase()) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper: base64 encode a string.
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

    fn make_client(server: &MockServer) -> GitHubClient {
        GitHubClient::new("").with_base_url(&server.uri())
    }

    const OWNER: &str = "test-owner";
    const REPO: &str = "test-repo";

    // ─── Integration tests using mock server ───────────────────────────

    /// Test: no workflows at all → Blocker.
    #[tokio::test]
    async fn test_no_workflows_returns_blocker() {
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
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Blocker);
        assert!(
            results[0]
                .description
                .contains("No GitHub Actions workflow files found")
        );
        assert_eq!(results[0].fixability, Fixability::Manual);
        assert!(results[0].fix_instructions.is_some());
    }

    /// Test: workflows exist but none are release-capable → Blocker.
    #[tokio::test]
    async fn test_non_release_workflows_returns_blocker() {
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
                    "name: CI\non:\n  push:\n    branches: [main]\n  pull_request:\n    branches: [main]\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Blocker);
        assert!(results[0].description.contains("No release-capable"));
        assert!(results[0].description.contains("CI"));
        assert_eq!(results[0].fixability, Fixability::Manual);
    }

    /// Test: release workflow without artifact upload → Warn.
    #[tokio::test]
    async fn test_release_workflow_no_artifacts_returns_warn() {
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

        // Workflow has tag trigger but no artifact upload.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: echo 'Creating release'\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Warn);
        assert!(results[0].description.contains("Release"));
        assert!(results[0].description.contains("artifact upload"));
        assert_eq!(results[0].fixability, Fixability::Manual);
        assert!(results[0].fix_instructions.is_some());
    }

    /// Test: release workflow with artifact upload → Info.
    #[tokio::test]
    async fn test_release_workflow_with_artifacts_returns_info() {
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

        // Workflow has tag trigger AND artifact upload.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/upload-artifact@v4\n        with:\n          name: dist\n          path: dist/\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("Release"));
        assert!(results[0].description.contains("artifact"));
        assert_eq!(results[0].fixability, Fixability::NotApplicable);
        assert!(results[0].fix_instructions.is_none());
    }

    /// Test: multiple workflows, one is release with artifacts → Info.
    #[tokio::test]
    async fn test_multiple_workflows_with_release_returns_info() {
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

        // CI workflow.
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

        // Release workflow with artifact upload.
        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/upload-artifact@v4\n        with:\n          name: artifacts\n          path: dist/\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("Release"));
        assert!(results[0].description.contains("CI"));
    }

    /// Test: tag pattern `*.*.*` triggers release detection.
    #[tokio::test]
    async fn test_sema_tag_pattern_detected() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Publish",
                    "path": ".github/workflows/publish.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/publish.yml",
                    "badge_url": "https://github.com/test/test/workflows/Publish/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/publish.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Publish\non:\n  push:\n    tags:\n      - '*.*.*'\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/upload-artifact@v4\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: `release-*` tag pattern triggers release detection.
    #[tokio::test]
    async fn test_release_dash_tag_pattern_detected() {
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
                    "name: Release\non:\n  push:\n    tags:\n      - 'release-*'\njobs:\n  deploy:\n    runs-on: ubuntu-latest\n    steps:\n      - run: gh release upload\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: artifact upload via `gh release upload`.
    #[tokio::test]
    async fn test_gh_release_upload_detected() {
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
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: gh release upload \"$GITHUB_REF\" dist/*\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.to_lowercase().contains("artifact"));
    }

    /// Test: artifact upload via `docker push`.
    #[tokio::test]
    async fn test_docker_push_detected() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Docker Release",
                    "path": ".github/workflows/docker.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/docker.yml",
                    "badge_url": "https://github.com/test/test/workflows/Docker/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/docker.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Docker Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: docker/build-push-action@v5\n      - run: docker push myimage:latest\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: artifact upload via `aws s3 cp`.
    #[tokio::test]
    async fn test_aws_s3_cp_detected() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Deploy",
                    "path": ".github/workflows/deploy.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/deploy.yml",
                    "badge_url": "https://github.com/test/test/workflows/Deploy/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/deploy.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Deploy\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  deploy:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: aws s3 cp dist/ s3://my-bucket/\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: branch-only push trigger is not release-capable.
    #[tokio::test]
    async fn test_branch_only_push_not_release() {
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
                    "name: CI\non:\n  push:\n    branches: [main, develop]\n  pull_request:\n    branches: [main]\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Blocker);
        assert!(results[0].description.contains("No release-capable"));
    }

    /// Test: `workflow_dispatch` with release input → release detected.
    #[tokio::test]
    async fn test_workflow_dispatch_with_release_input() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Manual Release",
                    "path": ".github/workflows/manual-release.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/manual-release.yml",
                    "badge_url": "https://github.com/test/test/workflows/Manual/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/manual-release.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Manual Release\non:\n  workflow_dispatch:\n    inputs:\n      release_version:\n        description: 'Release version'\n        required: true\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/upload-artifact@v4\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
        assert!(results[0].description.contains("Manual Release"));
    }

    /// Test: `workflow_dispatch` without release inputs → not release-capable.
    #[tokio::test]
    async fn test_workflow_dispatch_without_release_input_not_release() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Deploy",
                    "path": ".github/workflows/deploy.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/deploy.yml",
                    "badge_url": "https://github.com/test/test/workflows/Deploy/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/deploy.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Deploy\non:\n  workflow_dispatch:\n    inputs:\n      environment:\n        description: 'Deploy environment'\n        required: true\n        type: choice\n        options:\n          - staging\n          - production\njobs:\n  deploy:\n    runs-on: ubuntu-latest\n    steps:\n      - run: echo 'Deploying'\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        // workflow_dispatch without release keywords → not release-capable.
        assert_eq!(results[0].severity, Severity::Blocker);
    }

    /// Test: `release/*` pattern (with slash) detected.
    #[tokio::test]
    async fn test_release_slash_pattern_detected() {
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
                    "name: Release\non:\n  push:\n    tags:\n      - 'release/*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/upload-artifact@v4\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
    }

    /// Test: workflow with only `pull_request` trigger is not release-capable.
    #[tokio::test]
    async fn test_pr_only_trigger_not_release() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/workflows"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "total_count": 1,
                "workflows": [{
                    "id": 1,
                    "name": "Lint",
                    "path": ".github/workflows/lint.yml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/lint.yml",
                    "badge_url": "https://github.com/test/test/workflows/Lint/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/lint.yml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Lint\non:\n  pull_request:\n    branches: [main]\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Blocker);
    }

    /// Test: check name returns correct string.
    #[tokio::test]
    async fn test_check_name() {
        let check = ReleaseWorkflowCheck;
        assert_eq!(check.name(), "release_workflow");
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

        // CI workflow is readable.
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
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        // Should return Blocker since the only release workflow was unreadable.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].severity, Severity::Blocker);
        assert!(results[0].description.contains("No release-capable"));
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
                    "name": "Release",
                    "path": ".github/workflows/release.yaml",
                    "state": "active",
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "url": "https://api.github.com/repos/test/test/actions/workflows/1",
                    "html_url": "https://github.com/test/test/blob/main/.github/workflows/release.yaml",
                    "badge_url": "https://github.com/test/test/workflows/Release/badge.svg"
                }]
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path(
                "/repos/test-owner/test-repo/contents/.github/workflows/release.yaml",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "content": base64_encode(
                    "name: Release\non:\n  push:\n    tags:\n      - 'v*'\njobs:\n  release:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/upload-artifact@v4\n"
                ),
                "encoding": "base64"
            })))
            .mount(&server)
            .await;

        let client = make_client(&server);
        let check = ReleaseWorkflowCheck;
        let results = check.check(&client, OWNER, REPO).await.unwrap();

        assert_eq!(results[0].severity, Severity::Info);
    }

    // ─── Unit tests for detection helpers ──────────────────────────────

    #[test]
    fn test_is_release_workflow_push_tag_v_star() {
        let yaml = r#"
name: Release
on:
  push:
    tags:
      - 'v*'
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
"#;
        assert!(is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_push_tag_sema() {
        let yaml = r#"
name: Publish
on:
  push:
    tags:
      - '*.*.*'
jobs:
  build:
    runs-on: ubuntu-latest
"#;
        assert!(is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_push_tag_release_dash() {
        let yaml = r#"
name: Release
on:
  push:
    tags:
      - 'release-*'
jobs:
  release:
    runs-on: ubuntu-latest
"#;
        assert!(is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_push_tag_release_slash() {
        let yaml = r#"
name: Release
on:
  push:
    tags:
      - 'release/*'
jobs:
  release:
    runs-on: ubuntu-latest
"#;
        assert!(is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_branch_push_only() {
        let yaml = r#"
name: CI
on:
  push:
    branches: [main]
jobs:
  test:
    runs-on: ubuntu-latest
"#;
        assert!(!is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_pr_only() {
        let yaml = r#"
name: Lint
on:
  pull_request:
    branches: [main]
jobs:
  lint:
    runs-on: ubuntu-latest
"#;
        assert!(!is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_workflow_dispatch_with_release() {
        let yaml = r#"
name: Manual Release
on:
  workflow_dispatch:
    inputs:
      release_version:
        description: 'Release version'
jobs:
  release:
    runs-on: ubuntu-latest
"#;
        assert!(is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_is_release_workflow_workflow_dispatch_without_release_input() {
        let yaml = r#"
name: Deploy
on:
  workflow_dispatch:
    inputs:
      environment:
        description: 'Deploy environment'
jobs:
  deploy:
    runs-on: ubuntu-latest
"#;
        assert!(!is_release_workflow(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_artifact_upload_upload_artifact() {
        let yaml = "steps:\n  - uses: actions/upload-artifact@v4\n";
        assert!(has_artifact_upload_step(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_artifact_upload_gh_release_upload() {
        let yaml = "steps:\n  - run: gh release upload $GITHUB_REF dist/*\n";
        assert!(has_artifact_upload_step(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_artifact_upload_docker_push() {
        let yaml = "steps:\n  - run: docker push myimage:latest\n";
        assert!(has_artifact_upload_step(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_artifact_upload_aws_s3_cp() {
        let yaml = "steps:\n  - run: aws s3 cp dist/ s3://bucket/\n";
        assert!(has_artifact_upload_step(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_artifact_upload_aws_s3_sync() {
        let yaml = "steps:\n  - run: aws s3 sync dist/ s3://bucket/\n";
        assert!(has_artifact_upload_step(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_artifact_upload_no_artifacts() {
        let yaml = "steps:\n  - uses: actions/checkout@v4\n  - run: echo hello\n";
        assert!(!has_artifact_upload_step(&yaml.to_lowercase()));
    }

    #[test]
    fn test_has_matching_tag_pattern_v_star_quoted() {
        assert!(has_matching_tag_pattern("      - 'v*'\n"));
    }

    #[test]
    fn test_has_matching_tag_pattern_sema_quoted() {
        assert!(has_matching_tag_pattern("      - '*.*.*'\n"));
    }

    #[test]
    fn test_has_matching_tag_pattern_release_dash_quoted() {
        assert!(has_matching_tag_pattern("      - 'release-*'\n"));
    }

    #[test]
    fn test_has_matching_tag_pattern_release_slash_quoted() {
        assert!(has_matching_tag_pattern("      - 'release/*'\n"));
    }

    #[test]
    fn test_has_matching_tag_pattern_unquoted_v_star() {
        assert!(has_matching_tag_pattern("      - v*\n"));
    }

    #[test]
    fn test_has_matching_tag_pattern_no_match() {
        assert!(!has_matching_tag_pattern("      - 'feature/*'\n"));
    }
}
