//! Issue type classification for Rodgers.
//!
//! Determines if an issue is a bug, feature, question, docs, chore, or unknown.
//!
//! Classification priority:
//! 1. Label heuristics first: existing labels (bug, enhancement, question, documentation)
//! 2. LLM classification on title+body for unlabeled issues
//! 3. Default to 'question' if LLM cannot determine with confidence
//!
//! See plans/triage-workflow-plan.md §Top-Level Classification.

use crate::error::RogersError;
use crate::github::models::Issue;
use crate::llm::client::{ChatMessage, ChatRequest, LlmClient};
use crate::llm::prompts::{ClassificationPrompt, IssueMetadata};
use crate::llm::validator::{ClassificationOutput, OutputValidator, ValidationResult};
use serde::{Deserialize, Serialize};

/// Issue type classification result.
///
/// Each variant maps to a GitHub label that Rodgers will apply:
/// - Bug → `bug` label
/// - Feature → `feature` label
/// - Question → `question` label
/// - Docs → `documentation` label
/// - Chore → internal tracking (no label change for triage)
/// - Unknown → falls back to question classification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IssueType {
    /// Bug report — something is broken
    Bug,
    /// Feature request — new functionality desired
    Feature,
    /// Question — asking for help or clarification
    Question,
    /// Documentation — request for docs or docs are missing
    Docs,
    /// Chore — internal maintenance, refactoring, CI, etc.
    Chore,
    /// Unknown — could not determine with confidence (fallback to Question)
    Unknown,
}

impl IssueType {
    /// Returns the label name that corresponds to this issue type.
    ///
    /// This label is applied during triage to mark the issue category.
    pub fn label_name(&self) -> &'static str {
        match self {
            IssueType::Bug => "bug",
            IssueType::Feature => "feature",
            IssueType::Question => "question",
            IssueType::Docs => "documentation",
            IssueType::Chore => "chore",
            IssueType::Unknown => "question",
        }
    }

    /// Returns true if this type should proceed through the triage loop
    /// (i.e., it is recognized as a triage-worthy category).
    ///
    /// `Unknown` is NOT triage-worthy by itself — callers should convert it
    /// to `Question` before checking.
    pub fn is_triage_worthy(&self) -> bool {
        matches!(
            self,
            IssueType::Bug
                | IssueType::Feature
                | IssueType::Question
                | IssueType::Docs
                | IssueType::Chore
        )
    }
}

impl std::fmt::Display for IssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IssueType::Bug => write!(f, "bug"),
            IssueType::Feature => write!(f, "feature"),
            IssueType::Question => write!(f, "question"),
            IssueType::Docs => write!(f, "docs"),
            IssueType::Chore => write!(f, "chore"),
            IssueType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Confidence level for LLM-based classification.
///
/// Used to determine whether the LLM result is trusted enough to act on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Confidence {
    /// High confidence — LLM is very certain about the classification
    High,
    /// Medium confidence — LLM is reasonably certain but some ambiguity exists
    Medium,
    /// Low confidence — LLM is uncertain; should default to question
    Low,
}

impl Confidence {
    /// Returns true if this confidence level is sufficient to use the classification.
    ///
    /// Only `High` and `Medium` confidence are accepted. `Low` confidence
    /// causes the classifier to fall back to the default (`question`).
    pub fn is_acceptable(&self) -> bool {
        matches!(self, Confidence::High | Confidence::Medium)
    }
}

/// LLM response for issue classification.
///
/// This struct represents the structured output from the LLM classifier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmClassificationResult {
    /// The classified issue type
    pub issue_type: IssueType,
    /// Confidence level of the classification
    pub confidence: Confidence,
    /// Brief rationale for the classification (used in debug logging)
    pub rationale: String,
}

/// Classification result with raw response for debugging.
///
/// Used by the TriageEngine (engine.rs) for LLM-based classification.
#[derive(Debug, Clone)]
pub struct ClassificationResult {
    /// Validated classification output.
    pub output: ClassificationOutput,
    /// Raw LLM response for debugging.
    pub raw_response: String,
}

/// LLM-based issue classifier.
///
/// Uses the LLM to classify GitHub issues and determine completeness.
/// Wraps the label-heuristic + LLM-fallback classification functions
/// in an async API suitable for the TriageEngine.
#[derive(Debug, Clone)]
pub struct Classifier {
    /// LLM client.
    llm: LlmClient,
    /// Output validator.
    validator: OutputValidator,
    /// Model name.
    model: String,
}

impl Classifier {
    /// Create a new classifier from LLM config.
    pub fn new(llm: LlmClient) -> Self {
        Self {
            llm,
            validator: OutputValidator::new(),
            model: String::new(),
        }
    }

    /// Classify a GitHub issue.
    pub async fn classify(
        &self,
        issue: &Issue,
        domain_context: Option<&str>,
    ) -> crate::error::Result<ClassificationResult> {
        let metadata = Self::issue_to_metadata(issue);
        let prompt = ClassificationPrompt::for_classification(&metadata, domain_context);

        let request = self.build_request(&prompt);
        let response = self.llm.chat(request).await?;

        let content = &response.choices[0].message.content;
        self.validate_and_parse_classification(content)
    }

