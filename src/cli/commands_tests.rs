use super::*;
use crate::auth::{AuthState, AuthStatus, ProviderAuth};
use crate::message::{Message, StreamEvent, ToolDefinition};
use crate::provider::ModelRoute;
use crate::provider::{EventStream, Provider};
use crate::tool::Registry;
use async_trait::async_trait;
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::mpsc as tokio_mpsc;
use tokio_stream::wrappers::ReceiverStream;

struct SavedEnv {
    vars: Vec<(String, Option<String>)>,
}

impl SavedEnv {
    fn capture(keys: &[&str]) -> Self {
        Self {
            vars: keys
                .iter()
                .map(|key| (key.to_string(), std::env::var(key).ok()))
                .collect(),
        }
    }
}

impl Drop for SavedEnv {
    fn drop(&mut self) {
        for (key, value) in &self.vars {
            if let Some(value) = value {
                crate::env::set_var(key, value);
            } else {
                crate::env::remove_var(key);
            }
        }
    }
}

struct TestProvider;

#[async_trait]
impl Provider for TestProvider {
    async fn complete(
        &self,
        _messages: &[Message],
        _tools: &[ToolDefinition],
        _system: &str,
        _resume_session_id: Option<&str>,
    ) -> Result<EventStream> {
        let (tx, rx) = tokio_mpsc::channel::<Result<StreamEvent>>(4);
        tokio::spawn(async move {
            let _ = tx.send(Ok(StreamEvent::TextDelta("ok".to_string()))).await;
            let _ = tx
                .send(Ok(StreamEvent::MessageEnd {
                    stop_reason: Some("end_turn".to_string()),
                }))
                .await;
        });
        Ok(Box::pin(ReceiverStream::new(rx)))
    }

    fn name(&self) -> &str {
        "test"
    }

    fn fork(&self) -> Arc<dyn Provider> {
        Arc::new(Self)
    }
}

fn spawn_single_response_http_server(status: u16, body: &str) -> String {
    spawn_single_response_http_server_on_host("127.0.0.1", status, body)
}

fn spawn_single_response_http_server_on_host(host: &str, status: u16, body: &str) -> String {
    let listener = std::net::TcpListener::bind((host, 0)).expect("bind test server");
    let addr = listener.local_addr().expect("local addr");
    let body = body.to_string();
    std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 2048];
        let _ = stream.read(&mut buf);
        let status_text = match status {
            200 => "OK",
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            _ => "OK",
        };
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            status_text,
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
    });
    format!("http://{}:{}/v1", host, addr.port())
}

#[test]
fn test_parse_tailscale_dns_name_trims_trailing_dot() {
    let payload = br#"{"Self":{"DNSName":"yashmacbook.tailabc.ts.net."}}"#;
    let parsed = parse_tailscale_dns_name(payload);
    assert_eq!(parsed.as_deref(), Some("yashmacbook.tailabc.ts.net"));
}

#[test]
fn test_parse_tailscale_dns_name_handles_missing_or_empty() {
    let missing = br#"{"Self":{}}"#;
    assert!(parse_tailscale_dns_name(missing).is_none());

    let empty = br#"{"Self":{"DNSName":"   "}}"#;
    assert!(parse_tailscale_dns_name(empty).is_none());
}

#[test]
fn test_parse_tailscale_dns_name_invalid_json() {
    assert!(parse_tailscale_dns_name(b"not-json").is_none());
}

#[test]
fn configured_auth_test_targets_only_include_configured_supported_providers() {
    let _guard = crate::storage::lock_test_env();

    let status = AuthStatus {
        anthropic: ProviderAuth {
            state: AuthState::Available,
            has_oauth: true,
            oauth_state: AuthState::Available,
            has_api_key: false,
        },
        openai: AuthState::NotConfigured,
        gemini: AuthState::Available,
        google: AuthState::Expired,
        copilot: AuthState::Available,
        cursor: AuthState::NotConfigured,
        ..AuthStatus::default()
    };

    let targets = configured_auth_test_targets(&status);

    assert!(targets.contains(&ResolvedAuthTestTarget::Detailed(AuthTestTarget::Claude)));
    assert!(targets.contains(&ResolvedAuthTestTarget::Detailed(AuthTestTarget::Copilot)));
    assert!(targets.contains(&ResolvedAuthTestTarget::Detailed(AuthTestTarget::Gemini)));
    assert!(targets.contains(&ResolvedAuthTestTarget::Generic {
        provider: crate::provider_catalog::OPENROUTER_LOGIN_PROVIDER,
        choice: super::super::provider_init::ProviderChoice::Openrouter,
    }));

    assert!(!targets.contains(&ResolvedAuthTestTarget::Detailed(AuthTestTarget::Openai)));
    assert!(!targets.contains(&ResolvedAuthTestTarget::Detailed(AuthTestTarget::Google)));
    assert!(!targets.contains(&ResolvedAuthTestTarget::Detailed(AuthTestTarget::Cursor)));
}

#[test]
fn explicit_supported_provider_maps_to_single_auth_target() {
    let targets =
        resolve_auth_test_targets(&super::super::provider_init::ProviderChoice::Gemini, false)
            .expect("resolve target");
    assert_eq!(
        targets,
        vec![ResolvedAuthTestTarget::Detailed(AuthTestTarget::Gemini)]
    );
}

#[test]
fn explicit_generic_provider_maps_to_generic_auth_target() {
    let targets = resolve_auth_test_targets(
        &super::super::provider_init::ProviderChoice::Openrouter,
        false,
    )
    .expect("resolve target");
    assert_eq!(
        targets,
        vec![ResolvedAuthTestTarget::Generic {
            provider: crate::provider_catalog::OPENROUTER_LOGIN_PROVIDER,
            choice: super::super::provider_init::ProviderChoice::Openrouter,
        }]
    );
}

#[test]
fn collect_cli_model_names_prefers_available_routes_and_dedupes() {
    let routes = vec![
        ModelRoute {
            model: "gpt-5.4".to_string(),
            provider: "OpenAI".to_string(),
            api_method: "openai-oauth".to_string(),
            available: true,
            detail: String::new(),
            cheapness: None,
        },
        ModelRoute {
            model: "gpt-5.4".to_string(),
            provider: "auto".to_string(),
            api_method: "openrouter".to_string(),
            available: true,
            detail: String::new(),
            cheapness: None,
        },
        ModelRoute {
            model: "openrouter models".to_string(),
            provider: "—".to_string(),
            api_method: "openrouter".to_string(),
            available: false,
            detail: "OPENROUTER_API_KEY not set".to_string(),
            cheapness: None,
        },
    ];

    let models = collect_cli_model_names(
        &routes,
        vec!["gpt-5.4".to_string(), "claude-sonnet-4".to_string()],
    );

    assert_eq!(models, vec!["gpt-5.4", "claude-sonnet-4"]);
}

#[test]
fn queue_priority_parser_accepts_supported_values() {
    assert_eq!(
        parse_queue_priority(None).unwrap(),
        crate::queue::TaskPriority::Normal
    );
    assert_eq!(
        parse_queue_priority(Some("low")).unwrap(),
        crate::queue::TaskPriority::Low
    );
    assert_eq!(
        parse_queue_priority(Some("normal")).unwrap(),
        crate::queue::TaskPriority::Normal
    );
    assert_eq!(
        parse_queue_priority(Some("high")).unwrap(),
        crate::queue::TaskPriority::High
    );
    assert_eq!(
        parse_queue_priority(Some("urgent")).unwrap(),
        crate::queue::TaskPriority::Urgent
    );
}

#[test]
fn queue_status_parser_accepts_supported_values() {
    assert_eq!(
        parse_queue_status("backlog").unwrap(),
        crate::queue::TaskStatus::Backlog
    );
    assert_eq!(
        parse_queue_status("ready").unwrap(),
        crate::queue::TaskStatus::Ready
    );
    assert_eq!(
        parse_queue_status("running").unwrap(),
        crate::queue::TaskStatus::Running
    );
    assert_eq!(
        parse_queue_status("review").unwrap(),
        crate::queue::TaskStatus::Review
    );
    assert_eq!(
        parse_queue_status("done").unwrap(),
        crate::queue::TaskStatus::Done
    );
    assert_eq!(
        parse_queue_status("blocked").unwrap(),
        crate::queue::TaskStatus::Blocked
    );
    assert_eq!(
        parse_queue_status("cancelled").unwrap(),
        crate::queue::TaskStatus::Cancelled
    );
}

#[test]
fn queue_priority_parser_rejects_invalid_values() {
    let err = parse_queue_priority(Some("medium")).expect_err("invalid priority");
    assert!(err.to_string().contains("Invalid queue priority"));
    assert!(err.to_string().contains("low, normal, high, urgent"));
}

#[test]
fn queue_status_parser_rejects_invalid_values() {
    let err = parse_queue_status("waiting").expect_err("invalid status");
    assert!(err.to_string().contains("Invalid queue status"));
    assert!(
        err.to_string()
            .contains("backlog, ready, running, review, done, blocked, cancelled")
    );
}

