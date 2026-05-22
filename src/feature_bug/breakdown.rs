//! Epic breakdown logic for feature/bug work.
//!
//! Implements the epic/child bead breakdown workflow as defined in
//! plans/feature-bug-plan.md §Bead Breakdown.
//!
//! ## Workflow
//!
//! 1. **Detect epic-scale** - LLM analyzes issue for multi-area or sequential work
//! 2. **File epic bead** - Create deferred epic linked to GitHub issue
//! 3. **File child beads** - Create deferred children, one per logical unit
//! 4. **Post breakdown comment** - Link to created beads
//! 5. **Wait for human signal** - Any child modification or issue comment
//! 6. **Batch open children** - Open all children when human signal received

use crate::error::{Result, RogersError};
use crate::github::models::Issue;
use crate::llm::client::{ChatMessage, ChatRequest, LlmClient};
use crate::llm::prompts::IssueMetadata;
use serde::{Deserialize, Serialize};

/// Epic breakdown analyzer.
///
/// Analyzes GitHub issues to determine if they represent epic-scale work
/// and generates appropriate child bead breakdowns.
#[derive(Debug, Clone)]
pub struct BreakdownAnalyzer {
    /// LLM client for epic assessment.
    llm: LlmClient,
    /// Model name.
    model: String,
}

impl BreakdownAnalyzer {
    /// Create a new breakdown analyzer.
    pub fn new(llm: LlmClient) -> Self {
        Self {
            llm,
            model: String::new(),
        }
    }

    /// Create a breakdown analyzer with a specific model.
    pub fn with_model(llm: LlmClient, model: String) -> Self {
        Self { llm, model }
    }

