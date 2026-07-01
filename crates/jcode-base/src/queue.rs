use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const QUEUE_FILE_RELATIVE_PATH: &str = ".jcode/queue/tasks.json";
pub const QUEUE_RUNS_RELATIVE_PATH: &str = ".jcode/queue/runs";
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueRunRecord {
    pub run_id: String,
    pub task_id: String,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
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

#[derive(Debug, Clone, Default)]
pub struct QueueTaskEdit {
    pub title: Option<String>,
    pub body: Option<String>,
    pub priority: Option<String>,
    pub worker_profile: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct QueueEditUpdate {
    pub task: QueueTask,
}

#[derive(Debug, Clone)]
pub struct QueueRunStart {
    pub task: QueueTask,
    pub run_record: QueueRunRecord,
    pub run_record_path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct QueueRunFinish {
    pub task: QueueTask,
    pub run_record: QueueRunRecord,
    pub run_record_path: PathBuf,
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

pub fn queue_runs_dir(project_dir: &Path) -> PathBuf {
    project_dir.join(QUEUE_RUNS_RELATIVE_PATH)
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

pub fn start_project_queue_task_run(project_dir: &Path, id: &str) -> Result<QueueRunStart> {
    let path = queue_file_path(project_dir);
    let mut store = load_project_queue(project_dir)?;
    let now = Utc::now().to_rfc3339();
    let run_id = crate::id::new_id("queue_run");
    let task = {
        let task = store
            .tasks
            .iter_mut()
            .find(|task| task.id.as_str() == id)
            .ok_or_else(|| anyhow::anyhow!("queue task not found: {id}"))?;
        if task.archived_at.is_some() {
            anyhow::bail!("queue task is archived: {id}");
        }
        if task.status != "ready" {
            anyhow::bail!("queue task is not ready: {id} has status {}", task.status);
        }
        task.status = "running".to_string();
        task.updated_at = now.clone();
        task.clone()
    };
    write_queue_store(&path, &store)?;

    let run_record = QueueRunRecord {
        run_id: run_id.clone(),
        task_id: id.to_string(),
        started_at: now,
        finished_at: None,
        status: "running".to_string(),
        error: None,
    };
    let run_record_path = queue_run_record_path(project_dir, id, &run_id);
    write_queue_run_record_best_effort(&run_record_path, &run_record);

    Ok(QueueRunStart {
        task,
        run_record,
        run_record_path,
    })
}

pub fn finish_project_queue_task_run(
    project_dir: &Path,
    id: &str,
    run_record: &QueueRunRecord,
    status: &str,
    error: Option<String>,
) -> Result<QueueRunFinish> {
    if !matches!(status, "done" | "failed") {
        anyhow::bail!("invalid queue run finish status: {status}");
    }

    let path = queue_file_path(project_dir);
    let mut store = load_project_queue(project_dir)?;
    let now = Utc::now().to_rfc3339();
    let task = {
        let task = store
            .tasks
            .iter_mut()
            .find(|task| task.id.as_str() == id)
            .ok_or_else(|| anyhow::anyhow!("queue task not found: {id}"))?;
        task.status = status.to_string();
        task.updated_at = now.clone();
        task.clone()
    };
    write_queue_store(&path, &store)?;

    let mut finished_record = run_record.clone();
    finished_record.finished_at = Some(now);
    finished_record.status = status.to_string();
    finished_record.error = error;
    let run_record_path = queue_run_record_path(project_dir, id, &run_record.run_id);
    write_queue_run_record_best_effort(&run_record_path, &finished_record);

    Ok(QueueRunFinish {
        task,
        run_record: finished_record,
        run_record_path,
    })
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

pub fn edit_project_queue_task(
    project_dir: &Path,
    id: &str,
    edit: QueueTaskEdit,
) -> Result<QueueEditUpdate> {
    if edit.title.is_none()
        && edit.body.is_none()
        && edit.priority.is_none()
        && edit.worker_profile.is_none()
    {
        anyhow::bail!("no queue task edits provided");
    }

    let title = edit.title.map(|value| required_edit_text(value, "title"));
    let body = edit.body.map(|value| value.trim().to_string());
    let priority = edit
        .priority
        .map(|value| required_edit_text(value, "priority"));
    let worker_profile = edit.worker_profile.map(|value| {
        value.map(|profile| required_edit_text(profile, "worker_profile"))
            .transpose()
    });

    let path = queue_file_path(project_dir);
    let mut store = load_project_queue(project_dir)?;
    let now = Utc::now().to_rfc3339();
    let update = {
        let task = store
            .tasks
            .iter_mut()
            .find(|task| task.id.as_str() == id)
            .ok_or_else(|| anyhow::anyhow!("queue task not found: {id}"))?;

        if let Some(title) = title {
            task.title = title?;
        }
        if let Some(body) = body {
            task.body = body;
        }
        if let Some(priority) = priority {
            task.priority = priority?;
        }
        if let Some(worker_profile) = worker_profile {
            task.worker_profile = worker_profile?;
        }
        task.updated_at = now;
        QueueEditUpdate { task: task.clone() }
    };

    write_queue_store(&path, &store)?;
    Ok(update)
}

pub fn next_active_ready_task(store: &QueueStore) -> Option<&QueueTask> {
    store
        .tasks
        .iter()
        .find(|task| task.status == "ready" && task.archived_at.is_none())
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

fn queue_run_record_path(project_dir: &Path, task_id: &str, run_id: &str) -> PathBuf {
    queue_runs_dir(project_dir)
        .join(task_id)
        .join(format!("{run_id}.json"))
}

fn write_queue_run_record_best_effort(path: &Path, record: &QueueRunRecord) {
    if let Err(err) = write_queue_run_record(path, record) {
        crate::logging::warn(&format!(
            "failed to write queue run metadata at {}: {err}",
            path.display()
        ));
    }
}

fn write_queue_run_record(path: &Path, record: &QueueRunRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| anyhow::anyhow!("failed to create {}: {err}", parent.display()))?;
    }
    let content = serde_json::to_vec_pretty(record)?;
    std::fs::write(path, content)
        .map_err(|err| anyhow::anyhow!("failed to write {}: {err}", path.display()))?;
    Ok(())
}

fn required_edit_text(value: String, label: &str) -> Result<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("{label} cannot be empty");
    }
    Ok(trimmed.to_string())
}