#[test]
fn queue_list_format_handles_empty_and_tasks() {
    let empty = crate::queue::QueueState { tasks: Vec::new() };
    assert_eq!(format_queue_list(&empty), "Queue is empty.");

    let task = crate::queue::Task {
        id: "task_1".to_string(),
        title: "Fix docs".to_string(),
        description: "Update queue docs".to_string(),
        project: Some("jcode".to_string()),
        status: crate::queue::TaskStatus::Backlog,
        priority: crate::queue::TaskPriority::High,
        worker_profile: Some("default".to_string()),
        output_path: Some("out.md".to_string()),
        created_at: chrono::DateTime::parse_from_rfc3339("2026-06-20T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-06-20T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    };
    let state = crate::queue::QueueState { tasks: vec![task] };
    let output = format_queue_list(&state);
    assert!(output.contains("task_1"));
    assert!(output.contains("Fix docs"));
    assert!(output.contains("status: backlog"));
    assert!(output.contains("priority: high"));
    assert!(output.contains("project: jcode"));
    assert!(output.contains("worker_profile: default"));
    assert!(output.contains("output_path: out.md"));
}

#[test]
fn queue_show_format_prints_full_task_details() {
    let task = crate::queue::Task {
        id: "task_1".to_string(),
        title: "Fix docs".to_string(),
        description: "Update queue docs".to_string(),
        project: Some("jcode".to_string()),
        status: crate::queue::TaskStatus::Ready,
        priority: crate::queue::TaskPriority::Urgent,
        worker_profile: Some("default".to_string()),
        output_path: Some("out.md".to_string()),
        created_at: chrono::DateTime::parse_from_rfc3339("2026-06-20T12:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-06-20T13:00:00Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    };

    let output = format_queue_task(&task);
    assert!(output.contains("id: task_1"));
    assert!(output.contains("title: Fix docs"));
    assert!(output.contains("status: ready"));
    assert!(output.contains("priority: urgent"));
    assert!(output.contains("created_at: 2026-06-20T12:00:00+00:00"));
    assert!(output.contains("updated_at: 2026-06-20T13:00:00+00:00"));
    assert!(output.contains("description: Update queue docs"));
    assert!(output.contains("project: jcode"));
    assert!(output.contains("worker_profile: default"));
    assert!(output.contains("output_path: out.md"));
}

#[test]
fn queue_handoff_output_contains_agent_brief_fields() {
    let task = crate::queue::Task {
        id: "task_1".to_string(),
        title: "Fix docs".to_string(),
        description: "Update queue docs".to_string(),
        project: Some("jcode".to_string()),
        status: crate::queue::TaskStatus::Ready,
        priority: crate::queue::TaskPriority::High,
        worker_profile: Some("coder".to_string()),
        output_path: Some("out.md".to_string()),
        created_at: test_time("2026-06-20T12:00:00Z"),
        updated_at: test_time("2026-06-20T13:00:00Z"),
    };

    let output = format_queue_handoff(&task);

    assert!(output.contains("# Queue Task Handoff: Fix docs"));
    assert!(output.contains("- Task ID: task_1"));
    assert!(output.contains("- Title: Fix docs"));
    assert!(output.contains("- Description: Update queue docs"));
    assert!(output.contains("- Status: ready"));
    assert!(output.contains("- Priority: high"));
    assert!(output.contains("- Worker profile: coder"));
    assert!(output.contains("- Output path: out.md"));
    assert!(output.contains("- Understand the task before editing."));
    assert!(output.contains("- Report rollback instructions."));
}

#[test]
fn queue_handoff_missing_task_reports_helpful_error() {
    let state = crate::queue::QueueState { tasks: Vec::new() };

    let err = find_queue_task(&state, "missing").expect_err("missing task");

    assert!(
        err.to_string()
            .contains("Queue task 'missing' was not found")
    );
}

#[test]
fn queue_handoff_write_creates_expected_file() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    let task = crate::queue::Task {
        id: "task_1".to_string(),
        title: "Fix docs".to_string(),
        description: "Update queue docs".to_string(),
        project: None,
        status: crate::queue::TaskStatus::Ready,
        priority: crate::queue::TaskPriority::High,
        worker_profile: None,
        output_path: None,
        created_at: test_time("2026-06-20T12:00:00Z"),
        updated_at: test_time("2026-06-20T13:00:00Z"),
    };
    let brief = format_queue_handoff(&task);

    let path = write_queue_handoff(&task, &brief).expect("write handoff");

    assert_eq!(
        path,
        project
            .path()
            .join(".jcode")
            .join("queue")
            .join("handoffs")
            .join("task_1.md")
    );
    let written = std::fs::read_to_string(path).expect("read handoff");
    assert_eq!(written, brief);
}

#[test]
fn queue_handoff_next_selects_same_task_as_queue_next() {
    let older = test_time("2026-06-20T10:00:00Z");
    let newer = test_time("2026-06-20T11:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "coder_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                newer,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "research_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                older,
                Some("researcher"),
            ),
        ],
    };

    let next = next_queue_task(&state, None).expect("next task");
    let handoff = format_queue_handoff(next);

    assert_eq!(next.id, "research_ready");
    assert!(handoff.contains("- Task ID: research_ready"));
}

#[test]
fn queue_workers_format_handles_empty_and_profiles() {
    assert_eq!(
        format_worker_profiles(&[]),
        "No worker profiles found in .jcode/workers.toml."
    );

    let profiles = vec![
        crate::queue::WorkerProfile {
            name: "coder".to_string(),
            description: Some("Implements code changes".to_string()),
            command: Some("codex exec <handoff_file>".to_string()),
        },
        crate::queue::WorkerProfile {
            name: "reviewer".to_string(),
            description: None,
            command: None,
        },
    ];

    let output = format_worker_profiles(&profiles);
    assert!(output.contains("coder  Implements code changes"));
    assert!(output.contains("reviewer"));
}

#[test]
fn queue_worker_format_prints_one_profile() {
    let profile = crate::queue::WorkerProfile {
        name: "researcher".to_string(),
        description: Some("Researches sources and produces structured notes".to_string()),
        command: Some("opencode run <handoff_file>".to_string()),
    };

    let output = format_worker_profile(&profile);

    assert!(output.contains("name: researcher"));
    assert!(output.contains("description: Researches sources and produces structured notes"));
    assert!(output.contains("command: opencode run <handoff_file>"));
}

#[test]
fn queue_worker_format_reports_missing_command() {
    let profile = crate::queue::WorkerProfile {
        name: "reviewer".to_string(),
        description: None,
        command: None,
    };

    let output = format_worker_profile(&profile);

    assert!(output.contains("name: reviewer"));
    assert!(output.contains("command: not configured"));
}

#[test]
fn queue_worker_lookup_reports_missing_profile() {
    let profiles = vec![crate::queue::WorkerProfile {
        name: "coder".to_string(),
        description: None,
        command: None,
    }];

    let err = find_worker_profile(&profiles, "reviewer").expect_err("missing profile");

    assert!(
        err.to_string()
            .contains("Worker profile 'reviewer' was not found in .jcode/workers.toml")
    );
}

#[test]
fn queue_init_creates_expected_directories() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());

    let message = init_queue_project(false).expect("init queue");

    assert!(project.path().join(".jcode").is_dir());
    assert!(project.path().join(".jcode").join("queue").is_dir());
    assert!(
        project
            .path()
            .join(".jcode")
            .join("queue")
            .join("queue.json")
            .is_file()
    );
    assert!(
        project
            .path()
            .join(".jcode")
            .join("queue")
            .join("handoffs")
            .is_dir()
    );
    assert!(
        project
            .path()
            .join(".jcode")
            .join("queue")
            .join("runs")
            .is_dir()
    );
    assert!(message.contains("Created .jcode/"));
    assert!(message.contains("Created .jcode/queue/"));
    assert!(message.contains("Created .jcode/queue/queue.json"));
}

#[test]
fn queue_init_creates_queue_state_at_load_save_path() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());

    init_queue_project(false).expect("init queue");

    let queue_path = crate::queue::queue_file_path().expect("queue path");
    assert_eq!(
        queue_path,
        project
            .path()
            .join(".jcode")
            .join("queue")
            .join("queue.json")
    );
    assert!(queue_path.is_file());

    let state = crate::queue::load().expect("load local queue");
    assert!(state.tasks.is_empty());
}

#[test]
fn queue_init_creates_workers_toml_when_missing() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());

    let message = init_queue_project(false).expect("init queue");
    let workers = std::fs::read_to_string(project.path().join(".jcode").join("workers.toml"))
        .expect("workers.toml");

    assert!(workers.contains("[workers.coder]"));
    assert!(workers.contains("command = \"codex exec <handoff_file>\""));
    assert!(workers.contains("[workers.reviewer]"));
    assert!(workers.contains("[workers.researcher]"));
    assert!(workers.contains("command = \"opencode run <handoff_file>\""));
    assert!(message.contains("Created .jcode/workers.toml"));
}

#[test]
fn queue_init_does_not_overwrite_workers_toml_by_default() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    let jcode_dir = project.path().join(".jcode");
    std::fs::create_dir_all(&jcode_dir).expect("create .jcode");
    let workers_path = jcode_dir.join("workers.toml");
    std::fs::write(&workers_path, "[workers.custom]\ncommand = \"custom\"\n")
        .expect("write custom workers");

    let message = init_queue_project(false).expect("init queue");
    let workers = std::fs::read_to_string(workers_path).expect("workers.toml");

    assert_eq!(workers, "[workers.custom]\ncommand = \"custom\"\n");
    assert!(message.contains("Existing .jcode/workers.toml left unchanged"));
}

#[test]
fn queue_init_force_overwrites_workers_toml() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    let jcode_dir = project.path().join(".jcode");
    std::fs::create_dir_all(&jcode_dir).expect("create .jcode");
    let workers_path = jcode_dir.join("workers.toml");
    std::fs::write(&workers_path, "[workers.custom]\ncommand = \"custom\"\n")
        .expect("write custom workers");

    let message = init_queue_project(true).expect("init queue");
    let workers = std::fs::read_to_string(workers_path).expect("workers.toml");

    assert!(workers.contains("[workers.coder]"));
    assert!(!workers.contains("[workers.custom]"));
    assert!(message.contains("Overwrote .jcode/workers.toml"));
}

#[test]
fn queue_run_next_requires_worker_profile() {
    let err = run_queue_run_next_command(None, true, false).expect_err("missing worker profile");

    assert!(
        err.to_string()
            .contains("queue run-next requires --worker-profile <name>")
    );
}

#[test]
fn queue_run_next_requires_dry_run() {
    let err =
        run_queue_run_next_command(Some("coder"), false, false).expect_err("missing run mode");

    assert!(
        err.to_string()
            .contains("requires either --dry-run or --execute")
    );
    assert!(err.to_string().contains("--dry-run"));
    assert!(err.to_string().contains("--execute"));
}

#[test]
fn queue_run_next_rejects_dry_run_and_execute_together() {
    let err = run_queue_run_next_command(Some("coder"), true, true).expect_err("both run modes");

    assert!(
        err.to_string()
            .contains("cannot use --dry-run and --execute together")
    );
}

#[test]
fn queue_run_next_missing_worker_command_returns_helpful_error() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ndescription = \"Implements code changes\"\n",
    )
    .unwrap();

    let err =
        run_queue_run_next_command(Some("coder"), true, false).expect_err("missing worker command");

    assert!(
        err.to_string()
            .contains("Worker profile 'coder' has no command configured")
    );
}

#[test]
fn queue_run_next_dry_run_selects_worker_task_and_writes_handoff() {
    let _lock = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME"]);
    let home = tempfile::tempdir().expect("home tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    crate::env::set_var("JCODE_HOME", home.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ndescription = \"Implements code changes\"\ncommand = \"codex exec <handoff_file> --task <task_id>\"\n",
    )
    .unwrap();

    let older = test_time("2026-06-20T10:00:00Z");
    let newer = test_time("2026-06-20T11:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "other_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                older,
                Some("researcher"),
            ),
            test_queue_task_with_worker(
                "coder_newer",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                newer,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "coder_older",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                older,
                Some("coder"),
            ),
        ],
    };
    crate::queue::save(&state).expect("save queue state");

    run_queue_run_next_command(Some("coder"), true, false).expect("dry run");

    let handoff_path = project
        .path()
        .join(".jcode")
        .join("queue")
        .join("handoffs")
        .join("coder_older.md");
    let handoff = std::fs::read_to_string(handoff_path).expect("handoff written");
    assert!(handoff.contains("- Task ID: coder_older"));

    let reloaded = crate::queue::load().expect("reload queue");
    assert_eq!(reloaded.tasks[2].status, crate::queue::TaskStatus::Ready);
}

