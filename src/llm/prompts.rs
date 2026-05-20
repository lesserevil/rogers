//! LLM prompts for Rodgers question routing.
//!
//! This module contains prompts used to interact with the LLM for
//! determining how to route questions, including whether code search
//! is warranted for implementation-related questions.

/// Keywords that indicate a question is about implementation/code internals.
/// When these keywords appear in a question, the router should search
/// the codebase before filing a doc-gap bead.
pub const CODE_SEARCH_TRIGGERS: &[&str] = &[
    "how does",
    "what function",
    "which module",
    "internals",
    "implementation",
    "source code",
    "walk me through",
    "flow of",
    "under the hood",
    "can you show me",
    "how is",
    "where is",
    "what handles",
];

/// Stopwords to filter out false positives when detecting code search triggers.
pub const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
    "can", "this", "that", "these", "those", "i", "you", "we", "they", "he", "she", "it",
];

/// Default prompt sent to the LLM to determine if code search is warranted.
///
/// The LLM receives:
/// - Question title and body
/// - Project context from AGENTS.md
/// - All prior comments
/// - Existing labels
///
/// The LLM responds with a determination:
/// - Whether the question asks about implementation details
/// - Whether code search should be performed
/// - What specific code elements to search for
pub fn code_search_routing_prompt(
    question_title: &str,
    question_body: &str,
    project_context: &str,
) -> String {
    format!(
        r#"You are Rodgers, a GitHub-native community relations agent. Your task is to determine whether a question about the codebase should be answered by searching the source code.

## Project Context
{project_context}

## Question
Title: {question_title}
Body: {question_body}

## Your Task
Analyze this question and determine:
1. Does this question ask about implementation details, code internals, or how something works "under the hood"?
2. Would searching the source code directly answer this question better than user documentation?
3. What specific code elements (function names, module names, class names, file patterns) should we search for?

## Keywords that indicate code search is appropriate
- "how does", "what function", "which module", "internals", "implementation"
- "source code", "can you walk me through", "flow of", "under the hood"
- Questions about specific function/class/module behavior

## Keywords that suggest user docs are more appropriate
- "how do I use", "how to get started", "tutorial"
- Questions about end-user behavior, not implementation
- Questions already answered in user-facing docs

## Response Format
Respond with one of:
- "CODE_SEARCH_WARRANTED: [list of specific code elements to search]"
- "DOC_SEARCH_RECOMMENDED: [reason why docs are better]"
- "CLARIFICATION_NEEDED: [what additional context is needed]"

If CODE_SEARCH_WARRANTED, provide a specific list of code elements (function names, module paths, file patterns) that would help answer this question."#,
        question_title = question_title,
        question_body = question_body,
        project_context = project_context,
    )
}

/// Default prompt for explaining code in plain language with citations.
///
/// The LLM receives:
/// - The code snippets found by the search
/// - The original question
///
/// The LLM responds with:
/// - Plain-language explanation of how the code works
/// - File:line citations for all referenced code
/// - Whether the explanation fully answers the question
pub fn code_explanation_prompt(
    question: &str,
    code_snippets: &[(&str, usize, &str)], // (file_path, line_number, code_content)
    project_context: &str,
) -> String {
    let snippets_text = code_snippets
        .iter()
        .map(|(path, line, content)| format!("// {}:{}\n{}\n", path, line, content))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"You are Rodgers, a GitHub-native community relations agent. Your task is to explain how source code works in plain language.

## Project Context
{project_context}

## Question
{question}

## Relevant Code Snippets
{snippets_text}

## Your Task
1. Provide a plain-language explanation of how the code works
2. Use the specific code snippets above to support your explanation
3. Cite the source file and line numbers for all statements about code behavior
4. Determine if this explanation fully answers the question

## Response Format
Respond with a comment suitable for posting on a GitHub issue:

```
Hi @[requestor], thanks for this question! I took a look at the source code to find the answer.

[Plain-language explanation of how the code works]

Relevant source: [file path], [function/struct name]
If the code is complex, include a step-by-step walkthrough.

If the code is complex, offer to continue:
"If you'd like to dig further, the full implementation is at [file:line–line]."
```

Include specific file:line citations like "src/foo.rs:123-145".

IMPORTANT: Only cite code that actually exists in the snippets above. Do not hallucinate file paths or line numbers."#,
        question = question,
        snippets_text = snippets_text,
        project_context = project_context,
    )
}

/// Prompt for determining whether to close the issue after a code answer.
///
/// The LLM receives:
/// - The original question
/// - The code explanation provided
/// - Whether the explanation fully answers the question
pub fn close_decision_prompt(question: &str, explanation: &str, fully_answers: bool) -> String {
    let action = if fully_answers {
        "close the issue"
    } else {
        "leave the issue open for follow-up"
    };

    format!(
        r#"You are Rodgers, a community relations agent. A question was asked and you provided a code-based explanation.

## Original Question
{question}

## Your Explanation
{explanation}

## Decision
Based on whether your explanation fully answers the original question, should we {action}?

Respond with one of:
- "CLOSE: [reason why this fully answers the question]"
- "LEAVE_OPEN: [reason why follow-up may be needed]"

Be conservative: if there's any doubt that the requestor has what they need, leave the issue open."#,
        question = question,
        explanation = explanation,
        action = action,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_search_triggers_present() {
        assert!(CODE_SEARCH_TRIGGERS.contains(&"how does"));
        assert!(CODE_SEARCH_TRIGGERS.contains(&"implementation"));
        assert!(CODE_SEARCH_TRIGGERS.contains(&"under the hood"));
        assert!(CODE_SEARCH_TRIGGERS.contains(&"walk me through"));
    }

    #[test]
    fn test_code_search_routing_prompt_contains_context() {
        let prompt = code_search_routing_prompt(
            "How does the triage work?",
            "I'm curious about the internals of the triage system",
            "Rodgers is a GitHub agent",
        );
        assert!(prompt.contains("How does the triage work"));
        assert!(prompt.contains("internals of the triage system"));
        assert!(prompt.contains("CODE_SEARCH_WARRANTED"));
    }

    #[test]
    fn test_code_explanation_prompt_format() {
        let snippets = vec![
            ("src/triage.rs", 42, "pub fn process_issue() {"),
            ("src/triage.rs", 43, "    // TODO: implement"),
        ];
        let prompt = code_explanation_prompt(
            "How does the triage work?",
            &snippets,
            "Rodgers is a GitHub agent",
        );
        assert!(prompt.contains("src/triage.rs:42"));
        assert!(prompt.contains("Plain-language explanation"));
        // CODE_SEARCH_WARRANTED is in code_search_routing_prompt, not code_explanation_prompt
        assert!(prompt.contains("GitHub issue"));
    }
}
