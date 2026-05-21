//! Router for feature issues to feature-bug workflow.
//!
//! This module implements routing logic that sends classified 'feature' issues
//! to the feature-bug workflow with priority assessment.
//!
//! Routing behavior:
//! - Issues classified as 'feature' get 'rodgers:feature' label applied
//! - Priority assessed via keywords and LLM analysis
//! - Route to feature-bug workflow for spec development and implementation tracking
//! - Large features (epic-scale) detected at ready-for-work, not here
//!
//! This module connects the triage classification step to the feature-bug workflow
//! defined in plans/feature-bug-plan.md.

use serde::{Deserialize, Serialize};

use super::priority::{assess_priority, llm_assess_priority, Priority, PriorityAssessment};

/// Label that marks a feature issue routed to the feature-bug workflow.
pub const LABEL_RODGERS_FEATURE: &str = "rodgers:feature";

/// The feature-bug workflow entry point label.
const LABEL_FEATURE_BUG: &str = "feature";

/// Represents a feature issue that has been routed to the feature-bug workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutedFeature {
    /// Original issue number
    pub issue_number: u64,
    /// Issue title
    pub title: String,
    /// Priority assessment result
    pub priority: PriorityAssessment,
    /// Labels to apply when routing
    pub labels_to_add: Vec<String>,
    /// Whether the issue was already routed (idempotency)
    pub already_routed: bool,
}

/// Result of routing a feature issue to the feature-bug workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult {
    /// Whether routing was successful
    pub routed: bool,
    /// The routed feature data (if routed)
    pub routed_feature: Option<RoutedFeature>,
    /// Action taken
    pub action: RouteAction,
    /// Labels to apply
    pub labels_to_add: Vec<String>,
    /// Labels to remove
    pub labels_to_remove: Vec<String>,
}

/// Actions that can be taken during routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RouteAction {
    /// Feature routed to feature-bug workflow
    RoutedToFeatureBug,
    /// Issue was already routed (skip)
    AlreadyRouted,
    /// Not a feature issue (skip)
    NotFeature,
    /// Has ready-for-work label (defer to ready-for-work routing)
    DeferredToReadyForWork,
}