#[test]
fn queue_run_next_execute_marks_running_before_command_and_review_after_success() {
    let _lock = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME"]);
    let home = tempfile::tempdir().expect("home tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    crate::env::set_var("JCODE_HOME", home.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ncommand = \"test-worker <handoff_file> --task <task_id>\"\n",
    )
    .unwrap();

    let created_at = test_time("2026-06-20T10:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![test_queue_task_with_worker(
            "task_success",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::Urgent,
            created_at,
            Some("coder"),
        )],
    };
    crate::queue::save(&state).expect("save queue state");

    let mut saw_running = false;
    let executor = |_: &str| {
        let state = crate::queue::load().expect("load queue while command runs");
        saw_running = state.tasks[0].status == crate::queue::TaskStatus::Running;
        Ok(QueueRunCommandOutput {
            stdout: "ok\n".to_string(),
            stderr: String::new(),
            exit_code: 0,
        })
    };

    let output = run_queue_run_next_command_with_executor(Some("coder"), false, true, executor)
        .expect("execute success")
        .expect("selected task");

    assert!(saw_running);
    assert_eq!(output.task_id, "task_success");
    assert!(
        output
            .run_dir
            .parent()
            .is_some_and(|path| path.ends_with("task_success"))
    );
    assert!(output.run_dir.join("command.txt").exists());
    assert!(output.run_dir.join("stdout.txt").exists());
    assert!(output.run_dir.join("stderr.txt").exists());
    assert!(output.run_dir.join("run.json").exists());

    let reloaded = crate::queue::load().expect("reload queue");
    assert_eq!(reloaded.tasks[0].status, crate::queue::TaskStatus::Review);
    assert!(reloaded.tasks[0].updated_at > created_at);
}

#[test]
fn queue_run_next_execute_marks_blocked_after_failure() {
    let _lock = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME"]);
    let home = tempfile::tempdir().expect("home tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    crate::env::set_var("JCODE_HOME", home.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ncommand = \"test-worker <handoff_file> --task <task_id>\"\n",
    )
    .unwrap();

    let state = crate::queue::QueueState {
        tasks: vec![test_queue_task_with_worker(
            "task_failure",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::Urgent,
            test_time("2026-06-20T10:00:00Z"),
            Some("coder"),
        )],
    };
    crate::queue::save(&state).expect("save queue state");

    let executor = |_: &str| {
        Ok(QueueRunCommandOutput {
            stdout: String::new(),
            stderr: "failed\n".to_string(),
            exit_code: 23,
        })
    };

    let output = run_queue_run_next_command_with_executor(Some("coder"), false, true, executor)
        .expect("execute failure is recorded")
        .expect("selected task");

    assert_eq!(output.task_id, "task_failure");
    let run_json = std::fs::read_to_string(output.run_dir.join("run.json")).expect("run json");
    assert!(run_json.contains("\"exit_code\": 23"));

    let reloaded = crate::queue::load().expect("reload queue");
    assert_eq!(reloaded.tasks[0].status, crate::queue::TaskStatus::Blocked);
}

#[test]
fn queue_run_next_placeholder_replacement_uses_handoff_file_and_task_id() {
    let task = test_queue_task(
        "task_1",
        crate::queue::TaskStatus::Ready,
        crate::queue::TaskPriority::High,
        test_time("2026-06-20T12:00:00Z"),
    );
    let handoff_path = std::path::Path::new(".jcode/queue/handoffs/task_1.md");

    let rendered = render_worker_command(
        "codex exec <handoff_file> --task <task_id>",
        &task,
        handoff_path,
    );

    assert_eq!(
        rendered,
        "codex exec .jcode/queue/handoffs/task_1.md --task task_1"
    );
}

#[test]
fn queue_runs_empty_directory_returns_clear_message() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());

    let runs = list_queue_runs(&queue_runs_dir_path().unwrap(), None).expect("list runs");
    let output = format_queue_runs(&runs, 20);

    assert_eq!(output, "No queue runs found in .jcode/queue/runs.");
}

#[test]
fn queue_runs_lists_existing_runs_from_test_directories() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    write_queue_run_fixture(
        project.path(),
        "task_old",
        "20260620T100000Z",
        "coder",
        0,
        "old command",
        "old stdout",
        "",
    );
    write_queue_run_fixture(
        project.path(),
        "task_new",
        "20260620T110000Z",
        "reviewer",
        17,
        "new command",
        "new stdout",
        "new stderr",
    );

    let runs = list_queue_runs(&queue_runs_dir_path().unwrap(), None).expect("list runs");
    let output = format_queue_runs(&runs, 10);

    assert!(output.contains("Recent queue runs:"));
    assert!(output.contains("task_new  20260620T110000Z"));
    assert!(output.contains("worker_profile: reviewer"));
    assert!(output.contains("exit_code: 17"));
    assert!(output.contains("task_old  20260620T100000Z"));
    assert!(
        output.find("task_new").expect("new listed") < output.find("task_old").expect("old listed")
    );
}

#[test]
fn queue_runs_lists_run_directories_without_metadata_as_unknown() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    let run_dir = project
        .path()
        .join(".jcode")
        .join("queue")
        .join("runs")
        .join("task_without_json")
        .join("20260620T100000Z");
    std::fs::create_dir_all(&run_dir).expect("create run dir");

    let runs = list_queue_runs(&queue_runs_dir_path().unwrap(), None).expect("list runs");
    let output = format_queue_runs(&runs, 10);

    assert!(output.contains("task_without_json  20260620T100000Z"));
    assert!(output.contains("worker_profile: unknown"));
    assert!(output.contains("exit_code: unknown"));
}

#[test]
fn queue_runs_filters_by_task_id_and_applies_limit() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    write_queue_run_fixture(
        project.path(),
        "task_1",
        "20260620T100000Z",
        "coder",
        0,
        "command 1",
        "",
        "",
    );
    write_queue_run_fixture(
        project.path(),
        "task_1",
        "20260620T110000Z",
        "coder",
        0,
        "command 2",
        "",
        "",
    );
    write_queue_run_fixture(
        project.path(),
        "task_2",
        "20260620T120000Z",
        "coder",
        0,
        "command 3",
        "",
        "",
    );

    let runs = list_queue_runs(&queue_runs_dir_path().unwrap(), Some("task_1")).expect("list runs");
    let output = format_queue_runs(&runs, 1);

    assert!(output.contains("task_1  20260620T110000Z"));
    assert!(!output.contains("task_1  20260620T100000Z"));
    assert!(!output.contains("task_2"));
}

#[test]
fn queue_run_reads_summary_and_short_previews() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    let long_stdout = format!("{}tail", "x".repeat(2_100));
    write_queue_run_fixture(
        project.path(),
        "task_1",
        "20260620T100000Z",
        "coder",
        0,
        "codex exec .jcode/queue/handoffs/task_1.md --task task_1",
        &long_stdout,
        "stderr line",
    );

    let run = read_queue_run("task_1", "20260620T100000Z").expect("read run");
    let output = format_queue_run_summary(&run, false, false).expect("format summary");

    assert!(output.contains("task_id: task_1"));
    assert!(output.contains("worker_profile: coder"));
    assert!(output.contains("command: codex exec .jcode/queue/handoffs/task_1.md --task task_1"));
    assert!(output.contains("exit_code: 0"));
    assert!(output.contains("started_at: 2026-06-20T10:00:00+00:00"));
    assert!(output.contains("ended_at: 2026-06-20T10:00:01+00:00"));
    assert!(output.contains("stdout_path:"));
    assert!(output.contains("stderr_path:"));
    assert!(output.contains("stdout:"));
    assert!(output.contains("stderr line"));
    assert!(output.contains("truncated; pass --stdout or --stderr"));
    assert!(!output.contains("tail"));
}

#[test]
fn queue_run_full_stdout_and_stderr_flags_print_full_streams() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    write_queue_run_fixture(
        project.path(),
        "task_1",
        "20260620T100000Z",
        "coder",
        1,
        "worker command",
        "full stdout",
        "full stderr",
    );

    let run = read_queue_run("task_1", "20260620T100000Z").expect("read run");
    let output = format_queue_run_summary(&run, true, true).expect("format summary");

    assert!(output.contains("full stdout"));
    assert!(output.contains("full stderr"));
    assert!(!output.contains("truncated; pass --stdout or --stderr"));
}

#[test]
fn queue_run_missing_run_returns_helpful_error() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());

    let err = read_queue_run("missing", "20260620T100000Z").expect_err("missing run");

    assert!(err.to_string().contains("Queue run 'missing'"));
    assert!(err.to_string().contains("was not found"));
    assert!(err.to_string().contains(".jcode"));
}

#[test]
fn queue_run_missing_metadata_returns_helpful_error() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    let run_dir = project
        .path()
        .join(".jcode")
        .join("queue")
        .join("runs")
        .join("task_1")
        .join("20260620T100000Z");
    std::fs::create_dir_all(&run_dir).expect("create run dir");

    let err = read_queue_run("task_1", "20260620T100000Z").expect_err("missing metadata");

    assert!(err.to_string().contains("Queue run metadata is missing"));
    assert!(err.to_string().contains("run.json"));
}

#[test]
fn queue_add_accepts_existing_worker_profile() {
    let _lock = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME"]);
    let home = tempfile::tempdir().expect("home tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    crate::env::set_var("JCODE_HOME", home.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ndescription = \"Implements code changes\"\n",
    )
    .unwrap();

    run_queue_add_command(QueueAddOptions {
        title: "Implement feature".to_string(),
        description: None,
        project: None,
        priority: None,
        worker_profile: Some("coder".to_string()),
        output_path: None,
    })
    .expect("add task");

    let state = crate::queue::load().expect("load queue state");
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].worker_profile.as_deref(), Some("coder"));
}

#[test]
fn queue_add_rejects_missing_worker_profile_when_config_exists() {
    let _lock = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME"]);
    let home = tempfile::tempdir().expect("home tempdir");
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    crate::env::set_var("JCODE_HOME", home.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ndescription = \"Implements code changes\"\n",
    )
    .unwrap();

    let err = run_queue_add_command(QueueAddOptions {
        title: "Review feature".to_string(),
        description: None,
        project: None,
        priority: None,
        worker_profile: Some("reviewer".to_string()),
        output_path: None,
    })
    .expect_err("missing worker profile");

    assert!(
        err.to_string()
            .contains("Worker profile 'reviewer' was not found in .jcode/workers.toml")
    );
    let state = crate::queue::load().expect("load queue state");
    assert!(state.tasks.is_empty());
}