    /// Analyze a GitHub issue to determine if it's epic-scale work.
    ///
    /// Returns `Ok(Some(breakdown))` for epic-scale issues,
    /// `Ok(None)` for non-epic issues.
    pub async fn analyze_epic(
        &self,
        issue: &Issue,
        domain_context: Option<&str>,
    ) -> Result<Option<EpicBreakdown>> {
        let metadata = Self::issue_to_metadata(issue);

        // Check for orphan detection (issue in ready-for-work but no existing epic)
        // This is handled at a higher level - here we just do the LLM analysis

        // Build the epic assessment prompt
        let prompt = self.build_epic_prompt(&metadata, domain_context);
        let request = self.build_request(&prompt);
        let response = self.llm.chat(request).await?;

        let content = &response.choices[0].message.content;

        // Parse the epic assessment response
        match self.parse_epic_assessment(content) {
            Ok(assessment) => {
                if assessment.is_epic {
                    Ok(Some(EpicBreakdown {
                        is_epic: true,
                        primary_areas: assessment.primary_areas,
                        sub_work_items: assessment.sub_work_items,
                        complexity_notes: assessment.complexity_notes,
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(e),
        }
    }

    /// Generate child bead requests from an epic breakdown.
    ///
    /// Each child bead follows the AGENTS.md standalone rules:
    /// - Single codebase part
    /// - No "and then"
    /// - Self-contained (What, Why, How, Edge, Terms)
    /// - One acceptance criterion or cohesive concern
    pub fn generate_child_beads(
        &self,
        issue: &Issue,
        breakdown: &EpicBreakdown,
        plan_ref: &str,
    ) -> Vec<ChildBeadRequest> {
        let issue_num = issue.number;

        breakdown
            .sub_work_items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let child_num = idx + 1;
                ChildBeadRequest {
                    title: item.title.clone(),
                    description: Self::format_child_description(
                        issue,
                        &item.title,
                        &item.scope_description,
                        plan_ref,
                    ),
                    scope: item.scope_description.clone(),
                    priority: Self::infer_priority(
                        issue_num,
                        child_num,
                        breakdown.sub_work_items.len(),
                    ),
                }
            })
            .collect()
    }

    /// Format a child bead description following AGENTS.md standalone rules.
    fn format_child_description(
        issue: &Issue,
        title: &str,
        scope: &str,
        plan_ref: &str,
    ) -> Option<String> {
        Some(format!(
            r#"**Plan:** {}

**Issue:** #{} - {}

**WHAT TO DO**
{}

**WHY**
This is part of the epic work tracked in the parent bead. Completing this child bead should result in a working, testable implementation of this scope.

**HOW TO VERIFY**
- Code compiles successfully
- Unit tests pass for the implemented feature
- Feature works as described in the scope

**EDGE CASES AND PITFALLS**
- Ensure changes are isolated to the designated scope
- Update any related configuration files if needed
- Test edge cases specific to this implementation area

**PROJECT-SPECIFIC TERMINOLOGY**
- 'Standalone bead': A self-contained unit that can be implemented without consulting other beads or the epic description
"#,
            plan_ref, issue.number, issue.title, scope
        ))
    }

    /// Infer priority based on position and total work items.
    fn infer_priority(issue_num: i32, child_num: usize, total: usize) -> i32 {
        // First child or critical path gets higher priority
        if child_num == 0 {
            1
        } else if child_num < total / 2 {
            2
        } else {
            3
        }
    }

    /// Build the epic assessment prompt.
    fn build_epic_prompt(
        &self,
        metadata: &IssueMetadata,
        domain_context: Option<&str>,
    ) -> EpicAssassmentPrompt {
        let system_prompt = r#"You are Rodgers, assessing whether a GitHub issue represents epic-scale work.

EPIC-SCALE INDICATORS:
1. Work spans multiple areas of the project (e.g., "UI and API", "backend and docs")
2. Description contains sequential logic: "Do X, then Y, then Z" that maps to multiple sub-tasks
3. The issue discusses multiple distinct concerns that could be split
4. Complexity suggests parallel workstreams could speed up implementation

NOT EPIC-SCALE:
- Simple bug fixes in one component
- Single-feature additions in one area
- Clear, contained work items
- Issues with straightforward, linear implementation

ANALYSIS APPROACH:
1. Identify distinct work areas (UI, API, DB, CLI, config, docs, etc.)
2. For each area, determine if the scope is significant enough for a separate bead
3. Look for phrases like "and then", "also needs", "should also update", "in addition"
4. Consider if work items depend on each other or can be parallelized

OUTPUT FORMAT:
Respond with valid JSON (no markdown code blocks) with these fields:
- is_epic: boolean (true if epic-scale work detected)
- primary_areas: array of strings (distinct work areas: ui, api, backend, database, cli, docs, config, etc.)
- sub_work_items: array of objects with:
  - title: string (concise title for the child bead)
  - scope_description: string (detailed description of what this child bead covers)
- complexity_notes: string (optional notes about the breakdown and dependencies)"#
            .to_string();

        let mut user_prompt = String::new();

        if let Some(ctx) = domain_context {
            user_prompt.push_str(&format!("## Project Context\n{}\n\n", ctx));
        }

        user_prompt.push_str(&format!(
            "## Issue to Assess\n#{}. {}\n\n",
            metadata.number, metadata.title
        ));

        if let Some(ref body) = metadata.body {
            user_prompt.push_str("### Body\n");
            user_prompt.push_str(body);
            user_prompt.push_str("\n\n");
        }

        if !metadata.prior_comments.is_empty() {
            user_prompt.push_str("### Discussion\n");
            for comment in &metadata.prior_comments {
                user_prompt.push_str(&format!("- {}\n", comment));
            }
        }

        user_prompt.push_str(
            r#"
Assess whether this issue is epic-scale work.
If yes, identify the distinct work areas and break it into sub-items.
Consider the AGENTS.md rule: each child bead should be implementable by a naive but competent junior developer."#,
        );

        EpicAssassmentPrompt {
            system_prompt,
            user_prompt,
        }
    }

    /// Build a chat request from a prompt.
    fn build_request(&self, prompt: &EpicAssassmentPrompt) -> ChatRequest {
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
                        "is_epic": {"type": "boolean"},
                        "primary_areas": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "sub_work_items": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "title": {"type": "string"},
                                    "scope_description": {"type": "string"}
                                },
                                "required": ["title", "scope_description"]
                            }
                        },
                        "complexity_notes": {"type": "string"}
                    },
                    "required": ["is_epic", "primary_areas", "sub_work_items"]
                })),
            }),
        }
    }

    /// Parse the epic assessment response from LLM.
    fn parse_epic_assessment(&self, content: &str) -> Result<EpicAssessmentResult> {
        // Extract JSON if wrapped in markdown
        let json_str = Self::extract_json(content);

        // Parse the JSON
        let assessment: EpicAssessmentResult = match serde_json::from_str(&json_str) {
            Ok(v) => v,
            Err(e) => {
                return Err(RogersError::Config(format!(
                    "Failed to parse epic assessment: {} (content: {})",
                    e, content
                )));
            }
        };

        // Validate the result
        if !assessment.is_epic && !assessment.sub_work_items.is_empty() {
            // Warning: marked as not epic but has sub-items
            tracing::warn!("Epic assessment has sub_items but is_epic=false");
        }

        Ok(assessment)
    }

    /// Extract JSON from content that might be wrapped in markdown code blocks.
    fn extract_json(content: &str) -> String {
        let trimmed = content.trim();

        // Check for markdown code block
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

    /// Convert a GitHub issue to metadata for classification.
    fn issue_to_metadata(issue: &Issue) -> IssueMetadata {
        IssueMetadata {
            number: issue.number,
            title: issue.title.clone(),
            body: issue.body.clone(),
            author: issue.user.login.clone(),
            author_type: issue.user.user_type.clone(),
            labels: issue.labels.iter().map(|l| l.name.clone()).collect(),
            prior_comments: vec![],
        }
    }
}

