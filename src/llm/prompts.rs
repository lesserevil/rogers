//! Classification prompts for LLM triage.
//!
//! Provides structured prompts for issue classification, completeness checking,
//! and response drafting following the Fred Rogers warmth principle.

use serde::{Deserialize, Serialize};

/// Classification prompt with context.
/// Prompt for extracting fields from a bug report freeform description.
///
/// Returns structured information about which required fields are present.
pub const BUG_FIELD_EXTRACTION_PROMPT: &str = r#"You are analyzing a bug report. Identify which of the following required fields are present in the issue content.

Required fields for a complete bug report:
1. **Behavior observed** - A description of what happened that is wrong
2. **Behavior expected** - A description of what the reporter expected
3. **Reproduction steps** - Steps to reproduce the issue (or N/A with justification)
4. **Environment** - OS, version, hardware, browser, etc.

Analyze the issue content and respond with a JSON object indicating which fields are present.
Return ONLY the JSON object, no additional text.

Example response:
{"behavior_observed": true, "behavior_expected": true, "reproduction_steps": false, "environment": false}

Issue content:
{issue_content}
"#;

/// Prompt for extracting fields from a feature request freeform description.
///
/// Returns structured information about which required fields are present.
pub const FEATURE_FIELD_EXTRACTION_PROMPT: &str = r#"You are analyzing a feature request. Identify which of the following required fields are present in the issue content.

Required fields for a complete feature request:
1. **Use case** - Why the requester needs this feature (the problem they are solving)
2. **Proposed behavior** - How the feature should work once implemented
3. **Acceptance criteria** - How the feature would be verified (testable, enumerated list)

Analyze the issue content and respond with a JSON object indicating which fields are present.
Return ONLY the JSON object, no additional text.

Example response:
{"use_case": true, "proposed_behavior": true, "acceptance_criteria": false}

Issue content:
{issue_content}
"#;

/// Prompt for generating specific requests for missing bug report fields.
pub const BUG_MISSING_FIELDS_REQUEST_PROMPT: &str = r#"The following required fields are missing from this bug report:
{missing_fields}

Generate a friendly, specific request comment asking the user to provide ONLY the missing fields.
Do NOT request fields that are already present. Do NOT use generic phrases like "please provide more details".
Ask specifically for each missing field with a brief explanation of why it's needed.

Bug missing fields:
{bug_missing}

Example for missing environment and reproduction_steps:
"Thanks for the report! To help us reproduce this issue, could you provide:
- **Reproduction steps**: How can we reproduce what you're seeing?
- **Environment**: What OS, version, and relevant context are you using?"

Respond with ONLY the comment text.
"#;

/// Prompt for generating specific requests for missing feature request fields.
pub const FEATURE_MISSING_FIELDS_REQUEST_PROMPT: &str = r#"The following required fields are missing from this feature request:
{missing_fields}

Generate a friendly, specific request comment asking the user to provide ONLY the missing fields.
Do NOT request fields that are already present. Do NOT use generic phrases like "please provide more details".
Ask specifically for each missing field with a brief explanation of why it's needed.

Feature missing fields:
{feature_missing}

Example for missing use_case and acceptance_criteria:
"Thanks for the feature suggestion! To help us evaluate this, could you provide:
- **Use case**: Why do you need this feature? What problem are you solving?
- **Acceptance criteria**: How would you verify this feature works correctly? (Please provide a testable list)"

Respond with ONLY the comment text.
"#;

/// Prompt for generating a warm closure comment when an issue is declined (will-not-do).
///
/// The comment should:
/// - Express gratitude for the report/request
/// - Politely explain the decision not to pursue
/// - Be warm and respectful, NOT curt or dismissive
/// - Never just say "no" or "we won't do this"
pub const WARM_CLOSURE_PROMPT: &str = r#"You are writing a closure comment for a GitHub issue that will not be pursued.

Generate a warm, empathetic comment that:
1. Thanks the requestor for taking the time to report/submit this issue
2. Explains that after consideration, this will not be worked on at this time
3. Expresses regret that we cannot address this right now
4. Leaves the door open for future consideration