#[test]
fn queue_worker_filter_validation_rejects_missing_profile_when_config_exists() {
    let _lock = crate::storage::lock_test_env();
    let project = tempfile::tempdir().expect("project tempdir");
    let _cwd = CurrentDirGuard::change_to(project.path());
    std::fs::create_dir_all(project.path().join(".jcode")).unwrap();
    std::fs::write(
        project.path().join(".jcode").join("workers.toml"),
        "[workers.coder]\ndescription = \"Implements code changes\"\n",
    )
    .unwrap();

    let err = validate_queue_worker_profile(Some("researcher"))
        .expect_err("missing worker profile should fail");

    assert!(
        err.to_string()
            .contains("Worker profile 'researcher' was not found in .jcode/workers.toml")
    );
}

#[test]
fn queue_next_prefers_ready_over_backlog() {
    let newer = test_time("2026-06-20T13:00:00Z");
    let older = test_time("2026-06-20T12:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "backlog_urgent",
                crate::queue::TaskStatus::Backlog,
                crate::queue::TaskPriority::Urgent,
                older,
            ),
            test_queue_task(
                "ready_low",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Low,
                newer,
            ),
        ],
    };

    assert_eq!(next_queue_task(&state, None).unwrap().id, "ready_low");
}

#[test]
fn queue_next_sorts_priorities_then_created_at() {
    let oldest = test_time("2026-06-20T10:00:00Z");
    let middle = test_time("2026-06-20T11:00:00Z");
    let newest = test_time("2026-06-20T12:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "normal_oldest",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Normal,
                oldest,
            ),
            test_queue_task(
                "high_newest",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::High,
                newest,
            ),
            test_queue_task(
                "urgent_middle",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                middle,
            ),
        ],
    };

    assert_eq!(next_queue_task(&state, None).unwrap().id, "urgent_middle");

    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "high_newer",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::High,
                newest,
            ),
            test_queue_task(
                "high_older",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::High,
                oldest,
            ),
        ],
    };

    assert_eq!(next_queue_task(&state, None).unwrap().id, "high_older");
}

#[test]
fn queue_next_worker_filter_selects_only_requested_worker() {
    let older = test_time("2026-06-20T10:00:00Z");
    let newer = test_time("2026-06-20T11:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "research_backlog",
                crate::queue::TaskStatus::Backlog,
                crate::queue::TaskPriority::Urgent,
                older,
                Some("researcher"),
            ),
            test_queue_task_with_worker(
                "research_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Low,
                newer,
                Some("researcher"),
            ),
            test_queue_task_with_worker(
                "coder_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                older,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "unassigned_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                older,
                None,
            ),
        ],
    };

    assert_eq!(
        next_queue_task(&state, Some("researcher")).unwrap().id,
        "research_ready"
    );
}

#[test]
fn queue_next_worker_filter_ignores_other_workers() {
    let created_at = test_time("2026-06-20T10:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![test_queue_task_with_worker(
            "coder_ready",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::Urgent,
            created_at,
            Some("coder"),
        )],
    };

    assert!(next_queue_task(&state, Some("researcher")).is_none());
    assert_eq!(
        format_queue_next(&state, Some("researcher")),
        "No actionable queue tasks found for worker_profile 'researcher'."
    );
}

#[test]
fn queue_next_without_worker_filter_preserves_normal_behavior() {
    let older = test_time("2026-06-20T10:00:00Z");
    let newer = test_time("2026-06-20T11:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "coder_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                newer,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "research_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                older,
                Some("researcher"),
            ),
        ],
    };

    assert_eq!(next_queue_task(&state, None).unwrap().id, "research_ready");
}

#[test]
fn queue_next_ignores_non_actionable_statuses() {
    let created_at = test_time("2026-06-20T12:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "running",
                crate::queue::TaskStatus::Running,
                crate::queue::TaskPriority::Urgent,
                created_at,
            ),
            test_queue_task(
                "review",
                crate::queue::TaskStatus::Review,
                crate::queue::TaskPriority::Urgent,
                created_at,
            ),
            test_queue_task(
                "done",
                crate::queue::TaskStatus::Done,
                crate::queue::TaskPriority::Urgent,
                created_at,
            ),
            test_queue_task(
                "blocked",
                crate::queue::TaskStatus::Blocked,
                crate::queue::TaskPriority::Urgent,
                created_at,
            ),
            test_queue_task(
                "cancelled",
                crate::queue::TaskStatus::Cancelled,
                crate::queue::TaskPriority::Urgent,
                created_at,
            ),
        ],
    };

    assert!(next_queue_task(&state, None).is_none());
    assert_eq!(
        format_queue_next(&state, None),
        "No actionable queue tasks found."
    );
}

#[test]
fn queue_next_format_prints_selected_task() {
    let state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::High,
            test_time("2026-06-20T12:00:00Z"),
        )],
    };

    let output = format_queue_next(&state, None);
    assert!(output.starts_with("Next queue task:\n"));
    assert!(output.contains("id: task_1"));
    assert!(output.contains("status: ready"));
    assert!(output.contains("priority: high"));
}

#[test]
fn queue_review_lists_only_review_tasks() {
    let created_at = test_time("2026-06-20T10:00:00Z");
    let updated_at = test_time("2026-06-20T12:00:00Z");
    let mut review_task = test_queue_task_with_worker(
        "review_task",
        crate::queue::TaskStatus::Review,
        crate::queue::TaskPriority::High,
        created_at,
        Some("coder"),
    );
    review_task.title = "Review this change".to_string();
    review_task.output_path = Some("out.md".to_string());
    review_task.updated_at = updated_at;
    let state = crate::queue::QueueState {
        tasks: vec![
            review_task,
            test_queue_task(
                "ready_task",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                created_at,
            ),
        ],
    };

    let output = format_queue_review(&state, None, 20);

    assert!(output.contains("Review queue tasks:"));
    assert!(output.contains("review_task  Review this change"));
    assert!(output.contains("priority: high"));
    assert!(output.contains("worker_profile: coder"));
    assert!(output.contains("output_path: out.md"));
    assert!(output.contains("updated_at: 2026-06-20T12:00:00+00:00"));
    assert!(!output.contains("ready_task"));
}

#[test]
fn queue_review_filters_by_worker_profile_and_limit() {
    let created_at = test_time("2026-06-20T10:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "coder_old",
                crate::queue::TaskStatus::Review,
                crate::queue::TaskPriority::Normal,
                created_at,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "reviewer",
                crate::queue::TaskStatus::Review,
                crate::queue::TaskPriority::Normal,
                created_at,
                Some("reviewer"),
            ),
            test_queue_task_with_worker(
                "coder_new",
                crate::queue::TaskStatus::Review,
                crate::queue::TaskPriority::Normal,
                test_time("2026-06-20T11:00:00Z"),
                Some("coder"),
            ),
        ],
    };

    let output = format_queue_review(&state, Some("coder"), 1);

    assert!(output.contains("coder_new"));
    assert!(!output.contains("coder_old"));
    assert!(!output.contains("reviewer"));
}

#[test]
fn queue_review_empty_returns_clear_message() {
    let state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "ready_task",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::Normal,
            test_time("2026-06-20T10:00:00Z"),
        )],
    };

    assert_eq!(
        format_queue_review(&state, None, 20),
        "No queue tasks are waiting for review."
    );
    assert_eq!(
        format_queue_review(&state, Some("coder"), 20),
        "No queue tasks are waiting for review for worker_profile 'coder'."
    );
}

#[test]
fn queue_approve_moves_review_task_to_done() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let updated_time = test_time("2026-06-20T12:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Review,
            crate::queue::TaskPriority::Normal,
            original_time,
        )],
    };

    let message = approve_queue_task(&mut state, "task_1", updated_time).expect("approve task");

    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Done);
    assert_eq!(state.tasks[0].updated_at, updated_time);
    assert!(message.contains("Approved queue task 'task_1'"));
    assert!(message.contains("done"));
}

#[test]
fn queue_approve_rejects_non_review_task() {
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::Normal,
            test_time("2026-06-20T10:00:00Z"),
        )],
    };

    let err = approve_queue_task(&mut state, "task_1", chrono::Utc::now())
        .expect_err("non-review task should be rejected");

    assert!(err.to_string().contains("cannot be approved"));
    assert!(err.to_string().contains("Expected status: review"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Ready);
}

#[test]
fn queue_reopen_moves_review_done_and_blocked_tasks_to_ready() {
    for status in [
        crate::queue::TaskStatus::Review,
        crate::queue::TaskStatus::Done,
        crate::queue::TaskStatus::Blocked,
    ] {
        let original_time = test_time("2026-06-20T10:00:00Z");
        let updated_time = test_time("2026-06-20T12:00:00Z");
        let mut state = crate::queue::QueueState {
            tasks: vec![test_queue_task(
                "task_1",
                status,
                crate::queue::TaskPriority::Normal,
                original_time,
            )],
        };

        let message = reopen_queue_task(&mut state, "task_1", updated_time).expect("reopen task");

        assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Ready);
        assert_eq!(state.tasks[0].updated_at, updated_time);
        assert!(message.contains("Reopened queue task 'task_1'"));
        assert!(message.contains("ready"));
    }
}

#[test]
fn queue_reopen_rejects_running_task() {
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Running,
            crate::queue::TaskPriority::Normal,
            test_time("2026-06-20T10:00:00Z"),
        )],
    };

    let err = reopen_queue_task(&mut state, "task_1", chrono::Utc::now())
        .expect_err("running task should not reopen");

    assert!(err.to_string().contains("cannot be reopened while running"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Running);
}

#[test]
fn queue_review_mutators_report_missing_task() {
    let mut state = crate::queue::QueueState { tasks: Vec::new() };

    let approve_err = approve_queue_task(&mut state, "missing", chrono::Utc::now())
        .expect_err("missing approve task");
    assert!(
        approve_err
            .to_string()
            .contains("Queue task 'missing' was not found")
    );

    let reopen_err = reopen_queue_task(&mut state, "missing", chrono::Utc::now())
        .expect_err("missing reopen task");
    assert!(
        reopen_err
            .to_string()
            .contains("Queue task 'missing' was not found")
    );
}

#[test]
fn queue_dashboard_empty_queue_returns_clear_message() {
    let state = crate::queue::QueueState { tasks: Vec::new() };

    assert_eq!(
        format_queue_dashboard(&state, None, 20),
        "Queue is empty. No tasks to show."
    );
}

