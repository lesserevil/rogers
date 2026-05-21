//! Priority assessment for feature issues.
//!
//! This module implements priority classification (P1-P4) for feature issues
//! as defined in plans/triage-workflow-plan.md and plans/feature-bug-plan.md.
//!
//! Priority levels:
//! - P1 (critical): blockers, critical path items, urgent releases
//! - P2 (high): important, high value features
//! - P3 (normal): standard features, nice to have
//! - P4 (low): backlog items, low priority
//!
//! Assessment is keyword-based from issue body text. Human-set priorities
//! are never overridden. LLM assessment is available for ambiguous cases.

use serde::{Deserialize, Serialize};

/// Priority levels for feature issues. P1 is highest priority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    /// P1 - Critical: blocks releases or is on critical path
    P1,
    /// P2 - High: important, high value
    P2,
    /// P3 - Normal: standard, nice to have
    P3,
    /// P4 - Low: backlog, low priority
    P4,
}

impl Priority {
    /// Return the string label for this priority level.
    pub fn label(&self) -> &'static str {
        match self {
            Priority::P1 => "P1",
            Priority::P2 => "P2",
            Priority::P3 => "P3",
            Priority::P4 => "P4",
        }
    }

    /// Return the display name for this priority level.
    pub fn display(&self) -> &'static str {
        match self {
            Priority::P1 => "P1 - Critical",
            Priority::P2 => "P2 - High",
            Priority::P3 => "P3 - Normal",
            Priority::P4 => "P4 - Low",
        }
    }
}

/// Keywords that map to each priority level.
///
/// The mapping is case-insensitive and checks for word boundaries
/// to avoid false positives (e.g., "priority" should not match "high-priority").
pub const P1_KEYWORDS: &[&str] = &[
    "blocker",
    "critical",
    "urgent",
];

pub const P2_KEYWORDS: &[&str] = &[
    "important",
    "high value",
];

pub const P3_KEYWORDS: &[&str] = &[
    "normal",
    "nice to have",
    "nice-to-have",
];

pub const P4_KEYWORDS: &[&str] = &[
    "low priority",
    "backlog",
];

/// Represents a priority assessment result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriorityAssessment {
    /// The assessed priority level
    pub priority: Priority,
    /// Keywords found in the issue body that triggered this priority
    pub matched_keywords: Vec<String>,
    /// Whether the priority was set by a human (not auto-assessed)
    pub human_set: bool,
    /// Method used: "keyword" or "llm"
    pub method: String,
}

/// Assess priority from issue body text using keyword matching.
///
/// Scans the issue body (title and body combined) for priority keywords.
/// Returns the highest priority found (P1 > P2 > P3 > P4).
///
/// Human-set priority labels (e.g., "priority:P1") are detected and respected.
/// If no keywords match, defaults to P3 (normal).
pub fn assess_priority(issue_title: &str, issue_body: &str, existing_labels: &[String]) -> PriorityAssessment {
    let combined = format!("{} {}", issue_title.to_lowercase(), issue_body.to_lowercase());
    let mut matched_keywords: Vec<String> = Vec::new();
    let mut highest_priority = Priority::P3; // Default: normal

    // Check for human-set priority labels first
    if has_human_priority_label(existing_labels) {
        return extract_human_priority(existing_labels);
    }

    // Check P1 keywords (highest priority)
    for keyword in P1_KEYWORDS {
        if contains_keyword(&combined, keyword) {
            matched_keywords.push(keyword.to_string());
            highest_priority = Priority::P1;
        }
    }

    // Only check P2 if no P1 found (P1 takes precedence)
    if highest_priority != Priority::P1 {
        for keyword in P2_KEYWORDS {
            if contains_keyword(&combined, keyword) {
                matched_keywords.push(keyword.to_string());
                highest_priority = Priority::P2;
            }
        }
    }

    // Only check P3 if no P1 or P2 found
    if highest_priority != Priority::P1 && highest_priority != Priority::P2 {
        for keyword in P3_KEYWORDS {
            if contains_keyword(&combined, keyword) {
                matched_keywords.push(keyword.to_string());
                highest_priority = Priority::P3;
            }
        }
    }

    // Only check P4 if no P1, P2, or P3 found
    if highest_priority == Priority::P3 && matched_keywords.is_empty() {
        for keyword in P4_KEYWORDS {
            if contains_keyword(&combined, keyword) {
                matched_keywords.push(keyword.to_string());
                highest_priority = Priority::P4;
            }
        }
    }

    // If no keywords matched, default to P3 with no matched keywords
    if matched_keywords.is_empty() {
        PriorityAssessment {
            priority: Priority::P3,
            matched_keywords: Vec::new(),
            human_set: false,
            method: "keyword".to_string(),
        }
    } else {
        PriorityAssessment {
            priority: highest_priority,
            matched_keywords,
            human_set: false,
            method: "keyword".to_string(),
        }
    }
}