TONE: Warm, grateful, respectful. This person took time to file an issue - acknowledge that.
DO NOT USE: Curt phrases like "not a priority", "we won't implement this", or just "no"

Example good response:
"Thanks @username for the detailed feature request! I appreciate you taking the time to outline this use case.

After careful consideration, we're unable to prioritize this at the moment. The team has weighed this against other planned work and has decided not to move forward with this specific request.

We apologize for not being able to address this for you. If circumstances change in the future or you have other ideas, please don't hesitate to open a new issue.

Thanks again for contributing to the project!"

Issue details:
- Title: {issue_title}
- Author: @{issue_author}
- Type: {issue_type}

Respond with ONLY the comment text (no preamble or explanation).
"#;

/// Prompt for analyzing whether an issue requires epic-scale breakdown.
///
/// Epic-scale issues span multiple codebase areas or have sequential dependencies,
/// requiring breakdown into an epic bead + child beads. Standard work can be
/// handled as a single epic bead.
///
/// Epic-scale indicators:
/// - Multiple distinct codebase areas (CLI, API, DB, UI, config)
/// - Sequential dependencies ("and then...", step 1, step 2, etc.)
/// - Multiple logically distinct acceptance criteria groups
pub const EPIC_SCALE_ANALYSIS_PROMPT: &str = r#"You are analyzing a GitHub issue to determine whether it requires epic-scale breakdown.

An issue is epic-scale when it involves:
1. **Multiple codebase areas** - CLI, UI, API, database, configuration, auth, etc.
2. **Sequential dependencies** - work that must be done in phases, "and then..." patterns
3. **Multiple distinct units** - different logical concerns that could be worked on separately

Standard (single epic) issues:
- Describe work in one codebase area
- Can be described without "and then"
- One logical unit of acceptance criteria

Analyze the issue and respond with a JSON object:
{"is_epic_scale": true/false, "reasons": [...], "child_beads": [{"title": "...", "description": "..."}]}

If is_epic_scale is true, provide one child_beads entry per distinct unit of work.
Each child_beads title should indicate the codebase area it touches.
Do NOT provide more than 5 child beads - group if needed.

Issue content:
{issue_content}
"#;

/// Prompt for breaking down an epic-scale issue into child bead specifications.
///
/// Given an issue determined to be epic-scale, generate specific child bead
/// titles and descriptions following the two rules:
/// 1. **Single codebase part** - One entry per area (CLI, API, DB, UI, config)
/// 2. **No "...and then..." scope** - Each bead fits in one non-compound sentence
pub const EPIC_BREAKDOWN_PROMPT: &str = r#"You are breaking down an epic-scale GitHub issue into child bead specifications.

Each child bead must follow two rules:
1. **Single codebase part.** Touches at most one distinct area: CLI, UI, API, database, config, auth, etc.
2. **No "...and then..." scope.** Description fits in one non-compound sentence. If it naturally continues with "and then...", split into separate beads.

Generate child bead specifications as a JSON array:
[
  {"title": "Area: Short description of this unit", "description": "Concrete scope: what this bead does specifically", "priority": 2}
]

Maximum 5 child beads. Priority: 0=critical, 1=high, 2=medium, 3=low.
Group related work into a single bead rather than splitting finely.

Issue title: {issue_title}
Issue body: {issue_body}

Respond with ONLY the JSON array, no preamble.
"#;

/// Prompt for generating a standalone child bead description with all required sections.
///
/// A standalone bead is one that a naive but competent junior developer can implement
/// without consulting other beads or the epic description. Each bead MUST include:
/// 1. **WHAT TO DO** - Concrete files, packages, functions, or commands to create/modify
/// 2. **WHY** - User-visible behavior, constraint, or design rule this serves
/// 3. **HOW TO VERIFY** - Test, command, or observable result that proves work is done
/// 4. **EDGE CASES AND PITFALLS** - Non-obvious constraints a careful reader could miss
/// 5. **PROJECT-SPECIFIC TERMINOLOGY** - Project terms explained inline
pub const STANDALONE_BEAD_PROMPT: &str = r#"Generate a standalone child bead description for implementation.

