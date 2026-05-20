//! Classification prompts for LLM triage.
//!
//! Provides structured prompts for issue classification, completeness checking,
//! and response drafting following the Fred Rogers warmth principle.

use serde::{Deserialize, Serialize};

/// Classification prompt with context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationPrompt {
    /// System prompt for the LLM.
    pub system_prompt: String,
    /// User prompt for classification.
    pub user_prompt: String,
}

/// Represents issue metadata for classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueMetadata {
    /// Issue number.
    pub number: i32,
    /// Issue title.
    pub title: String,
    /// Issue body.
    pub body: Option<String>,
    /// Author login.
    pub author: String,
    /// Author type (User/Bot).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author_type: Option<String>,
    /// Existing labels.
    pub labels: Vec<String>,
    /// Prior comments.
    pub prior_comments: Vec<String>,
}

/// Bug report requirements.
const BUG_REQUIREMENTS: &[&str] = &[
    "behavior observed - what happened that seems wrong",
    "behavior expected - what should have happened instead",
    "reproduction steps - clear steps to reproduce (or N/A with explanation)",
    "environment - OS, version, relevant context",
];

/// Feature request requirements.
const FEATURE_REQUIREMENTS: &[&str] = &[
    "use case - why this feature is needed (the problem it solves)",
    "proposed behavior - how the feature should work",
    "acceptance criteria - testable enumerated list of success conditions",
];

impl ClassificationPrompt {
    /// Create a classification prompt for a new issue.
    pub fn for_classification(metadata: &IssueMetadata, domain_context: Option<&str>) -> Self {
        let system_prompt = Self::classification_system_prompt();
        let user_prompt = Self::classification_user_prompt(metadata, domain_context);

        Self {
            system_prompt,
            user_prompt,
        }
    }

    /// System prompt for classification.
    fn classification_system_prompt() -> String {
        r#"You are Rodgers, a github-native community relations agent named after Fred Rogers.
Your role is to classify GitHub issues and determine if they have complete information.

CLASSIFICATION RULES:
- Classify the issue as one of: bug, feature, question, docs, chore, unknown
- A bug report describes unexpected behavior that seems wrong
- A feature request asks for new capability or behavioral change
- A question asks for information or clarification
- docs is for documentation gaps or update requests
- chore is for maintenance, tooling, or meta issues
- unknown is for issues that don't fit other categories

COMPLETENESS CHECK:
- Bug reports require: behavior observed, behavior expected, reproduction steps, environment
- Feature requests require: use case, proposed behavior, acceptance criteria
- Questions may require clarification if too vague

RESPONSE DRAFTING (Fred Rogers warmth principle):
- Be warm, patient, and genuine
- Lead with gratitude and acknowledgment of the requestor's effort
- Never sound dismissive, curt, or performatively helpful
- Never use "as previously stated", "please refer to the documentation", etc.
- Use phrases like "thanks for reaching out", "you might find this helpful"

OUTPUT FORMAT:
Respond with valid JSON (no markdown code blocks) with these fields:
- issue_type: string (bug|feature|question|docs|chore|unknown)
- completeness: string (complete|incomplete)
- missing_fields: array of strings (required fields that are missing, empty if complete)
- severity: string (optional, for bug|feature: critical|high|medium|low|none)
- priority: string (optional, for bug|feature: critical|high|medium|low)
- response_draft: string (optional, a warm comment to post on the issue)
- confidence: number (0.0 to 1.0, how confident you are in this classification)"#
            .to_string()
    }

