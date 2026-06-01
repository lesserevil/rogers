//! Backlog.md task store client.

use crate::error::Result;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::path::{Path, PathBuf};

/// Task status values used by Rodgers and normalized from Backlog.md files.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Open,
    InProgress,
    Closed,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskStatus::Open => write!(f, "open"),
            TaskStatus::InProgress => write!(f, "in_progress"),
            TaskStatus::Closed => write!(f, "closed"),
        }
    }
}

/// Task type values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Epic,
    Feature,
    Bug,
    Chore,
    Spike,
    Decision,
    Milestone,
    Task,
}

impl std::fmt::Display for TaskType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskType::Epic => write!(f, "epic"),
            TaskType::Feature => write!(f, "feature"),
            TaskType::Bug => write!(f, "bug"),
            TaskType::Chore => write!(f, "chore"),
            TaskType::Spike => write!(f, "spike"),
            TaskType::Decision => write!(f, "decision"),
            TaskType::Milestone => write!(f, "milestone"),
            TaskType::Task => write!(f, "task"),
        }
    }
}

/// Request to file a new task in Backlog.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTaskRequest {
    pub title: String,
    pub description: String,
    pub task_type: TaskType,
    pub priority: i32,
    pub is_epic: bool,
    pub parent_id: Option<String>,
    pub status: TaskStatus,
    pub labels: Vec<String>,
}

/// A task loaded from Backlog.md frontmatter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub status: TaskStatus,
    pub github_issue_url: Option<String>,
    pub parent: Option<String>,
}

/// File-backed Backlog.md client.
#[derive(Debug, Clone)]
pub struct BacklogClient {
    root: PathBuf,
}

impl BacklogClient {
    /// Create a client rooted at the configured Backlog.md directory.
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Create a client from a configured path, defaulting to `backlog`.
    pub fn from_config(path: Option<&str>) -> Result<Self> {
        let root = PathBuf::from(path.unwrap_or("backlog"));
        Ok(Self::new(root))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Return every task whose normalized status is closed.
    pub async fn get_closed_tasks(&self) -> Result<Vec<Task>> {
        self.get_tasks_by_status(TaskStatus::Closed).await
    }

    /// Return every task matching the requested normalized status.
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> Result<Vec<Task>> {
        let mut tasks = Vec::new();
        for directory in ["tasks", "completed"] {
            let dir = self.root.join(directory);
            if !dir.is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(&dir)? {
                let path = entry?.path();
                if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                    continue;
                }
                if let Some(task) = read_task_file(&path)? {
                    if task.status == status {
                        tasks.push(task);
                    }
                }
            }
        }
        Ok(tasks)
    }

    /// Return tasks with offset/limit pagination.
    pub async fn get_tasks_paginated(
        &self,
        status: Option<TaskStatus>,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<Task>> {
        let mut tasks = if let Some(status) = status {
            self.get_tasks_by_status(status).await?
        } else {
            let mut all = Vec::new();
            for status in [TaskStatus::Open, TaskStatus::InProgress, TaskStatus::Closed] {
                all.extend(self.get_tasks_by_status(status).await?);
            }
            all
        };
        tasks.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(tasks.into_iter().skip(offset).take(limit).collect())
    }

    /// Create a Backlog.md task file and return the allocated task ID.
    pub fn create_task(&self, request: &FileTaskRequest) -> Result<String> {
        let task_id = self.next_task_id()?;
        let status = match request.status {
            TaskStatus::Open => "To Do",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Closed => "Done",
        };
        let folder = if request.status == TaskStatus::Closed {
            "completed"
        } else {
            "tasks"
        };
        let dir = self.root.join(folder);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!(
            "{} - {}.md",
            task_id.to_ascii_lowercase(),
            slug_title(&request.title)
        ));
        let priority = match request.priority {
            i32::MIN..=1 => "high",
            2 => "medium",
            _ => "low",
        };

        let mut labels = request.labels.clone();
        labels.sort();
        labels.dedup();

        let meta = serde_yaml::to_string(&serde_json::json!({
            "id": task_id,
            "title": request.title,
            "status": status,
            "assignee": Vec::<String>::new(),
            "labels": labels,
            "dependencies": Vec::<String>::new(),
            "priority": priority,
            "ordinal": 1000,
            "type": request.task_type.to_string(),
            "parent": request.parent_id,
        }))?;
        let body = format!(
            "---\n{}---\n## Description\n\n<!-- SECTION:DESCRIPTION:BEGIN -->\n{}\n<!-- SECTION:DESCRIPTION:END -->\n\n## Comments\n<!-- COMMENTS:BEGIN -->\n<!-- COMMENTS:END -->\n",
            meta,
            request.description.trim()
        );
        std::fs::write(&path, body)?;
        Ok(task_id)
    }