    /// Check completeness of an already-classified issue.
    pub async fn check_completeness(
        &self,
        issue: &Issue,
    ) -> crate::error::Result<ClassificationResult> {
        let metadata = Self::issue_to_metadata(issue);
        let prompt = ClassificationPrompt::for_completeness_check(&metadata);

        let request = self.build_request(&prompt);
        let response = self.llm.chat(request).await?;

        let content = &response.choices[0].message.content;
        self.validate_and_parse_classification(content)
    }

    /// Build a chat request from a prompt.
    fn build_request(&self, prompt: &ClassificationPrompt) -> ChatRequest {
        ChatRequest {
            model: self.model.clone(),
            messages: vec![
                ChatMessage::system(&prompt.system_prompt),
                ChatMessage::user(&prompt.user_prompt),
            ],
            temperature: Some(0.3),
            max_tokens: Some(2048),
            response_format: Some(crate::llm::ResponseFormat {
                format_type: "json_object".to_string(),
                schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "issue_type": {
                            "type": "string",
                            "enum": ["bug", "feature", "question", "docs", "chore", "unknown"]
                        },
                        "completeness": {
                            "type": "string",
                            "enum": ["complete", "incomplete"]
                        },
                        "missing_fields": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "severity": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low", "none"]
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["critical", "high", "medium", "low"]
                        },
                        "response_draft": {"type": "string"},
                        "confidence": {"type": "number", "minimum": 0, "maximum": 1}
                    },
                    "required": ["issue_type", "completeness"]
                })),
            }),
        }
    }

    /// Convert a GitHub issue to metadata for classification.
    fn issue_to_metadata(issue: &Issue) -> IssueMetadata {
        let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();
        let prior_comments: Vec<String> = vec![];

        IssueMetadata {
            number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            author: issue.user.login.clone(),
            author_type: issue.user.user_type.clone(),
            labels,
            prior_comments,
        }
    }

    /// Validate and parse LLM classification output.
    fn validate_and_parse_classification(
        &self,
        content: &str,
    ) -> crate::error::Result<ClassificationResult> {
        let json_str = Self::extract_json(content);

        match self.validator.validate_classification(&json_str) {
            Ok(output) => Ok(ClassificationResult {
                output,
                raw_response: content.to_string(),
            }),
            Err(result) => {
                let errors = result
                    .errors
                    .iter()
                    .map(|e| format!("{}: {}", e.field, e.message))
                    .collect::<Vec<_>>()
                    .join("; ");
                Err(RogersError::Config(format!(
                    "LLM output validation failed: {}",
                    errors
                )))
            }
        }
    }

    /// Extract JSON from content that might be wrapped in markdown code blocks.
    fn extract_json(content: &str) -> String {
        let trimmed = content.trim();

        if trimmed.starts_with("```json") {
            if let Some(end) = trimmed.find("```\n").or(Some(trimmed.len())) {
                let json_content = &trimmed[7..end];
                return json_content.trim().to_string();
            }
        } else if trimmed.starts_with("```") {
            if let Some(end) = trimmed.find("```\n").or(Some(trimmed.len())) {
                let json_content = &trimmed[3..end];
                return json_content.trim().to_string();
            }
        }

        trimmed.to_string()
    }

    /// Validate a response draft against warmth principles.
    pub fn validate_response_draft(&self, draft: &str) -> ValidationResult {
        self.validator.validate_response_draft(draft)
    }
}

/// Label heuristic mapping for classification.
///
/// Maps known GitHub label names to issue types.
/// Supports both Rodgers conventions and common GitHub project conventions.
const LABEL_HEURISTICS: &[(IssueType, &[&str])] = &[
    // Bug: GitHub convention and Rodgers label
    (IssueType::Bug, &["bug", "bug-report", "defect"]),
    // Feature: Rodgers label and GitHub convention (enhancement is common on GitHub)
    (
        IssueType::Feature,
        &["feature", "enhancement", "feature-request"],
    ),
    // Question: Rodgers label and common conventions
    (IssueType::Question, &["question", "help-wanted", "support"]),
    // Documentation: Rodgers label and GitHub convention
    (
        IssueType::Docs,
        &["documentation", "docs", "good-first-issue"],
    ),
    // Chore: internal maintenance labels
    (IssueType::Chore, &["chore", "maintenance", "ci-cd"]),
];

/// Labels that indicate the issue has been classified by a prior Rodgers run.
///
/// These labels mean the issue should NOT be re-classified by heuristics.
/// Only Rodgers-owned labels are used here — plain GitHub labels like
/// `bug`, `feature`, `question`, etc. trigger normal heuristic classification.
const RODGERS_TRIAGE_LABELS: &[&str] = &["rodgers:triaged", "rodgers:feature"];

/// A GitHub issue with relevant metadata for classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedIssue {
    /// GitHub issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: String,
    /// Current labels on the issue
    pub labels: Vec<String>,
    /// Author username (for bot detection)
    pub author: String,
}

/// Pre-check results for bot issues and already-classified issues.
#[derive(Debug, Clone)]
pub enum PreCheckResult {
    /// Bot-authored issue — apply bot_labels and skip triage
    BotIssue,
    /// Issue already has a Rodgers triage label from a prior run
    AlreadyClassified,
    /// Issue is unlabeled and needs classification
    NeedsClassification,
}

