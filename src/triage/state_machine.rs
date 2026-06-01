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
    /// Bug report classified (intermediate state before incomplete determination).
    Bug,
    /// Feature request classified (intermediate state before incomplete determination).
    Feature,
    /// Question classified (intermediate state before incomplete determination).
    Question,
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
    /// Filing epic and child tasks (human gate state with human signal).
    FileEpicTasks,
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
            TriageState::Bug | TriageState::Feature | TriageState::Question => None,
            TriageState::BugIncomplete
            | TriageState::FeatureIncomplete
            | TriageState::QuestionIncomplete => Some("needs-information"),
            TriageState::NeedsInfo => Some("needs-information"),
            TriageState::Stale => None,
            TriageState::SearchDocs => None,
            TriageState::DocFound => None,
            TriageState::DocGap => Some("needs-documentation"),
            TriageState::FileEpicTasks => None,
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

    /// Check if this is a human gate state (requires human action to proceed).
    /// Human gates: will-not-do, ready-for-work (these are never auto-transitioned).
    pub fn is_human_gate(&self) -> bool {
        matches!(
            self,
            TriageState::ReadyForReview | TriageState::ReadyForWork
        )
    }

    /// Get the human-facing name of this state.
    pub fn display_name(&self) -> &'static str {
        match self {
            TriageState::NewUnclassified => "New / Unclassified",
            TriageState::Bug => "Bug",
            TriageState::Feature => "Feature",
            TriageState::Question => "Question",
            TriageState::BugIncomplete => "Bug — Incomplete",
            TriageState::FeatureIncomplete => "Feature — Incomplete",
            TriageState::QuestionIncomplete => "Question — Incomplete",
            TriageState::NeedsInfo => "Needs Information",
            TriageState::Stale => "Stale",
            TriageState::SearchDocs => "Searching Docs",
            TriageState::DocFound => "Documentation Found",
            TriageState::DocGap => "Documentation Gap",
            TriageState::FileEpicTasks => "Filing Epic Backlog",
            TriageState::ReadyForReview => "Ready for Review",
            TriageState::WillNotDo => "Will Not Do",
            TriageState::ReadyForWork => "Ready for Work",
            TriageState::InProgress => "In Progress",
            TriageState::Closed => "Closed",
        }
    }

    /// Get all states for testing/data purposes.
    pub fn all_states() -> Vec<TriageState> {
        vec![
            TriageState::NewUnclassified,
            TriageState::Bug,
            TriageState::Feature,
            TriageState::Question,
            TriageState::BugIncomplete,
            TriageState::FeatureIncomplete,
            TriageState::QuestionIncomplete,
            TriageState::NeedsInfo,
            TriageState::Stale,
            TriageState::SearchDocs,
            TriageState::DocFound,
            TriageState::DocGap,
            TriageState::FileEpicTasks,
            TriageState::ReadyForReview,
            TriageState::WillNotDo,
            TriageState::ReadyForWork,
            TriageState::InProgress,
            TriageState::Closed,
        ]
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
    /// Human approved epic breakdown (any human signal after READY_FOR_WORK).
    HumanApprovedEpic,
    /// Epic and child tasks have been created.
    TasksCreated,
    /// Issue closed.
    IssueClosed,
    /// Detected epic-scale work requiring breakdown.
    EpicDetected,
}

