//! Triage state machine.
//!
//! Implements the triage workflow state machine as defined in
//! plans/triage-workflow-plan.md.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Triage states as defined in the workflow plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageState {
    /// Issue is new and unclassified.
    NewUnclassified,
    /// Bug report is incomplete.
    BugIncomplete,
    /// Feature request is incomplete.
    FeatureIncomplete,
    /// Question needs clarification.
    QuestionIncomplete,
    /// Waiting for requestor response.
    NeedsInfo,
    /// No response for 28 days.
    Stale,
    /// Searching docs for question answer.
    SearchDocs,
    /// Documentation found for question.
    DocFound,
    /// No documentation found - doc gap.
    DocGap,
    /// Issue is complete, awaiting human decision.
    ReadyForReview,
    /// Human decided not to work this.
    WillNotDo,
    /// Human approved for implementation.
    ReadyForWork,
    /// Work is underway.
    InProgress,
    /// Issue is closed.
    Closed,
}

impl TriageState {
    /// Get the label associated with this state.
    pub fn label(&self) -> Option<&'static str> {
        match self {
            TriageState::NewUnclassified => None,
            TriageState::BugIncomplete
            | TriageState::FeatureIncomplete
            | TriageState::QuestionIncomplete => Some("needs-information"),
            TriageState::NeedsInfo => Some("needs-information"),
            TriageState::Stale => None,
            TriageState::SearchDocs => None,
            TriageState::DocFound => None,
            TriageState::DocGap => Some("needs-documentation"),
            TriageState::ReadyForReview => Some("ready-for-review"),
            TriageState::WillNotDo => Some("will-not-do"),
            TriageState::ReadyForWork => Some("ready-for-work"),
            TriageState::InProgress => Some("in-progress"),
            TriageState::Closed => None,
        }
    }

    /// Check if this state is terminal (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(self, TriageState::Closed)
    }

    /// Get the human-facing name of this state.
    pub fn display_name(&self) -> &'static str {
        match self {
            TriageState::NewUnclassified => "New / Unclassified",
            TriageState::BugIncomplete => "Bug — Incomplete",
            TriageState::FeatureIncomplete => "Feature — Incomplete",
            TriageState::QuestionIncomplete => "Question — Incomplete",
            TriageState::NeedsInfo => "Needs Information",
            TriageState::Stale => "Stale",
            TriageState::SearchDocs => "Searching Docs",
            TriageState::DocFound => "Documentation Found",
            TriageState::DocGap => "Documentation Gap",
            TriageState::ReadyForReview => "Ready for Review",
            TriageState::WillNotDo => "Will Not Do",
            TriageState::ReadyForWork => "Ready for Work",
            TriageState::InProgress => "In Progress",
            TriageState::Closed => "Closed",
        }
    }
}

impl std::fmt::Display for TriageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// Triage transition event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageEvent {
    /// Issue has been classified.
    Classified,
    /// Missing information is present.
    InfoComplete,
    /// Request for more information posted.
    NeedsInfoPosted,
    /// Requestor responded.
    RequestorResponded,
    /// No response for 14+ days.
    StalePing,
    /// No response for 28+ days.
    StaleClose,
    /// Documentation search started.
    DocSearchStarted,
    /// Documentation found that answers the question.
    DocFound,
    /// No documentation found.
    DocGapFound,
    /// Human applied ready-for-review.
    HumanReadyForReview,
    /// Human applied will-not-do.
    HumanWillNotDo,
    /// Human applied ready-for-work.
    HumanReadyForWork,
    /// Epic and beads created.
    BeadsCreated,
    /// Issue closed.
    IssueClosed,
}

/// Transition error.
#[derive(Debug, Clone)]
pub struct TransitionError {
    pub message: String,
}

impl std::fmt::Display for TransitionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TransitionError {}

/// Triage state machine.
#[derive(Debug, Clone)]
pub struct TriageStateMachine {
    /// Current state.
    state: TriageState,
    /// When needs-information was applied (for stale tracking).
    needs_info_since: Option<DateTime<Utc>>,
    /// Last state change timestamp.
    last_transition: DateTime<Utc>,
    /// Track if we've done a stale ping (14 days).
    did_stale_ping: bool,
}