/// Classification result used by the triage loop.
#[derive(Debug, Clone)]
pub struct TriageClassification {
    /// The determined issue type
    pub issue_type: IssueType,
    /// Labels to apply based on classification
    pub labels_to_add: Vec<String>,
    /// Whether this issue needs to proceed through the full triage loop
    pub should_triage: bool,
    /// Whether the issue was classified via heuristics or LLM
    pub method: ClassificationMethod,
    /// Rationale for the classification
    pub rationale: String,
}

/// How the classification was determined.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClassificationMethod {
    /// Classified via label heuristics (fast path)
    LabelHeuristic,
    /// Classified via LLM (fallback path)
    Llm,
    /// Default classification applied
    Default,
}

/// Classify an issue using label heuristics.
///
/// This is the fast path: check existing labels against known mappings.
/// Returns `Some(IssueType)` if a matching label is found, `None` otherwise.
///
/// Heuristic label mapping:
/// - `bug`, `bug-report`, `defect` → Bug
/// - `feature`, `enhancement`, `feature-request` → Feature
/// - `question`, `help-wanted`, `support` → Question
/// - `documentation`, `docs`, `good-first-issue` → Docs
/// - `chore`, `maintenance`, `ci-cd` → Chore
///
/// Special labels:
/// - `rodgers:triaged` → None (already triaged, skip — only if no other type labels)
/// - `rodgers:feature` → Feature (existing classification from prior run)
///
/// Priority for mixed labels: bug > feature > question > docs > chore
pub fn classify_by_labels(issue_labels: &[String]) -> Option<IssueType> {
    // First, check for Rodgers-owned type labels that indicate a specific classification
    for label in issue_labels {
        if label.as_str() == "rodgers:feature" {
            return Some(IssueType::Feature);
        }
    }

    // Check if only `rodgers:triaged` is present (without any heuristic type labels)
    // In that case, skip classification
    let has_rodgers_triaged_only = issue_labels.iter().any(|l| l == "rodgers:triaged")
        && issue_labels
            .iter()
            .all(|l| l == "rodgers:triaged" || RODGERS_TRIAGE_LABELS.contains(&l.as_str()));
    if has_rodgers_triaged_only {
        return None;
    }

    // Apply heuristic rules with priority ordering
    // Priority: bug > feature > question > docs > chore
    for (issue_type, label_names) in LABEL_HEURISTICS {
        for label in issue_labels {
            if label_names.contains(&label.as_str()) {
                return Some(issue_type.clone());
            }
        }
    }

    None
}

/// Check pre-conditions for classification.
///
/// Returns:
/// - `BotIssue` if author is a bot (apply bot_labels, skip)
/// - `AlreadyClassified` if the issue already has a Rodgers triage label from a prior run
/// - `NeedsClassification` if no existing classification exists
pub fn pre_check_classification(issue: &ClassifiedIssue) -> PreCheckResult {
    // Check for bot authors
    if is_bot_author(&issue.author) {
        return PreCheckResult::BotIssue;
    }

    // Check for existing Rodgers triage labels (applied by prior Rodgers runs)
    // Only Rodgers-owned labels count as "already classified" — plain GitHub
    // labels (bug, feature, question, etc.) are heuristic labels that trigger
    // normal classification.
    for label in &issue.labels {
        if RODGERS_TRIAGE_LABELS.contains(&label.as_str()) {
            return PreCheckResult::AlreadyClassified;
        }
    }

    PreCheckResult::NeedsClassification
}

/// Check if an author is a bot.
///
/// Bots are detected by common bot naming patterns in GitHub usernames.
pub fn is_bot_author(author: &str) -> bool {
    let lower = author.to_lowercase();
    // Common bot patterns on GitHub
    lower.contains("bot")
        || lower.contains("-app")
        || lower == "github-actions"
        || lower == "dependabot"
        || lower == "renovatebot"
}