A standalone bead provides ALL context needed for a naive but competent junior developer
to implement it WITHOUT consulting other beads or the parent epic.

REQUIRED SECTIONS (write all 5):
1. **WHAT TO DO**: Name concrete files, packages, functions, and commands to create or modify.
2. **WHY**: Explain the user-visible behavior, constraint, or design rule this serves.
3. **HOW TO VERIFY**: Specify the test, command, or observable result that proves work is done.
4. **EDGE CASES AND PITFALLS**: Non-obvious constraints a careful reader could miss.
5. **PROJECT-SPECIFIC TERMINOLOGY**: Define project-specific terms inline.

RULES:
- Single codebase part only (CLI OR API OR DB OR UI OR Config OR Auth)
- No "and then..." patterns - each bead scope should fit in one non-compound sentence
- Write for a naive junior dev who can write code and run tools but hasn't read the plan

FORMAT your response as a JSON object:
{
  "title": "Area: Brief description (e.g., 'API: User profile endpoint')",
  "description": "Full standalone description with all 5 sections formatted as markdown"
}

Bead scope: {bead_scope}
Codebase area: {codebase_area}
Acceptance criteria context: {ac_context}

Respond with ONLY the JSON object, no preamble or explanation.
"#;

/// Prompt for validating that a child bead description is standalone-ready.
///
/// This prompt helps an LLM validate that generated beads meet standalone criteria:
/// - All 5 required sections present
/// - Single codebase part (no CLI+API+DB+UI in one bead)
/// - No compound "and then..." patterns
pub const STANDALONE_VALIDATION_PROMPT: &str = r#"Validate whether a child bead description is standalone-ready.

A standalone-ready bead can be implemented by a naive but competent junior developer
WITHOUT consulting other beads, the parent epic, or out-of-band knowledge.

Check for these issues:

1. **MISSING SECTIONS**: Verify all 5 sections exist:
   - WHAT TO DO
   - WHY
   - HOW TO VERIFY
   - EDGE CASES AND PITFALLS (or EDGE CASES)
   - PROJECT-SPECIFIC TERMINOLOGY (or TERMINOLOGY)

2. **MULTIPLE CODEBASE AREAS**: Flag if bead touches multiple distinct areas:
   - CLI alone
   - API alone
   - Database alone
   - UI alone
   - Config alone
   - Auth alone
   (Exception: API + Database may be combined as they're closely related)

3. **COMPOUND PATTERNS**: Flag if bead has sequential work patterns:
   - "and then" patterns
   - "first... second..." patterns
   - "Step 1... Step 2..." numbered patterns
   - "after that" or "afterwards"
   - Sequential work that should be separate beads

Bead description to validate:
{bead_description}

Respond with a JSON object:
{
  "is_standalone_ready": true/false,
  "issues": ["list of issues found"],
  "suggestions": ["list of suggestions to fix issues"]
}
"#;

/// Prompt for splitting a compound bead into separate standalone beads.
pub const BEAD_SPLIT_PROMPT: &str = r#"Split a compound bead into separate standalone beads.

The following bead has compound scope (touches multiple areas or has sequential patterns).
Split it into 2-5 separate beads, each touching ONE distinct codebase area.

Original bead:
{original_bead}

RULES FOR SPLIT BEADS:
1. Each bead touches only ONE codebase area: CLI, API, DB, UI, Config, or Auth
2. No "and then..." patterns in any single bead
3. Each bead is standalone: includes all 5 sections
4. Maximum 5 beads - group closely related work
5. Preserve ordering if beads have dependencies

FORMAT as JSON array:
[
  {
    "title": "Area: Brief description",
    "description": "Standalone description (5 sections) for this unit",
    "has_dependency_on": null or "Area: Previous bead title"
  }
]

Respond with ONLY the JSON array.
"#;

/// Prompt for classifying an unlabeled GitHub issue.
///
/// Used as LLM fallback when label heuristics don't match any known labels.
/// Asks the LLM to classify the issue into one of: Bug, Feature, Question, Docs, Chore.
/// Also requests a confidence level to enable low-confidence fallback to Question.
///
/// This is called only for issues that have NO matching heuristic labels
/// (no bug, enhancement, question, documentation, chore, etc.).
pub const ISSUE_CLASSIFICATION_PROMPT: &str = r#"You are classifying a GitHub issue to determine its type.

CLASSIFICATION CATEGORIES:
- **Bug**: Something is broken, not working as expected, produces errors, crashes
- **Feature**: Request for new functionality, improvement, or enhancement
- **Question**: Asking for help, how-to, clarification, or support
- **Docs**: Request for documentation, missing docs, documentation correction
- **Chore**: Internal maintenance, dependency updates, CI/CD, refactoring (not user-facing)

CLASSIFICATION RULES:
1. If the issue describes something that is BROKEN → Bug
2. If the issue requests NEW functionality → Feature
3. If the issue is asking HOW to do something → Question
4. If the issue is about MISSING or WRONG documentation → Docs
5. If the issue is internal maintenance (deps, CI, refactor) → Chore
6. If you cannot confidently determine the type → Question (default)

EXISTING LABELS: {existing_labels}

Issue Title: {title}
Issue Body: {body}

Respond with ONLY a JSON object (no preamble):
{
  "issue_type": "Bug" | "Feature" | "Question" | "Docs" | "Chore",
  "confidence": "High" | "Medium" | "Low",
  "rationale": "Brief explanation of why you chose this type (1-2 sentences)"
}

RULES:
- Set confidence to "Low" if you are uncertain or the issue is vague
- When in doubt, classify as "Question"
- Do NOT guess if the issue lacks sufficient information — use "Question" as default
"#;

/// Result from LLM field extraction for bug reports.

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

/// Breakdown prompt for epic analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownPrompt {
    /// System prompt for the LLM.
    pub system_prompt: String,
    /// User prompt for breakdown analysis.
    pub user_prompt: String,
}

