//! High-level Backlog.md task operations.

use crate::backlog::client::{BacklogClient, FileTaskRequest, TaskStatus, TaskType};
use crate::backlog::schema::{status, task_type, Child, Epic};
use crate::error::{Result, RogersError};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Controller for epic and child task operations.
#[derive(Debug, Clone)]
pub struct TaskController {
    client: Arc<BacklogClient>,
}

impl TaskController {
    pub fn new(client: Arc<BacklogClient>) -> Self {
        Self { client }
    }

    pub fn from_client(client: BacklogClient) -> Self {
        Self {
            client: Arc::new(client),
        }
    }

    /// File an epic task linked to a GitHub issue.
    pub async fn file_epic(&self, request: CreateEpicRequest) -> Result<Epic> {
        let now = Utc::now();
        let file_request = FileTaskRequest {
            title: request.title.clone(),
            description: request.description.clone().unwrap_or_default(),
            task_type: TaskType::Epic,
            priority: request.priority.unwrap_or(2),
            is_epic: true,
            parent_id: None,
            status: TaskStatus::Closed,
            labels: split_labels(request.rodgers_labels.as_deref()),
        };
        let id = self.client.create_task(&file_request)?;

        Ok(Epic {
            id,
            title: request.title,
            description: request.description,
            task_type: request
                .task_type
                .unwrap_or_else(|| task_type::EPIC.to_string()),
            status: status::CLOSED.to_string(),
            github_issue_url: request.github_issue_url,
            github_issue_state: Some("open".to_string()),
            rodgers_type: request.rodgers_type,
            rodgers_labels: request.rodgers_labels,
            rodgers_parent: None,
            discovered_from: request.discovered_from,
            acceptance_criteria: request.acceptance_criteria,
            priority: request.priority,
            assignee: None,
            created_at: now,
            updated_at: now,
        })
    }

    /// File child tasks for an epic.
    pub async fn file_children(
        &self,
        parent_id: &str,
        requests: Vec<CreateChildRequest>,
    ) -> Result<Vec<Child>> {
        let mut children = Vec::new();

        for request in requests {
            let now = Utc::now();
            let file_request = FileTaskRequest {
                title: request.title.clone(),
                description: request.description.clone().unwrap_or_default(),
                task_type: TaskType::Task,
                priority: request.priority.unwrap_or(2),
                is_epic: false,
                parent_id: if parent_id.is_empty() {
                    None
                } else {
                    Some(parent_id.to_string())
                },
                status: TaskStatus::Closed,
                labels: split_labels(request.rodgers_labels.as_deref()),
            };
            let id = self.client.create_task(&file_request)?;
            children.push(Child {
                id,
                parent_id: parent_id.to_string(),
                title: request.title,
                description: request.description,
                task_type: request
                    .task_type
                    .unwrap_or_else(|| task_type::TASK.to_string()),
                status: status::CLOSED.to_string(),
                github_issue_url: None,
                rodgers_type: request.rodgers_type,
                rodgers_labels: request.rodgers_labels,
                rodgers_parent: if parent_id.is_empty() {
                    None
                } else {
                    Some(parent_id.to_string())
                },
                discovered_from: None,
                acceptance_criteria: request.acceptance_criteria,
                priority: request.priority,
                assignee: None,
                created_at: now,
            });
        }

        Ok(children)
    }

    /// Backlog.md file updates are handled by the CLI in operator workflows.
    pub async fn batch_open_children(&self, parent_id: &str) -> Result<Vec<Child>> {
        self.get_children(parent_id).await
    }

    pub async fn get_children(&self, parent_id: &str) -> Result<Vec<Child>> {
        let tasks = self.client.get_tasks_paginated(None, 0, usize::MAX).await?;
        Ok(tasks
            .into_iter()
            .filter(|task| task.parent.as_deref() == Some(parent_id))
            .map(|task| Child {
                id: task.id,
                parent_id: parent_id.to_string(),
                title: task.title,
                description: None,
                task_type: task_type::TASK.to_string(),
                status: task.status.to_string(),
                github_issue_url: task.github_issue_url,
                rodgers_type: None,
                rodgers_labels: None,
                rodgers_parent: Some(parent_id.to_string()),
                discovered_from: None,
                acceptance_criteria: None,
                priority: None,
                assignee: None,
                created_at: Utc::now(),
            })
            .collect())
    }