impl TriageStateMachine {
    /// Create a new state machine in the initial state.
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            state: TriageState::NewUnclassified,
            needs_info_since: None,
            last_transition: now,
            did_stale_ping: false,
        }
    }

    /// Create a state machine with a specific state.
    pub fn from_state(state: TriageState) -> Self {
        let now = Utc::now();
        Self {
            state,
            needs_info_since: None,
            last_transition: now,
            did_stale_ping: false,
        }
    }

    /// Get the current state.
    pub fn state(&self) -> TriageState {
        self.state
    }

    /// Check if needs-info ping is due (14+ days since posting).
    pub fn is_stale_ping_due(&self) -> bool {
        if self.did_stale_ping {
            return false;
        }

        if let Some(since) = self.needs_info_since {
            let elapsed = Utc::now().signed_duration_since(since);
            return elapsed.num_days() >= 14;
        }

        false
    }

    /// Check if issue should be closed as stale (28+ days since posting).
    pub fn is_stale_close_due(&self) -> bool {
        if let Some(since) = self.needs_info_since {
            let elapsed = Utc::now().signed_duration_since(since);
            return elapsed.num_days() >= 28;
        }

        false
    }

    /// Get the stale threshold configuration.
    pub fn stale_days(&self) -> u64 {
        28
    }

    /// Process a transition event.
    pub fn transition(&mut self, event: TriageEvent) -> Result<TriageState, TransitionError> {
        let next_state = match (self.state, event) {
            // Classification flow
            (TriageState::NewUnclassified, TriageEvent::Classified) => {
                // This requires additional context (issue type) to determine next state
                // The caller should use transition_with_classification instead
                return Err(TransitionError {
                    message:
                        "Classified event requires issue type, use transition_with_classification"
                            .to_string(),
                });
            }

            // Bug classification
            (TriageState::NewUnclassified, TriageEvent::Classified) => TriageState::BugIncomplete,

            // Incomplete states
            (TriageState::BugIncomplete, TriageEvent::InfoComplete) => TriageState::ReadyForReview,
            (TriageState::FeatureIncomplete, TriageEvent::InfoComplete) => {
                TriageState::ReadyForReview
            }
            (TriageState::QuestionIncomplete, TriageEvent::InfoComplete) => TriageState::SearchDocs,

            // Request needs information
            (TriageState::BugIncomplete, TriageEvent::NeedsInfoPosted)
            | (TriageState::FeatureIncomplete, TriageEvent::NeedsInfoPosted)
            | (TriageState::QuestionIncomplete, TriageEvent::NeedsInfoPosted) => {
                self.needs_info_since = Some(Utc::now());
                self.did_stale_ping = false;
                TriageState::NeedsInfo
            }

            // Requestor responds - restart triage
            (TriageState::NeedsInfo, TriageEvent::RequestorResponded) => {
                self.needs_info_since = None;
                TriageState::NewUnclassified
            }

            // Stale ping
            (TriageState::NeedsInfo, TriageEvent::StalePing) => {
                self.did_stale_ping = true;
                TriageState::NeedsInfo // Still needs info, we just posted a ping
            }

            // Stale close
            (TriageState::NeedsInfo, TriageEvent::StaleClose) => TriageState::Stale,

            // Question flow
            (TriageState::QuestionIncomplete, TriageEvent::InfoComplete)
            | (TriageState::NewUnclassified, TriageEvent::DocSearchStarted) => {
                TriageState::SearchDocs
            }
            (TriageState::SearchDocs, TriageEvent::DocFound) => TriageState::DocFound,
            (TriageState::SearchDocs, TriageEvent::DocGapFound) => TriageState::DocGap,
            (TriageState::DocFound, TriageEvent::IssueClosed) => TriageState::Closed,
            (TriageState::DocGap, TriageEvent::BeadsCreated) => {
                TriageState::ReadyForReview // Or leave open, depends on config
            }

            // Human decision gate
            (TriageState::ReadyForReview, TriageEvent::HumanWillNotDo) => TriageState::WillNotDo,
            (TriageState::ReadyForReview, TriageEvent::HumanReadyForWork) => {
                TriageState::ReadyForWork
            }

            // Will not do flow
            (TriageState::WillNotDo, TriageEvent::IssueClosed) => TriageState::Closed,

            // Ready for work flow
            (TriageState::ReadyForWork, TriageEvent::BeadsCreated) => TriageState::InProgress,

            // In progress tracking
            (TriageState::InProgress, TriageEvent::IssueClosed) => TriageState::Closed,

            // Stale closure
            (TriageState::Stale, TriageEvent::IssueClosed) => TriageState::Closed,

            // Invalid transitions
            (state, event) => {
                return Err(TransitionError {
                    message: format!("Invalid transition: {:?} from {:?}", event, state),
                });
            }
        };

        self.state = next_state;
        self.last_transition = Utc::now();
        Ok(next_state)
    }

    /// Transition with classification context (for initial classification).
    pub fn transition_with_classification(
        &mut self,
        event: TriageEvent,
        issue_type: &str,
        completeness: &str,
    ) -> Result<TriageState, TransitionError> {
        // Handle initial classification specially
        if self.state == TriageState::NewUnclassified && event == TriageEvent::Classified {
            let next_state = match issue_type {
                "bug" => {
                    if completeness == "complete" {
                        TriageState::ReadyForReview
                    } else {
                        TriageState::BugIncomplete
                    }
                }
                "feature" => {
                    if completeness == "complete" {
                        TriageState::ReadyForReview
                    } else {
                        TriageState::FeatureIncomplete
                    }
                }
                "question" => {
                    if completeness == "complete" {
                        TriageState::SearchDocs
                    } else {
                        TriageState::QuestionIncomplete
                    }
                }
                // Docs, chore, unknown - leave unclassified until human decides
                _ => TriageState::NewUnclassified,
            };

            self.state = next_state;
            self.last_transition = Utc::now();
            return Ok(next_state);
        }

        // Fall back to regular transition
        self.transition(event)
    }

    /// Update state from GitHub labels.
    pub fn infer_from_labels(&mut self, labels: &[String], has_rodgers_label: bool) {
        // If we already have a Rodgers label, infer state from it
        if has_rodgers_label {
            for label in labels {
                if label == "needs-information" {
                    if self.state == TriageState::NewUnclassified
                        || self.state == TriageState::BugIncomplete
                        || self.state == TriageState::FeatureIncomplete
                        || self.state == TriageState::QuestionIncomplete
                    {
                        self.state = TriageState::NeedsInfo;
                        self.needs_info_since = Some(Utc::now());
                    }
                } else if label == "ready-for-review" {
                    self.state = TriageState::ReadyForReview;
                } else if label == "will-not-do" {
                    self.state = TriageState::WillNotDo;
                } else if label == "ready-for-work" {
                    self.state = TriageState::ReadyForWork;
                } else if label == "in-progress" {
                    self.state = TriageState::InProgress;
                } else if label == "needs-documentation" {
                    self.state = TriageState::DocGap;
                }
            }
        }
    }
}

