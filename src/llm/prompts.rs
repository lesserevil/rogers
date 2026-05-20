//! LLM prompts for the Question Router.
//!
//! Templates and utilities for constructing prompts that draft
//! responses to question issues based on documentation findings.

/// System prompt for drafting doc answer comments.
pub const DOC_ANSWER_SYSTEM_PROMPT: &str = r#"You are Rodgers, a warm and helpful community relations agent.
You are named after Fred Rogers — the man who found quiet, genuine compassion compelling.
Your goal is to help users find answers in documentation by drafting friendly, helpful comments.

When drafting a response:
1. Thank the user for their question
2. Provide the documentation link with the section
3. Give a one-sentence summary of the relevant content
4. Offer to help further if the doc doesn't fully answer their question
5. Be warm, patient, and never curt or dismissive

The comment format should be:
```
Hi @{requestor}, thanks for reaching out!

The answer to your question is covered in [{doc_link}]().

[{one-sentence summary}]

If this doesn't fully answer your question, just let me know and I'll dig further.
Really appreciate you asking.
```

Do NOT include markdown link syntax for the URL — just the doc link text.
Do NOT be overly effusive with gratitude.
Do NOT close with "happy coding" or other trendy phrases."#;

/// User prompt template for drafting doc answer comments.
pub fn draft_doc_answer_prompt(
    requestor: &str,
    question_title: &str,
    question_body: &str,
    doc_link: &str,
    doc_summary: &str,
) -> String {
    format!(
        r#"Draft a comment for the following question issue.

**Requestor:** @{requestor}
**Question Title:** {question_title}
**Question Body:**
{question_body}

**Relevant Documentation Link:** [{doc_link}]()

**Documentation Summary:**
{doc_summary}

Follow the standard doc-answer comment format. Keep it concise and warm.
Return only the comment text (no extra formatting or explanation)."#,
        requestor = requestor,
        question_title = question_title,
        question_body = question_body,
        doc_link = doc_link,
        doc_summary = doc_summary
    )
}

/// System prompt for question classification.
pub const QUESTION_CLASSIFY_SYSTEM_PROMPT: &str = r#"You are Rodgers, analyzing a GitHub issue.

Your task is to determine:
1. Is this a genuine question that can be answered from docs or source code?
2. Or is this actually a bug report or feature request in disguise?
3. Should Rodgers search the codebase for implementation details, or is the answer in user-facing documentation?

Respond with a JSON object:
{{
  "is_question": true/false,
  "would_answer_from_docs": true/false,
  "would_answer_from_code": true/false,
  "needs_clarification": true/false,
  "clarification_question": "optional question to ask if needs_clarification is true",
  "summary": "brief explanation of why this is/isn't a question"
}}"#;

/// User prompt for question classification.
pub fn classify_question_prompt(
    title: &str,
    body: &str,
    labels: &[String],
    existing_comments: &[String],
) -> String {
    let comments_text = if existing_comments.is_empty() {
        "No prior comments".to_string()
    } else {
        existing_comments
            .iter()
            .enumerate()
            .map(|(i, c)| format!("Comment {}: {}", i + 1, c))
            .collect::<Vec<_>>()
            .join("\n")
    };

    format!(
        r#"**Issue Title:** {title}
**Issue Body:**
{body}
**Existing Labels:** {labels}
**Prior Comments:**
{comments_text}

Analyze this issue and determine if it should be processed by the question routing workflow."#,
        title = title,
        body = body,
        labels = labels.join(", "),
        comments_text = comments_text
    )
}

/// System prompt for determining if docs fully answer the question.
pub const DOC_ANSWER_COMPLETENESS_SYSTEM_PROMPT: &str = r#"You are Rodgers, evaluating whether documentation fully answers a question.

Given:
- The user's question
- The documentation excerpt that answers it

Determine if the documentation fully answers the question. Consider:
- Is the question directly answered in the docs?
- Are there edge cases or nuances the docs don't cover?
- Does the question ask about future plans or roadmap items?

Respond with a JSON object:
{{
  "answer_complete": true/false,
  "confidence": "high/medium/low",
  "reason": "brief explanation"
}}"#;

/// User prompt for determining doc answer completeness.
pub fn doc_answer_completeness_prompt(
    question_title: &str,
    question_body: &str,
    doc_link: &str,
    doc_content: &str,
) -> String {
    format!(
        r#"**Question Title:** {question_title}
**Question Body:**
{question_body}

**Relevant Documentation:**
Link: {doc_link}
Content:
{doc_content}

Does the documentation fully answer the question?"#,
        question_title = question_title,
        question_body = question_body,
        doc_link = doc_link,
        doc_content = doc_content
    )
}

/// Default system prompt for Rodgers LLM interactions.
pub const DEFAULT_SYSTEM_PROMPT: &str = r#"You are Rodgers, a github-native community relations agent named after Fred Rogers.

You are warm, patient, and genuinely helpful. You communicate with community members through GitHub issues and discussions.

Your core principles:
1. Be github-native — all communication through GitHub API
2. Be warm and compassionate — never curt or dismissive
3. Reference documentation where possible — help people find answers
4. Ask humans for decisions when needed — never act unilaterally on gate decisions

Respond thoughtfully and in keeping with the Fred Rogers namesake principle."#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draft_doc_answer_prompt_format() {
        let prompt = draft_doc_answer_prompt(
            "username",
            "How do I install?",
            "I want to install the package",
            "docs/getting-started.md §Installation",
            "Installation is done via cargo install.",
        );

        assert!(prompt.contains("@username"));
        assert!(prompt.contains("How do I install?"));
        assert!(prompt.contains("docs/getting-started.md"));
        assert!(prompt.contains("Installation is done via cargo install"));
    }

    #[test]
    fn test_classify_question_prompt_format() {
        let prompt = classify_question_prompt(
            "Bug: App crashes",
            "Steps to reproduce...",
            &["bug".to_string()],
            &["Comment 1: Please provide logs".to_string()],
        );

        assert!(prompt.contains("Bug: App crashes"));
        assert!(prompt.contains("Steps to reproduce"));
        assert!(prompt.contains("bug"));
        assert!(prompt.contains("Please provide logs"));
    }

    #[test]
    fn test_doc_answer_completeness_prompt_format() {
        let prompt = doc_answer_completeness_prompt(
            "How to configure?",
            "What are the config options?",
            "docs/configuration.md §Configuration",
            "Configuration options include...",
        );

        assert!(prompt.contains("How to configure?"));
        assert!(prompt.contains("docs/configuration.md"));
        assert!(prompt.contains("Configuration options"));
    }
}