impl BreakdownPrompt {
    /// Create a breakdown prompt for epic-scale detection and child bead generation.
    pub fn for_epic_breakdown(metadata: &IssueMetadata, domain_context: Option<&str>) -> Self {
        let system_prompt = Self::breakdown_system_prompt();
        let user_prompt = Self::breakdown_user_prompt(metadata, domain_context);

        Self {
            system_prompt,
            user_prompt,
        }
    }

    /// System prompt for epic breakdown.
    fn breakdown_system_prompt() -> String {
        r#"You are Rodgers, analyzing whether a GitHub issue represents epic-scale work and generating child bead breakdowns.

CHILD BEAD RULES (from AGENTS.md):
- Single codebase part: Each bead should touch at most one distinct area (CLI, UI, API, DB, config, docs)
- No 'and then': Each bead's scope should be describable in a single, non-compound sentence
- Standalone: A naive but competent junior developer could implement it without consulting other beads
- One acceptance criterion or cohesive concern per child bead

EPIC-SCALE INDICATORS:
1. Work spans multiple areas of the project (e.g., "UI and API", "backend and docs")
2. Description contains sequential logic: "Do X, then Y, then Z" that maps to multiple sub-tasks
3. The issue discusses multiple distinct concerns that could be split
4. Complexity suggests parallel workstreams could speed up implementation

OUTPUT FORMAT:
Respond with valid JSON (no markdown code blocks) with these fields:
- primary_areas: array of strings (distinct work areas: ui, api, backend, database, cli, docs, config)
- sub_work_items: array of objects with:
  - title: string (concise title for the child bead)
  - scope_description: string (detailed description of what this child bead covers, following standalone bead rules)
- complexity_notes: string (optional notes about the breakdown and dependencies)

IMPORTANT:
- Generate at least 2 child beads for epic work
- Each child bead should be independently implementable
- Focus on distinct codebase areas as child bead scopes
- Consider AGENTS.md standalone rules: complete, self-contained descriptions"#
            .to_string()
    }