    fn next_task_id(&self) -> Result<String> {
        let mut max_id = 0;
        for directory in ["tasks", "completed"] {
            let dir = self.root.join(directory);
            if !dir.is_dir() {
                continue;
            }
            for entry in std::fs::read_dir(dir)? {
                let path = entry?.path();
                let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                if let Some(number) = stem
                    .strip_prefix("task-")
                    .and_then(|rest| rest.split_whitespace().next())
                    .and_then(|n| n.parse::<usize>().ok())
                {
                    max_id = max_id.max(number);
                }
                if let Some(task) = read_task_file(&path)? {
                    if let Some(number) = task
                        .id
                        .strip_prefix("TASK-")
                        .and_then(|n| n.parse::<usize>().ok())
                    {
                        max_id = max_id.max(number);
                    }
                }
            }
        }
        Ok(format!("TASK-{}", max_id + 1))
    }
}

fn read_task_file(path: &Path) -> Result<Option<Task>> {
    let content = std::fs::read_to_string(path)?;
    let Some(frontmatter) = extract_frontmatter(&content) else {
        return Ok(None);
    };
    let meta: Value = serde_yaml::from_str(frontmatter)?;
    let Some(id) = yaml_string(&meta, "id") else {
        return Ok(None);
    };
    let title = yaml_string(&meta, "title").unwrap_or_else(|| id.clone());
    let status = yaml_string(&meta, "status")
        .as_deref()
        .map(parse_status)
        .unwrap_or(TaskStatus::Open);
    let github_issue_url = yaml_string(&meta, "github_issue_url")
        .or_else(|| yaml_nested_string(&meta, "github", "issue_url"));
    let parent = yaml_string(&meta, "parent");

    Ok(Some(Task {
        id,
        title,
        status,
        github_issue_url,
        parent,
    }))
}

fn extract_frontmatter(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn yaml_string(meta: &Value, key: &str) -> Option<String> {
    meta.get(key)?.as_str().map(str::to_string)
}

fn yaml_nested_string(meta: &Value, parent: &str, key: &str) -> Option<String> {
    meta.get(parent)?.get(key)?.as_str().map(str::to_string)
}

fn parse_status(status: &str) -> TaskStatus {
    match status.trim().to_ascii_lowercase().as_str() {
        "done" | "closed" => TaskStatus::Closed,
        "in progress" | "in_progress" | "in-progress" => TaskStatus::InProgress,
        _ => TaskStatus::Open,
    }
}

fn slug_title(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "task".to_string()
    } else {
        slug.chars().take(80).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_status_display_is_normalized() {
        assert_eq!(TaskStatus::Open.to_string(), "open");
        assert_eq!(TaskStatus::InProgress.to_string(), "in_progress");
        assert_eq!(TaskStatus::Closed.to_string(), "closed");
    }

    #[test]
    fn task_type_display_is_normalized() {
        assert_eq!(TaskType::Chore.to_string(), "chore");
        assert_eq!(TaskType::Epic.to_string(), "epic");
    }

    #[test]
    fn slug_title_removes_unsafe_characters() {
        assert_eq!(slug_title("Release: 1.2 / final"), "Release--1.2---final");
    }

    #[tokio::test]
    async fn reads_backlog_task_frontmatter() {
        let dir = tempfile::tempdir().unwrap();
        let tasks = dir.path().join("tasks");
        std::fs::create_dir_all(&tasks).unwrap();
        std::fs::write(
            tasks.join("task-1 - sample.md"),
            "---\nid: TASK-1\ntitle: Sample\nstatus: Done\ngithub_issue_url: https://github.com/o/r/issues/1\n---\n",
        )
        .unwrap();

        let client = BacklogClient::new(dir.path());
        let closed = client.get_closed_tasks().await.unwrap();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].id, "TASK-1");
        assert_eq!(closed[0].status, TaskStatus::Closed);
    }

    #[test]
    fn create_task_allocates_next_id() {
        let dir = tempfile::tempdir().unwrap();
        let client = BacklogClient::new(dir.path());
        let id = client
            .create_task(&FileTaskRequest {
                title: "Write docs".to_string(),
                description: "Do the thing".to_string(),
                task_type: TaskType::Chore,
                priority: 2,
                is_epic: false,
                parent_id: None,
                status: TaskStatus::Open,
                labels: vec!["docs".to_string()],
            })
            .unwrap();
        assert_eq!(id, "TASK-1");
    }
}