/// Assess priority using LLM analysis for ambiguous cases.
///
/// This is a hook for LLM-based priority assessment when keyword matching
/// produces ambiguous or conflicting results. The LLM is prompted with
/// the issue title, body, and any matched keywords to determine priority.
///
/// In production, this would call the LLM API. For now, it returns P3
/// (normal) with method "llm" as a placeholder.
pub fn llm_assess_priority(_issue_title: &str, _issue_body: &str, matched_keywords: &[String]) -> PriorityAssessment {
    // Placeholder: In production, this would call the LLM API with a prompt like:
    // "Based on this feature request, what priority should it receive?\n\n\
    // Title: {title}\nBody: {body}\n\n\
    // Keywords found: {keywords}\n\n\
    // Return one of: P1 (critical), P2 (high), P3 (normal), P4 (low)."
    //
    // The LLM response would be validated before acting on it (per edge cases).
    // For now, default to P3.

    PriorityAssessment {
        priority: Priority::P3,
        matched_keywords: matched_keywords.to_vec(),
        human_set: false,
        method: "llm".to_string(),
    }
}

/// Check if labels contain a human-set priority label.
///
/// Human-set priority labels follow the pattern "priority:P1", "priority:P2",
/// "priority:P3", or "priority:P4". If present, these take precedence over
/// keyword-based assessment.
fn has_human_priority_label(labels: &[String]) -> bool {
    labels.iter().any(|l| {
        let lower = l.to_lowercase();
        matches!(lower.as_str(), "priority:p1" | "priority:p2" | "priority:p3" | "priority:p4")
    })
}

/// Extract the human-set priority from labels.
///
/// Returns a PriorityAssessment with human_set=true and the extracted priority.
fn extract_human_priority(labels: &[String]) -> PriorityAssessment {
    for label in labels {
        let lower = label.to_lowercase();
        let priority = match lower.as_str() {
            "priority:p1" => Priority::P1,
            "priority:p2" => Priority::P2,
            "priority:p3" => Priority::P3,
            "priority:p4" => Priority::P4,
            _ => continue,
        };
        return PriorityAssessment {
            priority,
            matched_keywords: vec![format!("human label: {}", label)],
            human_set: true,
            method: "human".to_string(),
        };
    }

    // Should not reach here if has_human_priority_label returned true
    PriorityAssessment {
        priority: Priority::P3,
        matched_keywords: Vec::new(),
        human_set: false,
        method: "keyword".to_string(),
    }
}