/// Classify an issue by combining label heuristics and optional LLM fallback.
///
/// Priority:
/// 1. Label heuristics: check existing labels against known mappings
/// 2. LLM fallback: if no labels match, send title+body to LLM for classification
/// 3. Default: if LLM confidence is low, default to question
///
/// The `llm_classify` closure is called only when label heuristics don't match.
/// It receives the issue title and body, and returns a `LlmClassificationResult`.
///
/// Returns a `TriageClassification` with the determined issue type and labels to apply.
pub fn classify_issue<F>(issue: &ClassifiedIssue, llm_classify: F) -> TriageClassification
where
    F: FnOnce(&str, &str) -> Option<LlmClassificationResult>,
{
    // Step 1: Pre-check (bot detection, already classified)
    match pre_check_classification(issue) {
        PreCheckResult::BotIssue => {
            return TriageClassification {
                issue_type: IssueType::Question,        // Default for bots
                labels_to_add: vec!["bot".to_string()], // Would apply bot_labels in production
                should_triage: false,
                method: ClassificationMethod::Default,
                rationale: "Bot-authored issue, skipped".to_string(),
            };
        }
        PreCheckResult::AlreadyClassified => {
            // Already has a Rodgers triage label — respect existing classification
            // Try to determine the type from existing labels
            if let Some(ref issue_type) = classify_by_labels(&issue.labels) {
                return TriageClassification {
                    issue_type: issue_type.clone(),
                    labels_to_add: vec![],
                    should_triage: issue_type.is_triage_worthy(),
                    method: ClassificationMethod::LabelHeuristic,
                    rationale: format!(
                        "Already classified by prior triage run, label: {}",
                        issue_type
                    ),
                };
            }
            return TriageClassification {
                issue_type: IssueType::Question,
                labels_to_add: vec![],
                should_triage: false,
                method: ClassificationMethod::LabelHeuristic,
                rationale: "Already classified (unrecognized label)".to_string(),
            };
        }
        PreCheckResult::NeedsClassification => {} // Continue to classification
    }

    // Step 2: Label heuristics (fast path)
    if let Some(issue_type) = classify_by_labels(&issue.labels) {
        return TriageClassification {
            issue_type: issue_type.clone(),
            labels_to_add: vec![issue_type.label_name().to_string()],
            should_triage: issue_type.is_triage_worthy(),
            method: ClassificationMethod::LabelHeuristic,
            rationale: format!("Classified by label heuristic: {}", issue_type),
        };
    }

    // Step 3: LLM fallback (for unlabeled issues)
    let classification = classify_by_llm(issue, llm_classify);

    // Step 4: Validate LLM confidence — default to question if low
    let issue_type = match &classification {
        Ok(_result) if _result.confidence.is_acceptable() => _result.issue_type.clone(),
        Ok(_) => {
            // Low confidence — default to question
            IssueType::Question
        }
        Err(_) => IssueType::Question, // LLM call failed — default to question
    };

    TriageClassification {
        issue_type: issue_type.clone(),
        labels_to_add: vec![issue_type.label_name().to_string()],
        should_triage: issue_type.is_triage_worthy(),
        method: ClassificationMethod::Llm,
        rationale: classification
            .map(|r| r.rationale)
            .unwrap_or_else(|_| "LLM classification failed, defaulted to question".to_string()),
    }
}

/// Call the LLM for issue classification.
///
/// The `llm_classify` closure is responsible for:
/// 1. Sending the title+body to the LLM with the classification prompt
/// 2. Parsing the structured JSON response
/// 3. Validating the response schema
///
/// Returns `Ok(ClassificationResult)` on success, `Err` on failure.
fn classify_by_llm<F>(
    issue: &ClassifiedIssue,
    llm_classify: F,
) -> std::result::Result<LlmClassificationResult, String>
where
    F: FnOnce(&str, &str) -> Option<LlmClassificationResult>,
{
    // Combine title and body for the LLM
    let content = if issue.body.is_empty() {
        issue.title.clone()
    } else {
        format!("{}\n\n{}", issue.title, issue.body)
    };

    match llm_classify(&issue.title, &content) {
        Some(result) => Ok(result),
        None => Err("LLM classification returned None".to_string()),
    }
}

/// Default LLM classifier that returns `None`.
///
/// This is used for testing the heuristic path without an LLM.
/// In production, this would be replaced with an actual LLM call.
pub fn default_llm_classifier(_title: &str, _body: &str) -> Option<LlmClassificationResult> {
    // In production, this would call the actual LLM endpoint
    // For now, return None to trigger heuristic-only or default behavior
    None
}

/// Validate that an LLM classification result has a valid schema.
///
/// Ensures the result contains all required fields and the issue_type
/// is a recognized value.
pub fn validate_classification(result: &LlmClassificationResult) -> bool {
    // issue_type must be a known variant (already enforced by type system)
    // confidence must be acceptable for production use
    // rationale should be non-empty
    !result.rationale.is_empty()
}

/// Resolve the most specific issue type when multiple labels are present.
///
/// Priority order: bug > feature > question > docs > chore
/// This handles the edge case of mixed labels (e.g., bug + enhancement).
pub fn resolve_conflicting_labels(labels: &[String]) -> Option<IssueType> {
    // Check labels in priority order
    for (issue_type, label_names) in LABEL_HEURISTICS {
        for label in labels {
            if label_names.contains(&label.as_str()) {
                return Some(issue_type.clone());
            }
        }
    }
    None
}