    /// User prompt for breakdown analysis.
    fn breakdown_user_prompt(metadata: &IssueMetadata, domain_context: Option<&str>) -> String {
        let mut prompt = String::new();

        if let Some(ctx) = domain_context {
            prompt.push_str(&format!("## Project Context\n{}\n\n", ctx));
        }

        prompt.push_str(&format!(
            "## Issue to Analyze\n#{}. {}\n\n",
            metadata.number, metadata.title
        ));

        if let Some(ref body) = metadata.body {
            prompt.push_str("### Body\n");
            prompt.push_str(body);
            prompt.push_str("\n\n");
        }

        if !metadata.labels.is_empty() {
            prompt.push_str(&format!(
                "### Existing Labels\n{}\n\n",
                metadata.labels.join(", ")
            ));
        }

        if !metadata.prior_comments.is_empty() {
            prompt.push_str("### Discussion\n");
            for comment in &metadata.prior_comments {
                prompt.push_str(&format!("- {}\n", comment));
            }
        }

        prompt.push_str(
            r#"
Analyze this issue for epic-scale work:
1. Is this epic-scale (multi-area or sequential work)?
2. What distinct work areas are involved?
3. Break down into standalone child beads following AGENTS.md rules.

Consider the project structure and how work could be parallelized."#,
        );

        prompt
    }
}

/// Question routing prompt for determining search scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionRoutingPrompt {
    /// System prompt for the LLM.
    pub system_prompt: String,
    /// User prompt for question analysis.
    pub user_prompt: String,
}

impl QuestionRoutingPrompt {
    /// Create a question routing prompt.
    pub fn for_routing(metadata: &IssueMetadata, domain_context: Option<&str>) -> Self {
        let system_prompt = Self::routing_system_prompt();
        let user_prompt = Self::routing_user_prompt(metadata, domain_context);

        Self {
            system_prompt,
            user_prompt,
        }
    }

    /// System prompt for question routing.
    fn routing_system_prompt() -> String {
        r#"You are Rodgers, determining how to route a GitHub question issue.

QUESTION ROUTING RULES:
- If this is not a genuine question (it's a bug report or feature request), indicate it should be re-labeled
- If this is a genuine question, determine the best search scope:
  - "docs" - answer is in user-facing documentation (package usage, configuration, workflow)
  - "code" - answer requires looking at source code (how X works internally, implementation details)
  - "both" - both docs and code might have relevant info
  - "none" - question cannot be answered from available sources (file a doc gap)

IMPLEMENTATION QUESTION INDICATORS:
- Keywords: "how does", "what function", "what method", "which module", "internals", "implementation"
- "under the hood", "source code", "can you walk me through", "flow of"
- Asking about a specific function, class, module by name
- Asking how a feature is implemented vs how to use it

RESPONSE DRAFTING:
- Be warm, patient, and genuine
- Lead with gratitude before any redirect
- Never sound dismissive

OUTPUT FORMAT:
Respond with valid JSON:
- is_question: boolean (is this a genuine question?)
- is_implementation_question: boolean (does this ask about code internals?)
- search_scope: string (docs|code|both|none)
- re_label_to: string (optional, if this should be re-labeled as bug/feature/etc.)
- answer_context: string (what info would answer this question?)
- confidence: number (0.0 to 1.0)"#
            .to_string()
    }

    /// User prompt for question routing.
    fn routing_user_prompt(metadata: &IssueMetadata, domain_context: Option<&str>) -> String {
        let mut prompt = String::new();

        if let Some(ctx) = domain_context {
            prompt.push_str(&format!("## Project Context\n{}\n\n", ctx));
        }

        prompt.push_str("## Question Issue\n");
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

        prompt.push_str(
            r#"
ANALYZE THIS QUESTION:
1. Is this a genuine question (as opposed to a bug report or feature request)?
2. Does this ask about implementation internals (how something works under the hood)?
3. Should Rodgers search documentation, source code, or both?
4. What specific information would answer this question?
5. Should this be re-labeled to a different type?

Respond with JSON only."#,
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
    fn test_breakdown_prompt() {
        let metadata = create_test_metadata();
        let prompt = BreakdownPrompt::for_epic_breakdown(&metadata, None);

        assert!(prompt.system_prompt.contains("CHILD BEAD"));
        assert!(prompt.system_prompt.contains("epic-scale"));
        assert!(prompt.user_prompt.contains("Issue to Analyze"));
    }

    #[test]
    fn test_breakdown_prompt_with_context() {
        let metadata = create_test_metadata();
        let context = "This project has: cli/, backend/, frontend/, docs/ directories.";
        let prompt = BreakdownPrompt::for_epic_breakdown(&metadata, Some(context));

        assert!(prompt.user_prompt.contains("cli/"));
        assert!(prompt.user_prompt.contains("backend/"));
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

    #[test]
    fn test_question_routing_prompt() {
        let metadata = create_test_metadata();
        let prompt = QuestionRoutingPrompt::for_routing(&metadata, None);

        assert!(prompt.system_prompt.contains("QUESTION ROUTING"));
        assert!(prompt.user_prompt.contains("Question Issue"));
        assert!(prompt.user_prompt.contains("123"));
    }

    #[test]
    fn test_question_routing_prompt_with_impl_context() {
        let metadata = IssueMetadata {
            number: 789,
            title: "How does the router work?".to_string(),
            body: Some("I want to understand the implementation internals.".to_string()),
            author: "devel".to_string(),
            author_type: None,
            labels: vec!["question".to_string()],
            prior_comments: vec![],
        };

        let prompt = QuestionRoutingPrompt::for_routing(&metadata, None);
        assert!(prompt.system_prompt.contains("implementation"));
        assert!(prompt.user_prompt.contains("internals"));
    }

    // =============================================================================
    // Issue Classification Prompt Tests (CRIT-2)
    // =============================================================================

    #[test]
    fn test_classification_prompt_includes_all_categories() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Bug"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Feature"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Question"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Docs"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Chore"));
    }

    #[test]
    fn test_classification_prompt_includes_placeholders() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("{existing_labels}"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("{title}"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("{body}"));
    }

    #[test]
    fn test_classification_prompt_requires_json_output() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("JSON"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("issue_type"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("confidence"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("rationale"));
    }

    #[test]
    fn test_classification_prompt_confidence_levels() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("High"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Medium"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Low"));
    }

    #[test]
    fn test_classification_prompt_defaults_to_question() {
        // When in doubt, should default to Question
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Question"));
        assert!(
            ISSUE_CLASSIFICATION_PROMPT.contains("in doubt")
                || ISSUE_CLASSIFICATION_PROMPT.contains("default")
        );
    }

    #[test]
    fn test_classification_prompt_classifies_bugs_as_broken() {
        // Bug classification rule: describes something broken
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("BROKEN"));
    }

    #[test]
    fn test_classification_prompt_classifies_features_as_new_functionality() {
        // Feature classification rule: request for new functionality
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("NEW functionality"));
    }

