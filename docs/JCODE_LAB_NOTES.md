# jcode Lab Notes

Setup notes:
- Fork cloned successfully.
- origin points to my fork.
- upstream points to original jcode repo.
- Release build works.
- Debug help command caused a stack overflow on Windows, so for now use target/release/jcode.exe.
- cargo test appeared to hang on some provider/auth/Ollama-related tests, so use cargo check as the basic setup check for now.

Current safe command:
cargo build --release

Current usable binary:
.\target\release\jcode.exe

## Queue Mode Phase 2 Background Runner

Status:
- Queue Mode now has project-local `.jcode/` storage, worker profiles in `.jcode/workers.toml`, foreground execution, and background execution.
- Phase 2 background runs start with `queue run-next --worker-profile <name> --background`.
- RunState and RunIndex are stored under `.jcode/queue/runs/`, including `.jcode/queue/runs/index.json`.
- Background runs write stdout/stderr to run files and write `exit_code.txt` as the completion marker.
- `queue active`, `queue run-status <run-id>`, `queue logs <run-id>`, `queue refresh-runs`, and `queue cancel-run <run-id>` are available.
- Review flow remains `queue review`, `queue approve`, and `queue reopen`.
- Dashboard flow remains `queue dashboard`.

Current Queue Mode commands:
- `jcode queue init`
- `jcode queue add`
- `jcode queue list`
- `jcode queue status`
- `jcode queue show`
- `jcode queue set-status`
- `jcode queue set-priority`
- `jcode queue next`
- `jcode queue start-next`
- `jcode queue finish`
- `jcode queue workers`
- `jcode queue worker`
- `jcode queue handoff`
- `jcode queue handoff-next`
- `jcode queue run-next --worker-profile <name> --dry-run`
- `jcode queue run-next --worker-profile <name> --execute`
- `jcode queue run-next --worker-profile <name> --background`
- `jcode queue runs`
- `jcode queue run`
- `jcode queue active`
- `jcode queue run-status <run-id>`
- `jcode queue logs <run-id>`
- `jcode queue refresh-runs`
- `jcode queue cancel-run <run-id>`
- `jcode queue review`
- `jcode queue approve`
- `jcode queue reopen`
- `jcode queue dashboard`

Minimal background workflow:
```bash
jcode queue init
```

Configure `.jcode/workers.toml`:
```toml
[workers.smoke]
description = "Safe smoke-test worker"
command = "echo smoke worker ran task=<task_id> handoff=<handoff_file>"
```

```bash
jcode queue add "Smoke test background queue" --worker-profile smoke
jcode queue run-next --worker-profile smoke --dry-run
jcode queue run-next --worker-profile smoke --background
jcode queue active
jcode queue logs <run-id>
jcode queue refresh-runs
jcode queue review
jcode queue approve <task-id>
```

Cancellation workflow:
```bash
jcode queue run-next --worker-profile smoke --background
jcode queue active
jcode queue cancel-run <run-id>
jcode queue run-status <run-id>
jcode queue dashboard
```

Current limitations:
- No daemon.
- No automatic refresh; run `queue refresh-runs` manually.
- No parallel/swarm scheduler.
- No TUI/Kanban yet.

Windows notes:
- `queue logs` may display lossy characters for non-UTF-8 command output.
- `queue cancel-run` uses forced process-tree termination on Windows.