impl Default for TriageStateMachine {
    fn default() -> Self {
        Self::new()
    }
}

/// State transition for logging/debugging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateTransition {
    /// Issue number.
    pub issue: i32,
    /// Previous state.
    pub from: TriageState,
    /// New state.
    pub to: TriageState,
    /// Event that triggered the transition.
    pub event: TriageEvent,
    /// When the transition occurred.
    pub timestamp: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial_state() {
        let sm = TriageStateMachine::new();
        assert_eq!(sm.state(), TriageState::NewUnclassified);
    }

    #[test]
    fn test_bug_incomplete_to_ready() {
        let mut sm = TriageStateMachine::from_state(TriageState::BugIncomplete);
        let result = sm.transition(TriageEvent::InfoComplete);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::ReadyForReview);
    }

    #[test]
    fn test_classification_bug_complete() {
        let mut sm = TriageStateMachine::new();
        let result = sm.transition_with_classification(TriageEvent::Classified, "bug", "complete");
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::ReadyForReview);
    }

    #[test]
    fn test_classification_bug_incomplete() {
        let mut sm = TriageStateMachine::new();
        let result =
            sm.transition_with_classification(TriageEvent::Classified, "bug", "incomplete");
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::BugIncomplete);
    }

    #[test]
    fn test_classification_feature_incomplete() {
        let mut sm = TriageStateMachine::new();
        let result =
            sm.transition_with_classification(TriageEvent::Classified, "feature", "incomplete");
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::FeatureIncomplete);
    }

    #[test]
    fn test_classification_question_complete() {
        let mut sm = TriageStateMachine::new();
        let result =
            sm.transition_with_classification(TriageEvent::Classified, "question", "complete");
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::SearchDocs);
    }

    #[test]
    fn test_classification_question_incomplete() {
        let mut sm = TriageStateMachine::new();
        let result =
            sm.transition_with_classification(TriageEvent::Classified, "question", "incomplete");
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::QuestionIncomplete);
    }

    #[test]
    fn test_needs_info_workflow() {
        let mut sm = TriageStateMachine::from_state(TriageState::BugIncomplete);

        // Post needs info
        let result = sm.transition(TriageEvent::NeedsInfoPosted);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::NeedsInfo);
        assert!(sm.needs_info_since.is_some());

        // Requestor responds
        let result = sm.transition(TriageEvent::RequestorResponded);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::NewUnclassified);
        assert!(sm.needs_info_since.is_none());
    }

    #[test]
    fn test_stale_ping_tracking() {
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);

        // Before posting needs info, set the timestamp to 15 days ago
        let fifteen_days_ago = Utc::now() - chrono::Duration::days(15);
        sm.needs_info_since = Some(fifteen_days_ago);

        // Should be due for stale ping
        assert!(sm.is_stale_ping_due());
        assert!(!sm.is_stale_close_due());
    }

    #[test]
    fn test_stale_close_tracking() {
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);

        // Set timestamp to 29 days ago
        let twenty_nine_days_ago = Utc::now() - chrono::Duration::days(29);
        sm.needs_info_since = Some(twenty_nine_days_ago);

        // After posting first ping, we shouldn't ping again
        sm.did_stale_ping = true;

        // Should be due for stale close
        assert!(!sm.is_stale_ping_due()); // We already pinged
        assert!(sm.is_stale_close_due());
    }

    #[test]
    fn test_human_gate_will_not_do() {
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForReview);
        let result = sm.transition(TriageEvent::HumanWillNotDo);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::WillNotDo);
    }

    #[test]
    fn test_human_gate_ready_for_work() {
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForReview);
        let result = sm.transition(TriageEvent::HumanReadyForWork);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::ReadyForWork);
    }

    #[test]
    fn test_ready_for_work_to_in_progress() {
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForWork);
        let result = sm.transition(TriageEvent::BeadsCreated);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::InProgress);
    }

    #[test]
    fn test_invalid_transition() {
        let mut sm = TriageStateMachine::new();
        let result = sm.transition(TriageEvent::InfoComplete);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().message,
            "Invalid transition: InfoComplete from NewUnclassified"
        );
    }

    #[test]
    fn test_doc_flow() {
        let mut sm = TriageStateMachine::new();

        // Classify as question complete
        sm.transition_with_classification(TriageEvent::Classified, "question", "complete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::SearchDocs);

        // Find docs
        let result = sm.transition(TriageEvent::DocFound);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::DocFound);

        // Close it
        let result = sm.transition(TriageEvent::IssueClosed);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::Closed);
    }

    #[test]
    fn test_infer_from_labels() {
        let mut sm = TriageStateMachine::new();
        let labels = vec!["needs-information".to_string()];
        sm.infer_from_labels(&labels, true);
        assert_eq!(sm.state(), TriageState::NeedsInfo);
    }

    #[test]
    fn test_state_label_mapping() {
        assert_eq!(
            TriageState::BugIncomplete.label(),
            Some("needs-information")
        );
        assert_eq!(
            TriageState::ReadyForReview.label(),
            Some("ready-for-review")
        );
        assert_eq!(TriageState::WillNotDo.label(), Some("will-not-do"));
        assert_eq!(TriageState::InProgress.label(), Some("in-progress"));
        assert_eq!(TriageState::Closed.label(), None);
    }

    #[test]
    fn test_is_terminal() {
        assert!(!TriageState::NewUnclassified.is_terminal());
        assert!(!TriageState::NeedsInfo.is_terminal());
        assert!(TriageState::Closed.is_terminal());
    }

    #[test]
    fn test_state_display_name() {
        assert_eq!(
            TriageState::NewUnclassified.display_name(),
            "New / Unclassified"
        );
        assert_eq!(
            TriageState::ReadyForReview.display_name(),
            "Ready for Review"
        );
    }

    #[test]
    fn test_state_serialization() {
        let json = serde_json::to_string(&TriageState::NewUnclassified).unwrap();
        assert!(json.contains("new_unclassified"));

        let state: TriageState = serde_json::from_str(&json).unwrap();
        assert_eq!(state, TriageState::NewUnclassified);
    }

    #[test]
    fn test_event_serialization() {
        let json = serde_json::to_string(&TriageEvent::Classified).unwrap();
        assert!(json.contains("classified"));

        let event: TriageEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, TriageEvent::Classified);
    }
}