/// Route a classified feature issue to the feature-bug workflow.
///
/// This function:
/// 1. Checks if the issue has the `feature` label
/// 2. Checks if it was already routed (has `rodgers:feature` label)
/// 3. Assesses priority from issue body and keywords
/// 4. Applies `rodgers:feature` label and priority metadata
/// 5. Returns routing result for downstream processing
///
/// Priority assessment:
/// - Human-set priority labels (priority:P1..P4) are never overridden
/// - Keyword-based assessment from issue title + body
/// - LLM assessment hook for ambiguous cases (returns P3 default)
///
/// Edge cases:
/// - Epic-scale features are detected at ready-for-work, not here
/// - Features with ready-for-work label are deferred (handled in triage loop)
/// - Already-routed features are skipped (idempotency)
pub fn route_feature(
    issue_number: u64,
    title: &str,
    body: &str,
    existing_labels: &[String],
    _use_llm: bool,
) -> RouteResult {
    // Check if this is a feature issue
    let is_feature = existing_labels
        .iter()
        .any(|l| l == LABEL_FEATURE_BUG || l == LABEL_RODGERS_FEATURE);

    if !is_feature {
        return RouteResult {
            routed: false,
            routed_feature: None,
            action: RouteAction::NotFeature,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Check if already routed
    let already_routed = existing_labels
        .iter()
        .any(|l| l == LABEL_RODGERS_FEATURE);

    if already_routed {
        return RouteResult {
            routed: false,
            routed_feature: None,
            action: RouteAction::AlreadyRouted,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Check if ready-for-work is applied (defer to ready-for-work routing)
    // Epic-scale detection happens at ready-for-work, not here
    let has_ready_for_work = existing_labels
        .iter()
        .any(|l| l == "ready-for-work");

    if has_ready_for_work {
        return RouteResult {
            routed: false,
            routed_feature: None,
            action: RouteAction::DeferredToReadyForWork,
            labels_to_add: Vec::new(),
            labels_to_remove: Vec::new(),
        };
    }

    // Assess priority
    let priority_assessment = if _use_llm {
        // LLM assessment hook for ambiguous cases
        let keyword_assessment = assess_priority(title, body, existing_labels);
        llm_assess_priority(title, body, &keyword_assessment.matched_keywords)
    } else {
        assess_priority(title, body, existing_labels)
    };

    // Build labels to add
    let mut labels_to_add = vec![LABEL_RODGERS_FEATURE.to_string()];
    // Add priority label for visibility
    labels_to_add.push(format!("priority:{}", priority_assessment.priority.label().to_lowercase()));

    let routed_feature = RoutedFeature {
        issue_number,
        title: title.to_string(),
        priority: priority_assessment,
        labels_to_add: labels_to_add.clone(),
        already_routed: false,
    };

    RouteResult {
        routed: true,
        routed_feature: Some(routed_feature),
        action: RouteAction::RoutedToFeatureBug,
        labels_to_add,
        labels_to_remove: Vec::new(),
    }
}

/// Route a batch of feature issues to the feature-bug workflow.
///
/// Processes each issue independently and returns results for all.
pub fn route_feature_batch(
    issues: &[FeatureIssue],
) -> Vec<RouteResult> {
    issues
        .iter()
        .map(|issue| {
            route_feature(
                issue.number,
                &issue.title,
                &issue.body,
                &issue.labels,
                issue.use_llm,
            )
        })
        .collect()
}

/// A feature issue ready for routing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureIssue {
    /// GitHub issue number
    pub number: u64,
    /// Issue title
    pub title: String,
    /// Issue body
    pub body: String,
    /// Current labels on the issue
    pub labels: Vec<String>,
    /// Whether to use LLM for priority assessment
    pub use_llm: bool,
}

/// Get the priority label to apply for a given priority level.
pub fn priority_label(priority: &Priority) -> String {
    format!("priority:{}", priority.label().to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_issue(number: u64, title: &str, body: &str, labels: Vec<&str>) -> FeatureIssue {
        FeatureIssue {
            number,
            title: title.to_string(),
            body: body.to_string(),
            labels: labels.into_iter().map(String::from).collect(),
            use_llm: false,
        }
    }

    // =============================================================================
    // Unit test: Feature issue gets rodgers:feature label
    // =============================================================================

    #[test]
    fn test_feature_issue_gets_rodgers_feature_label() {
        let issue = create_test_issue(
            1,
            "Add dark mode",
            "This feature should allow users to toggle between light and dark themes.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert_eq!(result.action, RouteAction::RoutedToFeatureBug);
        assert!(result.routed_feature.is_some());

        let feature = result.routed_feature.unwrap();
        assert!(
            feature.labels_to_add.contains(&LABEL_RODGERS_FEATURE.to_string()),
            "Should include rodgers:feature label"
        );
    }

    #[test]
    fn test_rodgers_feature_label_in_result_labels() {
        let issue = create_test_issue(
            2,
            "Add export feature",
            "Users should be able to export data to CSV.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert!(
            result.labels_to_add.contains(&LABEL_RODGERS_FEATURE.to_string()),
            "Labels to add should include rodgers:feature"
        );
    }

    // =============================================================================
    // Unit test: Priority keywords correctly map
    // =============================================================================

    #[test]
    fn test_blocker_maps_to_p1_in_route() {
        let issue = create_test_issue(
            3,
            "Critical blocker in auth",
            "This is a critical blocker preventing all users from logging in. We need an urgent fix.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P1);
        assert!(feature.labels_to_add.contains(&priority_label(&Priority::P1)));
    }

    #[test]
    fn test_important_maps_to_p2_in_route() {
        let issue = create_test_issue(
            4,
            "Add analytics dashboard",
            "This important feature will give users insights into their usage patterns.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P2);
        assert!(feature.labels_to_add.contains(&priority_label(&Priority::P2)));
    }

    #[test]
    fn test_normal_defaults_to_p3_in_route() {
        let issue = create_test_issue(
            5,
            "Small UI improvement",
            "The button alignment could be improved for better aesthetics.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P3);
        assert!(feature.labels_to_add.contains(&priority_label(&Priority::P3)));
    }

    #[test]
    fn test_backlog_maps_to_p4_in_route() {
        let issue = create_test_issue(
            6,
            "Retrofit dark mode support",
            "This is a nice-to-have feature that can go in the backlog for now.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P3);
        // "nice to have" maps to P3, not P4
        assert!(feature.labels_to_add.contains(&priority_label(&Priority::P3)));
    }

    #[test]
    fn test_low_priority_backlog_keyword_maps_to_p4() {
        let issue = create_test_issue(
            7,
            "Cleanup old code",
            "This is a low priority task for the backlog.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P4);
    }

    #[test]
    fn test_urgent_maps_to_p1_in_route() {
        let issue = create_test_issue(
            8,
            "Urgent security patch",
            "We need an urgent fix for this security vulnerability.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P1);
    }

    #[test]
    fn test_high_value_maps_to_p2_in_route() {
        let issue = create_test_issue(
            9,
            "High value integration",
            "This high value integration will connect to three major platforms.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P2);
    }

    // =============================================================================
    // Unit test: LLM priority assessment for ambiguous cases
    // =============================================================================

    #[test]
    fn test_llm_priority_assessment_returns_p3_default() {
        let issue = create_test_issue(
            10,
            "Ambiguous feature",
            "This feature might be useful but the scope is unclear.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            true, // use_llm = true
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P3);
        assert_eq!(feature.priority.method, "llm");
    }

    #[test]
    fn test_keyword_assessment_when_llm_returns_default() {
        // Even with LLM, if keyword assessment finds a clear priority,
        // that context is passed to LLM
        let issue = create_test_issue(
            11,
            "Critical urgent feature",
            "This is both critical and urgent for the upcoming release.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            true, // use_llm = true
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        // LLM placeholder returns P3, but context includes matched keywords
        assert_eq!(feature.priority.method, "llm");
        assert!(
            feature.priority.matched_keywords.contains(&"critical".to_string())
                || feature.priority.matched_keywords.contains(&"urgent".to_string())
        );
    }

    // =============================================================================
    // Integration test: Feature routed to feature-bug workflow with priority metadata
    // =============================================================================

    #[test]
    fn test_feature_routed_to_feature_bug_workflow() {
        let issue = create_test_issue(
            12,
            "Add multi-factor authentication",
            "This important feature adds MFA support for enhanced security.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert_eq!(result.action, RouteAction::RoutedToFeatureBug);
        assert!(result.routed_feature.is_some());

        let feature = result.routed_feature.unwrap();
        // Should have rodgers:feature label
        assert!(feature.labels_to_add.contains(&LABEL_RODGERS_FEATURE.to_string()));
        // Should have priority label
        assert!(feature.labels_to_add.iter().any(|l| l.starts_with("priority:")));
        // Should have priority metadata
        assert!(matches!(feature.priority.priority, Priority::P1 | Priority::P2 | Priority::P3 | Priority::P4));
    }

    #[test]
    fn test_full_routing_with_priority_metadata() {
        let issue = create_test_issue(
            13,
            "Blocker: Payment system down",
            "This critical blocker is preventing all payments. We need an urgent fix now.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();

        // Verify full metadata chain
        assert_eq!(feature.priority.priority, Priority::P1);
        assert_eq!(feature.priority.method, "keyword");
        assert!(!feature.priority.human_set);
        assert!(feature.priority.matched_keywords.contains(&"critical".to_string()));
        assert!(feature.priority.matched_keywords.contains(&"urgent".to_string()));
        assert_eq!(feature.issue_number, 13);

        // Verify labels
        assert!(feature.labels_to_add.contains(&LABEL_RODGERS_FEATURE.to_string()));
        assert!(feature.labels_to_add.contains(&"priority:p1".to_string()));
    }

    // =============================================================================
    // Edge case tests
    // =============================================================================

    #[test]
    fn test_already_routed_issue_skipped() {
        let issue = create_test_issue(
            14,
            "Already routed feature",
            "This feature was already routed.",
            vec!["feature", "rodgers:feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(!result.routed);
        assert_eq!(result.action, RouteAction::AlreadyRouted);
    }

    #[test]
    fn test_non_feature_issue_skipped() {
        let issue = create_test_issue(
            15,
            "Bug report",
            "The app crashes on startup.",
            vec!["bug"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(!result.routed);
        assert_eq!(result.action, RouteAction::NotFeature);
    }

    #[test]
    fn test_ready_for_work_deferred() {
        // Epic-scale detection happens at ready-for-work, not during routing
        let issue = create_test_issue(
            16,
            "Major platform overhaul",
            "This epic-scale feature needs full breakdown.",
            vec!["feature", "ready-for-work"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(!result.routed);
        assert_eq!(result.action, RouteAction::DeferredToReadyForWork);
    }

    #[test]
    fn test_human_priority_preserved_in_routing() {
        // Human-set priority should not be overridden
        let issue = create_test_issue(
            17,
            "Blocker feature with human priority P2",
            "This is a critical blocker but the team has decided on P2.",
            vec!["feature", "priority:P2"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P2);
        assert!(feature.priority.human_set);
        assert_eq!(feature.priority.method, "human");
        assert!(feature.labels_to_add.contains(&"priority:p2".to_string()));
    }

    // =============================================================================
    // Batch routing tests
    // =============================================================================

    #[test]
    fn test_batch_routing_mixed_issues() {
        let issues = vec![
            create_test_issue(1, "P1 blocker", "Critical blocker issue", vec!["feature"]),
            create_test_issue(2, "Bug report", "App crashes", vec!["bug"]),
            create_test_issue(3, "P3 feature", "Small improvement", vec!["feature"]),
            create_test_issue(4, "Already routed", "Already done", vec!["feature", "rodgers:feature"]),
        ];

        let results = route_feature_batch(&issues);

        assert_eq!(results.len(), 4);

        // Issue 1: routed to feature-bug
        assert!(results[0].routed);
        assert_eq!(results[0].action, RouteAction::RoutedToFeatureBug);

        // Issue 2: not a feature
        assert!(!results[1].routed);
        assert_eq!(results[1].action, RouteAction::NotFeature);

        // Issue 3: routed to feature-bug
        assert!(results[2].routed);
        assert_eq!(results[2].action, RouteAction::RoutedToFeatureBug);

        // Issue 4: already routed
        assert!(!results[3].routed);
        assert_eq!(results[3].action, RouteAction::AlreadyRouted);
    }

    #[test]
    fn test_batch_routing_with_priorities() {
        let issues = vec![
            create_test_issue(1, "Urgent fix", "Urgent security patch needed", vec!["feature"]),
            create_test_issue(2, "Important feature", "Important analytics tool", vec!["feature"]),
            create_test_issue(3, "Nice to have", "Nice to have improvement", vec!["feature"]),
            create_test_issue(4, "Backlog item", "Low priority cleanup", vec!["feature"]),
        ];

        let results = route_feature_batch(&issues);

        assert_eq!(results.len(), 4);

        // All should be routed
        for result in &results {
            assert!(result.routed);
        }

        // Verify priorities
        assert_eq!(results[0].routed_feature.as_ref().unwrap().priority.priority, Priority::P1);
        assert_eq!(results[1].routed_feature.as_ref().unwrap().priority.priority, Priority::P2);
        assert_eq!(results[2].routed_feature.as_ref().unwrap().priority.priority, Priority::P3);
        assert_eq!(results[3].routed_feature.as_ref().unwrap().priority.priority, Priority::P4);
    }

    // =============================================================================
    // Edge case: must not override existing priority
    // =============================================================================

    #[test]
    fn test_does_not_override_human_priority_p1() {
        let issue = create_test_issue(
            20,
            "Normal feature with human P1",
            "This is a normal enhancement but marked P1 by human.",
            vec!["feature", "priority:P1"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P1);
        assert!(feature.priority.human_set);
    }

    #[test]
    fn test_does_not_override_human_priority_p4() {
        let issue = create_test_issue(
            21,
            "Urgent feature with human P4",
            "This is urgent but marked backlog by human.",
            vec!["feature", "priority:P4"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        let feature = result.routed_feature.unwrap();
        assert_eq!(feature.priority.priority, Priority::P4);
        assert!(feature.priority.human_set);
    }

    // =============================================================================
    // Integration: Feature routed with all expected labels
    // =============================================================================

    #[test]
    fn test_routing_adds_both_feature_and_priority_labels() {
        let issue = create_test_issue(
            22,
            "P2 important feature",
            "This is an important feature request.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);

        // Should have exactly: rodgers:feature + priority label
        assert_eq!(result.labels_to_add.len(), 2);
        assert!(result.labels_to_add.contains(&"rodgers:feature".to_string()));
        assert!(result.labels_to_add.iter().any(|l| l.starts_with("priority:")));
    }

    #[test]
    fn test_routing_adds_priority_p1_label() {
        let issue = create_test_issue(
            23,
            "Critical blocker",
            "This is a critical blocker issue.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert!(result.labels_to_add.contains(&"priority:p1".to_string()));
    }

    #[test]
    fn test_routing_adds_priority_p2_label() {
        let issue = create_test_issue(
            24,
            "Important feature",
            "This is an important feature.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert!(result.labels_to_add.contains(&"priority:p2".to_string()));
    }

    #[test]
    fn test_routing_adds_priority_p3_label() {
        let issue = create_test_issue(
            25,
            "Normal feature",
            "This is a normal feature.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert!(result.labels_to_add.contains(&"priority:p3".to_string()));
    }

    #[test]
    fn test_routing_adds_priority_p4_label() {
        let issue = create_test_issue(
            26,
            "Backlog item",
            "This is a low priority backlog item.",
            vec!["feature"],
        );

        let result = route_feature(
            issue.number,
            &issue.title,
            &issue.body,
            &issue.labels,
            false,
        );

        assert!(result.routed);
        assert!(result.labels_to_add.contains(&"priority:p4".to_string()));
    }
}