    /// User prompt for classification.
    fn classification_user_prompt(
        metadata: &IssueMetadata,
        domain_context: Option<&str>,
    ) -> String {
        let mut prompt = String::new();

        // Domain context if provided
        if let Some(ctx) = domain_context {
            prompt.push_str(&format!("## Project Context\n{}\n\n", ctx));
        }

        // Issue metadata
        prompt.push_str("## Issue Information\n");
        prompt.push_str(&format!("- Number: #{}\n", metadata.number));
        prompt.push_str(&format!("- Title: {}\n", metadata.title));

        if let Some(ref body) = metadata.body {
            prompt.push_str("- Body:\n```\n");
            prompt.push_str(body);
            prompt.push_str("\n```\n");
        }

        prompt.push_str(&format!(
            "- Author: @{} ({})\n",
            metadata.author,
            metadata.author_type.as_deref().unwrap_or("User")
        ));
        prompt.push_str(&format!(
            "- Existing labels: {}\n",
            metadata.labels.join(", ")
        ));

        if !metadata.prior_comments.is_empty() {
            prompt.push_str("\n## Prior Comments\n");
            for (i, comment) in metadata.prior_comments.iter().enumerate() {
                prompt.push_str(&format!("{}. {}\n", i + 1, comment));
            }
        }

        prompt.push_str(
            r#"
CLASSIFY THIS ISSUE:
1. What type is this: bug, feature, question, docs, chore, or unknown?
2. Does it have complete information for its type?
3. If incomplete, what specific information is missing?
4. What severity/priority should this have (if bug/feature)?
5. Draft a warm response comment (if action is needed).

Respond with JSON only."#,
        );

        prompt
    }

    /// Create a completeness check prompt for an existing issue.
    pub fn for_completeness_check(metadata: &IssueMetadata) -> Self {
        let system_prompt = Self::completeness_system_prompt();
        let user_prompt = Self::completeness_user_prompt(metadata);

        Self {
            system_prompt,
            user_prompt,
        }
    }

