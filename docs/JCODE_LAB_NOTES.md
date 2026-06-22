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

Minimal background workflow:
```bash
jcode queue init
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