#[test]
fn queue_dashboard_prints_status_counts() {
    let created_at = test_time("2026-06-20T10:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "backlog",
                crate::queue::TaskStatus::Backlog,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
            test_queue_task(
                "ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
            test_queue_task(
                "running",
                crate::queue::TaskStatus::Running,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
            test_queue_task(
                "review",
                crate::queue::TaskStatus::Review,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
            test_queue_task(
                "done",
                crate::queue::TaskStatus::Done,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
            test_queue_task(
                "blocked",
                crate::queue::TaskStatus::Blocked,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
            test_queue_task(
                "cancelled",
                crate::queue::TaskStatus::Cancelled,
                crate::queue::TaskPriority::Normal,
                created_at,
            ),
        ],
    };

    let output = format_queue_dashboard(&state, None, 20);

    assert!(output.contains("Queue dashboard"));
    assert!(output.contains("total: 7"));
    assert!(output.contains("backlog: 1"));
    assert!(output.contains("ready: 1"));
    assert!(output.contains("running: 1"));
    assert!(output.contains("review: 1"));
    assert!(output.contains("done: 1"));
    assert!(output.contains("blocked: 1"));
    assert!(output.contains("cancelled: 1"));
}

#[test]
fn queue_dashboard_includes_next_actionable_task() {
    let older = test_time("2026-06-20T10:00:00Z");
    let newer = test_time("2026-06-20T11:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "backlog_urgent",
                crate::queue::TaskStatus::Backlog,
                crate::queue::TaskPriority::Urgent,
                older,
            ),
            test_queue_task(
                "ready_low",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Low,
                newer,
            ),
        ],
    };

    let output = format_queue_dashboard(&state, None, 20);

    assert!(output.contains("Next actionable task:"));
    assert!(output.contains("ready_low  ready_low"));
    assert!(output.contains("priority: low"));
    assert!(!output.contains("backlog_urgent  backlog_urgent"));
}

#[test]
fn queue_dashboard_includes_running_review_and_blocked_sections() {
    let created_at = test_time("2026-06-20T10:00:00Z");
    let updated_at = test_time("2026-06-20T12:00:00Z");
    let mut running = test_queue_task_with_worker(
        "running_task",
        crate::queue::TaskStatus::Running,
        crate::queue::TaskPriority::High,
        created_at,
        Some("coder"),
    );
    running.title = "Worker is active".to_string();
    running.updated_at = updated_at;

    let mut review = test_queue_task_with_worker(
        "review_task",
        crate::queue::TaskStatus::Review,
        crate::queue::TaskPriority::Normal,
        created_at,
        Some("reviewer"),
    );
    review.title = "Needs approval".to_string();
    review.output_path = Some("out.md".to_string());
    review.updated_at = updated_at;

    let mut blocked = test_queue_task_with_worker(
        "blocked_task",
        crate::queue::TaskStatus::Blocked,
        crate::queue::TaskPriority::Urgent,
        created_at,
        Some("coder"),
    );
    blocked.title = "Needs help".to_string();
    blocked.updated_at = updated_at;

    let state = crate::queue::QueueState {
        tasks: vec![running, review, blocked],
    };

    let output = format_queue_dashboard(&state, None, 20);

    assert!(output.contains("Running tasks:"));
    assert!(output.contains("running_task  Worker is active"));
    assert!(output.contains("worker_profile: coder"));
    assert!(output.contains("updated_at: 2026-06-20T12:00:00+00:00"));
    assert!(output.contains("Review tasks:"));
    assert!(output.contains("review_task  Needs approval"));
    assert!(output.contains("worker_profile: reviewer"));
    assert!(output.contains("output_path: out.md"));
    assert!(output.contains("Blocked tasks:"));
    assert!(output.contains("blocked_task  Needs help"));
}

#[test]
fn queue_dashboard_filters_by_worker_profile_and_limit() {
    let created_at = test_time("2026-06-20T10:00:00Z");
    let state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "coder_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Normal,
                created_at,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "reviewer_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                created_at,
                Some("reviewer"),
            ),
            test_queue_task_with_worker(
                "coder_running_old",
                crate::queue::TaskStatus::Running,
                crate::queue::TaskPriority::Normal,
                created_at,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "coder_running_new",
                crate::queue::TaskStatus::Running,
                crate::queue::TaskPriority::Normal,
                test_time("2026-06-20T11:00:00Z"),
                Some("coder"),
            ),
        ],
    };

    let output = format_queue_dashboard(&state, Some("coder"), 1);

    assert!(output.contains("worker_profile: coder"));
    assert!(output.contains("total: 3"));
    assert!(output.contains("coder_ready  coder_ready"));
    assert!(!output.contains("reviewer_ready"));
    assert!(output.contains("coder_running_new"));
    assert!(!output.contains("coder_running_old"));
}

#[test]
fn queue_start_next_marks_selected_task_running() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let updated_time = test_time("2026-06-20T13:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "backlog_urgent",
                crate::queue::TaskStatus::Backlog,
                crate::queue::TaskPriority::Urgent,
                original_time,
            ),
            test_queue_task(
                "ready_high",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::High,
                original_time,
            ),
        ],
    };

    let output = start_next_queue_task(&mut state, updated_time, None);

    assert!(output.started);
    assert!(output.message.starts_with("Started queue task:\n"));
    assert!(output.message.contains("id: ready_high"));
    assert!(output.message.contains("status: running"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Backlog);
    assert_eq!(state.tasks[0].updated_at, original_time);
    assert_eq!(state.tasks[1].status, crate::queue::TaskStatus::Running);
    assert_eq!(state.tasks[1].updated_at, updated_time);
}

#[test]
fn queue_start_next_uses_priority_and_age_ordering() {
    let oldest = test_time("2026-06-20T10:00:00Z");
    let newest = test_time("2026-06-20T12:00:00Z");
    let updated_time = test_time("2026-06-20T13:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task(
                "high_newer",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::High,
                newest,
            ),
            test_queue_task(
                "high_older",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::High,
                oldest,
            ),
            test_queue_task(
                "normal_oldest",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Normal,
                oldest,
            ),
        ],
    };

    let output = start_next_queue_task(&mut state, updated_time, None);

    assert!(output.started);
    assert!(output.message.contains("id: high_older"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Ready);
    assert_eq!(state.tasks[1].status, crate::queue::TaskStatus::Running);
    assert_eq!(state.tasks[2].status, crate::queue::TaskStatus::Ready);
}

#[test]
fn queue_start_next_does_not_modify_when_no_actionable_task_exists() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "blocked",
            crate::queue::TaskStatus::Blocked,
            crate::queue::TaskPriority::Urgent,
            original_time,
        )],
    };
    let before = state.clone();

    let output = start_next_queue_task(&mut state, test_time("2026-06-20T13:00:00Z"), None);

    assert!(!output.started);
    assert_eq!(output.message, "No actionable queue tasks found.");
    assert_eq!(state, before);
}

#[test]
fn queue_start_next_worker_filter_marks_selected_task_running() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let updated_time = test_time("2026-06-20T13:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![
            test_queue_task_with_worker(
                "coder_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Urgent,
                original_time,
                Some("coder"),
            ),
            test_queue_task_with_worker(
                "research_ready",
                crate::queue::TaskStatus::Ready,
                crate::queue::TaskPriority::Low,
                original_time,
                Some("researcher"),
            ),
        ],
    };

    let output = start_next_queue_task(&mut state, updated_time, Some("researcher"));

    assert!(output.started);
    assert!(output.message.contains("id: research_ready"));
    assert!(output.message.contains("status: running"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Ready);
    assert_eq!(state.tasks[0].updated_at, original_time);
    assert_eq!(state.tasks[1].status, crate::queue::TaskStatus::Running);
    assert_eq!(state.tasks[1].updated_at, updated_time);
}

#[test]
fn queue_start_next_worker_filter_reports_no_match_without_modifying_state() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task_with_worker(
            "coder_ready",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::Urgent,
            original_time,
            Some("coder"),
        )],
    };
    let before = state.clone();

    let output = start_next_queue_task(
        &mut state,
        test_time("2026-06-20T13:00:00Z"),
        Some("researcher"),
    );

    assert!(!output.started);
    assert_eq!(
        output.message,
        "No actionable queue tasks found for worker_profile 'researcher'."
    );
    assert_eq!(state, before);
}

#[test]
fn queue_finish_running_task_to_review() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let updated_time = test_time("2026-06-20T13:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Running,
            crate::queue::TaskPriority::High,
            original_time,
        )],
    };

    let output = finish_queue_task(&mut state, "task_1", false, None, updated_time).unwrap();

    assert!(output.starts_with("Finished queue task:\n"));
    assert!(output.contains("id: task_1"));
    assert!(output.contains("status: review"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Review);
    assert_eq!(state.tasks[0].updated_at, updated_time);
}

#[test]
fn queue_finish_running_task_to_done() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let updated_time = test_time("2026-06-20T13:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Running,
            crate::queue::TaskPriority::High,
            original_time,
        )],
    };

    let output = finish_queue_task(&mut state, "task_1", true, None, updated_time).unwrap();

    assert!(output.contains("status: done"));
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Done);
    assert_eq!(state.tasks[0].updated_at, updated_time);
}

#[test]
fn queue_finish_updates_output_path() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let updated_time = test_time("2026-06-20T13:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Running,
            crate::queue::TaskPriority::High,
            original_time,
        )],
    };

    let output = finish_queue_task(
        &mut state,
        "task_1",
        false,
        Some("out.md".to_string()),
        updated_time,
    )
    .unwrap();

    assert!(output.contains("output_path: out.md"));
    assert_eq!(state.tasks[0].output_path.as_deref(), Some("out.md"));
    assert_eq!(state.tasks[0].updated_at, updated_time);
}

#[test]
fn queue_finish_reports_missing_task() {
    let mut state = crate::queue::QueueState { tasks: Vec::new() };
    let err = finish_queue_task(
        &mut state,
        "missing",
        false,
        None,
        test_time("2026-06-20T13:00:00Z"),
    )
    .expect_err("missing task");

    assert!(
        err.to_string()
            .contains("Queue task 'missing' was not found")
    );
}

#[test]
fn queue_finish_rejects_not_running_task() {
    let original_time = test_time("2026-06-20T10:00:00Z");
    let mut state = crate::queue::QueueState {
        tasks: vec![test_queue_task(
            "task_1",
            crate::queue::TaskStatus::Ready,
            crate::queue::TaskPriority::High,
            original_time,
        )],
    };
    let before = state.clone();

    let err = finish_queue_task(
        &mut state,
        "task_1",
        false,
        None,
        test_time("2026-06-20T13:00:00Z"),
    )
    .expect_err("not running");

    assert!(err.to_string().contains("Queue task 'task_1' is 'ready'"));
    assert!(err.to_string().contains("Expected status: running"));
    assert_eq!(state, before);
}

#[test]
fn queue_status_format_counts_all_statuses() {
    let make_task = |status| crate::queue::Task {
        id: crate::id::new_id("task"),
        title: "Task".to_string(),
        description: String::new(),
        project: None,
        status,
        priority: crate::queue::TaskPriority::Normal,
        worker_profile: None,
        output_path: None,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    let state = crate::queue::QueueState {
        tasks: vec![
            make_task(crate::queue::TaskStatus::Backlog),
            make_task(crate::queue::TaskStatus::Backlog),
            make_task(crate::queue::TaskStatus::Ready),
            make_task(crate::queue::TaskStatus::Done),
        ],
    };

    let output = format_queue_status(&state);
    assert!(output.contains("backlog: 2"));
    assert!(output.contains("ready: 1"));
    assert!(output.contains("running: 0"));
    assert!(output.contains("review: 0"));
    assert!(output.contains("done: 1"));
    assert!(output.contains("blocked: 0"));
    assert!(output.contains("cancelled: 0"));
    assert!(output.contains("total: 4"));
}

fn test_time(raw: &str) -> chrono::DateTime<chrono::Utc> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .unwrap()
        .with_timezone(&chrono::Utc)
}

