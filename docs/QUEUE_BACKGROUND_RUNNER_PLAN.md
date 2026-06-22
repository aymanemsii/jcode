# Queue Background Runner / Active Agent Mode

This document records the completed Phase 2 background-runner checkpoint for Queue Mode.

Queue Mode can now start worker commands in the background, record durable run state, inspect active and historical runs, show logs, reconcile completed processes, and cancel running background runs.

## Phase 2 Checkpoint Summary

Implemented:

- Project-local `.jcode/` queue storage.
- `queue init`.
- Worker profiles in `.jcode/workers.toml`.
- `queue run-next --worker-profile <name> --dry-run`.
- `queue run-next --worker-profile <name> --execute`.
- `queue run-next --worker-profile <name> --background`.
- RunState and RunIndex stored under `.jcode/queue/runs/`, including `.jcode/queue/runs/index.json`.
- `queue active`.
- `queue run-status <run-id>`.
- `queue logs <run-id>`.
- `queue refresh-runs`.
- `queue cancel-run <run-id>`.
- Review workflow with `queue review`, `queue approve`, and `queue reopen`.
- Dashboard workflow with `queue dashboard`.

Known behavior:

- Background runs start in `running`.
- Background runs write stdout and stderr to run files.
- Background runs write `exit_code.txt` as the completion marker.
- `queue refresh-runs` updates completed background runs: exit code `0` moves the task to `review`; non-zero exits move the task to `blocked`.
- `queue cancel-run` cancels running background runs.
- On Windows, cancellation uses forced process-tree termination.
- `queue logs` handles non-UTF-8 log bytes safely using lossy display.

Current limitations:

- No daemon.
- No automatic polling or refresh.
- No parallel/swarm scheduler.
- No TUI.
- `queue refresh-runs` is manual.

## Current Command List

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

## Storage Shape

```text
.jcode/
  workers.toml
  queue/
    queue.json
    handoffs/
      <task-id>.md
    runs/
      index.json
      <task-id>/
        <timestamp>/
          run.json
          command.txt
          stdout.log
          stderr.log
          exit_code.txt
```

## Minimal Background Workflow

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

## Cancellation Workflow

```bash
jcode queue run-next --worker-profile smoke --background
jcode queue active
jcode queue cancel-run <run-id>
jcode queue run-status <run-id>
jcode queue dashboard
```

Cancelled tasks move to `blocked`.

## Task Status Transitions

- Background start: `backlog` or `ready` -> `running`.
- `refresh-runs` with exit code `0`: `running` -> `review`.
- `refresh-runs` with non-zero exit: `running` -> `blocked`.
- Cancel: `running` -> `blocked`.

Successful worker execution still requires human review; it does not move directly to `done`.

## Windows Notes

- Worker command execution follows local shell semantics.
- Background logs are redirected to files from process start.
- `queue logs` may display lossy replacement characters for non-UTF-8 command output.
- `queue cancel-run` uses forced process-tree termination on Windows.
- Paths in run metadata should not assume Unix separators.

## Remaining Roadmap Boundary

Queue Mode still does not include a daemon, automatic polling, parallel/swarm scheduling, or a TUI. Those remain outside this checkpoint.