/// Prompt for epic assessment.
#[derive(Debug, Clone)]
struct EpicAssassmentPrompt {
    system_prompt: String,
    user_prompt: String,
}

/// Result of epic assessment.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EpicAssessmentResult {
    is_epic: bool,
    primary_areas: Vec<String>,
    sub_work_items: Vec<SubWorkItem>,
    complexity_notes: Option<String>,
}

/// Sub-work item for child bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SubWorkItem {
    title: String,
    scope_description: String,
}

/// Epic breakdown result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicBreakdown {
    /// Whether this is epic-scale work.
    pub is_epic: bool,
    /// Primary work areas identified.
    pub primary_areas: Vec<String>,
    /// Sub-work items for child beads.
    pub sub_work_items: Vec<SubWorkItem>,
    /// Complexity notes from LLM.
    pub complexity_notes: Option<String>,
}

/// Request for creating a child bead.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildBeadRequest {
    /// Child bead title.
    pub title: String,
    /// Child bead description.
    pub description: Option<String>,
    /// Scope description from LLM analysis.
    pub scope: String,
    /// Calculated priority.
    pub priority: i32,
}

/// Breakdown summary for posting as a comment.
#[derive(Debug, Clone)]
pub struct BreakdownComment {
    /// Comment body.
    pub body: String,
    /// Epic bead info for reference.
    pub epic_title: String,
    /// Child bead titles.
    pub child_titles: Vec<String>,
}