fn test_queue_task(
    id: &str,
    status: crate::queue::TaskStatus,
    priority: crate::queue::TaskPriority,
    created_at: chrono::DateTime<chrono::Utc>,
) -> crate::queue::Task {
    crate::queue::Task {
        id: id.to_string(),
        title: id.to_string(),
        description: String::new(),
        project: None,
        status,
        priority,
        worker_profile: None,
        output_path: None,
        created_at,
        updated_at: created_at,
    }
}

fn test_queue_task_with_worker(
    id: &str,
    status: crate::queue::TaskStatus,
    priority: crate::queue::TaskPriority,
    created_at: chrono::DateTime<chrono::Utc>,
    worker_profile: Option<&str>,
) -> crate::queue::Task {
    let mut task = test_queue_task(id, status, priority, created_at);
    task.worker_profile = worker_profile.map(ToOwned::to_owned);
    task
}

#[allow(clippy::too_many_arguments)]
fn write_queue_run_fixture(
    project: &std::path::Path,
    task_id: &str,
    timestamp: &str,
    worker_profile: &str,
    exit_code: i32,
    command: &str,
    stdout: &str,
    stderr: &str,
) {
    let run_dir = project
        .join(".jcode")
        .join("queue")
        .join("runs")
        .join(task_id)
        .join(timestamp);
    std::fs::create_dir_all(&run_dir).expect("create run dir");
    std::fs::write(run_dir.join("stdout.txt"), stdout).expect("write stdout");
    std::fs::write(run_dir.join("stderr.txt"), stderr).expect("write stderr");
    std::fs::write(run_dir.join("command.txt"), command).expect("write command");
    let metadata = serde_json::json!({
        "task_id": task_id,
        "worker_profile": worker_profile,
        "command": command,
        "exit_code": exit_code,
        "started_at": "2026-06-20T10:00:00+00:00",
        "ended_at": "2026-06-20T10:00:01+00:00"
    });
    std::fs::write(
        run_dir.join("run.json"),
        serde_json::to_string_pretty(&metadata).expect("serialize metadata"),
    )
    .expect("write run metadata");
}

struct CurrentDirGuard {
    previous: std::path::PathBuf,
}

impl CurrentDirGuard {
    fn change_to(path: &std::path::Path) -> Self {
        let previous = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(path).expect("set current dir");
        Self { previous }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.previous).expect("restore current dir");
    }
}

#[test]
fn queue_task_mutators_update_requested_field_and_timestamp() {
    let original_time = chrono::DateTime::parse_from_rfc3339("2026-06-20T12:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let updated_time = chrono::DateTime::parse_from_rfc3339("2026-06-20T13:00:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let task = crate::queue::Task {
        id: "task_1".to_string(),
        title: "Fix docs".to_string(),
        description: String::new(),
        project: None,
        status: crate::queue::TaskStatus::Backlog,
        priority: crate::queue::TaskPriority::Normal,
        worker_profile: None,
        output_path: None,
        created_at: original_time,
        updated_at: original_time,
    };
    let mut state = crate::queue::QueueState { tasks: vec![task] };

    update_queue_task_status(
        &mut state,
        "task_1",
        crate::queue::TaskStatus::Review,
        updated_time,
    )
    .unwrap();
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Review);
    assert_eq!(state.tasks[0].priority, crate::queue::TaskPriority::Normal);
    assert_eq!(state.tasks[0].updated_at, updated_time);

    update_queue_task_priority(
        &mut state,
        "task_1",
        crate::queue::TaskPriority::High,
        original_time,
    )
    .unwrap();
    assert_eq!(state.tasks[0].status, crate::queue::TaskStatus::Review);
    assert_eq!(state.tasks[0].priority, crate::queue::TaskPriority::High);
    assert_eq!(state.tasks[0].updated_at, original_time);
}

#[test]
fn queue_task_mutators_report_missing_task() {
    let mut state = crate::queue::QueueState { tasks: Vec::new() };
    let err = update_queue_task_status(
        &mut state,
        "missing",
        crate::queue::TaskStatus::Done,
        chrono::Utc::now(),
    )
    .expect_err("missing task");
    assert!(
        err.to_string()
            .contains("Queue task 'missing' was not found")
    );
}

fn test_route(model: &str, provider: &str, api_method: &str) -> ModelRoute {
    ModelRoute {
        model: model.to_string(),
        provider: provider.to_string(),
        api_method: api_method.to_string(),
        available: true,
        detail: String::new(),
        cheapness: None,
    }
}

#[test]
fn cli_route_display_uses_typed_api_methods() {
    assert_eq!(cli_api_method_display("openai-oauth"), "oauth");
    assert_eq!(cli_api_method_display("openai-api-key"), "api key");
    assert_eq!(
        cli_api_method_display("openai-compatible:cerebras"),
        "api key"
    );
    assert_eq!(cli_api_method_display("mock-auth:profile"), "mock-auth");
    assert_eq!(
        cli_route_provider_display("DeepSeek", "openrouter"),
        "OpenRouter/DeepSeek"
    );
}

fn test_todo(
    id: &str,
    status: &str,
    priority: &str,
    confidence: Option<u8>,
    completion_confidence: Option<u8>,
) -> crate::todo::TodoItem {
    crate::todo::TodoItem {
        id: id.to_string(),
        content: format!("todo {id}"),
        status: status.to_string(),
        priority: priority.to_string(),
        confidence,
        completion_confidence,
        ..Default::default()
    }
}

#[test]
fn run_auto_poke_followup_sends_confidence_summary_when_todos_are_done() {
    let todos = vec![
        test_todo("a", "completed", "high", Some(90), Some(90)),
        test_todo("b", "completed", "low", Some(80), Some(80)),
    ];

    let followup = build_run_auto_poke_follow_up_from_todos(&todos, false);

    match followup {
        Some(RunAutoPokeFollowUp::ConfidenceSummary {
            total_todos,
            message,
        }) => {
            assert_eq!(total_todos, 2);
            assert!(message.contains("All todos are done. Todo confidence summary:"));
            assert!(message.contains("- Weighted completion confidence: 88%."));
            assert!(message.contains("- 1 completed todo is below the 90% confidence threshold."));
        }
        _ => panic!("expected confidence-summary follow-up"),
    }
}

#[test]
fn run_auto_poke_followup_prioritizes_incomplete_todos() {
    let todos = vec![
        test_todo("a", "completed", "high", Some(95), Some(95)),
        test_todo("b", "in_progress", "medium", Some(80), None),
    ];

    let followup = build_run_auto_poke_follow_up_from_todos(&todos, false);

    match followup {
        Some(RunAutoPokeFollowUp::Incomplete { count, message }) => {
            assert_eq!(count, 1);
            assert_eq!(
                message,
                "You have 1 incomplete todo. Continue working, or update the todo tool."
            );
        }
        _ => panic!("expected incomplete-todo follow-up"),
    }
}

#[test]
fn run_auto_poke_followup_sends_confidence_summary_once() {
    let todos = vec![test_todo("a", "completed", "high", Some(95), Some(95))];

    assert!(build_run_auto_poke_follow_up_from_todos(&todos, true).is_none());
}

#[test]
fn cli_provider_choice_filter_uses_typed_api_methods() {
    let routes = vec![
        test_route("claude-opus-4-6", "Anthropic", "claude-oauth"),
        test_route("claude-opus-4-6", "Anthropic", "claude-api"),
        test_route("gpt-5.5", "OpenAI", "openai-oauth"),
        test_route("gpt-5.5", "OpenAI", "openai-api-key"),
        test_route("deepseek/deepseek-v4-pro", "auto", "openrouter"),
        test_route("grok-code-fast-1", "Copilot", "copilot"),
    ];

    let openai = filter_cli_model_routes_for_choice(
        &super::super::provider_init::ProviderChoice::Openai,
        &routes,
    );
    assert_eq!(openai.len(), 1);
    assert_eq!(
        openai[0].api_method_kind(),
        crate::provider::ModelRouteApiMethod::OpenAIOAuth
    );

    let claude = filter_cli_model_routes_for_choice(
        &super::super::provider_init::ProviderChoice::Claude,
        &routes,
    );
    assert_eq!(claude.len(), 2);
    assert!(
        claude
            .iter()
            .all(|route| route.api_method_kind().is_anthropic_credential_route())
    );
}

#[test]
fn cloud_sessions_args_match_jade_helper_contract() {
    let args = build_jade_sessions_args(CloudSessionsSubcommand::UploadLatest {
        sessions_dir: "/tmp/sessions".to_string(),
        raw: true,
        user_id: "jeremy".to_string(),
        profile: Some("test-profile".to_string()),
        region: Some("us-east-1".to_string()),
        helper: None,
    });

    assert_eq!(
        args,
        vec![
            "upload-latest",
            "--user-id",
            "jeremy",
            "--profile",
            "test-profile",
            "--region",
            "us-east-1",
            "--sessions-dir",
            "/tmp/sessions",
            "--raw",
        ]
    );

    let args = build_jade_sessions_args(CloudSessionsSubcommand::View {
        session_id: "session_123".to_string(),
        format: "html".to_string(),
        output: Some("/tmp/session.html".to_string()),
        open: true,
        user_id: "dev".to_string(),
        profile: Some("profile".to_string()),
        region: Some("region".to_string()),
        helper: None,
    });

    assert_eq!(
        args,
        vec![
            "view",
            "--user-id",
            "dev",
            "--profile",
            "profile",
            "--region",
            "region",
            "--format",
            "html",
            "--output",
            "/tmp/session.html",
            "--open",
            "session_123",
        ]
    );
}

