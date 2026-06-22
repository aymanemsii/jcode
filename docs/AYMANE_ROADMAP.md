# Aymane's jcode Fork Roadmap

Goal:
Turn jcode into a personal AI workday manager.

Core direction:
I want jcode to become a tool where I can give it tasks, assign/customize agents per project, see active agents, and review work through a queue/kanban-style TUI.

Planned features:
1. Project-local custom workers - CLI MVP implemented
2. Queue mode - Phase 2 background runner implemented
3. Kanban-style TUI view
4. Active agents panel
5. Review inbox
6. Sequential task runner - CLI foreground/background control loop implemented
7. Parallel task execution later

## Queue Mode Phase 2 Status

Implemented:
- Project-local `.jcode/` queue storage.
- `queue init`.
- Worker profiles in `.jcode/workers.toml`.
- `queue run-next --worker-profile <name> --dry-run`.
- `queue run-next --worker-profile <name> --execute`.
- `queue run-next --worker-profile <name> --background`.
- RunState and RunIndex under `.jcode/queue/runs/`, including `.jcode/queue/runs/index.json`.
- `queue active`, `queue run-status <run-id>`, `queue logs <run-id>`, `queue refresh-runs`, and `queue cancel-run <run-id>`.
- Review workflow with `queue review`, `queue approve`, and `queue reopen`.
- Dashboard workflow with `queue dashboard`.

Background-run workflow:
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

Current limitations:
- No daemon.
- No automatic refresh; `queue refresh-runs` is manual.
- No parallel/swarm scheduler.
- No TUI/Kanban yet.

Windows notes:
- `queue logs` may display lossy characters for non-UTF-8 command output.
- `queue cancel-run` uses forced process-tree termination on Windows.

Important rule:
Do not rush into source-code changes. First understand the architecture, then add features in small safe steps.
