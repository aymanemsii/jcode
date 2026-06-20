use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Backlog,
    Ready,
    Running,
    Review,
    Done,
    Blocked,
    Cancelled,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Urgent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: String,
    pub project: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub worker_profile: Option<String>,
    pub output_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn new(
        title: String,
        description: String,
        project: Option<String>,
        priority: TaskPriority,
        worker_profile: Option<String>,
        output_path: Option<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: crate::id::new_id("task"),
            title,
            description,
            project,
            status: TaskStatus::Backlog,
            priority,
            worker_profile,
            output_path,
            created_at: now,
            updated_at: now,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct QueueState {
    pub tasks: Vec<Task>,
}

pub fn default_queue_state() -> QueueState {
    QueueState::default()
}

/// Resolve the path where the queue state JSON is persisted.
pub fn queue_file_path() -> Result<PathBuf> {
    Ok(jcode_storage::jcode_dir()?.join("queue").join("queue.json"))
}

/// Load the queue state from disk, creating an empty state if none exists yet.
pub fn load() -> Result<QueueState> {
    let path = queue_file_path()?;
    if !path.exists() {
        let default_state = default_queue_state();
        save(&default_state)?;
        Ok(default_state)
    } else {
        Ok(jcode_storage::read_json(&path)?)
    }
}

/// Persist the queue state to disk.
pub fn save(state: &QueueState) -> Result<()> {
    let path = queue_file_path()?;
    if let Some(parent) = path.parent() {
        jcode_storage::ensure_dir(parent)?;
    }
    jcode_storage::write_json(&path, state)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_status_serialization() {
        let statuses = vec![
            (TaskStatus::Backlog, "\"backlog\""),
            (TaskStatus::Ready, "\"ready\""),
            (TaskStatus::Running, "\"running\""),
            (TaskStatus::Review, "\"review\""),
            (TaskStatus::Done, "\"done\""),
            (TaskStatus::Blocked, "\"blocked\""),
            (TaskStatus::Cancelled, "\"cancelled\""),
        ];

        for (status, expected_json) in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected_json);

            let deserialized: TaskStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_default_empty_queue_creation() {
        let _lock = crate::storage::lock_test_env();
        let temp = tempfile::TempDir::new().unwrap();
        let prev_home = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_HOME", temp.path());

        let initial_path = queue_file_path().unwrap();
        assert!(!initial_path.exists());

        let loaded_empty = load().unwrap();
        assert!(loaded_empty.tasks.is_empty());
        assert!(initial_path.exists());

        if let Some(prev) = prev_home {
            crate::env::set_var("JCODE_HOME", prev);
        } else {
            crate::env::remove_var("JCODE_HOME");
        }
    }

    #[test]
    fn test_save_and_load_queue_state() {
        let _lock = crate::storage::lock_test_env();
        let temp = tempfile::TempDir::new().unwrap();
        let prev_home = std::env::var_os("JCODE_HOME");
        crate::env::set_var("JCODE_HOME", temp.path());

        let mut state = default_queue_state();
        let task = Task::new(
            "Test Task".to_string(),
            "This is a test description".to_string(),
            Some("jcode".to_string()),
            TaskPriority::High,
            None,
            None,
        );
        state.tasks.push(task.clone());

        save(&state).unwrap();

        let reloaded = load().unwrap();
        assert_eq!(reloaded, state);

        if let Some(prev) = prev_home {
            crate::env::set_var("JCODE_HOME", prev);
        } else {
            crate::env::remove_var("JCODE_HOME");
        }
    }
}
