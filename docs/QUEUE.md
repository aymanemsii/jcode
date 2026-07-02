# Queue v1

Queue v1 is a small, project-local task queue for recording work items and running one explicit task at a time from the CLI. It is intended for manual operator-driven workflows, not background automation.

Queue v1 stores queue state in the current project and keeps execution visible in the current terminal. It does not claim tasks, run workers in the background, schedule retries, or coordinate multiple agents.

## Storage

Queue task storage lives in the project:

```text
./.jcode/queue/tasks.json
```

Run metadata lives next to the queue:

```text
./.jcode/queue/runs/<task-id>/<run-id>.json
```

`tasks.json` is the compact task index. It contains task metadata such as id, title, body, status, priority, timestamps, archived state, and optional `worker_profile`.

Run metadata JSON records each manual run attempt. It includes the run id, task id, started time, optional finished time, final run status, and an optional error summary.

## Lifecycle

The normal Queue v1 lifecycle is:

```text
ready -> running -> done
ready -> running -> failed
```

If a manual run is interrupted by process kill, terminal closure, crash, power loss, or another path that prevents cleanup, the task can be left in `running`. Recover it with:

```text
jcode queue reset-running <id>
```

That moves the task back through:

```text
running -> ready
```

`reset-running` is for interrupted manual runs. It only resets non-archived tasks that are currently `running`.

## Commands

Initialize queue storage for the current project:

```text
jcode queue init
```

Add a ready task:

```text
jcode queue add "Task title"
```

Add task details:

```text
jcode queue add "Task title" --body "Details"
```

Set a priority:

```text
jcode queue add "Task title" --priority high
```

Set a worker profile hint:

```text
jcode queue add "Task title" --worker-profile manual
```

List active, non-archived tasks:

```text
jcode queue list
```

List all tasks, including archived tasks:

```text
jcode queue list --all
```

Show the next active ready task:

```text
jcode queue next
```

`queue next` is read-only. It does not claim, lock, update, or run the task.

Show one task:

```text
jcode queue show <id>
```

Edit task metadata:

```text
jcode queue edit <id> --title "New title"
jcode queue edit <id> --body "New details"
jcode queue edit <id> --priority high
jcode queue edit <id> --worker-profile manual
jcode queue edit <id> --clear-worker-profile
```

Manually set task status:

```text
jcode queue status <id> ready
jcode queue status <id> running
jcode queue status <id> done
jcode queue status <id> failed
```

Archive a task so it is hidden from the default list:

```text
jcode queue archive <id>
```

Run one ready task:

```text
jcode queue run <id>
```

`queue run <id>` is manual, foreground-only, explicit-id-only, and current-process. It runs in the terminal where it was invoked, updates the selected ready task to `running`, then updates it to `done` or `failed` when the command returns.

List recorded run metadata for a task:

```text
jcode queue runs <id>
```

Recover an interrupted running task:

```text
jcode queue reset-running <id>
```

## Worker Profile

`worker_profile` is metadata only in Queue v1.

It can be set on a task and is included for visibility, but it does not choose provider, model, tools, permissions, worker class, or multi-agent behavior yet.

## Happy Path

```text
jcode queue init
jcode queue add "Update README" --body "Document the new CLI option." --priority high
jcode queue next
jcode queue run <id-from-next>
jcode queue runs <id-from-next>
jcode queue show <id-from-next>
```

Expected flow:

1. `init` creates `./.jcode/queue/tasks.json` if it does not already exist.
2. `add` creates a `ready` task.
3. `next` displays the next active ready task without changing it.
4. `run` executes that explicit task in the foreground.
5. `runs` shows run metadata under `./.jcode/queue/runs/<task-id>/`.
6. `show` displays the task with its updated status.

## Recovery Example

If a task is stuck in `running` after an interrupted manual run:

```text
jcode queue show <id>
jcode queue reset-running <id>
jcode queue run <id>
```

Expected flow:

1. `show` confirms the task is still `running`.
2. `reset-running` moves it back to `ready`.
3. `run` starts a new foreground run for the same explicit task id.

Existing run metadata is preserved when a running task is reset.

## Deferred / Not In Queue v1

Queue v1 intentionally does not include:

* `queue run-next`
* claiming
* locks
* background workers
* daemon behavior
* Queue Board/TUI
* server protocol changes
* scheduling
* retries
* parallel execution
* `worker_profile` provider/model/tool mapping
* multi-agent execution
