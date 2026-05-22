//! Triage engine core.
//!
//! The main triage engine that coordinates issue classification and state transitions.

use crate::config::schema::{Config, TriageConfig};
use crate::error::{Result, RogersError};
use crate::github::client::GitHubClient;
use crate::github::models::Issue;
use crate::llm::LlmClient;
use crate::triage::classifier::Classifier;
use crate::triage::state_machine::{TriageEvent, TriageState, TriageStateMachine};
use serde::Serialize;

/// Triage engine for processing GitHub issues.
#[derive(Debug, Clone)]
pub struct TriageEngine {
    /// GitHub client.
    github: GitHubClient,
    /// LLM client.
    llm: LlmClient,
    /// Classifier.
    classifier: Classifier,
    /// Configuration.
    config: Config,
}

/// Triage action to be performed.
#[derive(Debug, Clone, Serialize)]
pub struct TriageAction {
    /// Action type.
    pub action_type: TriageActionType,
    /// Issue number.
    pub issue: i32,
    /// Labels to apply.
    pub labels_to_add: Vec<String>,
    /// Labels to remove.
    pub labels_to_remove: Vec<String>,
    /// Comment to post.
    pub comment: Option<String>,
    /// Whether to close the issue.
    pub close_issue: bool,
}

/// Types of triage actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TriageActionType {
    /// Apply initial classification label.
    ApplyLabel,
    /// Request missing information.
    RequestInfo,
    /// Post stale ping.
    StalePing,
    /// Close as stale.
    CloseStale,
    /// Post doc answer.
    PostDocAnswer,
    /// File doc gap bead.
    FileDocGap,
    /// Post will-not-do response.
    PostWillNotDo,
    /// Post ready-for-review summary.
    PostReadyForReview,
    /// File epic and child beads.
    FileEpic,
    /// No action needed.
    NoAction,
}

impl TriageEngine {
    /// Create a new triage engine.
    pub fn new(config: Config, github: GitHubClient, llm: LlmClient) -> Self {
        let classifier = Classifier::new(llm.clone());
        Self {
            github,
            llm,
            classifier,
            config,
        }
    }

    /// Get the triage configuration.
    pub fn triage_config(&self) -> TriageConfig {
        self.config.triage.clone().unwrap_or_default()
    }

    /// Check if an issue author is a bot.
    pub fn is_bot_issue(issue: &Issue) -> bool {
        issue
            .user
            .user_type
            .as_ref()
            .map(|t| t == "Bot")
            .unwrap_or(false)
    }

