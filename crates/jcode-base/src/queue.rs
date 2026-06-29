use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const QUEUE_FILE_RELATIVE_PATH: &str = ".jcode/queue/tasks.json";
pub const VALID_QUEUE_STATUSES: &[&str] = &["ready", "running", "done", "failed"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStore {
    #[serde(default)]
    pub tasks: Vec<QueueTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueTask {
    pub id: String,
    pub title: String,
    pub body: String,
    pub status: String,
    pub priority: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archived_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NewQueueTask {
    pub title: String,
    pub body: String,
    pub priority: String,
    pub worker_profile: Option<String>,
}

#[derive(Debug, Clone)]
pub struct QueueStatusUpdate {
    pub task: QueueTask,
    pub old_status: String,
}

#[derive(Debug, Clone)]
pub struct QueueArchiveUpdate {
    pub task: QueueTask,
}

impl Default for QueueStore {
    fn default() -> Self {
        Self { tasks: Vec::new() }
    }
}

impl QueueStore {
    pub fn new() -> Self {
        Self::default()
    }
}

pub fn queue_file_path(project_dir: &Path) -> PathBuf {
    project_dir.join(QUEUE_FILE_RELATIVE_PATH)
}

pub fn init_project_queue(project_dir: &Path) -> Result<PathBuf> {
    let path = queue_file_path(project_dir);
    if path.exists() {
        return Ok(path);
    }

    write_queue_store(&path, &QueueStore::new())?;
    Ok(path)
}

pub fn load_project_queue(project_dir: &Path) -> Result<QueueStore> {
    let path = queue_file_path(project_dir);
    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|err| anyhow::anyhow!("failed to parse {}: {err}", path.display()))
}

pub fn add_project_queue_task(project_dir: &Path, new_task: NewQueueTask) -> Result<QueueTask> {
    init_project_queue(project_dir)?;
    let path = queue_file_path(project_dir);
    let mut store = load_project_queue(project_dir)?;
    let now = Utc::now().to_rfc3339();
    let task = QueueTask {
        id: crate::id::new_id("queue_task"),
        title: new_task.title,
        body: new_task.body,
        status: "ready".to_string(),
        priority: new_task.priority,
        created_at: now.clone(),
        updated_at: now,
        archived_at: None,
        worker_profile: new_task.worker_profile,
    };
    store.tasks.push(task.clone());
    write_queue_store(&path, &store)?;
    Ok(task)
}

pub fn update_project_queue_task_status(
    project_dir: &Path,
    id: &str,
    status: &str,
) -> Result<QueueStatusUpdate> {
    if !is_valid_queue_status(status) {
        anyhow::bail!(
            "invalid queue status: {status}. Expected one of: {}",
            VALID_QUEUE_STATUSES.join(", ")
        );
    }

    let path = queue_file_path(project_dir);
    let mut store = load_project_queue(project_dir)?;
    let now = Utc::now().to_rfc3339();
    let update = {
        let task = store
            .tasks
            .iter_mut()
            .find(|task| task.id.as_str() == id)
            .ok_or_else(|| anyhow::anyhow!("queue task not found: {id}"))?;
        let old_status = task.status.clone();
        task.status = status.to_string();
        task.updated_at = now;
        QueueStatusUpdate {
            task: task.clone(),
            old_status,
        }
    };

    write_queue_store(&path, &store)?;
    Ok(update)
}

pub fn archive_project_queue_task(project_dir: &Path, id: &str) -> Result<QueueArchiveUpdate> {
    let path = queue_file_path(project_dir);
    let mut store = load_project_queue(project_dir)?;
    let now = Utc::now().to_rfc3339();
    let update = {
        let task = store
            .tasks
            .iter_mut()
            .find(|task| task.id.as_str() == id)
            .ok_or_else(|| anyhow::anyhow!("queue task not found: {id}"))?;
        task.archived_at = Some(now.clone());
        task.updated_at = now;
        QueueArchiveUpdate { task: task.clone() }
    };

    write_queue_store(&path, &store)?;
    Ok(update)
}

pub fn is_valid_queue_status(status: &str) -> bool {
    VALID_QUEUE_STATUSES.contains(&status)
}

fn write_queue_store(path: &Path, store: &QueueStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| anyhow::anyhow!("failed to create {}: {err}", parent.display()))?;
    }
    let content = serde_json::to_vec_pretty(store)?;
    std::fs::write(path, content)
        .map_err(|err| anyhow::anyhow!("failed to write {}: {err}", path.display()))?;
    Ok(())
}
