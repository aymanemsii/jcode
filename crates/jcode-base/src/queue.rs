use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunState {
    pub run_id: String,
    pub task_id: String,
    pub worker_profile: String,
    pub command: String,
    pub pid: Option<u32>,
    pub status: RunStatus,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub exit_code: Option<i32>,
    pub run_dir: String,
    pub stdout_path: String,
    pub stderr_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RunIndex {
    pub runs: Vec<RunState>,
}

impl RunIndex {
    pub fn add_or_update(&mut self, run: RunState) {
        if let Some(existing) = self
            .runs
            .iter_mut()
            .find(|existing| existing.run_id == run.run_id)
        {
            *existing = run;
        } else {
            self.runs.push(run);
        }
    }

    pub fn find(&self, run_id: &str) -> Option<&RunState> {
        self.runs.iter().find(|run| run.run_id == run_id)
    }

    pub fn running_runs(&self) -> Vec<&RunState> {
        self.runs
            .iter()
            .filter(|run| run.status == RunStatus::Running)
            .collect()
    }

    pub fn active_runs(&self) -> Vec<&RunState> {
        self.runs
            .iter()
            .filter(|run| matches!(run.status, RunStatus::Running | RunStatus::Unknown))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkerProfile {
    pub name: String,
    pub description: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct WorkerProfilesFile {
    #[serde(default)]
    workers: BTreeMap<String, WorkerProfileConfig>,
}

#[derive(Debug, Default, Deserialize)]
struct WorkerProfileConfig {
    description: Option<String>,
    command: Option<String>,
}

pub fn default_queue_state() -> QueueState {
    QueueState::default()
}

/// Resolve the current project's `.jcode` directory.
pub fn project_jcode_dir_path() -> Result<PathBuf> {
    Ok(std::env::current_dir()?.join(".jcode"))
}

/// Resolve the current project's Queue Mode root directory.
pub fn queue_dir_path() -> Result<PathBuf> {
    Ok(project_jcode_dir_path()?.join("queue"))
}

/// Resolve the path where the queue state JSON is persisted.
pub fn queue_file_path() -> Result<PathBuf> {
    Ok(queue_dir_path()?.join("queue.json"))
}

/// Resolve the current project's Queue Mode handoff directory.
pub fn queue_handoffs_dir_path() -> Result<PathBuf> {
    Ok(queue_dir_path()?.join("handoffs"))
}

/// Resolve the path for a task handoff file.
pub fn queue_handoff_file_path(task_id: &str) -> Result<PathBuf> {
    Ok(queue_handoffs_dir_path()?.join(format!("{task_id}.md")))
}

/// Resolve the current project's Queue Mode run artifacts directory.
pub fn queue_runs_dir_path() -> Result<PathBuf> {
    Ok(queue_dir_path()?.join("runs"))
}

/// Resolve the project-local Queue Mode run index path.
pub fn queue_run_index_path() -> Result<PathBuf> {
    Ok(queue_runs_dir_path()?.join("index.json"))
}

/// Resolve the artifact directory for a specific queue task run.
pub fn queue_run_artifact_dir_path(task_id: &str, timestamp: &str) -> Result<PathBuf> {
    queue_run_dir_path(task_id, timestamp)
}

/// Resolve the artifact directory for a specific queue run.
pub fn queue_run_dir_path(task_id: &str, timestamp: &str) -> Result<PathBuf> {
    Ok(queue_runs_dir_path()?.join(task_id).join(timestamp))
}

/// Resolve the stdout log path for a specific queue run.
pub fn queue_run_stdout_path(task_id: &str, timestamp: &str) -> Result<PathBuf> {
    Ok(queue_run_dir_path(task_id, timestamp)?.join("stdout.log"))
}

/// Resolve the stderr log path for a specific queue run.
pub fn queue_run_stderr_path(task_id: &str, timestamp: &str) -> Result<PathBuf> {
    Ok(queue_run_dir_path(task_id, timestamp)?.join("stderr.log"))
}

/// Resolve the expanded command artifact path for a specific queue run.
pub fn queue_run_command_path(task_id: &str, timestamp: &str) -> Result<PathBuf> {
    Ok(queue_run_dir_path(task_id, timestamp)?.join("command.txt"))
}

/// Resolve the metadata JSON path for a specific queue run.
pub fn queue_run_metadata_path(task_id: &str, timestamp: &str) -> Result<PathBuf> {
    Ok(queue_run_dir_path(task_id, timestamp)?.join("run.json"))
}

/// Resolve the project-local worker profile configuration path.
pub fn worker_profiles_file_path() -> Result<PathBuf> {
    Ok(project_jcode_dir_path()?.join("workers.toml"))
}

/// Load worker profiles from a project-local TOML file.
///
/// Missing files are treated as an empty profile collection so projects can opt
/// in gradually.
pub fn load_worker_profiles() -> Result<Vec<WorkerProfile>> {
    load_worker_profiles_from_path(worker_profiles_file_path()?)
}

pub fn load_worker_profiles_from_path(path: PathBuf) -> Result<Vec<WorkerProfile>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|err| anyhow::anyhow!("failed to read {}: {err}", path.display()))?;
    let parsed: WorkerProfilesFile = toml::from_str(&content)
        .map_err(|err| anyhow::anyhow!("failed to parse {}: {err}", path.display()))?;

    Ok(parsed
        .workers
        .into_iter()
        .map(|(name, profile)| WorkerProfile {
            name,
            description: profile.description,
            command: profile.command,
        })
        .collect())
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

/// Load the project-local Queue Mode run index, returning an empty index when missing.
pub fn load_run_index() -> Result<RunIndex> {
    let path = queue_run_index_path()?;
    if !path.exists() {
        return Ok(RunIndex::default());
    }
    Ok(jcode_storage::read_json(&path)?)
}

/// Persist the project-local Queue Mode run index.
pub fn save_run_index(index: &RunIndex) -> Result<()> {
    let path = queue_run_index_path()?;
    if let Some(parent) = path.parent() {
        jcode_storage::ensure_dir(parent)?;
    }
    jcode_storage::write_json(&path, index)?;
    Ok(())
}

/// Add a run to the project-local index or replace the existing entry with the same run ID.
pub fn add_or_update_run_state(run: RunState) -> Result<RunIndex> {
    let mut index = load_run_index()?;
    index.add_or_update(run);
    save_run_index(&index)?;
    Ok(index)
}

/// Find a run in the project-local index by run ID.
pub fn find_run_by_id(run_id: &str) -> Result<Option<RunState>> {
    Ok(load_run_index()?.find(run_id).cloned())
}

/// List project-local runs currently marked as running.
pub fn list_running_runs() -> Result<Vec<RunState>> {
    Ok(load_run_index()?
        .running_runs()
        .into_iter()
        .cloned()
        .collect())
}

/// List project-local runs that need active-run inspection.
pub fn list_active_runs() -> Result<Vec<RunState>> {
    Ok(load_run_index()?
        .active_runs()
        .into_iter()
        .cloned()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CurrentDirGuard {
        previous: PathBuf,
    }

    impl CurrentDirGuard {
        fn change_to(path: &std::path::Path) -> Self {
            let previous = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self { previous }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.previous).unwrap();
        }
    }

    #[test]
    fn test_queue_file_path_is_project_local() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let path = queue_file_path().unwrap();

        assert_eq!(
            path,
            project
                .path()
                .join(".jcode")
                .join("queue")
                .join("queue.json")
        );
    }

    #[test]
    fn test_worker_profiles_file_path_is_project_local() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let path = worker_profiles_file_path().unwrap();

        assert_eq!(path, project.path().join(".jcode").join("workers.toml"));
    }

    #[test]
    fn test_queue_handoff_path_is_project_local() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let dir = queue_handoffs_dir_path().unwrap();
        let path = queue_handoff_file_path("task_1").unwrap();

        assert_eq!(
            dir,
            project.path().join(".jcode").join("queue").join("handoffs")
        );
        assert_eq!(path, dir.join("task_1.md"));
    }

    #[test]
    fn test_queue_run_artifact_path_is_project_local() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let runs_dir = queue_runs_dir_path().unwrap();
        let path = queue_run_artifact_dir_path("task_1", "20260620T100000Z").unwrap();

        assert_eq!(
            runs_dir,
            project.path().join(".jcode").join("queue").join("runs")
        );
        assert_eq!(path, runs_dir.join("task_1").join("20260620T100000Z"));
    }

    #[test]
    fn test_queue_run_index_path_is_project_local() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let path = queue_run_index_path().unwrap();

        assert_eq!(
            path,
            project
                .path()
                .join(".jcode")
                .join("queue")
                .join("runs")
                .join("index.json")
        );
    }

    #[test]
    fn test_queue_run_artifact_file_paths_are_project_local() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let dir = queue_run_dir_path("task_1", "20260620T100000Z").unwrap();

        assert_eq!(
            dir,
            project
                .path()
                .join(".jcode")
                .join("queue")
                .join("runs")
                .join("task_1")
                .join("20260620T100000Z")
        );
        assert_eq!(
            queue_run_stdout_path("task_1", "20260620T100000Z").unwrap(),
            dir.join("stdout.log")
        );
        assert_eq!(
            queue_run_stderr_path("task_1", "20260620T100000Z").unwrap(),
            dir.join("stderr.log")
        );
        assert_eq!(
            queue_run_command_path("task_1", "20260620T100000Z").unwrap(),
            dir.join("command.txt")
        );
        assert_eq!(
            queue_run_metadata_path("task_1", "20260620T100000Z").unwrap(),
            dir.join("run.json")
        );
    }

    #[test]
    fn test_run_status_serialization() {
        let statuses = vec![
            (RunStatus::Running, "\"running\""),
            (RunStatus::Succeeded, "\"succeeded\""),
            (RunStatus::Failed, "\"failed\""),
            (RunStatus::Cancelled, "\"cancelled\""),
            (RunStatus::Unknown, "\"unknown\""),
        ];

        for (status, expected_json) in statuses {
            let serialized = serde_json::to_string(&status).unwrap();
            assert_eq!(serialized, expected_json);

            let deserialized: RunStatus = serde_json::from_str(&serialized).unwrap();
            assert_eq!(deserialized, status);
        }
    }

    #[test]
    fn test_run_state_serialization() {
        let state = test_run_state("run_1", "task_1", RunStatus::Running);

        let serialized = serde_json::to_string(&state).unwrap();
        let deserialized: RunState = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized, state);
    }

    #[test]
    fn test_missing_run_index_returns_empty_index() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let index = load_run_index().unwrap();

        assert!(index.runs.is_empty());
        assert!(!queue_run_index_path().unwrap().exists());
    }

    #[test]
    fn test_save_and_load_run_index() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());
        let state = test_run_state("run_1", "task_1", RunStatus::Running);
        let index = RunIndex {
            runs: vec![state.clone()],
        };

        save_run_index(&index).unwrap();

        let loaded = load_run_index().unwrap();
        assert_eq!(loaded.runs, vec![state]);
    }

    #[test]
    fn test_add_or_update_run_state_replaces_existing_run() {
        let mut index = RunIndex::default();
        let running = test_run_state("run_1", "task_1", RunStatus::Running);
        let mut failed = running.clone();
        failed.status = RunStatus::Failed;
        failed.exit_code = Some(1);
        failed.ended_at = Some(test_time("2026-06-20T12:05:00Z"));

        index.add_or_update(running);
        index.add_or_update(failed.clone());

        assert_eq!(index.runs, vec![failed]);
    }

    #[test]
    fn test_find_run_by_run_id() {
        let run_1 = test_run_state("run_1", "task_1", RunStatus::Running);
        let run_2 = test_run_state("run_2", "task_2", RunStatus::Succeeded);
        let index = RunIndex {
            runs: vec![run_1, run_2.clone()],
        };

        assert_eq!(index.find("run_2"), Some(&run_2));
        assert!(index.find("missing").is_none());
    }

    #[test]
    fn test_list_running_runs() {
        let running = test_run_state("run_running", "task_1", RunStatus::Running);
        let unknown = test_run_state("run_unknown", "task_2", RunStatus::Unknown);
        let succeeded = test_run_state("run_succeeded", "task_3", RunStatus::Succeeded);
        let index = RunIndex {
            runs: vec![running.clone(), unknown, succeeded],
        };

        assert_eq!(index.running_runs(), vec![&running]);
    }

    #[test]
    fn test_list_active_runs_includes_running_and_unknown() {
        let running = test_run_state("run_running", "task_1", RunStatus::Running);
        let unknown = test_run_state("run_unknown", "task_2", RunStatus::Unknown);
        let succeeded = test_run_state("run_succeeded", "task_3", RunStatus::Succeeded);
        let index = RunIndex {
            runs: vec![running.clone(), unknown.clone(), succeeded],
        };

        assert_eq!(index.active_runs(), vec![&running, &unknown]);
    }

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
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

        let initial_path = queue_file_path().unwrap();
        assert!(!initial_path.exists());

        let loaded_empty = load().unwrap();
        assert!(loaded_empty.tasks.is_empty());
        assert!(initial_path.exists());
    }

    #[test]
    fn test_save_and_load_queue_state() {
        let _lock = crate::storage::lock_test_env();
        let project = tempfile::TempDir::new().unwrap();
        let _cwd = CurrentDirGuard::change_to(project.path());

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
    }

    #[test]
    fn test_parse_worker_profiles_toml() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("workers.toml");
        std::fs::write(
            &path,
            r#"
[workers.researcher]
description = "Researches sources and produces structured notes"
command = "opencode run <handoff_file>"

[workers.coder]
description = "Implements code changes"
command = "codex exec <handoff_file>"

[workers.reviewer]
description = "Reviews outputs and checks quality"
"#,
        )
        .unwrap();

        let profiles = load_worker_profiles_from_path(path).unwrap();

        assert_eq!(
            profiles,
            vec![
                WorkerProfile {
                    name: "coder".to_string(),
                    description: Some("Implements code changes".to_string()),
                    command: Some("codex exec <handoff_file>".to_string()),
                },
                WorkerProfile {
                    name: "researcher".to_string(),
                    description: Some(
                        "Researches sources and produces structured notes".to_string()
                    ),
                    command: Some("opencode run <handoff_file>".to_string()),
                },
                WorkerProfile {
                    name: "reviewer".to_string(),
                    description: Some("Reviews outputs and checks quality".to_string()),
                    command: None,
                },
            ]
        );
    }

    #[test]
    fn test_missing_worker_profiles_toml_returns_empty_profiles() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join(".jcode").join("workers.toml");

        let profiles = load_worker_profiles_from_path(path).unwrap();

        assert!(profiles.is_empty());
    }

    #[test]
    fn test_invalid_worker_profiles_toml_reports_path() {
        let temp = tempfile::TempDir::new().unwrap();
        let path = temp.path().join("workers.toml");
        std::fs::write(&path, "[workers.coder\n").unwrap();

        let err = load_worker_profiles_from_path(path.clone()).expect_err("invalid toml");

        let message = err.to_string();
        assert!(message.contains("failed to parse"));
        assert!(message.contains(&path.display().to_string()));
    }

    fn test_time(value: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(value)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn test_run_state(run_id: &str, task_id: &str, status: RunStatus) -> RunState {
        RunState {
            run_id: run_id.to_string(),
            task_id: task_id.to_string(),
            worker_profile: "coder".to_string(),
            command: "codex exec .jcode/queue/handoffs/task_1.md".to_string(),
            pid: Some(1234),
            status,
            started_at: test_time("2026-06-20T12:00:00Z"),
            ended_at: None,
            exit_code: None,
            run_dir: ".jcode/queue/runs/task_1/20260620T120000Z".to_string(),
            stdout_path: ".jcode/queue/runs/task_1/20260620T120000Z/stdout.log".to_string(),
            stderr_path: ".jcode/queue/runs/task_1/20260620T120000Z/stderr.log".to_string(),
        }
    }
}