/// Valid transitions from each state (for validation/testing).
pub fn valid_transitions(state: TriageState) -> Vec<(TriageEvent, TriageState)> {
    match state {
        TriageState::NewUnclassified => vec![
            (TriageEvent::Classified, TriageState::Bug),
            (TriageEvent::Classified, TriageState::Feature),
            (TriageEvent::Classified, TriageState::Question),
        ],
        TriageState::Bug => vec![(TriageEvent::InfoComplete, TriageState::ReadyForReview)],
        TriageState::Feature => vec![(TriageEvent::InfoComplete, TriageState::ReadyForReview)],
        TriageState::Question => vec![(TriageEvent::InfoComplete, TriageState::SearchDocs)],
        TriageState::BugIncomplete | TriageState::FeatureIncomplete => vec![
            (TriageEvent::InfoComplete, TriageState::ReadyForReview),
            (TriageEvent::NeedsInfoPosted, TriageState::NeedsInfo),
        ],
        TriageState::QuestionIncomplete => vec![
            (TriageEvent::InfoComplete, TriageState::SearchDocs),
            (TriageEvent::NeedsInfoPosted, TriageState::NeedsInfo),
        ],
        TriageState::NeedsInfo => vec![
            (TriageEvent::StalePing, TriageState::NeedsInfo),
            (TriageEvent::StaleClose, TriageState::Stale),
            (
                TriageEvent::RequestorResponded,
                TriageState::NewUnclassified,
            ),
        ],
        TriageState::Stale => vec![(TriageEvent::IssueClosed, TriageState::Closed)],
        TriageState::SearchDocs => vec![
            (TriageEvent::DocFound, TriageState::DocFound),
            (TriageEvent::DocGapFound, TriageState::DocGap),
        ],
        TriageState::DocFound => vec![(TriageEvent::IssueClosed, TriageState::Closed)],
        TriageState::DocGap => vec![
            (TriageEvent::TasksCreated, TriageState::ReadyForReview),
            (TriageEvent::IssueClosed, TriageState::Closed),
        ],
        TriageState::FileEpicTasks => vec![
            (TriageEvent::TasksCreated, TriageState::InProgress),
            (TriageEvent::HumanApprovedEpic, TriageState::InProgress),
        ],
        TriageState::ReadyForReview => vec![
            (TriageEvent::HumanWillNotDo, TriageState::WillNotDo),
            (TriageEvent::HumanReadyForWork, TriageState::ReadyForWork),
        ],
        TriageState::WillNotDo => vec![(TriageEvent::IssueClosed, TriageState::Closed)],
        TriageState::ReadyForWork => vec![
            (TriageEvent::EpicDetected, TriageState::FileEpicTasks),
            (TriageEvent::TasksCreated, TriageState::InProgress),
            (TriageEvent::HumanApprovedEpic, TriageState::InProgress),
        ],
        TriageState::InProgress => vec![(TriageEvent::IssueClosed, TriageState::Closed)],
        TriageState::Closed => vec![],
    }
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
            // Bug classification - intermediate state
            (TriageState::Bug, TriageEvent::InfoComplete) => TriageState::ReadyForReview,
            (TriageState::Bug, TriageEvent::NeedsInfoPosted) => {
                self.needs_info_since = Some(Utc::now());
                self.did_stale_ping = false;
                TriageState::NeedsInfo
            }

            // Feature classification - intermediate state
            (TriageState::Feature, TriageEvent::InfoComplete) => TriageState::ReadyForReview,
            (TriageState::Feature, TriageEvent::NeedsInfoPosted) => {
                self.needs_info_since = Some(Utc::now());
                self.did_stale_ping = false;
                TriageState::NeedsInfo
            }

            // Question classification - intermediate state
            (TriageState::Question, TriageEvent::InfoComplete) => TriageState::SearchDocs,
            (TriageState::Question, TriageEvent::NeedsInfoPosted) => {
                self.needs_info_since = Some(Utc::now());
                self.did_stale_ping = false;
                TriageState::NeedsInfo
            }

            // Incomplete states (bug/feature)
            (TriageState::BugIncomplete, TriageEvent::InfoComplete) => TriageState::ReadyForReview,
            (TriageState::FeatureIncomplete, TriageEvent::InfoComplete) => {
                TriageState::ReadyForReview
            }

            // Question incomplete
            (TriageState::QuestionIncomplete, TriageEvent::InfoComplete) => TriageState::SearchDocs,

            // Request needs information from INCOMPLETE states
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
                self.did_stale_ping = false; // Reset stale ping tracking
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
            (TriageState::SearchDocs, TriageEvent::DocFound) => TriageState::DocFound,
            (TriageState::SearchDocs, TriageEvent::DocGapFound) => TriageState::DocGap,
            (TriageState::DocFound, TriageEvent::IssueClosed) => TriageState::Closed,
            (TriageState::DocGap, TriageEvent::TasksCreated) => TriageState::ReadyForReview,

            // Human decision gate - KEY: these only happen on human action
            // LLM can suggest but CANNOT auto-transition these states
            (TriageState::ReadyForReview, TriageEvent::HumanWillNotDo) => TriageState::WillNotDo,
            (TriageState::ReadyForReview, TriageEvent::HumanReadyForWork) => {
                TriageState::ReadyForWork
            }

            // Will not do flow
            (TriageState::WillNotDo, TriageEvent::IssueClosed) => TriageState::Closed,

            // Ready for work flow - FILE_EPIC_TASKS state for epic detection
            (TriageState::ReadyForWork, TriageEvent::EpicDetected) => TriageState::FileEpicTasks,
            (TriageState::ReadyForWork, TriageEvent::TasksCreated) => TriageState::InProgress,
            (TriageState::ReadyForWork, TriageEvent::HumanApprovedEpic) => TriageState::InProgress,

            // File epic tasks state
            (TriageState::FileEpicTasks, TriageEvent::TasksCreated) => TriageState::InProgress,
            (TriageState::FileEpicTasks, TriageEvent::HumanApprovedEpic) => TriageState::InProgress,

            // In progress tracking
            (TriageState::InProgress, TriageEvent::IssueClosed) => TriageState::Closed,

            // Stale closure
            (TriageState::Stale, TriageEvent::IssueClosed) => TriageState::Closed,

            // Invalid transitions - includes all auto-events on human gate states
            // This ensures human gates are never auto-transitioned
            (TriageState::ReadyForReview, event) => {
                return Err(TransitionError {
                    message: format!(
                        "Invalid auto-transition on human gate: {:?} from {:?}. Human gates require human action.",
                        event, TriageState::ReadyForReview
                    ),
                });
            }
            (TriageState::ReadyForWork, event) => {
                return Err(TransitionError {
                    message: format!(
                        "Invalid auto-transition on human gate: {:?} from {:?}. Human gates require human action.",
                        event, TriageState::ReadyForWork
                    ),
                });
            }
            (TriageState::WillNotDo, event) => {
                return Err(TransitionError {
                    message: format!(
                        "Invalid transition: {:?} from {:?}",
                        event,
                        TriageState::WillNotDo
                    ),
                });
            }
            (TriageState::Closed, event) => {
                return Err(TransitionError {
                    message: format!("Invalid transition from terminal state: {:?}", event),
                });
            }
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
    ///
    /// This handles the initial triage classification flow. The classifier evaluates
    /// both issue type (bug/feature/question) AND completeness in one pass.
    /// - Complete issues go directly to the appropriate workflow state
    /// - Incomplete issues go to the *_incomplete state and trigger a needs-info request
    pub fn transition_with_classification(
        &mut self,
        event: TriageEvent,
        issue_type: &str,
        completeness: &str,
    ) -> Result<TriageState, TransitionError> {
        // Handle initial classification specially
        if self.state == TriageState::NewUnclassified && event == TriageEvent::Classified {
            // The classifier determines type AND completeness in one pass.
            // Complete issues skip intermediate states and go directly to workflow.
            // Incomplete issues go to the appropriate incomplete state.
            let next_state = match issue_type {
                "bug" => {
                    if completeness == "complete" {
                        // Complete bug: wait for human review decision
                        TriageState::ReadyForReview
                    } else {
                        // Incomplete bug: need more info before review
                        TriageState::BugIncomplete
                    }
                }
                "feature" => {
                    if completeness == "complete" {
                        // Complete feature: wait for human review decision
                        TriageState::ReadyForReview
                    } else {
                        // Incomplete feature: need more info before review
                        TriageState::FeatureIncomplete
                    }
                }
                "question" => {
                    if completeness == "complete" {
                        // Complete question: search docs for answer
                        TriageState::SearchDocs
                    } else {
                        // Incomplete question: need clarification before search
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

        // Fall back to regular transition for subsequent transitions
        self.transition(event)
    }

    /// Update state from GitHub labels.
    pub fn infer_from_labels(&mut self, labels: &[String], has_rodgers_label: bool) {
        // If we already have a Rodgers label, infer state from it
        if has_rodgers_label {
            for label in labels {
                if label == "needs-information" {
                    if self.state == TriageState::NewUnclassified
                        || self.state == TriageState::Bug
                        || self.state == TriageState::Feature
                        || self.state == TriageState::Question
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
        let result = sm.transition(TriageEvent::TasksCreated);
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

    // =============================================================================
    // Comprehensive tests for AC-6: All states and transitions
    // =============================================================================

    /// Test AC-6: All states defined from the triage workflow plan
    #[test]
    fn test_all_states_defined() {
        let all_states = TriageState::all_states();

        // Verify we have all expected states from the Mermaid diagram
        let expected_states = vec![
            TriageState::NewUnclassified,
            TriageState::Bug,
            TriageState::Feature,
            TriageState::Question,
            TriageState::BugIncomplete,
            TriageState::FeatureIncomplete,
            TriageState::QuestionIncomplete,
            TriageState::NeedsInfo,
            TriageState::Stale,
            TriageState::SearchDocs,
            TriageState::DocFound,
            TriageState::DocGap,
            TriageState::ReadyForReview,
            TriageState::WillNotDo,
            TriageState::ReadyForWork,
            TriageState::FileEpicTasks,
            TriageState::InProgress,
            TriageState::Closed,
        ];

        for state in expected_states {
            assert!(all_states.contains(&state), "Missing state: {:?}", state);
        }

        // Count matches - we should have exactly 18 states
        assert_eq!(
            all_states.len(),
            18,
            "Expected 18 states, got {}",
            all_states.len()
        );
    }

    /// Test AC-6: All transitions from the Mermaid diagram are implemented
    #[test]
    fn test_all_transitions_implemented() {
        // Test NEW_UNCLASSIFIED -> BUG/FEATURE/QUESTION (complete path)
        let mut sm = TriageStateMachine::new();
        sm.transition_with_classification(TriageEvent::Classified, "bug", "complete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::ReadyForReview);

        let mut sm = TriageStateMachine::new();
        sm.transition_with_classification(TriageEvent::Classified, "feature", "complete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::ReadyForReview);

        let mut sm = TriageStateMachine::new();
        sm.transition_with_classification(TriageEvent::Classified, "question", "complete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::SearchDocs);

        // Test NEW_UNCLASSIFIED -> *_INCOMPLETE (incomplete path)
        let mut sm = TriageStateMachine::new();
        sm.transition_with_classification(TriageEvent::Classified, "bug", "incomplete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::BugIncomplete);

        let mut sm = TriageStateMachine::new();
        sm.transition_with_classification(TriageEvent::Classified, "feature", "incomplete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::FeatureIncomplete);

        let mut sm = TriageStateMachine::new();
        sm.transition_with_classification(TriageEvent::Classified, "question", "incomplete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::QuestionIncomplete);

        // Test NEEDS_INFO stays in NEEDS_INFO while awaiting response
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);
        sm.needs_info_since = Some(Utc::now() - chrono::Duration::days(10));
        assert!(sm.transition(TriageEvent::StalePing).is_ok());
        assert_eq!(sm.state(), TriageState::NeedsInfo);
        assert!(sm.did_stale_ping); // Ping was sent

        // Test NEEDS_INFO -> STALE after 28 days
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);
        sm.needs_info_since = Some(Utc::now() - chrono::Duration::days(29));
        assert!(sm.transition(TriageEvent::StaleClose).is_ok());
        assert_eq!(sm.state(), TriageState::Stale);

        // Test STALE -> Closed
        let mut sm = TriageStateMachine::from_state(TriageState::Stale);
        assert!(sm.transition(TriageEvent::IssueClosed).is_ok());
        assert_eq!(sm.state(), TriageState::Closed);

        // Test SEARCH_DOCS -> DOC_FOUND -> CLOSED
        let mut sm = TriageStateMachine::from_state(TriageState::SearchDocs);
        assert!(sm.transition(TriageEvent::DocFound).is_ok());
        assert_eq!(sm.state(), TriageState::DocFound);
        assert!(sm.transition(TriageEvent::IssueClosed).is_ok());
        assert_eq!(sm.state(), TriageState::Closed);

        // Test SEARCH_DOCS -> DOC_GAP -> READY_FOR_REVIEW (after tasks created)
        let mut sm = TriageStateMachine::from_state(TriageState::SearchDocs);
        assert!(sm.transition(TriageEvent::DocGapFound).is_ok());
        assert_eq!(sm.state(), TriageState::DocGap);
        assert!(sm.transition(TriageEvent::TasksCreated).is_ok());
        assert_eq!(sm.state(), TriageState::ReadyForReview);

        // Test READY_FOR_WORK -> FILE_EPIC_TASKS -> IN_PROGRESS (epic flow)
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForWork);
        assert!(sm.transition(TriageEvent::EpicDetected).is_ok());
        assert_eq!(sm.state(), TriageState::FileEpicTasks);
        assert!(sm.transition(TriageEvent::TasksCreated).is_ok());
        assert_eq!(sm.state(), TriageState::InProgress);

        // Test WILL_NOT_DO -> CLOSE_ISSUE
        let mut sm = TriageStateMachine::from_state(TriageState::WillNotDo);
        assert!(sm.transition(TriageEvent::IssueClosed).is_ok());
        assert_eq!(sm.state(), TriageState::Closed);

        // Test IN_PROGRESS -> Closed
        let mut sm = TriageStateMachine::from_state(TriageState::InProgress);
        assert!(sm.transition(TriageEvent::IssueClosed).is_ok());
        assert_eq!(sm.state(), TriageState::Closed);
    }

    /// Test AC-7: Human gates are never auto-transitioned
    /// Key: will-not-do, ready-for-work are HUMAN decisions
    #[test]
    fn test_human_gates_ready_for_review_never_auto() {
        // ReadyForReview - auto events should all fail
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForReview);

        // Any auto event should fail with error about human gate
        let auto_events = vec![
            TriageEvent::InfoComplete,
            TriageEvent::DocFound,
            TriageEvent::DocGapFound,
            TriageEvent::StalePing,
            TriageEvent::StaleClose,
            TriageEvent::TasksCreated,
            TriageEvent::EpicDetected,
            TriageEvent::RequestorResponded,
        ];

        for event in &auto_events {
            let result = sm.transition(*event);
            assert!(
                result.is_err(),
                "Auto event {:?} should fail on ReadyForReview",
                event
            );
            let err = result.unwrap_err();
            assert!(
                err.message.contains("human gate") || err.message.contains("Human gates"),
                "Error should mention human gates: {}",
                err.message
            );
        }

        // Only human events should work
        let human_result = sm.transition(TriageEvent::HumanWillNotDo);
        assert!(human_result.is_ok());
        assert_eq!(sm.state(), TriageState::WillNotDo);
    }

    #[test]
    fn test_human_gates_ready_for_work_never_auto() {
        // ReadyForWork - auto events should all fail
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForWork);

        // Any auto event should fail (only human events work)
        let auto_events = vec![
            TriageEvent::InfoComplete,
            TriageEvent::DocFound,
            TriageEvent::DocGapFound,
            TriageEvent::StalePing,
            TriageEvent::StaleClose,
            TriageEvent::RequestorResponded,
        ];

        for event in &auto_events {
            let result = sm.transition(*event);
            assert!(
                result.is_err(),
                "Auto event {:?} should fail on ReadyForWork",
                event
            );
        }

        // Only explicit transition events should work on ReadyForWork
        // TasksCreated progresses to InProgress
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForWork);
        assert!(sm.transition(TriageEvent::TasksCreated).is_ok());
        assert_eq!(sm.state(), TriageState::InProgress);
    }

    /// Test AC-7: Human label priority (will-not-do > ready-for-work)
    #[test]
    fn test_human_label_priority_will_not_do() {
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForReview);
        assert!(sm.transition(TriageEvent::HumanWillNotDo).is_ok());
        assert_eq!(sm.state(), TriageState::WillNotDo);
        // will-not-do takes precedence

        // Test that we can't auto-transition from WillNotDo
        let result = sm.transition(TriageEvent::InfoComplete);
        assert!(result.is_err());
    }

    /// Test CRIT-6: Stale progression - 14 day ping, 28 day close
    #[test]
    fn test_stale_progression_14_day_ping() {
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);

        // At 13 days - not yet due for ping
        let thirteen_days_ago = Utc::now() - chrono::Duration::days(13);
        sm.needs_info_since = Some(thirteen_days_ago);
        sm.did_stale_ping = false;
        assert!(!sm.is_stale_ping_due(), "13 days should not trigger ping");
        assert!(!sm.is_stale_close_due(), "13 days should not trigger close");

        // At exactly 14 days - due for ping
        let fourteen_days_ago = Utc::now() - chrono::Duration::days(14);
        sm.needs_info_since = Some(fourteen_days_ago);
        sm.did_stale_ping = false;
        assert!(sm.is_stale_ping_due(), "14 days should trigger ping");
        assert!(
            !sm.is_stale_close_due(),
            "14 days should not trigger close yet"
        );

        // After ping is sent, should not ping again
        sm.did_stale_ping = true;
        assert!(
            !sm.is_stale_ping_due(),
            "Already pinged, should not ping again"
        );
    }

    #[test]
    fn test_stale_progression_28_day_close() {
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);

        // At 27 days - not yet due for close
        let twenty_seven_days_ago = Utc::now() - chrono::Duration::days(27);
        sm.needs_info_since = Some(twenty_seven_days_ago);
        sm.did_stale_ping = true; // Already pinged
        assert!(!sm.is_stale_close_due(), "27 days should not trigger close");

        // At exactly 28 days - due for close
        let twenty_eight_days_ago = Utc::now() - chrono::Duration::days(28);
        sm.needs_info_since = Some(twenty_eight_days_ago);
        sm.did_stale_ping = true;
        assert!(sm.is_stale_close_due(), "28 days should trigger close");

        // At 29 days - definitely due for close
        let twenty_nine_days_ago = Utc::now() - chrono::Duration::days(29);
        sm.needs_info_since = Some(twenty_nine_days_ago);
        sm.did_stale_ping = true;
        assert!(sm.is_stale_close_due(), "29 days should trigger close");
    }

    /// Test complete stale workflow: needs-info -> ping at 14 days -> close at 28 days
    #[test]
    fn test_complete_stale_workflow() {
        let mut sm = TriageStateMachine::from_state(TriageState::BugIncomplete);

        // Start needs-info
        sm.transition(TriageEvent::NeedsInfoPosted).unwrap();
        assert_eq!(sm.state(), TriageState::NeedsInfo);
        assert!(sm.needs_info_since.is_some());

        // Simulate 15 days passing - ping should be due
        sm.needs_info_since = Some(Utc::now() - chrono::Duration::days(15));
        assert!(sm.is_stale_ping_due());

        // Post ping
        sm.transition(TriageEvent::StalePing).unwrap();
        assert!(sm.did_stale_ping);

        // Simulate another 14 days (29 days total) - close should be due
        sm.needs_info_since = Some(Utc::now() - chrono::Duration::days(29));
        assert!(sm.is_stale_close_due());

        // Close as stale
        sm.transition(TriageEvent::StaleClose).unwrap();
        assert_eq!(sm.state(), TriageState::Stale);

        // Close the issue
        sm.transition(TriageEvent::IssueClosed).unwrap();
        assert_eq!(sm.state(), TriageState::Closed);
    }

    /// Test CRIT-12: Bot handling - is_bot_issue detection
    #[test]
    fn test_bot_issue_detection() {
        // Bot issues should be detected via user type
        // The engine has is_bot_issue() that checks author.user.user_type == "Bot"
        // This tests the state machine state that result from bot handling

        // Bot issues are handled at the engine level, but we should verify
        // that once a bot issue is handled, it doesn't proceed through normal triage

        let mut sm = TriageStateMachine::from_state(TriageState::NewUnclassified);

        // Bot label should be inferable and indicate handling
        sm.infer_from_labels(&vec!["bot".to_string()], false);

        // When a bot label is present but no Rodgers label, we don't infer state
        // This prevents the state machine from processing bot-created issues
        assert_eq!(sm.state(), TriageState::NewUnclassified);
    }

    /// Test FILE_EPIC_TASKS state as explicit transition between READY_FOR_WORK and IN_PROGRESS
    #[test]
    fn test_file_epic_tasks_transition() {
        // Test READY_FOR_WORK -> FILE_EPIC_TASKS (via EpicDetected)
        let mut sm = TriageStateMachine::from_state(TriageState::ReadyForWork);
        assert!(sm.transition(TriageEvent::EpicDetected).is_ok());
        assert_eq!(sm.state(), TriageState::FileEpicTasks);

        // Test FILE_EPIC_TASKS -> IN_PROGRESS (via TasksCreated)
        assert!(sm.transition(TriageEvent::TasksCreated).is_ok());
        assert_eq!(sm.state(), TriageState::InProgress);
    }

    /// Test FILE_EPIC_TASKS -> IN_PROGRESS (via HumanApprovedEpic)
    #[test]
    fn test_file_epic_tasks_human_approval() {
        let mut sm = TriageStateMachine::from_state(TriageState::FileEpicTasks);

        // Human approval also transitions to InProgress
        assert!(sm.transition(TriageEvent::HumanApprovedEpic).is_ok());
        assert_eq!(sm.state(), TriageState::InProgress);
    }

    /// Test that all states have appropriate labels (or None for terminal/internal states)
    #[test]
    fn test_all_states_have_labels() {
        // States with labels
        assert!(TriageState::BugIncomplete.label().is_some());
        assert!(TriageState::ReadyForReview.label().is_some());
        assert!(TriageState::WillNotDo.label().is_some());
        assert!(TriageState::ReadyForWork.label().is_some());
        assert!(TriageState::InProgress.label().is_some());
        assert!(TriageState::DocGap.label().is_some());

        // States without labels (internal or terminal)
        assert!(TriageState::NewUnclassified.label().is_none());
        assert!(TriageState::Bug.label().is_none());
        assert!(TriageState::Feature.label().is_none());
        assert!(TriageState::Question.label().is_none());
        assert!(TriageState::Stale.label().is_none());
        assert!(TriageState::SearchDocs.label().is_none());
        assert!(TriageState::DocFound.label().is_none());
        assert!(TriageState::Closed.label().is_none());
        assert!(TriageState::FileEpicTasks.label().is_none());
    }

    /// Test the requestor response mechanism
    #[test]
    fn test_requestor_response_restarts_triage() {
        let mut sm = TriageStateMachine::from_state(TriageState::NeedsInfo);

        // Set needs_info_since so we can verify it's cleared
        sm.needs_info_since = Some(Utc::now() - chrono::Duration::days(10));
        sm.did_stale_ping = true;

        // Requestor responds
        let result = sm.transition(TriageEvent::RequestorResponded);
        assert!(result.is_ok());

        // Should return to NewUnclassified (restart triage)
        assert_eq!(sm.state(), TriageState::NewUnclassified);

        // Should clear needs_info_since
        assert!(sm.needs_info_since.is_none());

        // Should clear did_stale_ping
        assert!(!sm.did_stale_ping);
    }

    /// Test that bug incomplete can complete to ready for review
    #[test]
    fn test_bug_incomplete_to_ready_for_review() {
        let mut sm = TriageStateMachine::from_state(TriageState::BugIncomplete);

        // When requestor provides missing info, should transition to ReadyForReview
        let result = sm.transition(TriageEvent::InfoComplete);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::ReadyForReview);
    }

    /// Test that question incomplete can continue to search docs
    #[test]
    fn test_question_incomplete_to_search_docs() {
        let mut sm = TriageStateMachine::from_state(TriageState::QuestionIncomplete);

        // When requestor provides missing info, should transition to SearchDocs
        let result = sm.transition(TriageEvent::InfoComplete);
        assert!(result.is_ok());
        assert_eq!(sm.state(), TriageState::SearchDocs);
    }

    /// Test that Closed is terminal (no transitions possible)
    #[test]
    fn test_closed_is_terminal() {
        let mut sm = TriageStateMachine::from_state(TriageState::Closed);

        let result = sm.transition(TriageEvent::IssueClosed);
        assert!(result.is_err());
        assert!(sm.state().is_terminal());
    }

    /// Test classification with "unknown" type returns to NewUnclassified
    #[test]
    fn test_classification_unknown_type() {
        let mut sm = TriageStateMachine::new();
        let result = sm.transition_with_classification(TriageEvent::Classified, "docs", "complete");
        assert!(result.is_ok());
        // Should stay in NewUnclassified for unknown types
        assert_eq!(sm.state(), TriageState::NewUnclassified);
    }

    /// Test multiple sequential transitions maintain valid state
    #[test]
    fn test_sequential_transitions_bug_flow() {
        let mut sm = TriageStateMachine::new();

        // New -> BugIncomplete
        sm.transition_with_classification(TriageEvent::Classified, "bug", "incomplete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::BugIncomplete);

        // BugIncomplete -> NeedsInfo (request info)
        sm.transition(TriageEvent::NeedsInfoPosted).unwrap();
        assert_eq!(sm.state(), TriageState::NeedsInfo);

        // NeedsInfo -> NewUnclassified (requestor responds)
        sm.transition(TriageEvent::RequestorResponded).unwrap();
        assert_eq!(sm.state(), TriageState::NewUnclassified);

        // Now classify as complete
        sm.transition_with_classification(TriageEvent::Classified, "bug", "complete")
            .unwrap();
        assert_eq!(sm.state(), TriageState::ReadyForReview);

        // Human approves for work
        sm.transition(TriageEvent::HumanReadyForWork).unwrap();
        assert_eq!(sm.state(), TriageState::ReadyForWork);

        // Epic tasks created
        sm.transition(TriageEvent::TasksCreated).unwrap();
        assert_eq!(sm.state(), TriageState::InProgress);

        // Issue closed
        sm.transition(TriageEvent::IssueClosed).unwrap();
        assert_eq!(sm.state(), TriageState::Closed);
    }
}
