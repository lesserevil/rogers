//! Question Router - Entry point for the question-routing workflow.
//!
//! Implements plans/question-routing-plan.md §Step 1→2→3.
//!
//! When the triage engine passes a question issue here, we:
//! 1. Detect vague questions → apply `needs-information`
//! 2. Detect bug/feature in disguise → reclassify
//! 3. Search docs/ for an answer
//! 4. Search source code for implementation answers
//! 5. File a doc-gap bead if nothing is found

use crate::triage::{IssueState, TriageAction, TriageIssue};
use std::collections::HashSet;

/// Output from the question router.
#[derive(Debug, Clone)]
pub struct QuestionRouterOutput {
    /// Whether the question was processed.
    pub processed: bool,
    /// Action taken.
    pub action: TriageAction,
    /// Comment to post (if any).
    pub comment: Option<String>,
    /// Labels to add.
    pub labels_to_add: Vec<String>,
    /// Labels to remove.
    pub labels_to_remove: Vec<String>,
}

/// Process a question issue through the question-routing workflow.
///
/// This function is synchronous and completes within one triage run.
pub fn process_question(issue: &TriageIssue) -> QuestionRouterOutput {
    // CRIT-6: Non-question issues must never enter this workflow.
    // The router already guards this, but we double-check.
    let has_question_label = issue.labels.iter().any(|l| l == "question");
    if !has_question_label || issue.state == IssueState::Closed {
        return QuestionRouterOutput {
            processed: false,
            action: TriageAction::NoAction,
            comment: None,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Step 1: Vague / needs clarification?
    if is_vague_question(issue) {
        return handle_needs_clarification(issue);
    }

    // Step 2: Bug or feature in disguise?
    if let Some(output) = detect_reclassification(issue) {
        return output;
    }

    // Step 3a: Search documentation
    if let Some(output) = search_docs(issue) {
        return output;
    }

    // Step 3a-ii: Search source code (implementation questions)
    if let Some(output) = search_code(issue) {
        return output;
    }

    // Step 3b: No answer found → file doc-gap
    handle_doc_gap(issue)
}

// =============================================================================
// Vague question detection
// =============================================================================

fn is_vague_question(issue: &TriageIssue) -> bool {
    let body = issue.body.trim();
    let title = issue.title.trim();

    // Body is extremely short
    if body.len() < 15 {
        return true;
    }

    // Count meaningful words (>2 chars) in title + body
    let combined = format!("{} {}", title, body);
    let meaningful_word_count = combined
        .split_whitespace()
        .filter(|w| w.chars().any(|c| c.is_alphabetic()) && w.len() > 2)
        .count();

    if meaningful_word_count < 3 {
        return true;
    }

    false
}

fn handle_needs_clarification(issue: &TriageIssue) -> QuestionRouterOutput {
    let comment = format!(
        r#"Hi @{author}, thanks for reaching out!

To help us give you the best answer, could you provide a bit more detail about what you're looking for? A specific scenario, error message, or the part of the project you're asking about would really help us point you in the right direction.

Thanks!"#,
        author = issue.author
    );

    QuestionRouterOutput {
        processed: true,
        action: TriageAction::QuestionNeedsClarification,
        comment: Some(comment),
        labels_to_add: vec!["needs-information".to_string()],
        labels_to_remove: Vec::new(),
    }
}

// =============================================================================
// Reclassification detection (bug/feature in disguise)
// =============================================================================

fn detect_reclassification(issue: &TriageIssue) -> Option<QuestionRouterOutput> {
    let body_lower = issue.body.to_lowercase();
    let title_lower = issue.title.to_lowercase();
    let combined = format!("{} {}", title_lower, body_lower);

    // Bug indicators
    let bug_indicators = [
        "crash",
        "stack trace",
        "error message",
        "exception",
        "regression",
        "broken",
        "doesn't work",
        "not working",
    ];
    let has_bug_indicator = bug_indicators.iter().any(|ind| combined.contains(ind));

    // Explicit bug prefixes
    let is_explicit_bug = title_lower.starts_with("bug:")
        || title_lower.starts_with("[bug]")
        || title_lower.contains("bug report");

    if is_explicit_bug || has_bug_indicator {
        return Some(QuestionRouterOutput {
            processed: true,
            action: TriageAction::QuestionReclassified,
            comment: Some(format!(
                "Hi @{author}, this looks like a bug report rather than a question. I'm re-labeling it as `bug` so it can go through the bug-triage workflow.",
                author = issue.author
            )),
            labels_to_add: vec!["bug".to_string()],
            labels_to_remove: vec!["question".to_string()],
        });
    }

    // Feature indicators
    let feature_indicators = [
        "feature request",
        "should add",
        "would be nice",
        "please add",
        "add support",
    ];
    let has_feature_indicator = feature_indicators.iter().any(|ind| combined.contains(ind));

    let is_explicit_feature = title_lower.starts_with("feature:")
        || title_lower.starts_with("[feature]")
        || title_lower.contains("enhancement");

    if is_explicit_feature || has_feature_indicator {
        return Some(QuestionRouterOutput {
            processed: true,
            action: TriageAction::QuestionReclassified,
            comment: Some(format!(
                "Hi @{author}, this looks like a feature request rather than a question. I'm re-labeling it as `feature` so it can go through the feature-triage workflow.",
                author = issue.author
            )),
            labels_to_add: vec!["feature".to_string()],
            labels_to_remove: vec!["question".to_string()],
        });
    }

    None
}

// =============================================================================
// Doc search
// =============================================================================

fn search_docs(issue: &TriageIssue) -> Option<QuestionRouterOutput> {
    let keywords = extract_keywords(&issue.title, &issue.body);
    if keywords.is_empty() {
        return None;
    }

    let docs_dir = std::path::Path::new("docs");
    if !docs_dir.exists() || !docs_dir.is_dir() {
        return None;
    }

    let best_match = find_best_matching_file(docs_dir, &keywords);

    if let Some((path, _score)) = best_match {
        let filename = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "documentation".to_string());

        let comment = format!(
            r#"Hi @{author}, thanks for the question!

The answer to your question is covered in [{filename}]({filename}).

If this doesn't fully answer your question, please let us know and we will follow up."#,
            author = issue.author,
            filename = filename
        );

        return Some(QuestionRouterOutput {
            processed: true,
            action: TriageAction::QuestionAnsweredDoc,
            comment: Some(comment),
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        });
    }

    None
}

// =============================================================================
// Code search
// =============================================================================

fn search_code(issue: &TriageIssue) -> Option<QuestionRouterOutput> {
    let combined = format!("{} {}", issue.title, issue.body).to_lowercase();

    // Only search code for implementation-level questions
    let code_phrases = [
        "how does",
        "what function",
        "which module",
        "internals",
        "implementation",
        "source code",
        "walk me through",
        "flow of",
        "under the hood",
        "which file",
        "where is",
        "how is",
    ];
    let is_code_question = code_phrases.iter().any(|p| combined.contains(p));

    if !is_code_question {
        return None;
    }

    let keywords = extract_keywords(&issue.title, &issue.body);
    if keywords.is_empty() {
        return None;
    }

    let src_dir = std::path::Path::new("src");
    if !src_dir.exists() || !src_dir.is_dir() {
        return None;
    }

    let best_match = find_best_matching_file(src_dir, &keywords);

    if let Some((path, _score)) = best_match {
        let relative_path = path.strip_prefix("src").unwrap_or(&path);
        let path_str = relative_path.to_string_lossy();

        let comment = format!(
            r#"Hi @{author}, thanks for this question! I took a look at the source code to find the answer.

The relevant implementation is in `src/{path_str}`.

If you'd like to dig further, the full implementation is there. Let us know if you have follow-up questions!"#,
            author = issue.author,
            path_str = path_str
        );

        return Some(QuestionRouterOutput {
            processed: true,
            action: TriageAction::QuestionAnsweredCode,
            comment: Some(comment),
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        });
    }

    None
}

// =============================================================================
// Doc gap handling
// =============================================================================

fn handle_doc_gap(issue: &TriageIssue) -> QuestionRouterOutput {
    let comment = format!(
        r#"Hi @{author}, thanks for the question! We do not currently have documentation that answers this. We have opened a task to add an answer to our documentation — it will be linked here when complete."#,
        author = issue.author
    );

    QuestionRouterOutput {
        processed: true,
        action: TriageAction::QuestionDocGapFiled,
        comment: Some(comment),
        labels_to_add: vec!["needs-documentation".to_string()],
        labels_to_remove: Vec::new(),
    }
}

// =============================================================================
// Keyword extraction and file search helpers
// =============================================================================

fn extract_keywords(title: &str, body: &str) -> Vec<String> {
    let combined = format!("{} {}", title, body).to_lowercase();
    let words: Vec<&str> = combined
        .split(|c: char| !c.is_alphanumeric() && c != ':')
        .collect();

    let stop_words: HashSet<&str> = [
        "a", "an", "the", "how", "to", "do", "i", "you", "we", "can", "is", "does", "what",
        "where", "when", "why", "which", "who", "me", "my", "in", "on", "at", "for", "with",
        "about", "of", "it", "this", "that", "from", "get", "use", "be", "are", "was", "were",
        "have", "has", "had", "will", "would", "could", "should", "did", "does", "doing", "done",
        "and", "or", "but", "if", "then", "than", "so", "as", "by", "via", "through", "into",
        "onto", "up", "out", "over", "under", "again", "further", "once", "here", "there", "all",
        "any", "both", "each", "few", "more", "most", "other", "some", "such", "no", "nor", "not",
        "only", "own", "same", "just", "also", "very", "too", "may", "might", "shall", "am",
    ]
    .iter()
    .cloned()
    .collect();

    let mut keywords = Vec::new();
    let mut seen = HashSet::new();

    for word in words {
        let trimmed = word.trim_matches(|c: char| !c.is_alphanumeric());
        if trimmed.len() > 2 && !stop_words.contains(trimmed) && seen.insert(trimmed.to_string()) {
            keywords.push(trimmed.to_string());
        }
    }

    keywords
}

fn find_best_matching_file(
    dir: &std::path::Path,
    keywords: &[String],
) -> Option<(std::path::PathBuf, usize)> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return None,
    };

    let mut best: Option<(std::path::PathBuf, usize)> = None;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if let Some(sub_best) = find_best_matching_file(&path, keywords) {
                if best
                    .as_ref()
                    .map_or(true, |(_, b_score)| sub_best.1 > *b_score)
                {
                    best = Some(sub_best);
                }
            }
            continue;
        }

        // Only search text files
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if !matches!(ext, "md" | "rs" | "txt" | "toml" | "yaml" | "yml" | "json") {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c.to_lowercase(),
            Err(_) => continue,
        };

        let score = keywords
            .iter()
            .filter(|kw| content.contains(kw.as_str()))
            .count();

        if score > 0 && best.as_ref().map_or(true, |(_, b_score)| score > *b_score) {
            best = Some((path, score));
        }
    }

    best
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(labels: Vec<&str>, title: &str, body: &str) -> TriageIssue {
        TriageIssue {
            number: 1,
            title: title.to_string(),
            body: body.to_string(),
            author: "testuser".to_string(),
            labels: labels.into_iter().map(String::from).collect(),
            state: IssueState::Open,
            url: Some("https://github.com/org/repo/issues/1".to_string()),
        }
    }

    #[test]
    fn test_vague_question_needs_clarification() {
        let issue = create_test_issue(vec!["question"], "Help?", "???");
        let output = process_question(&issue);

        assert!(output.processed);
        assert_eq!(output.action, TriageAction::QuestionNeedsClarification);
        assert!(
            output
                .labels_to_add
                .contains(&"needs-information".to_string())
        );
        assert!(output.comment.as_ref().unwrap().contains("more detail"));
    }

    #[test]
    fn test_vague_short_body_needs_clarification() {
        let issue = create_test_issue(vec!["question"], "How?", "help");
        let output = process_question(&issue);

        assert_eq!(output.action, TriageAction::QuestionNeedsClarification);
    }

    #[test]
    fn test_clear_question_not_vague() {
        let issue = create_test_issue(
            vec!["question"],
            "How do I configure the scheduler?",
            "I want to change the triage interval from 60 minutes to 30 minutes. Where do I set this?",
        );
        let output = process_question(&issue);

        assert_ne!(output.action, TriageAction::QuestionNeedsClarification);
    }

    #[test]
    fn test_bug_in_disguise_reclassified() {
        let issue = create_test_issue(
            vec!["question"],
            "Why does the app crash?",
            "Every time I click save the app crashes with a stack trace.",
        );
        let output = process_question(&issue);

        assert_eq!(output.action, TriageAction::QuestionReclassified);
        assert!(output.labels_to_add.contains(&"bug".to_string()));
        assert!(output.labels_to_remove.contains(&"question".to_string()));
    }

    #[test]
    fn test_feature_in_disguise_reclassified() {
        let issue = create_test_issue(
            vec!["question"],
            "Can you add dark mode?",
            "It would be nice to have dark mode support for the UI.",
        );
        let output = process_question(&issue);

        assert_eq!(output.action, TriageAction::QuestionReclassified);
        assert!(output.labels_to_add.contains(&"feature".to_string()));
        assert!(output.labels_to_remove.contains(&"question".to_string()));
    }

    #[test]
    fn test_non_question_not_processed() {
        let issue = create_test_issue(
            vec!["bug"],
            "Crash on startup",
            "The app crashes immediately.",
        );
        let output = process_question(&issue);

        assert!(!output.processed);
        assert_eq!(output.action, TriageAction::NoAction);
    }

    #[test]
    fn test_doc_search_finds_answer() {
        // "configuration" should match docs/configuration.md
        let issue = create_test_issue(
            vec!["question"],
            "How is configuration handled?",
            "I want to understand the configuration file format and environment variables.",
        );
        let output = process_question(&issue);

        assert_eq!(output.action, TriageAction::QuestionAnsweredDoc);
        assert!(
            output
                .comment
                .as_ref()
                .unwrap()
                .contains("configuration.md")
        );
    }

    #[test]
    fn test_code_search_finds_answer() {
        // "how does" triggers code search, "triage" matches src/triage/
        let issue = create_test_issue(
            vec!["question"],
            "How does the triage loop work under the hood?",
            "I want to understand the implementation details of the triage state machine.",
        );
        let output = process_question(&issue);

        assert_eq!(output.action, TriageAction::QuestionAnsweredCode);
        assert!(
            output.comment.as_ref().unwrap().contains("src/"),
            "code answer should reference a source file, got: {:?}",
            output.comment
        );
    }

    #[test]
    fn test_doc_gap_filed_when_no_answer() {
        // Use a topic that is unlikely to exist in docs or source
        let issue = create_test_issue(
            vec!["question"],
            "What is the quantum flux capacitor setting?",
            "I need to know the exact quantum flux capacitor calibration value for my deployment.",
        );
        let output = process_question(&issue);

        assert_eq!(output.action, TriageAction::QuestionDocGapFiled);
        assert!(
            output
                .labels_to_add
                .contains(&"needs-documentation".to_string())
        );
        assert!(
            output
                .comment
                .as_ref()
                .unwrap()
                .contains("do not currently have documentation")
        );
    }

    #[test]
    fn test_closed_question_is_noop() {
        let issue = TriageIssue {
            number: 1,
            title: "How?".to_string(),
            body: "help".to_string(),
            author: "testuser".to_string(),
            labels: vec!["question".to_string()],
            state: IssueState::Closed,
            url: None,
        };
        let output = process_question(&issue);

        assert!(!output.processed);
        assert_eq!(output.action, TriageAction::NoAction);
    }

    #[test]
    fn test_keyword_extraction_filters_stop_words() {
        let keywords = extract_keywords("How do I configure the app?", "I want to use it.");
        assert!(keywords.contains(&"configure".to_string()));
        assert!(keywords.contains(&"want".to_string()));
        assert!(!keywords.contains(&"how".to_string()));
        assert!(!keywords.contains(&"the".to_string()));
    }

    #[test]
    fn test_find_best_matching_file_existing_docs() {
        let keywords = vec!["configuration".to_string(), "yaml".to_string()];
        let result = find_best_matching_file(std::path::Path::new("docs"), &keywords);

        assert!(result.is_some(), "should find a matching doc file");
        let (path, score) = result.unwrap();
        assert!(path.to_string_lossy().contains("configuration"));
        assert!(score > 0);
    }
}