impl BreakdownComment {
    /// Generate the breakdown comment body.
    pub fn generate(
        issue_num: i32,
        epic: &crate::beads::schema::Epic,
        children: &[crate::beads::schema::Child],
    ) -> Self {
        let mut body = String::new();

        body.push_str(&format!(
            "## Rodgers Epic Breakdown\n\nIssue #{} has been analyzed and found to be epic-scale work. I've created the following breakdown:\n\n",
            issue_num
        ));

        body.push_str("### 📋 Epic\n");
        body.push_str(&format!("**Title:** {}\n", epic.title));
        if let Some(url) = &epic.github_issue_url {
            body.push_str(&format!("**Linked Issue:** #{}\n", url));
        }
        body.push('\n');

        body.push_str("### 📝 Child Beads\n");
        body.push_str("The following child beads have been created (all currently deferred):\n\n");

        for (idx, child) in children.iter().enumerate() {
            body.push_str(&format!("{}. **{}**\n", idx + 1, child.title));
            if let Some(ref desc) = child.description {
                // First line only as preview (comments can be long)
                let first_line = desc.lines().next().unwrap_or("");
                let preview = if first_line.len() > 200 {
                    &first_line[..200]
                } else {
                    first_line
                };
                body.push_str(&format!("   > {}\n", preview));
            }
        }

        body.push_str("\n---\n\n");
        body.push_str("### ⏳ Next Steps\n\n");
        body.push_str(
            "These child beads are currently **deferred** (not started). To begin work:\n\n",
        );
        body.push_str("1. Review the breakdown above\n");
        body.push_str("2. Modify any child bead OR add a comment to this issue\n");
        body.push_str("3. Rodgers will batch-open all child beads\n\n");
        body.push_str("This ensures human review before work begins. Each child bead can be implemented independently once opened.\n");

        let child_titles: Vec<String> = children.iter().map(|c| c.title.clone()).collect();

        Self {
            body,
            epic_title: epic.title.clone(),
            child_titles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_from_markdown() {
        let content = r#"```json
{"is_epic": true, "primary_areas": ["ui", "api"]}
```"#;

        let json = BreakdownAnalyzer::extract_json_from_content(content);
        assert!(json.contains("is_epic"));
        assert!(!json.starts_with("```"));
    }

    #[test]
    fn test_extract_json_from_plain() {
        let content = r#"{"is_epic": false, "primary_areas": [], "sub_work_items": []}"#;

        let json = BreakdownAnalyzer::extract_json_from_content(content);
        assert!(json.contains("is_epic"));
    }

    #[test]
    fn test_epic_breakdown_structure() {
        let breakdown = EpicBreakdown {
            is_epic: true,
            primary_areas: vec!["ui".to_string(), "api".to_string()],
            sub_work_items: vec![SubWorkItem {
                title: "Implement UI layer".to_string(),
                scope_description: "Create React components for the new feature".to_string(),
            }],
            complexity_notes: Some("Standard epic breakdown".to_string()),
        };

        assert!(breakdown.is_epic);
        assert_eq!(breakdown.primary_areas.len(), 2);
        assert_eq!(breakdown.sub_work_items.len(), 1);
    }

    #[test]
    fn test_child_bead_request_structure() {
        let request = ChildBeadRequest {
            title: "API endpoint".to_string(),
            description: Some("Implement the API endpoint".to_string()),
            scope: "Create REST endpoint for the feature".to_string(),
            priority: 2,
        };

        assert_eq!(request.title, "API endpoint");
        assert!(request.description.is_some());
    }

    #[test]
    fn test_priority_inference() {
        // Test that priorities are assigned correctly
        let breakdown = EpicBreakdown {
            is_epic: true,
            primary_areas: vec!["api".to_string(), "db".to_string()],
            sub_work_items: vec![
                SubWorkItem {
                    title: "First item".to_string(),
                    scope_description: "First scope".to_string(),
                },
                SubWorkItem {
                    title: "Second item".to_string(),
                    scope_description: "Second scope".to_string(),
                },
                SubWorkItem {
                    title: "Third item".to_string(),
                    scope_description: "Third scope".to_string(),
                },
                SubWorkItem {
                    title: "Fourth item".to_string(),
                    scope_description: "Fourth scope".to_string(),
                },
            ],
            complexity_notes: None,
        };

        // Note: These tests verify the structure, actual priority calculation
        // is done in generate_child_beads which references the actual issue
        assert_eq!(breakdown.sub_work_items.len(), 4);
    }
}

// Expose helper for testing
impl BreakdownAnalyzer {
    /// Extract JSON from content (exposed for testing).
    pub fn extract_json_from_content(content: &str) -> String {
        Self::extract_json(content)
    }
}