    /// Check if an issue should be ignored based on labels.
    pub fn should_ignore(issue: &Issue, config: &Config) -> bool {
        if let Some(ref rogation) = config.rogation {
            if let Some(ref ignore_labels) = rogation.ignore_labels {
                for label in &issue.labels {
                    if ignore_labels.contains(&label.name) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Check if we already have a Rodgers-applied label on this issue.
    pub fn has_rodgers_label(issue: &Issue) -> bool {
        let rodgers_label_prefixes = ["bug", "feature", "question"];
        for label in &issue.labels {
            if rodgers_label_prefixes.contains(&label.name.as_str()) {
                return true;
            }
        }
        false
    }

    /// Process a single issue through the triage engine.
    pub async fn process_issue(&self, issue: &mut Issue) -> Result<Vec<TriageAction>> {
        let mut actions = Vec::new();

        // Check for bot issues
        if Self::is_bot_issue(issue) {
            tracing::info!(
                "Issue #{} is from a bot ({}), applying bot labels and skipping triage",
                issue.number,
                issue.user.login
            );
            actions.extend(self.handle_bot_issue(issue).await?);
            return Ok(actions);
        }

        // Check if we should ignore this issue
        if Self::should_ignore(issue, &self.config) {
            tracing::debug!("Issue #{} has ignore label, skipping", issue.number);
            return Ok(actions);
        }

        // Initialize state machine from existing labels
        let mut state_machine = TriageStateMachine::new();
        state_machine.infer_from_labels(
            &issue
                .labels
                .iter()
                .map(|l| l.name.clone())
                .collect::<Vec<_>>(),
            Self::has_rodgers_label(issue),
        );

        // Process based on current state
        match state_machine.state() {
            TriageState::NewUnclassified => {
                if !Self::has_rodgers_label(issue) {
                    // Classify the issue
                    let classify_result = self.classifier.classify(issue, None).await?;

                    tracing::info!(
                        "Classified issue #{} as {} (confidence: {:?})",
                        issue.number,
                        classify_result.output.issue_type,
                        classify_result.output.confidence
                    );

                    // Transition the state machine
                    state_machine
                        .transition_with_classification(
                            TriageEvent::Classified,
                            &classify_result.output.issue_type,
                            &classify_result.output.completeness,
                        )
                        .map_err(|e| RogersError::Config(e.to_string()))?;

                    // Generate actions based on transition
                    actions.extend(self.generate_actions_for_transition(
                        issue,
                        &state_machine,
                        &classify_result,
                    ));
                }
            }
            TriageState::BugIncomplete
            | TriageState::FeatureIncomplete
            | TriageState::QuestionIncomplete => {
                // Check for requestor response
                if self.has_requestor_response(issue) {
                    state_machine
                        .transition(TriageEvent::RequestorResponded)
                        .ok();
                    // Re-classify
                    let classify_result = self.classifier.check_completeness(issue).await?;
                    state_machine
                        .transition_with_classification(
                            TriageEvent::Classified,
                            &classify_result.output.issue_type,
                            &classify_result.output.completeness,
                        )
                        .map_err(|e| RogersError::Config(e.to_string()))?;
                } else {
                    // Request missing info
                    let classify_result = self.classifier.check_completeness(issue).await?;
                    if !classify_result.output.missing_fields.is_empty() {
                        state_machine.transition(TriageEvent::NeedsInfoPosted).ok();
                        actions.push(self.generate_request_info_action(
                            issue,
                            &classify_result.output.response_draft,
                        ));
                    }
                }
            }
            TriageState::NeedsInfo => {
                // Check timestamps for stale handling
                if state_machine.is_stale_close_due() {
                    state_machine.transition(TriageEvent::StaleClose).ok();
                    actions.push(self.generate_close_stale_action(issue));
                } else if state_machine.is_stale_ping_due() {
                    state_machine.transition(TriageEvent::StalePing).ok();
                    actions.push(self.generate_stale_ping_action(issue));
                } else if self.has_requestor_response(issue) {
                    state_machine
                        .transition(TriageEvent::RequestorResponded)
                        .ok();
                    // No actions needed - will restart triage on next run
                }
            }
            TriageState::ReadyForReview => {
                // Wait for human decision, no action needed
            }
            TriageState::ReadyForWork => {
                // File epic and child beads
                state_machine.transition(TriageEvent::BeadsCreated).ok();
                actions.push(self.generate_file_epic_action(issue));
            }
            TriageState::InProgress => {
                // Check if all work is done
                if self.is_work_complete(issue) {
                    state_machine.transition(TriageEvent::IssueClosed).ok();
                }
            }
            _ => {
                // Other states - take no action
            }
        }

        Ok(actions)
    }

    /// Handle bot-issued issues.
    async fn handle_bot_issue(&self, issue: &Issue) -> Result<Vec<TriageAction>> {
        let mut actions = Vec::new();
        let triage = self.triage_config();

        if let Some(ref bot_labels) = triage.bot_labels {
            if !bot_labels.is_empty() {
                let labels_to_add: Vec<String> = bot_labels
                    .iter()
                    .filter(|l| !issue.labels.iter().any(|il| &il.name == *l))
                    .cloned()
                    .collect();

                if !labels_to_add.is_empty() {
                    actions.push(TriageAction {
                        action_type: TriageActionType::ApplyLabel,
                        issue: issue.number,
                        labels_to_add,
                        labels_to_remove: vec![],
                        comment: None,
                        close_issue: false,
                    });
                }
            }
        }

        Ok(actions)
    }

    /// Generate actions for a state transition.
    fn generate_actions_for_transition(
        &self,
        issue: &Issue,
        state_machine: &TriageStateMachine,
        classification: &crate::triage::classifier::ClassificationResult,
    ) -> Vec<TriageAction> {
        let mut actions = Vec::new();

        let state = state_machine.state();

        // Determine the label to apply
        if let Some(label) = state.label() {
            if !issue.labels.iter().any(|l| l.name == label) {
                actions.push(TriageAction {
                    action_type: TriageActionType::ApplyLabel,
                    issue: issue.number,
                    labels_to_add: vec![label.to_string()],
                    labels_to_remove: vec![],
                    comment: None,
                    close_issue: false,
                });
            }
        }

        // If incomplete, request missing info
        if matches!(
            state,
            TriageState::BugIncomplete
                | TriageState::FeatureIncomplete
                | TriageState::QuestionIncomplete
        ) {
            if let Some(ref response_draft) = classification.output.response_draft {
                // Validate the response draft
                let validation = self.classifier.validate_response_draft(response_draft);
                if validation.is_valid {
                    actions.push(TriageAction {
                        action_type: TriageActionType::RequestInfo,
                        issue: issue.number,
                        labels_to_add: vec![],
                        labels_to_remove: vec![],
                        comment: Some(response_draft.clone()),
                        close_issue: false,
                    });
                }
            }
        }

        actions
    }

    /// Generate action for requesting missing info.
    fn generate_request_info_action(
        &self,
        issue: &Issue,
        response_draft: &Option<String>,
    ) -> TriageAction {
        TriageAction {
            action_type: TriageActionType::RequestInfo,
            issue: issue.number,
            labels_to_add: vec!["needs-information".to_string()],
            labels_to_remove: vec![],
            comment: response_draft.clone(),
            close_issue: false,
        }
    }

    /// Generate action for stale ping.
    fn generate_stale_ping_action(&self, issue: &Issue) -> TriageAction {
        TriageAction {
            action_type: TriageActionType::StalePing,
            issue: issue.number,
            labels_to_add: vec![],
            labels_to_remove: vec![],
            comment: Some(
                "Hi @{}, just following up on this — we want to make sure we haven't missed \
                anything on our end. If you're still seeing the issue, please let us know \
                and we'll keep the conversation going. Otherwise we'll go ahead and close \
                this in a few days."
                    .to_string(),
            ),
            close_issue: false,
        }
    }

    /// Generate action for closing as stale.
    fn generate_close_stale_action(&self, issue: &Issue) -> TriageAction {
        TriageAction {
            action_type: TriageActionType::CloseStale,
            issue: issue.number,
            labels_to_add: vec![],
            labels_to_remove: vec![],
            comment: Some(
                "Hi @{}, we haven't heard back on the information needed to move this \
                forward. If you still want to pursue this, please reopen with the \
                requested details."
                    .to_string(),
            ),
            close_issue: true,
        }
    }

    /// Generate action for filing epic beads.
    fn generate_file_epic_action(&self, issue: &Issue) -> TriageAction {
        // In a full implementation, this would file beads via the bead controller
        // For now, we generate the action for tracking
        TriageAction {
            action_type: TriageActionType::FileEpic,
            issue: issue.number,
            labels_to_add: vec!["in-progress".to_string()],
            labels_to_remove: vec!["ready-for-work".to_string()],
            comment: None,
            close_issue: false,
        }
    }

    /// Check if the requestor has responded since needs-info was posted.
    fn has_requestor_response(&self, issue: &Issue) -> bool {
        // In a full implementation, this would check the issue comments
        // for a comment from the original author
        // For simplicity, we assume response exists if there are comments
        issue.comments > 0
    }

    /// Check if the work on an issue is complete.
    fn is_work_complete(&self, issue: &Issue) -> bool {
        // Check if issue is closed and no open beads linked
        issue.state == "closed"
    }

    /// Execute a triage action on GitHub.
    pub async fn execute_action(&mut self, action: &TriageAction) -> Result<()> {
        tracing::info!(
            "Executing action {:?} for issue #{}",
            action.action_type,
            action.issue
        );

        // Apply labels
        if !action.labels_to_add.is_empty() || !action.labels_to_remove.is_empty() {
            let labels_to_add: Vec<&str> =
                action.labels_to_add.iter().map(|s| s.as_str()).collect();
            let labels_to_remove: Vec<&str> =
                action.labels_to_remove.iter().map(|s| s.as_str()).collect();

            if !labels_to_add.is_empty() {
                self.github
                    .add_issue_labels(action.issue, labels_to_add)
                    .await?;
            }

            for label in labels_to_remove {
                self.github
                    .remove_issue_label(action.issue, label)
                    .await
                    .ok();
            }
        }

        // Post comment
        if let Some(ref comment) = action.comment {
            self.github
                .create_issue_comment(action.issue, comment)
                .await?;
        }

        // Close issue
        if action.close_issue {
            self.github
                .update_issue(
                    action.issue,
                    crate::github::models::UpdateIssueRequest {
                        title: None,
                        body: None,
                        state: Some("closed".to_string()),
                        labels: None,
                        assignees: None,
                        milestone: None,
                    },
                )
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_config() -> Config {
        Config {
            github: crate::config::GitHubConfig {
                owner: "test".to_string(),
                repo: "test".to_string(),
                token: "test".to_string(),
                api_url: None,
            },
            scheduler: crate::config::SchedulerConfig::default(),
            beads: crate::config::BeadsConfig::default(),
            llm: crate::config::LlmConfig {
                provider: Some("openai".to_string()),
                base_url: Some("https://api.openai.com/v1".to_string()),
                model: "gpt-4o-mini".to_string(),
                api_key: "test".to_string(),
            },
            triage: Some(TriageConfig::default()),
            release: None,
            rogation: None,
            log_level: None,
            error_channel: None,
        }
    }

    #[test]
    fn test_is_bot_issue() {
        let bot_issue = Issue {
            number: 1,
            title: "Bot issue".to_string(),
            body: None,
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "snyk-bot".to_string(),
                id: 1,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: Some("Bot".to_string()),
            },
            labels: vec![],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        assert!(TriageEngine::is_bot_issue(&bot_issue));

        let user_issue = Issue {
            number: 2,
            title: "User issue".to_string(),
            body: None,
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "user123".to_string(),
                id: 2,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: vec![],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        assert!(!TriageEngine::is_bot_issue(&user_issue));
    }

    #[test]
    fn test_should_ignore() {
        let mut config = create_test_config();
        config.rogation = Some(crate::config::RogationConfig {
            ignore_labels: Some(vec!["pinned".to_string()]),
            labels_never_bot_managed: None,
            custom_type_names: None,
            format: None,
            agent_file: None,
            template_dir: None,
            security_label: None,
        });

        let ignored_issue = Issue {
            number: 1,
            title: "Pinned issue".to_string(),
            body: None,
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "user".to_string(),
                id: 1,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: vec![crate::github::models::Label {
                id: 1,
                name: "pinned".to_string(),
                description: None,
                color: None,
                node_id: None,
            }],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        assert!(TriageEngine::should_ignore(&ignored_issue, &config));

        let normal_issue = Issue {
            number: 2,
            title: "Normal issue".to_string(),
            body: None,
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "user".to_string(),
                id: 2,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: vec![crate::github::models::Label {
                id: 2,
                name: "bug".to_string(),
                description: None,
                color: None,
                node_id: None,
            }],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        assert!(!TriageEngine::should_ignore(&normal_issue, &config));
    }

    #[test]
    fn test_has_rodgers_label() {
        let issue = Issue {
            number: 1,
            title: "Test".to_string(),
            body: None,
            state: "open".to_string(),
            user: crate::github::models::User {
                login: "user".to_string(),
                id: 1,
                node_id: None,
                avatar_url: None,
                html_url: None,
                user_type: None,
            },
            labels: vec![crate::github::models::Label {
                id: 1,
                name: "bug".to_string(),
                description: None,
                color: None,
                node_id: None,
            }],
            assignees: vec![],
            milestone: None,
            comments: 0,
            closed_at: None,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            pull_request: None,
            node_id: None,
            url: None,
            html_url: None,
        };

        assert!(TriageEngine::has_rodgers_label(&issue));
    }

    #[test]
    fn test_triage_action_serialization() {
        let action = TriageAction {
            action_type: TriageActionType::ApplyLabel,
            issue: 123,
            labels_to_add: vec!["bug".to_string()],
            labels_to_remove: vec![],
            comment: None,
            close_issue: false,
        };

        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("apply_label"));
        assert!(json.contains("123"));
    }

    #[test]
    fn test_validation_result() {
        let config = create_test_config();
        let llm = LlmClient::new(&config.llm);
        let github = GitHubClient::new(
            &config.github.owner,
            &config.github.repo,
            crate::github::GitHubAuth::new_with_default_api(&config.github.token),
        );
        let _engine = TriageEngine::new(config, github, llm);
    }
}
