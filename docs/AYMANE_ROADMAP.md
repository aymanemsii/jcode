# Aymane's jcode Fork Roadmap

Goal:
Turn jcode into a personal AI workday manager.

Core direction:
I want jcode to become a tool where I can give it tasks, assign/customize agents per project, see active agents, and review work through a queue/kanban-style TUI.

Planned features:
1. Project-local custom workers - CLI MVP implemented
2. Queue mode - Phase 2 background runner implemented
3. Kanban-style TUI view - standalone board implemented
4. Active agents panel
5. Review inbox
6. Sequential task runner - CLI foreground/background control loop implemented
7. Parallel task execution later

## Queue Mode Standalone TUI/Kanban Status

Implemented:
- Project-local `.jcode/` queue storage.
- `queue init`.
- Worker profiles in `.jcode/workers.toml`.
- Handoffs.
- `queue run-next --worker-profile <name> --dry-run`.
- `queue run-next --worker-profile <name> --execute`.
- `queue run-next --worker-profile <name> --background`.
- RunState and RunIndex under `.jcode/queue/runs/`, including `.jcode/queue/runs/index.json`.
- `queue active`, `queue run-status <run-id>`, `queue logs <run-id>`, `queue refresh-runs`, and `queue cancel-run <run-id>`.
- Review workflow with `queue review`, `queue approve`, and `queue reopen`.
- Dashboard workflow with `queue dashboard`.
- CLI board with `jcode queue board`.
- Standalone TUI board with `jcode queue board --tui`.

TUI/Kanban capabilities:
- Columns: `backlog`, `ready`, `running`, `review`, `blocked`, `done`, `cancelled`.
- 2D navigation with `Left`/`Right`, `Up`/`Down`, and `j`/`k`.
- `n` creates a task and asks for title and worker profile.
- `x` runs the selected actionable task in the background.
- `r` manually refreshes board and run status.
- Auto-refresh updates running tasks while the board is open.
- `a` approves the selected review task.
- `q` quits.

Safe TUI workflow:
```bash
jcode queue board --tui
```

Then:
1. Press `n`.
2. Enter task title.
3. Enter worker profile, such as `planner`.
4. Select the task.
5. Press `x` to run it in the background.
6. Wait for auto-refresh to move it to review.
7. Press `a` to approve it to done.

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
- `jcode queue board`
- `jcode queue board --tui`

Current limitations:
- No integration into the main `jcode` interactive app yet.
- No drag-and-drop.
- No edit task action.
- No selected-task logs/details panel yet.
- No daemon.
- No automatic task scheduler.
- No parallel/swarm scheduler.

Next planned phase:
- Inspect and map the main `jcode` interactive TUI integration path.
- Integrate Queue Board into the main `jcode` terminal app safely.

Windows notes:
- `queue logs` may display lossy characters for non-UTF-8 command output.
- `queue cancel-run` uses forced process-tree termination on Windows.

Important rule:
Do not rush into source-code changes. First understand the architecture, then add features in small safe steps.