#[test]
fn cloud_sessions_config_persists_secret_and_feeds_helper_env_without_args() {
    let _guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME", "JADE_TOKEN_FOR_TEST"]);
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());
    crate::env::set_var("JADE_TOKEN_FOR_TEST", "secret-token-value");

    run_cloud_sessions_configure(
        Some("https://jade.example".to_string()),
        None,
        Some("JADE_TOKEN_FOR_TEST".to_string()),
        Some("dev-admin".to_string()),
        Some("alice".to_string()),
        Some("/tmp/jade_sessions.py".to_string()),
        false,
    )
    .expect("configure");

    let path = cloud_sessions_config_path().expect("config path");
    assert!(path.exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(path.metadata().unwrap().permissions().mode() & 0o777, 0o600);
    }

    let config = load_cloud_sessions_config()
        .expect("load config")
        .expect("config exists");
    assert_eq!(config.api_base.as_deref(), Some("https://jade.example"));
    assert_eq!(config.api_token.as_deref(), Some("secret-token-value"));
    assert_eq!(config.api_token_id.as_deref(), Some("dev-admin"));
    assert_eq!(config.user_id.as_deref(), Some("alice"));
    assert_eq!(config.helper.as_deref(), Some("/tmp/jade_sessions.py"));

    let env = cloud_sessions_helper_env(&config);
    assert!(env.contains(&("JADE_API_BASE", "https://jade.example".to_string())));
    assert!(env.contains(&("JADE_API_TOKEN", "secret-token-value".to_string())));
    assert!(env.contains(&("JADE_API_TOKEN_ID", "dev-admin".to_string())));

    let args = build_jade_sessions_args_with_config(
        CloudSessionsSubcommand::List {
            limit: 2,
            json: true,
            user_id: "dev".to_string(),
            profile: None,
            region: None,
            helper: None,
        },
        &config,
    );
    assert_eq!(
        args,
        vec!["list", "--user-id", "alice", "--limit", "2", "--json"]
    );
    assert!(!args.iter().any(|arg| arg.contains("secret-token-value")));

    run_cloud_sessions_configure(None, None, None, None, None, None, true).expect("clear");
    assert!(!path.exists());
}

#[test]
fn is_syncable_session_stem_filters_non_session_files() {
    assert!(is_syncable_session_stem("session_abc_123"));
    assert!(is_syncable_session_stem("imported_codex_456"));
    assert!(!is_syncable_session_stem("req"));
    assert!(!is_syncable_session_stem("test_selfdev_session"));
    assert!(!is_syncable_session_stem("session_abc.journal"));
}

#[test]
fn collect_sync_candidates_picks_only_session_json() {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    std::fs::write(dir.join("session_one.json"), b"{\"id\":\"one\"}").unwrap();
    std::fs::write(dir.join("imported_codex_two.json"), b"{\"id\":\"two\"}").unwrap();
    std::fs::write(dir.join("req.json"), b"{}").unwrap();
    std::fs::write(dir.join("session_three.journal.json"), b"{}").unwrap();
    std::fs::write(dir.join("session_four.bak"), b"{}").unwrap();

    let mut ids: Vec<String> = collect_sync_candidates(dir)
        .expect("collect")
        .into_iter()
        .map(|candidate| candidate.session_id)
        .collect();
    ids.sort();
    assert_eq!(ids, vec!["imported_codex_two", "session_one"]);
}

#[test]
fn cloud_sessions_sync_dry_run_reports_without_uploading_or_writing_state() {
    let _guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME", "JCODE_JADE_SESSIONS_HELPER"]);
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());

    // A dummy helper that should never run during a dry run.
    let helper = temp.path().join("never_runs.sh");
    std::fs::write(&helper, b"#!/bin/sh\nexit 7\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    crate::env::set_var("JCODE_JADE_SESSIONS_HELPER", &helper);

    let sessions_dir = temp.path().join("sessions");
    std::fs::create_dir_all(&sessions_dir).unwrap();
    std::fs::write(sessions_dir.join("session_alpha.json"), b"{\"id\":\"a\"}").unwrap();
    std::fs::write(sessions_dir.join("session_beta.json"), b"{\"id\":\"b\"}").unwrap();

    run_cloud_sessions_sync(CloudSessionsSyncRequest {
        sessions_dir: Some(sessions_dir.display().to_string()),
        since_days: None,
        all: true,
        max: 50,
        min_interval_mins: None,
        raw: false,
        dry_run: true,
        force: false,
        json: true,
        user_id: "dev".to_string(),
        profile: None,
        region: None,
        helper: None,
    })
    .expect("dry run sync");

    // Dry run must not persist any sync state.
    assert!(!cloud_sessions_sync_state_path().unwrap().exists());
}

#[test]
fn cloud_sessions_sync_respects_min_interval_throttle() {
    let _guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&["JCODE_HOME", "JCODE_JADE_SESSIONS_HELPER"]);
    let temp = tempfile::tempdir().expect("tempdir");
    crate::env::set_var("JCODE_HOME", temp.path());

    // Helper that would fail loudly if it ever ran during a throttled run.
    let helper = temp.path().join("must_not_run.sh");
    std::fs::write(&helper, b"#!/bin/sh\nexit 13\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&helper, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    crate::env::set_var("JCODE_JADE_SESSIONS_HELPER", &helper);

    let sessions_dir = temp.path().join("sessions");
    std::fs::create_dir_all(&sessions_dir).unwrap();
    std::fs::write(sessions_dir.join("session_gamma.json"), b"{\"id\":\"g\"}").unwrap();

    // Seed sync state with a very recent last_sync_at so throttle should trigger.
    let state = CloudSessionsSyncState {
        last_sync_at: Some(chrono::Utc::now().to_rfc3339()),
        ..Default::default()
    };
    save_cloud_sessions_sync_state(&state).expect("seed state");

    // Should be skipped (not error) because last sync was just now.
    run_cloud_sessions_sync(CloudSessionsSyncRequest {
        sessions_dir: Some(sessions_dir.display().to_string()),
        since_days: None,
        all: true,
        max: 50,
        min_interval_mins: Some(60),
        raw: false,
        dry_run: false,
        force: false,
        json: true,
        user_id: "dev".to_string(),
        profile: None,
        region: None,
        helper: None,
    })
    .expect("throttled sync returns ok without running helper");

    // The session should NOT be recorded as uploaded.
    let reloaded = load_cloud_sessions_sync_state().expect("reload state");
    assert!(!reloaded.sessions.contains_key("session_gamma"));
}

#[test]
fn render_cloud_sessions_dashboard_html_escapes_and_lists_rows() {
    let items: Vec<CloudSessionListItem> = serde_json::from_str(
        r#"[
          {"session_id":"session_x","title":"Hello <b> & \"world\"","message_count":12,"uploaded_at":"2026-05-29T00:00:00Z"},
          {"session_id":"session_y","short_name":"shorty","message_count":"3","uploaded_at":"2026-05-28T00:00:00Z"}
        ]"#,
    )
    .expect("parse items");

    let html =
        render_cloud_sessions_dashboard_html("alice", &items, &std::collections::BTreeMap::new());
    assert!(html.contains("Jade Cloud Sessions"));
    assert!(html.contains("user: alice"));
    assert!(html.contains("2 session(s)"));
    assert!(html.contains("session_x"));
    assert!(html.contains("shorty"));
    // Raw title must be escaped (no live markup, quotes escaped).
    assert!(!html.contains("Hello <b>"));
    assert!(html.contains("Hello &lt;b&gt; &amp; &quot;world&quot;"));
    // Numeric and string message counts both render.
    assert!(html.contains(">12<"));
    assert!(html.contains(">3<"));
}

#[test]
fn render_cloud_sessions_dashboard_html_handles_empty() {
    let html = render_cloud_sessions_dashboard_html("dev", &[], &std::collections::BTreeMap::new());
    assert!(html.contains("0 session(s)"));
    assert!(html.contains("No uploaded sessions found."));
}

#[test]
fn render_cloud_sessions_dashboard_html_links_rows_with_view_files() {
    let items: Vec<CloudSessionListItem> = serde_json::from_str(
        r#"[
          {"session_id":"session_x","title":"X","message_count":1,"uploaded_at":"2026-05-29T00:00:00Z"},
          {"session_id":"session_y","title":"Y","message_count":2,"uploaded_at":"2026-05-28T00:00:00Z"}
        ]"#,
    )
    .expect("parse items");
    let mut links = std::collections::BTreeMap::new();
    links.insert(
        "session_x".to_string(),
        "dash-views/session_x.html".to_string(),
    );

    let html = render_cloud_sessions_dashboard_html("alice", &items, &links);
    // Linked session gets an anchor to its relative viewer file.
    assert!(html.contains("<a href='dash-views/session_x.html'>session_x</a>"));
    // Session without a generated viewer stays plain text (no anchor).
    assert!(html.contains("<td class='id'>session_y</td>"));
}

#[test]
fn sanitize_filename_keeps_safe_chars_and_replaces_others() {
    assert_eq!(
        sanitize_filename("session_abc-123.json"),
        "session_abc-123.json"
    );
    assert_eq!(sanitize_filename("a/b c:d"), "a_b_c_d");
}

#[test]
fn dashboard_views_dir_is_sibling_of_dashboard() {
    let dir = dashboard_views_dir(std::path::Path::new("/tmp/out/dash.html"));
    assert_eq!(dir, std::path::PathBuf::from("/tmp/out/dash-views"));
}

#[test]
fn relative_link_is_relative_to_dashboard_parent() {
    let link = relative_link(
        std::path::Path::new("/tmp/out/dash.html"),
        std::path::Path::new("/tmp/out/dash-views/session_x.html"),
    );
    assert_eq!(link.as_deref(), Some("dash-views/session_x.html"));
}