    #[test]
    fn test_classification_prompt_classifies_docs_as_missing_docs() {
        // Docs classification rule: missing or wrong documentation
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("MISSING"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("documentation"));
    }

    // =============================================================================
    // Issue Classification Prompt Tests (CRIT-2)
    // =============================================================================

    #[test]
    fn test_classification_prompt_includes_all_categories() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Bug"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Feature"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Question"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Docs"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Chore"));
    }

    #[test]
    fn test_classification_prompt_includes_placeholders() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("{existing_labels}"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("{title}"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("{body}"));
    }

    #[test]
    fn test_classification_prompt_requires_json_output() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("JSON"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("issue_type"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("confidence"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("rationale"));
    }

    #[test]
    fn test_classification_prompt_confidence_levels() {
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("High"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Medium"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Low"));
    }

    #[test]
    fn test_classification_prompt_defaults_to_question() {
        // When in doubt, should default to Question
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("Question"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("in doubt") || ISSUE_CLASSIFICATION_PROMPT.contains("default"));
    }

    #[test]
    fn test_classification_prompt_classifies_bugs_as_broken() {
        // Bug classification rule: describes something broken
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("BROKEN"));
    }

    #[test]
    fn test_classification_prompt_classifies_features_as_new_functionality() {
        // Feature classification rule: request for new functionality
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("NEW functionality"));
    }

    #[test]
    fn test_classification_prompt_classifies_docs_as_missing_docs() {
        // Docs classification rule: missing or wrong documentation
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("MISSING"));
        assert!(ISSUE_CLASSIFICATION_PROMPT.contains("documentation"));
    }
}