/// Check if the text contains the keyword with word boundary awareness.
///
/// For multi-word keywords, checks if all words appear in order.
/// For single-word keywords, checks for word boundary matches.
fn contains_keyword(text: &str, keyword: &str) -> bool {
    if keyword.contains(' ') {
        // Multi-word keyword: check for substring match
        text.contains(keyword)
    } else {
        // Single-word keyword: check for word boundary match
        // Use regex-like patterns to avoid matching substrings
        // e.g., "priority" should not match "high-priority" in some contexts
        // But for priority keywords, we want to match "critical" in "critical-path" too
        // So we use a simple approach: check for the word surrounded by non-alpha chars
        let keyword_lower = keyword.to_lowercase();
        let text_lower = text.to_lowercase();

        // Check each position in the text for the keyword
        for i in 0..=text_lower.len().saturating_sub(keyword_lower.len()) {
            if text_lower[i..].starts_with(&keyword_lower) {
                let before_ok = i == 0 || !text_lower.chars().nth(i - 1).is_some_and(|c| c.is_alphanumeric());
                let after_pos = i + keyword_lower.len();
                let after_ok = after_pos >= text_lower.len()
                    || !text_lower.chars().nth(after_pos).is_some_and(|c| c.is_alphanumeric());

                if before_ok && after_ok {
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================================================
    // Priority enum tests
    // =============================================================================

    #[test]
    fn test_priority_label() {
        assert_eq!(Priority::P1.label(), "P1");
        assert_eq!(Priority::P2.label(), "P2");
        assert_eq!(Priority::P3.label(), "P3");
        assert_eq!(Priority::P4.label(), "P4");
    }

    #[test]
    fn test_priority_display() {
        assert_eq!(Priority::P1.display(), "P1 - Critical");
        assert_eq!(Priority::P2.display(), "P2 - High");
        assert_eq!(Priority::P3.display(), "P3 - Normal");
        assert_eq!(Priority::P4.display(), "P4 - Low");
    }

    // =============================================================================
    // Keyword mapping tests (CRIT-5 verification: Priority keywords correctly map)
    // =============================================================================

    #[test]
    fn test_blocker_maps_to_p1() {
        let assessment = assess_priority(
            "Fix blocker in login",
            "This is a critical blocker that prevents all users from logging in",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P1);
        assert!(assessment.matched_keywords.contains(&"blocker".to_string()));
    }

    #[test]
    fn test_critical_maps_to_p1() {
        let assessment = assess_priority(
            "Critical data loss bug",
            "This critical issue is causing data loss in production",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P1);
        assert!(assessment.matched_keywords.contains(&"critical".to_string()));
    }

    #[test]
    fn test_urgent_maps_to_p1() {
        let assessment = assess_priority(
            "Urgent security patch needed",
            "This urgent fix needs to be applied before the release",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P1);
        assert!(assessment.matched_keywords.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_important_maps_to_p2() {
        let assessment = assess_priority(
            "Important dashboard feature",
            "This important feature will improve user productivity",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P2);
        assert!(assessment.matched_keywords.contains(&"important".to_string()));
    }

    #[test]
    fn test_high_value_maps_to_p2() {
        let assessment = assess_priority(
            "High value analytics",
            "This high value feature will give users better insights",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P2);
        assert!(assessment.matched_keywords.contains(&"high value".to_string()));
    }

    #[test]
    fn test_normal_maps_to_p3() {
        let assessment = assess_priority(
            "Normal UI update",
            "This normal update improves the user interface",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P3);
        assert!(assessment.matched_keywords.contains(&"normal".to_string()));
    }

    #[test]
    fn test_nice_to_have_maps_to_p3() {
        let assessment = assess_priority(
            "Nice to have feature",
            "This is a nice to have feature that would be convenient",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P3);
        assert!(assessment.matched_keywords.contains(&"nice to have".to_string()));
    }

    #[test]
    fn test_low_priority_maps_to_p4() {
        let assessment = assess_priority(
            "Low priority cleanup",
            "This is a low priority cleanup task",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P4);
        assert!(assessment.matched_keywords.contains(&"low priority".to_string()));
    }

    #[test]
    fn test_backlog_maps_to_p4() {
        let assessment = assess_priority(
            "Add to backlog",
            "This feature belongs in the backlog for future consideration",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P4);
        assert!(assessment.matched_keywords.contains(&"backlog".to_string()));
    }

    #[test]
    fn test_no_keywords_defaults_to_p3() {
        let assessment = assess_priority(
            "Simple feature request",
            "It would be convenient to have a dark mode toggle",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P3);
        assert!(assessment.matched_keywords.is_empty());
    }

    #[test]
    fn test_p1_takes_precedence_over_p2() {
        let assessment = assess_priority(
            "Urgent important feature",
            "This is an important but also urgent feature",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P1);
    }

    #[test]
    fn test_p2_takes_precedence_over_p3() {
        let assessment = assess_priority(
            "Important normal feature",
            "This is an important feature with normal scope",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P2);
    }

    #[test]
    fn test_multiple_p1_keywords_all_recorded() {
        let assessment = assess_priority(
            "Critical urgent issue",
            "This is both critical and urgent",
            &[],
        );
        assert_eq!(assessment.priority, Priority::P1);
        assert!(assessment.matched_keywords.contains(&"critical".to_string()));
        assert!(assessment.matched_keywords.contains(&"urgent".to_string()));
    }

    // =============================================================================
    // Human priority label tests (Edge case: must not override)
    // =============================================================================

    #[test]
    fn test_human_p1_label_preserved_over_keywords() {
        // Even though "blocker" is in the text, human label takes precedence
        let assessment = assess_priority(
            "Blocker issue",
            "This is a blocker",
            &["priority:P2".to_string()],
        );
        assert_eq!(assessment.priority, Priority::P2);
        assert!(assessment.human_set);
        assert!(!assessment.matched_keywords.contains(&"blocker".to_string()));
    }

    #[test]
    fn test_human_p4_label_preserved_over_keywords() {
        let assessment = assess_priority(
            "Important urgent feature",
            "This is important and urgent",
            &["priority:P4".to_string()],
        );
        assert_eq!(assessment.priority, Priority::P4);
        assert!(assessment.human_set);
    }

    #[test]
    fn test_human_priority_detection() {
        assert!(has_human_priority_label(&["priority:P1".to_string()]));
        assert!(has_human_priority_label(&["priority:P2".to_string()]));
        assert!(has_human_priority_label(&["priority:P3".to_string()]));
        assert!(has_human_priority_label(&["priority:P4".to_string()]));
        assert!(!has_human_priority_label(&["bug".to_string()]));
        assert!(!has_human_priority_label(&["feature".to_string()]));
        assert!(!has_human_priority_label(&[]));
    }

    #[test]
    fn test_human_priority_case_insensitive() {
        assert!(has_human_priority_label(&["Priority:P1".to_string()]));
        assert!(has_human_priority_label(&["PRIORITY:P2".to_string()]));
    }

    // =============================================================================
    // LLM assessment tests
    // =============================================================================

    #[test]
    fn test_llm_assess_priority_returns_p3_placeholder() {
        let assessment = llm_assess_priority(
            "Ambiguous feature",
            "Something that could be anything",
            &["normal".to_string()],
        );
        assert_eq!(assessment.priority, Priority::P3);
        assert_eq!(assessment.method, "llm");
        assert!(assessment.matched_keywords.contains(&"normal".to_string()));
    }

    #[test]
    fn test_llm_preserves_matched_keywords() {
        let keywords = vec!["urgent".to_string(), "important".to_string()];
        let assessment = llm_assess_priority(
            "Ambiguous",
            "Body text",
            &keywords,
        );
        assert_eq!(assessment.matched_keywords, keywords);
    }

    // =============================================================================
    // Keyword boundary detection tests
    // =============================================================================

    #[test]
    fn test_keyword_boundary_single_word() {
        // "blocker" should match "blocker" but not "blockers" (with 's')
        assert!(contains_keyword("this is a blocker issue", "blocker"));
        assert!(!contains_keyword("blockers are being fixed", "blocker"));
    }

    #[test]
    fn test_keyword_boundary_in_sentence() {
        assert!(contains_keyword("this is critical", "critical"));
        assert!(contains_keyword("the critical path", "critical"));
        assert!(!contains_keyword("critically important", "critical"));
    }

    #[test]
    fn test_multilword_keyword() {
        assert!(contains_keyword("this is high value", "high value"));
        assert!(contains_keyword("nice to have feature", "nice to have"));
        assert!(contains_keyword("low priority task", "low priority"));
        assert!(contains_keyword("backlog item", "backlog"));
    }

    // =============================================================================
    // PriorityAssessment struct tests
    // =============================================================================

    #[test]
    fn test_priority_assessment_keyword_method() {
        let assessment = assess_priority(
            "Urgent fix",
            "This urgent fix is needed",
            &[],
        );
        assert_eq!(assessment.method, "keyword");
        assert!(!assessment.human_set);
    }

    #[test]
    fn test_priority_assessment_human_method() {
        let assessment = assess_priority(
            "Urgent fix",
            "This urgent fix is needed",
            &["priority:P4".to_string()],
        );
        assert_eq!(assessment.method, "human");
        assert!(assessment.human_set);
    }

    // =============================================================================
    // Integration: Full priority assessment flow
    // =============================================================================

    #[test]
    fn test_full_priority_flow_feature_issue() {
        let title = "Add user authentication";
        let body = "This is an important feature for security. It should support OAuth and SSO.";
        let labels = vec!["feature".to_string()];

        let assessment = assess_priority(title, body, &labels);

        assert_eq!(assessment.priority, Priority::P2);
        assert_eq!(assessment.method, "keyword");
        assert!(assessment.matched_keywords.contains(&"important".to_string()));
    }

    #[test]
    fn test_full_priority_flow_no_matches() {
        let title = "Small UI tweak";
        let body = "Change the button color from blue to green.";
        let labels = vec!["feature".to_string()];

        let assessment = assess_priority(title, body, &labels);

        assert_eq!(assessment.priority, Priority::P3);
        assert!(assessment.matched_keywords.is_empty());
    }

    #[test]
    fn test_full_priority_flow_with_multiple_priority_keywords() {
        // When multiple priority keywords are found, the highest wins
        let title = "Critical urgent important feature";
        let body = "This is critical and urgent and important for the release";
        let labels = vec!["feature".to_string()];

        let assessment = assess_priority(title, body, &labels);

        assert_eq!(assessment.priority, Priority::P1);
        // Should contain all matched keywords
        assert!(assessment.matched_keywords.contains(&"critical".to_string()));
        assert!(assessment.matched_keywords.contains(&"urgent".to_string()));
        // "important" should NOT be included since P1 takes precedence
    }
}