#[test]
fn parse_cloud_session_list_json_accepts_array_and_object_wrappers() {
    // Real helper shape: a top-level array.
    let array = parse_cloud_session_list_json(
        r#"[{"session_id":"session_a","message_count":2,"uploaded_at":"2026-05-29T00:00:00Z"}]"#,
    )
    .expect("parse array");
    assert_eq!(array.len(), 1);
    assert_eq!(array[0].session_id.as_deref(), Some("session_a"));

    // Tolerated object wrappers.
    let items = parse_cloud_session_list_json(r#"{"items":[{"session_id":"session_b"}]}"#)
        .expect("parse items wrapper");
    assert_eq!(items[0].session_id.as_deref(), Some("session_b"));

    let sessions = parse_cloud_session_list_json(r#"{"sessions":[{"session_id":"session_c"}]}"#)
        .expect("parse sessions wrapper");
    assert_eq!(sessions[0].session_id.as_deref(), Some("session_c"));

    // Empty array stays empty.
    assert!(
        parse_cloud_session_list_json("[]")
            .expect("parse empty")
            .is_empty()
    );
}

#[test]
fn parse_cloud_session_list_json_rejects_unexpected_shapes() {
    // A bare object without a recognized array key is an error.
    let err = parse_cloud_session_list_json(r#"{"unexpected":true}"#)
        .expect_err("object without items/sessions");
    assert!(err.to_string().contains("items"));

    // A scalar is also rejected with a descriptive message.
    let err = parse_cloud_session_list_json("42").expect_err("scalar");
    assert!(err.to_string().contains("a number"));
}

#[test]
fn resolve_jade_sessions_helper_prefers_explicit_and_env_paths() {
    let _saved = SavedEnv::capture(&["JCODE_JADE_SESSIONS_HELPER"]);
    crate::env::set_var("JCODE_JADE_SESSIONS_HELPER", "/tmp/from-env.py");

    assert_eq!(
        resolve_jade_sessions_helper(Some("/tmp/explicit.py")).unwrap(),
        std::path::PathBuf::from("/tmp/explicit.py")
    );
    assert_eq!(
        resolve_jade_sessions_helper(None).unwrap(),
        std::path::PathBuf::from("/tmp/from-env.py")
    );
}

#[test]
fn auth_test_retryable_error_detection_handles_rate_limits() {
    let err = anyhow::anyhow!(
        "Gemini request generateContent failed (HTTP 429 Too Many Requests): RESOURCE_EXHAUSTED"
    );
    assert!(auth_test_error_is_retryable(&err));
}

#[test]
fn auth_test_retryable_error_detection_rejects_schema_errors() {
    let err = anyhow::anyhow!(
        "Gemini request generateContent failed (HTTP 400 Bad Request): invalid argument"
    );
    assert!(!auth_test_error_is_retryable(&err));
}

#[tokio::test]
async fn auth_test_choice_plan_preserves_explicit_model_for_local_provider() {
    let plan = auth_test_choice_plan(
        &super::super::provider_init::ProviderChoice::Ollama,
        Some("llama3.2"),
    )
    .await
    .expect("choice plan");

    match plan {
        AuthTestChoicePlan::Run { model } => assert_eq!(model.as_deref(), Some("llama3.2")),
        AuthTestChoicePlan::Skip(detail) => panic!("unexpected skip: {detail}"),
    }
}

#[tokio::test]
async fn auth_test_choice_plan_leaves_non_compat_provider_unchanged() {
    let plan = auth_test_choice_plan(
        &super::super::provider_init::ProviderChoice::Openrouter,
        None,
    )
    .await
    .expect("choice plan");

    match plan {
        AuthTestChoicePlan::Run { model } => assert!(model.is_none()),
        AuthTestChoicePlan::Skip(detail) => panic!("unexpected skip: {detail}"),
    }
}

#[tokio::test]
async fn auth_test_choice_plan_discovers_model_for_local_custom_compat_endpoint() {
    let _env_guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&[
        "JCODE_OPENAI_COMPAT_API_BASE",
        "JCODE_OPENAI_COMPAT_API_KEY_NAME",
        "JCODE_OPENAI_COMPAT_ENV_FILE",
        "JCODE_OPENAI_COMPAT_DEFAULT_MODEL",
        "JCODE_OPENAI_COMPAT_LOCAL_ENABLED",
        "JCODE_OPENROUTER_API_BASE",
        "JCODE_OPENROUTER_API_KEY_NAME",
        "JCODE_OPENROUTER_ENV_FILE",
        "JCODE_OPENROUTER_ALLOW_NO_AUTH",
    ]);
    let api_base = spawn_single_response_http_server(200, r#"{"data":[{"id":"llama3.2"}]}"#);
    crate::env::set_var("JCODE_OPENAI_COMPAT_API_BASE", &api_base);
    crate::env::remove_var("JCODE_OPENAI_COMPAT_DEFAULT_MODEL");
    crate::env::remove_var("JCODE_OPENAI_COMPAT_LOCAL_ENABLED");
    crate::provider_catalog::apply_openai_compatible_profile_env(None);

    let plan = auth_test_choice_plan(
        &super::super::provider_init::ProviderChoice::OpenaiCompatible,
        None,
    )
    .await
    .expect("choice plan");

    match plan {
        AuthTestChoicePlan::Run { model } => assert_eq!(model.as_deref(), Some("llama3.2")),
        AuthTestChoicePlan::Skip(detail) => panic!("unexpected skip: {detail}"),
    }
}

#[tokio::test]
async fn auth_test_choice_plan_discovers_model_for_hosted_custom_compat_endpoint_with_api_key() {
    let _env_guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&[
        "JCODE_OPENAI_COMPAT_API_BASE",
        "JCODE_OPENAI_COMPAT_API_KEY_NAME",
        "JCODE_OPENAI_COMPAT_ENV_FILE",
        "JCODE_OPENAI_COMPAT_DEFAULT_MODEL",
        "JCODE_OPENAI_COMPAT_LOCAL_ENABLED",
        "JCODE_OPENROUTER_API_BASE",
        "JCODE_OPENROUTER_API_KEY_NAME",
        "JCODE_OPENROUTER_ENV_FILE",
        "JCODE_OPENROUTER_ALLOW_NO_AUTH",
        "OPENAI_COMPAT_API_KEY",
        "NO_PROXY",
        "no_proxy",
    ]);
    // 0.0.0.0 is accepted as an insecure HTTP test host but is not treated as
    // localhost by resolve_openai_compatible_profile, so this exercises the
    // hosted/API-key code path while still serving the response locally.
    let api_base = spawn_single_response_http_server_on_host(
        "0.0.0.0",
        200,
        r#"{"data":[{"id":"hosted-compatible-model"}]}"#,
    );
    crate::env::set_var("JCODE_OPENAI_COMPAT_API_BASE", &api_base);
    crate::env::set_var("OPENAI_COMPAT_API_KEY", "test-key");
    crate::env::set_var("NO_PROXY", "0.0.0.0,127.0.0.1,localhost");
    crate::env::set_var("no_proxy", "0.0.0.0,127.0.0.1,localhost");
    crate::env::remove_var("JCODE_OPENAI_COMPAT_DEFAULT_MODEL");
    crate::env::remove_var("JCODE_OPENAI_COMPAT_LOCAL_ENABLED");
    crate::provider_catalog::apply_openai_compatible_profile_env(None);

    let resolved = crate::provider_catalog::resolve_openai_compatible_profile(
        crate::provider_catalog::OPENAI_COMPAT_PROFILE,
    );
    assert!(resolved.requires_api_key);

    let plan = auth_test_choice_plan(
        &super::super::provider_init::ProviderChoice::OpenaiCompatible,
        None,
    )
    .await
    .expect("choice plan");

    match plan {
        AuthTestChoicePlan::Run { model } => {
            assert_eq!(model.as_deref(), Some("hosted-compatible-model"))
        }
        AuthTestChoicePlan::Skip(detail) => panic!("unexpected skip: {detail}"),
    }
}

#[tokio::test]
async fn auth_test_choice_plan_skips_local_custom_compat_endpoint_without_models() {
    let _env_guard = crate::storage::lock_test_env();
    let _saved = SavedEnv::capture(&[
        "JCODE_OPENAI_COMPAT_API_BASE",
        "JCODE_OPENAI_COMPAT_API_KEY_NAME",
        "JCODE_OPENAI_COMPAT_ENV_FILE",
        "JCODE_OPENAI_COMPAT_DEFAULT_MODEL",
        "JCODE_OPENAI_COMPAT_LOCAL_ENABLED",
        "JCODE_OPENROUTER_API_BASE",
        "JCODE_OPENROUTER_API_KEY_NAME",
        "JCODE_OPENROUTER_ENV_FILE",
        "JCODE_OPENROUTER_ALLOW_NO_AUTH",
    ]);
    let api_base = spawn_single_response_http_server(200, r#"{"data":[]}"#);
    crate::env::set_var("JCODE_OPENAI_COMPAT_API_BASE", &api_base);
    crate::env::remove_var("JCODE_OPENAI_COMPAT_DEFAULT_MODEL");
    crate::env::remove_var("JCODE_OPENAI_COMPAT_LOCAL_ENABLED");
    crate::provider_catalog::apply_openai_compatible_profile_env(None);

    let plan = auth_test_choice_plan(
        &super::super::provider_init::ProviderChoice::OpenaiCompatible,
        None,
    )
    .await
    .expect("choice plan");

    match plan {
        AuthTestChoicePlan::Run { model } => panic!("unexpected run plan: {model:?}"),
        AuthTestChoicePlan::Skip(detail) => {
            assert!(detail.contains("reported no models"));
            assert!(detail.contains("openai-compatible"));
        }
    }
}

#[test]
fn collect_cli_model_names_falls_back_when_no_routes_are_available() {
    let routes = vec![ModelRoute {
        model: "claude-opus-4-6".to_string(),
        provider: "Anthropic".to_string(),
        api_method: "claude-oauth".to_string(),
        available: false,
        detail: "no credentials".to_string(),
        cheapness: None,
    }];

    let models = collect_cli_model_names(&routes, vec!["gpt-5.4".to_string()]);

    assert_eq!(models, vec!["claude-opus-4-6", "gpt-5.4"]);
}

#[test]
fn list_cli_providers_includes_auto_and_openai() {
    let providers = super::report_info::list_cli_providers();
    assert!(providers.iter().any(|provider| provider.id == "auto"));
    assert!(providers.iter().any(|provider| {
        provider.id == "openai"
            && provider.display_name == "OpenAI"
            && provider.auth_kind.as_deref() == Some("OAuth")
    }));
    assert!(providers.iter().any(|provider| provider.id == "groq"));
    assert!(providers.iter().any(|provider| provider.id == "xai"));
}

#[test]
fn version_command_plain_output_includes_core_fields() {
    let report = super::report_info::VersionReport {
        version: "v1.2.3 (abc1234)".to_string(),
        semver: "1.2.3".to_string(),
        base_semver: "1.2.0".to_string(),
        update_semver: "1.2.0".to_string(),
        git_hash: "abc1234".to_string(),
        git_tag: "v1.2.3".to_string(),
        build_time: "2026-03-18 18:00:00 +0000".to_string(),
        git_date: "2026-03-18 17:59:00 +0000".to_string(),
        release_build: false,
    };
    let text = format!(
        "version\t{}\nsemver\t{}\nbase_semver\t{}\nupdate_semver\t{}\ngit_hash\t{}\ngit_tag\t{}\nbuild_time\t{}\ngit_date\t{}\nrelease_build\t{}\n",
        report.version,
        report.semver,
        report.base_semver,
        report.update_semver,
        report.git_hash,
        report.git_tag,
        report.build_time,
        report.git_date,
        report.release_build
    );

    assert!(text.contains("version\tv1.2.3 (abc1234)"));
    assert!(text.contains("semver\t1.2.3"));
    assert!(text.contains("git_hash\tabc1234"));
    assert!(text.contains("release_build\tfalse"));
}

#[tokio::test]
async fn restore_agent_session_if_requested_restores_resumed_session() {
    let _guard = crate::storage::lock_test_env();

    let provider: Arc<dyn Provider> = Arc::new(TestProvider);
    let registry = Registry::new(provider.clone()).await;
    let mut original = crate::agent::Agent::new(provider.clone(), registry);
    let original_session_id = original.session_id().to_string();
    original
        .run_once_capture("seed session for resume test")
        .await
        .expect("seed session");

    let registry = Registry::new(provider.clone()).await;
    let mut resumed = crate::agent::Agent::new(provider, registry);
    let fresh_session_id = resumed.session_id().to_string();
    assert_ne!(fresh_session_id, original_session_id);

    restore_agent_session_if_requested(&mut resumed, Some(&original_session_id))
        .expect("restore session");

    assert_eq!(resumed.session_id(), original_session_id);
}