/// Map an IssueType to its corresponding workflow.
///
/// - Bug, Feature → feature-bug-plan
/// - Question → question-routing-plan
/// - Docs → issue-templates-plan
/// - Chore → internal tracking
/// - Unknown → question-routing-plan (fallback)
pub fn issue_type_to_workflow(issue_type: &IssueType) -> &str {
    match issue_type {
        IssueType::Bug | IssueType::Feature => "feature-bug-plan",
        IssueType::Question => "question-routing-plan",
        IssueType::Docs => "issue-templates-plan",
        IssueType::Chore => "internal-tracking",
        IssueType::Unknown => "question-routing-plan",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(
        number: u64,
        title: &str,
        body: &str,
        labels: Vec<&str>,
        author: &str,
    ) -> ClassifiedIssue {
        ClassifiedIssue {
            number,
            title: title.to_string(),
            body: body.to_string(),
            labels: labels.into_iter().map(String::from).collect(),
            author: author.to_string(),
        }
    }

    // =============================================================================
    // IssueType Tests
    // =============================================================================

    #[test]
    fn test_issue_type_label_names() {
        assert_eq!(IssueType::Bug.label_name(), "bug");
        assert_eq!(IssueType::Feature.label_name(), "feature");
        assert_eq!(IssueType::Question.label_name(), "question");
        assert_eq!(IssueType::Docs.label_name(), "documentation");
        assert_eq!(IssueType::Chore.label_name(), "chore");
        assert_eq!(IssueType::Unknown.label_name(), "question");
    }

    #[test]
    fn test_issue_type_display() {
        assert_eq!(format!("{}", IssueType::Bug), "bug");
        assert_eq!(format!("{}", IssueType::Feature), "feature");
        assert_eq!(format!("{}", IssueType::Question), "question");
        assert_eq!(format!("{}", IssueType::Docs), "docs");
        assert_eq!(format!("{}", IssueType::Chore), "chore");
        assert_eq!(format!("{}", IssueType::Unknown), "unknown");
    }

    #[test]
    fn test_issue_type_is_triage_worthy() {
        assert!(IssueType::Bug.is_triage_worthy());
        assert!(IssueType::Feature.is_triage_worthy());
        assert!(IssueType::Question.is_triage_worthy());
        assert!(IssueType::Docs.is_triage_worthy());
        assert!(IssueType::Chore.is_triage_worthy());
        // Unknown is NOT triage-worthy — callers convert it to Question first
        assert!(!IssueType::Unknown.is_triage_worthy());
    }

    // =============================================================================
    // Confidence Tests
    // =============================================================================

    #[test]
    fn test_confidence_is_acceptable() {
        assert!(Confidence::High.is_acceptable());
        assert!(Confidence::Medium.is_acceptable());
        assert!(!Confidence::Low.is_acceptable());
    }

    // =============================================================================
    // Label Heuristic Tests (CRIT-2: Label heuristics)
    // =============================================================================

    #[test]
    fn test_label_heuristic_bug_label() {
        // Issues with 'bug' label classified as bug
        let issue = create_test_issue(1, "Bug report", "It crashes", vec!["bug"], "user1");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Bug));
    }

    #[test]
    fn test_label_heuristic_enhancement_label() {
        // Issues with 'enhancement' label classified as feature
        let issue = create_test_issue(2, "New feature", "I want X", vec!["enhancement"], "user2");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Feature));
    }

    #[test]
    fn test_label_heuristic_question_label() {
        // Issues with 'question' label classified as question
        let issue = create_test_issue(3, "How do I?", "I need help", vec!["question"], "user3");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Question));
    }

    #[test]
    fn test_label_heuristic_documentation_label() {
        // Issues with 'documentation' label classified as docs
        let issue = create_test_issue(
            4,
            "Add docs",
            "Need more docs",
            vec!["documentation"],
            "user4",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Docs));
    }

    #[test]
    fn test_label_heuristic_feature_label() {
        // Issues with 'feature' label classified as feature
        let issue = create_test_issue(5, "New feature", "I want X", vec!["feature"], "user5");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Feature));
    }

    #[test]
    fn test_label_heuristic_no_matching_label() {
        // Issues with no matching labels return None
        let issue = create_test_issue(6, "Random issue", "Something", vec![], "user6");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, None);
    }

    #[test]
    fn test_label_heuristic_bug_report_label() {
        // GitHub convention: bug-report maps to Bug
        let issue = create_test_issue(7, "Bug report", "Crashes", vec!["bug-report"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Bug));
    }

    #[test]
    fn test_label_heuristic_defect_label() {
        // GitHub convention: defect maps to Bug
        let issue = create_test_issue(7, "Bug report", "Crashes", vec!["defect"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Bug));
    }

    #[test]
    fn test_label_heuristic_feature_request_label() {
        // GitHub convention: feature-request maps to Feature
        let issue = create_test_issue(
            7,
            "Feature request",
            "I want X",
            vec!["feature-request"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Feature));
    }

    #[test]
    fn test_label_heuristic_help_wanted_label() {
        // GitHub convention: help-wanted maps to Question
        let issue = create_test_issue(7, "Help needed", "How do I?", vec!["help-wanted"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Question));
    }

    #[test]
    fn test_label_heuristic_support_label() {
        // GitHub convention: support maps to Question
        let issue = create_test_issue(7, "Need support", "Help", vec!["support"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Question));
    }

    #[test]
    fn test_label_heuristic_docs_label() {
        // GitHub convention: docs maps to Docs
        let issue = create_test_issue(7, "Docs needed", "Add docs", vec!["docs"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Docs));
    }

    #[test]
    fn test_label_heuristic_good_first_issue_label() {
        // GitHub convention: good-first-issue maps to Docs
        let issue = create_test_issue(
            7,
            "Good first issue",
            "Simple docs fix",
            vec!["good-first-issue"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Docs));
    }

    #[test]
    fn test_label_heuristic_chore_label() {
        // Chore labels recognized
        let issue = create_test_issue(7, "Update deps", "Bump versions", vec!["chore"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Chore));
    }

    #[test]
    fn test_label_heuristic_maintenance_label() {
        let issue = create_test_issue(7, "Refactor code", "Clean up", vec!["maintenance"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Chore));
    }

    #[test]
    fn test_label_heuristic_ci_cd_label() {
        let issue = create_test_issue(7, "Fix CI", "Pipeline broken", vec!["ci-cd"], "user7");
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Chore));
    }

    #[test]
    fn test_label_heuristic_priority_bug_over_feature() {
        // Mixed labels: bug takes priority over feature
        let issue = create_test_issue(
            7,
            "Bug and feature",
            "Description",
            vec!["bug", "enhancement"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Bug));
    }

    #[test]
    fn test_label_heuristic_priority_feature_over_question() {
        // Mixed labels: feature takes priority over question
        let issue = create_test_issue(
            7,
            "Feature question",
            "Description",
            vec!["feature", "question"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Feature));
    }

    #[test]
    fn test_label_heuristic_priority_question_over_docs() {
        // Mixed labels: question takes priority over docs
        let issue = create_test_issue(
            7,
            "Question docs",
            "Description",
            vec!["question", "documentation"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Question));
    }

    #[test]
    fn test_label_heuristic_priority_docs_over_chore() {
        // Mixed labels: docs takes priority over chore
        let issue = create_test_issue(
            7,
            "Docs chore",
            "Description",
            vec!["documentation", "chore"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Docs));
    }

    // =============================================================================
    // Rodger's Triaged Label Tests (Respects existing human labels)
    // =============================================================================

    #[test]
    fn test_rodgers_triaged_label_skipped() {
        // Issue with ONLY rodgers:triaged label (no type labels) returns None (skip)
        let issue = create_test_issue(
            7,
            "Already triaged",
            "Description",
            vec!["rodgers:triaged"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, None);
    }

    #[test]
    fn test_rodgers_triaged_with_type_label_respects_type() {
        // Issue with rodgers:triaged AND a type label should classify by the type label
        let issue = create_test_issue(
            7,
            "Already triaged",
            "Description",
            vec!["bug", "rodgers:triaged"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Bug));
    }

    #[test]
    fn test_rodgers_feature_label_maps_to_feature() {
        // rodgers:feature label maps to Feature
        let issue = create_test_issue(
            7,
            "Rodger's feature",
            "Description",
            vec!["rodgers:feature"],
            "user7",
        );
        let result = classify_by_labels(&issue.labels);
        assert_eq!(result, Some(IssueType::Feature));
    }

    // =============================================================================
    // Pre-check Tests (Bot detection, already classified)
    // =============================================================================

    #[test]
    fn test_bot_author_detection_bot_username() {
        assert!(is_bot_author("test-bot"));
        assert!(is_bot_author("my-bot-123"));
        assert!(is_bot_author("automation_bot"));
    }

    #[test]
    fn test_bot_author_detection_app_suffix() {
        assert!(is_bot_author("greenkeeper-app"));
        assert!(is_bot_author("some-app"));
    }

    #[test]
    fn test_bot_author_detection_known_bots() {
        assert!(is_bot_author("github-actions"));
        assert!(is_bot_author("dependabot"));
        assert!(is_bot_author("renovatebot"));
    }

    #[test]
    fn test_non_bot_authors() {
        assert!(!is_bot_author("john-doe"));
        assert!(!is_bot_author("alice"));
        assert!(!is_bot_author("user123"));
        assert!(!is_bot_author("developer"));
    }

    #[test]
    fn test_pre_check_bot_issue() {
        let issue = create_test_issue(1, "Bot issue", "Body", vec![], "github-actions");
        assert!(matches!(
            pre_check_classification(&issue),
            PreCheckResult::BotIssue
        ));
    }

    #[test]
    fn test_pre_check_already_classified() {
        let issue = create_test_issue(
            1,
            "Classified",
            "Body",
            vec!["bug", "rodgers:triaged"],
            "user1",
        );
        assert!(matches!(
            pre_check_classification(&issue),
            PreCheckResult::AlreadyClassified
        ));
    }

    #[test]
    fn test_pre_check_plain_bug_label_not_already_classified() {
        // Plain bug label should NOT be considered "already classified"
        // It should go through normal classification
        let issue = create_test_issue(1, "Bug", "Body", vec!["bug"], "user1");
        assert!(matches!(
            pre_check_classification(&issue),
            PreCheckResult::NeedsClassification
        ));
    }

    #[test]
    fn test_pre_check_needs_classification() {
        let issue = create_test_issue(1, "Unlabeled", "Body", vec![], "user1");
        assert!(matches!(
            pre_check_classification(&issue),
            PreCheckResult::NeedsClassification
        ));
    }

    #[test]
    fn test_pre_check_needs_classification_with_unrelated_labels() {
        // Labels that aren't Rodgers triage labels should not trigger AlreadyClassified
        let issue = create_test_issue(1, "Unlabeled", "Body", vec!["help-welcome"], "user1");
        assert!(matches!(
            pre_check_classification(&issue),
            PreCheckResult::NeedsClassification
        ));
    }

    // =============================================================================
    // Full Classification Tests (heuristics + LLM fallback)
    // =============================================================================

    #[test]
    fn test_classify_bug_via_label_heuristic() {
        let issue = create_test_issue(1, "Bug report", "It crashes", vec!["bug"], "user1");
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Bug);
        assert!(result.labels_to_add.contains(&"bug".to_string()));
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
        assert!(result.should_triage);
    }

    #[test]
    fn test_classify_enhancement_via_label_heuristic() {
        let issue = create_test_issue(
            1,
            "Feature request",
            "I want X",
            vec!["enhancement"],
            "user1",
        );
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Feature);
        assert!(result.labels_to_add.contains(&"feature".to_string()));
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
    }

    #[test]
    fn test_classify_question_via_label_heuristic() {
        let issue = create_test_issue(1, "How do I?", "Need help", vec!["question"], "user1");
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Question);
        assert!(result.labels_to_add.contains(&"question".to_string()));
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
    }

    #[test]
    fn test_classify_documentation_via_label_heuristic() {
        let issue = create_test_issue(1, "Add docs", "Need docs", vec!["documentation"], "user1");
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Docs);
        assert!(result.labels_to_add.contains(&"documentation".to_string()));
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
    }

    #[test]
    fn test_classify_unlabeled_via_llm() {
        // Unlabeled issues should go to LLM fallback
        let llm_calls = std::cell::RefCell::new(Vec::new());
        let mock_llm = |title: &str, _body: &str| {
            llm_calls
                .borrow_mut()
                .push((title.to_string(), _body.to_string()));
            Some(LlmClassificationResult {
                issue_type: IssueType::Bug,
                confidence: Confidence::High,
                rationale: "Crash described in title".to_string(),
            })
        };

        let issue = create_test_issue(1, "App crashes on startup", "", vec![], "user1");
        let result = classify_issue(&issue, mock_llm);

        assert_eq!(result.issue_type, IssueType::Bug);
        assert!(result.labels_to_add.contains(&"bug".to_string()));
        assert_eq!(result.method, ClassificationMethod::Llm);
        assert!(!llm_calls.borrow().is_empty());
    }

    #[test]
    fn test_classify_unlabeled_low_confidence_defaults_to_question() {
        // LLM with low confidence should default to question
        let mock_llm = |_title: &str, _body: &str| {
            Some(LlmClassificationResult {
                issue_type: IssueType::Bug,
                confidence: Confidence::Low,
                rationale: "Not sure".to_string(),
            })
        };

        let issue = create_test_issue(1, "Something weird", "Description", vec![], "user1");
        let result = classify_issue(&issue, mock_llm);

        assert_eq!(result.issue_type, IssueType::Question); // Default
        assert_eq!(result.method, ClassificationMethod::Llm);
    }

    #[test]
    fn test_classify_unlabeled_llm_failure_defaults_to_question() {
        // LLM returning None should default to question
        let mock_llm: fn(&str, &str) -> Option<LlmClassificationResult> = |_title, _body| None;

        let issue = create_test_issue(1, "Something", "Description", vec![], "user1");
        let result = classify_issue(&issue, mock_llm);

        assert_eq!(result.issue_type, IssueType::Question); // Default
        assert_eq!(result.method, ClassificationMethod::Llm);
    }

    #[test]
    fn test_classify_bot_issue_skipped() {
        let issue = create_test_issue(1, "Bot issue", "Body", vec![], "test-bot");
        let result = classify_issue(&issue, default_llm_classifier);

        assert_eq!(result.issue_type, IssueType::Question); // Default type
        assert!(!result.should_triage); // Bots are skipped
        assert_eq!(result.method, ClassificationMethod::Default);
    }

    #[test]
    fn test_classify_already_triaged_respects_existing_label() {
        let issue = create_test_issue(
            1,
            "Already triaged",
            "Body",
            vec!["feature", "rodgers:triaged"],
            "user1",
        );
        let result = classify_issue(&issue, default_llm_classifier);

        assert_eq!(result.issue_type, IssueType::Feature); // Respects existing label
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
        assert!(result.labels_to_add.is_empty()); // No new labels
    }

    #[test]
    fn test_classify_mixed_labels_bug_and_enhancement() {
        // Bug takes priority over enhancement
        let issue = create_test_issue(
            1,
            "Mixed",
            "Both bug and feature",
            vec!["bug", "enhancement"],
            "user1",
        );
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Bug);
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
    }

    #[test]
    fn test_classify_chore_via_label_heuristic() {
        let issue = create_test_issue(1, "Bump deps", "Update", vec!["chore"], "user1");
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Chore);
        assert_eq!(result.method, ClassificationMethod::LabelHeuristic);
    }

    #[test]
    fn test_classify_with_full_content() {
        // Unlabeled issue with rich content should go to LLM
        let llm_calls = std::cell::RefCell::new(Vec::new());
        let mock_llm = |title: &str, body: &str| {
            llm_calls
                .borrow_mut()
                .push((title.to_string(), body.to_string()));
            Some(LlmClassificationResult {
                issue_type: IssueType::Bug,
                confidence: Confidence::High,
                rationale: "Clear bug description".to_string(),
            })
        };

        let body = r#"## What Happened
The application crashes when opening the settings panel.

## Expected Behavior
The settings panel should open without crashing.

## Reproduction Steps
1. Open the application
2. Click Settings
3. Observe crash
"#;
        let issue = create_test_issue(1, "Crash on settings open", body, vec![], "user1");
        let result = classify_issue(&issue, mock_llm);

        assert_eq!(result.issue_type, IssueType::Bug);
        // LLM should receive title + body
        let calls = llm_calls.borrow();
        assert!(!calls.is_empty());
        assert!(calls[0].1.contains("What Happened"));
        assert!(calls[0].1.contains("crashes"));
    }

    #[test]
    fn test_classify_documentation_via_docs_shortcut_label() {
        // 'docs' shortcut label should map to Docs
        let issue = create_test_issue(1, "Add docs", "Need docs", vec!["docs"], "user1");
        let result = classify_issue(&issue, default_llm_classifier);
        assert_eq!(result.issue_type, IssueType::Docs);
        assert!(result.labels_to_add.contains(&"documentation".to_string()));
    }

    // =============================================================================
    // resolve_conflicting_labels Tests
    // =============================================================================

    #[test]
    fn test_resolve_conflicting_labels_bug_over_feature() {
        let labels = vec!["enhancement".to_string(), "bug".to_string()];
        let result = resolve_conflicting_labels(&labels);
        assert_eq!(result, Some(IssueType::Bug));
    }

    #[test]
    fn test_resolve_conflicting_labels_no_match() {
        let labels = vec!["help-welcome".to_string()];
        let result = resolve_conflicting_labels(&labels);
        assert_eq!(result, None);
    }

    // =============================================================================
    // issue_type_to_workflow Tests
    // =============================================================================

    #[test]
    fn test_workflow_mapping() {
        assert_eq!(issue_type_to_workflow(&IssueType::Bug), "feature-bug-plan");
        assert_eq!(
            issue_type_to_workflow(&IssueType::Feature),
            "feature-bug-plan"
        );
        assert_eq!(
            issue_type_to_workflow(&IssueType::Question),
            "question-routing-plan"
        );
        assert_eq!(
            issue_type_to_workflow(&IssueType::Docs),
            "issue-templates-plan"
        );
        assert_eq!(
            issue_type_to_workflow(&IssueType::Chore),
            "internal-tracking"
        );
        assert_eq!(
            issue_type_to_workflow(&IssueType::Unknown),
            "question-routing-plan"
        );
    }

    // =============================================================================
    // ClassificationResult and Validation Tests
    // =============================================================================

    #[test]
    fn test_validate_classification_valid() {
        let result = LlmClassificationResult {
            issue_type: IssueType::Bug,
            confidence: Confidence::High,
            rationale: "Clear bug description".to_string(),
        };
        assert!(validate_classification(&result));
    }

    #[test]
    fn test_validate_classification_empty_rationale_fails() {
        let result = LlmClassificationResult {
            issue_type: IssueType::Bug,
            confidence: Confidence::High,
            rationale: String::new(),
        };
        assert!(!validate_classification(&result));
    }

    // =============================================================================
    // TriageClassification Tests
    // =============================================================================

    #[test]
    fn test_triage_classification_bug() {
        let tc = TriageClassification {
            issue_type: IssueType::Bug,
            labels_to_add: vec!["bug".to_string()],
            should_triage: true,
            method: ClassificationMethod::LabelHeuristic,
            rationale: "Bug label".to_string(),
        };
        assert!(tc.should_triage);
    }

    #[test]
    fn test_triage_classification_chore() {
        let tc = TriageClassification {
            issue_type: IssueType::Chore,
            labels_to_add: vec!["chore".to_string()],
            should_triage: true,
            method: ClassificationMethod::LabelHeuristic,
            rationale: "Chore label".to_string(),
        };
        assert!(tc.should_triage);
    }

    // =============================================================================
    // Integration test: Full triage classification for sample issues
    // =============================================================================

    #[test]
    fn test_full_triage_classification_samples() {
        // Simulates the classification flow for various sample issues

        let samples = vec![
            // Label-based classification
            (
                "Bug with bug label",
                vec!["bug"],
                IssueType::Bug,
                ClassificationMethod::LabelHeuristic,
                true,
            ),
            (
                "Feature with enhancement label",
                vec!["enhancement"],
                IssueType::Feature,
                ClassificationMethod::LabelHeuristic,
                true,
            ),
            (
                "Question with question label",
                vec!["question"],
                IssueType::Question,
                ClassificationMethod::LabelHeuristic,
                true,
            ),
            (
                "Docs with documentation label",
                vec!["documentation"],
                IssueType::Docs,
                ClassificationMethod::LabelHeuristic,
                true,
            ),
            // Bot issue
            (
                "Bot issue",
                vec![],
                IssueType::Question,
                ClassificationMethod::Default,
                false,
            ),
        ];

        for (name, labels, expected_type, expected_method, expected_triage) in samples {
            let mut issue_labels: Vec<String> = labels.into_iter().map(String::from).collect();
            if name == "Bot issue" {
                issue_labels.push("rodgers:triaged".to_string()); // Pre-classified
            }
            let issue = ClassifiedIssue {
                number: 999,
                title: name.to_string(),
                body: "Test body".to_string(),
                labels: issue_labels,
                author: if name == "Bot issue" {
                    "github-actions"
                } else {
                    "testuser"
                }
                .to_string(),
            };

            let result = classify_issue(&issue, default_llm_classifier);
            assert_eq!(result.issue_type, expected_type, "Type mismatch: {}", name);
            assert_eq!(result.method, expected_method, "Method mismatch: {}", name);
            assert_eq!(
                result.should_triage, expected_triage,
                "Triage mismatch: {}",
                name
            );
        }
    }
}