    pub async fn get_epic_by_issue(&self, issue_url: &str) -> Result<Option<Epic>> {
        let tasks = self.client.get_tasks_paginated(None, 0, usize::MAX).await?;
        Ok(tasks
            .into_iter()
            .find(|task| task.github_issue_url.as_deref() == Some(issue_url))
            .map(|task| {
                let now = Utc::now();
                Epic {
                    id: task.id,
                    title: task.title,
                    description: None,
                    task_type: task_type::EPIC.to_string(),
                    status: task.status.to_string(),
                    github_issue_url: task.github_issue_url,
                    github_issue_state: Some("open".to_string()),
                    rodgers_type: None,
                    rodgers_labels: None,
                    rodgers_parent: None,
                    discovered_from: None,
                    acceptance_criteria: None,
                    priority: None,
                    assignee: None,
                    created_at: now,
                    updated_at: now,
                }
            }))
    }

    pub async fn epic_has_children(&self, epic_id: &str) -> Result<bool> {
        Ok(!self.get_children(epic_id).await?.is_empty())
    }

    pub async fn update_epic_status(&self, _epic_id: &str, new_status: &str) -> Result<()> {
        validate_status(new_status)
    }

    pub async fn update_child_status(&self, _child_id: &str, new_status: &str) -> Result<()> {
        validate_status(new_status)
    }
}

fn validate_status(new_status: &str) -> Result<()> {
    if status::is_valid(new_status) {
        Ok(())
    } else {
        Err(RogersError::Config(format!(
            "Invalid task status: {}",
            new_status
        )))
    }
}

fn split_labels(labels: Option<&str>) -> Vec<String> {
    labels
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|label| !label.is_empty())
        .map(str::to_string)
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEpicRequest {
    pub title: String,
    pub description: Option<String>,
    pub task_type: Option<String>,
    pub github_issue_url: Option<String>,
    pub rodgers_type: Option<String>,
    pub rodgers_labels: Option<String>,
    pub discovered_from: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateChildRequest {
    pub title: String,
    pub description: Option<String>,
    pub task_type: Option<String>,
    pub rodgers_type: Option<String>,
    pub rodgers_labels: Option<String>,
    pub acceptance_criteria: Option<String>,
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BreakdownResult {
    pub epic: Epic,
    pub children: Vec<Child>,
    pub epic_url: Option<String>,
    pub child_urls: Vec<Option<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn file_epic_creates_backlog_task() {
        let dir = tempfile::tempdir().unwrap();
        let controller = TaskController::from_client(BacklogClient::new(dir.path()));
        let epic = controller
            .file_epic(CreateEpicRequest {
                title: "Test Epic".to_string(),
                description: Some("Test description".to_string()),
                task_type: None,
                github_issue_url: Some("https://github.com/test/repo/issues/123".to_string()),
                rodgers_type: Some("epic".to_string()),
                rodgers_labels: None,
                discovered_from: None,
                acceptance_criteria: Some("- [ ] AC-1: Test".to_string()),
                priority: Some(1),
            })
            .await
            .unwrap();

        assert_eq!(epic.title, "Test Epic");
        assert!(dir.path().join("completed").is_dir());
    }

    #[tokio::test]
    async fn file_children_creates_backlog_tasks() {
        let dir = tempfile::tempdir().unwrap();
        let controller = TaskController::from_client(BacklogClient::new(dir.path()));
        let children = controller
            .file_children(
                "TASK-1",
                vec![CreateChildRequest {
                    title: "Test Child".to_string(),
                    description: Some("Child description".to_string()),
                    task_type: Some("feature".to_string()),
                    rodgers_type: Some("feature".to_string()),
                    rodgers_labels: None,
                    acceptance_criteria: Some("- [ ] AC-1: Test child".to_string()),
                    priority: Some(2),
                }],
            )
            .await
            .unwrap();

        assert_eq!(children.len(), 1);
        assert_eq!(children[0].parent_id, "TASK-1");
    }
}