    /// System prompt for completeness checking.
    fn completeness_system_prompt() -> String {
        let bug_reqs: String = BUG_REQUIREMENTS
            .iter()
            .map(|r| format!("- {}", r))
            .collect::<Vec<_>>()
            .join("\n");

        let feature_reqs: String = FEATURE_REQUIREMENTS
            .iter()
            .map(|r| format!("- {}", r))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"You are Rodgers, evaluating whether an issue has complete information.

COMPLETENESS REQUIREMENTS:

### Bug Reports require ALL of:
{}

### Feature Requests require ALL of:
{}

Respond with JSON (no markdown):
- completeness: "complete" or "incomplete"
- missing_fields: array of specific field names that are missing
- severity: (bug only) critical|high|medium|low|none
- priority: (bug/feature only) critical|high|medium|low
- response_draft: (if incomplete) warm comment requesting specific missing information"#,
            bug_reqs, feature_reqs
        )
    }

    /// User prompt for completeness check.
    fn for_completeness_user_prompt_body(metadata: &IssueMetadata) -> String {
        let mut prompt = String::new();
        prompt.push_str(&format!(
            "Issue #{}: {}\n\n",
            metadata.number, metadata.title
        ));

        if let Some(ref body) = metadata.body {
            prompt.push_str("## Issue Body\n");
            prompt.push_str(body);
            prompt.push_str("\n\n");
        }

        prompt.push_str(&format!("Labels: {}\n", metadata.labels.join(", ")));
        prompt.push_str(&format!("Author: @{}\n", metadata.author));

        if !metadata.prior_comments.is_empty() {
            prompt.push_str("\n## Comments\n");
            for comment in &metadata.prior_comments {
                prompt.push_str(&format!("- {}\n", comment));
            }
        }

        prompt.push_str(
            r#"
Check if this issue has complete information for its type.
List only the specific missing fields."#,
        );

        prompt
    }

    /// Completeness user prompt.
    fn completeness_user_prompt(metadata: &IssueMetadata) -> String {
        Self::for_completeness_user_prompt_body(metadata)
    }

    /// Create a response draft prompt for closing/will-not-do.
    pub fn for_response_draft(
        metadata: &IssueMetadata,
        intent: &str,
        context: Option<&str>,
    ) -> Self {
        let system_prompt = Self::response_draft_system_prompt(intent);
        let user_prompt = Self::response_draft_user_prompt(metadata, intent, context);

        Self {
            system_prompt,
            user_prompt,
        }
    }

    /// System prompt for response drafting.
    fn response_draft_system_prompt(intent: &str) -> String {
        let base = r#"You are Rodgers, drafting warm, respectful GitHub comments.
You are named after Fred Rogers - the man who found quiet, genuine compassion compelling.

TONE GUIDE:
- Be warm, patient, and genuine
- Lead with gratitude before any redirect
- Acknowledge effort before redirecting
- Never sound dismissive, curt, or performatively helpful
- Avoid patterns that sound cold:

| Instead of... | Write... |
|---------------|----------|
| "As previously stated..." | "To restate what you shared..." |
| "Please refer to the documentation." | "You might find this helpful — I've linked the relevant doc above." |
| "This is not a bug." | "After looking into this, this might be expected behavior — here's why..." |
| "We cannot pursue this." | "Thank you for this suggestion. We've decided not to move forward..." |
| "Why did you file this without the template?" | "Thanks for reaching out! Would you help me with a few quick details?" |

OUTPUT FORMAT:
Respond with valid JSON:
- response_draft: string (the complete comment body, including greetings and closings)
- warmth_score: number (0.0 to 1.0, self-assessed warmth of draft)
"#;

        let intent_desc = match intent {
            "close_stale" => {
                "Closing an issue that has received no response after needs-information was applied."
            }
            "will_not_do" => "Closing an issue that was decided not to be worked on.",
            "doc_answer" => "Answering a question with documentation.",
            "code_answer" => "Answering a question based on source code analysis.",
            "incomplete" => "Requesting specific missing information from the requestor.",
            "doc_gap_ack" => "Acknowledging a documentation gap and promising follow-up.",
            _ => "General response.",
        };

        format!("{}\n\nINTENT: {}", base, intent_desc)
    }

    /// User prompt for response drafting.
    fn response_draft_user_prompt(
        metadata: &IssueMetadata,
        intent: &str,
        context: Option<&str>,
    ) -> String {
        let mut prompt = String::new();

        if let Some(ctx) = context {
            prompt.push_str(&format!("## Context\n{}\n\n", ctx));
        }

        prompt.push_str(&format!("Issue #{}: {}\n", metadata.number, metadata.title));

        if let Some(ref body) = metadata.body {
            prompt.push_str("## Body\n");
            prompt.push_str(body);
            prompt.push_str("\n");
        }

        prompt.push_str(&format!("Author: @{}\n", metadata.author));
        prompt.push_str(&format!("Intent: {}\n", intent));

        prompt.push_str(
            r#"
Draft a warm comment for this GitHub issue.
The comment should:
- Address the requestor respectfully
- Provide clear next steps or explanations
- Match the intent specified above"#,
        );

        prompt
    }

    /// Create an epic assessment prompt for ready-for-work issues.
    pub fn for_epic_assessment(metadata: &IssueMetadata, domain_context: Option<&str>) -> Self {
        let system_prompt = Self::epic_assessment_system_prompt();
        let user_prompt = Self::epic_assessment_user_prompt(metadata, domain_context);

        Self {
            system_prompt,
            user_prompt,
        }
    }

    /// System prompt for epic assessment.
    fn epic_assessment_system_prompt() -> String {
        r#"You are Rodgers, assessing whether a GitHub issue represents epic-scale work.

EPIC-SCALE INDICATORS:
1. Work spans multiple areas of the project (e.g., "UI and API", "backend and docs")
2. Description contains sequential logic: "Do X, then Y, then Z" that maps to multiple sub-tasks
3. The issue discusses multiple distinct concerns that could be split

NOT EPIC-SCALE:
- Simple bug fixes in one component
- Single-feature additions in one area
- Clear, contained work items

OUTPUT FORMAT:
Respond with JSON (no markdown):
- is_epic: boolean
- primary_areas: array of strings (e.g., ["frontend", "backend", "docs"])
- sub_work_items: array of objects (title, scope_description)
- complexity_notes: string (optional notes about the breakdown)"#
            .to_string()
    }

    /// User prompt for epic assessment.
    fn epic_assessment_user_prompt(
        metadata: &IssueMetadata,
        domain_context: Option<&str>,
    ) -> String {
        let mut prompt = String::new();

        if let Some(ctx) = domain_context {
            prompt.push_str(&format!("## Project Context\n{}\n\n", ctx));
        }

        prompt.push_str(&format!(
            "## Issue to Assess\n#{}. {}\n\n",
            metadata.number, metadata.title
        ));

        if let Some(ref body) = metadata.body {
            prompt.push_str("### Body\n");
            prompt.push_str(body);
            prompt.push_str("\n\n");
        }

        if !metadata.prior_comments.is_empty() {
            prompt.push_str("### Discussion\n");
            for comment in &metadata.prior_comments {
                prompt.push_str(&format!("- {}\n", comment));
            }
        }

        prompt.push_str(
            r#"
Assess whether this issue is epic-scale work.
If yes, identify the distinct work areas and break it into sub-items."#,
        );

        prompt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_metadata() -> IssueMetadata {
        IssueMetadata {
            number: 123,
            title: "Test Issue".to_string(),
            body: Some("This is a test issue body.".to_string()),
            author: "testuser".to_string(),
            author_type: Some("User".to_string()),
            labels: vec!["bug".to_string()],
            prior_comments: vec![],
        }
    }

    #[test]
    fn test_classification_prompt_format() {
        let metadata = create_test_metadata();
        let prompt = ClassificationPrompt::for_classification(&metadata, None);

        assert!(!prompt.system_prompt.is_empty());
        assert!(!prompt.user_prompt.is_empty());
        assert!(prompt.user_prompt.contains("Test Issue"));
        assert!(prompt.user_prompt.contains("123"));
    }

    #[test]
    fn test_classification_prompt_with_context() {
        let metadata = create_test_metadata();
        let context = "This is a Rust project for GitHub automation.";
        let prompt = ClassificationPrompt::for_classification(&metadata, Some(context));

        assert!(prompt.user_prompt.contains("Rust"));
    }

    #[test]
    fn test_completeness_prompt() {
        let metadata = create_test_metadata();
        let prompt = ClassificationPrompt::for_completeness_check(&metadata);

        assert!(prompt.user_prompt.contains("Test Issue"));
        assert!(prompt.system_prompt.contains("completeness"));
    }

    #[test]
    fn test_response_draft_prompt_incomplete() {
        let metadata = create_test_metadata();
        let prompt = ClassificationPrompt::for_response_draft(&metadata, "incomplete", None);

        assert!(prompt.system_prompt.contains("warm"));
        assert!(prompt.user_prompt.contains("incomplete"));
    }

    #[test]
    fn test_response_draft_prompt_with_context() {
        let metadata = create_test_metadata();
        let context = "Missing: reproduction steps and environment.";
        let prompt =
            ClassificationPrompt::for_response_draft(&metadata, "incomplete", Some(context));

        assert!(prompt.user_prompt.contains("reproduction"));
    }

    #[test]
    fn test_epic_assessment_prompt() {
        let metadata = IssueMetadata {
            number: 456,
            title: "Large Epic Feature".to_string(),
            body: Some("Do X, then Y, then Z.".to_string()),
            author: "testuser".to_string(),
            author_type: None,
            labels: vec![],
            prior_comments: vec![],
        };
        let prompt = ClassificationPrompt::for_epic_assessment(&metadata, None);

        assert!(prompt.system_prompt.contains("epic"));
        assert!(prompt.user_prompt.contains("456"));
    }

    #[test]
    fn test_issue_metadata_serialization() {
        let metadata = create_test_metadata();
        let json = serde_json::to_string(&metadata).unwrap();
        let parsed: IssueMetadata = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.number, 123);
        assert_eq!(parsed.title, "Test Issue");
        assert_eq!(parsed.author, "testuser");
    }
}
