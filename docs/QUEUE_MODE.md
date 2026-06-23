# Queue Mode CLI

Queue Mode is a project-local task queue for handing work to command-line workers. It stores tasks in the current repository, renders each selected task into a Markdown handoff, and can run a configured worker command against that handoff.

Queue Mode is CLI-first and now includes a standalone TUI/Kanban board with `jcode queue board --tui`. It does not start a daemon, schedule tasks automatically, or run tasks in parallel.

## Standalone TUI/Kanban Checkpoint

Queue Mode now has a working standalone Kanban board launched with:

```bash
jcode queue board --tui
```

The board uses project-local `.jcode/` queue storage, worker profiles from `.jcode/workers.toml`, handoffs, run logs, and the run index/state. It displays `backlog`, `ready`, `running`, `review`, `blocked`, `done`, and `cancelled` columns, supports 2D navigation, can create tasks, start actionable tasks in the background, refresh run status, and approve review tasks to `done`.

TUI controls:

- `Left`/`Right`: move between columns.
- `Up`/`Down` or `j`/`k`: move within a column.
- `n`: create a new task; the flow asks for title and worker profile.
- `x`: run the selected actionable task in the background.
- `r`: manually refresh queue and run status.
- `a`: approve the selected review task.
- `q`: quit.

End-to-end TUI workflow:

1. Open the board with `jcode queue board --tui`.
2. Press `n`.
3. Enter a task title.
4. Enter a worker profile, such as `planner`.
5. Select the task.
6. Press `x` to run it in the background.
7. Wait for auto-refresh to move it to `review`.
8. Press `a` to approve it to `done`.

Current Queue Mode also supports CLI dry-run execution, synchronous execution, background execution, active runs, run status, logs, refresh-runs, cancel-run, review/approve/reopen, dashboard, and the CLI board with `jcode queue board`.

Current limitations:

- No integration into the main `jcode` interactive app yet.
- No drag-and-drop.
- No edit task action.
- No selected-task logs/details panel yet.
- No daemon.
- No automatic task scheduler.
- No parallel/swarm scheduler.

Next planned phase:

1. Inspect and map the main `jcode` interactive TUI integration path.
2. Integrate Queue Board into the main `jcode` terminal app safely.

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
- `jcode queue board`
- `jcode queue board --tui`

## Why It Exists

Queue Mode gives agent-heavy development a simple control loop:

1. Capture work as explicit tasks.
2. Assign tasks to worker profiles such as `coder`, `reviewer`, or `researcher`.
3. Preview the exact worker command before it runs.
4. Execute one worker task in the foreground or background.
5. Inspect run artifacts.
6. Approve finished work or reopen it for another pass.

This keeps queue state, worker configuration, generated handoffs, and run logs inside the project instead of in global agent state.

## Project-Local Storage

`jcode queue init` creates the Queue Mode files under the current working directory:

```text
.jcode/
  workers.toml
  queue/
    queue.json
    handoffs/
    runs/
      index.json
```

- `.jcode/workers.toml` defines project-local worker profiles and the command each worker runs.
- `.jcode/queue/queue.json` stores queue tasks, statuses, priorities, worker assignments, and timestamps.
- `.jcode/queue/handoffs/` stores generated Markdown handoff files such as `.jcode/queue/handoffs/<task-id>.md`.
- `.jcode/queue/runs/` stores run artifacts under `.jcode/queue/runs/<task-id>/<timestamp>/`.
- `.jcode/queue/runs/index.json` stores the RunIndex used to find runs by run ID.

Queue Mode resolves these paths relative to the directory where you run `jcode queue ...`.

## Quickstart

Initialize Queue Mode in a project:

```bash
jcode queue init
```

Edit `.jcode/workers.toml` and configure the workers you want to use:

```toml
[workers.coder]
description = "Implements code changes from queue handoffs"
command = "codex exec <handoff_file>"

[workers.smoke]
description = "Safe smoke-test worker"
command = "echo smoke worker ran task=<task_id> handoff=<handoff_file>"
```

Add tasks:

```bash
jcode queue add "Document Queue Mode" \
  --description "Write practical CLI docs and examples" \
  --priority high \
  --worker-profile coder \
  --output-path docs/QUEUE_MODE.md
```

Check the dashboard:

```bash
jcode queue dashboard
```

Preview the next task for a worker:

```bash
jcode queue next --worker-profile coder
```

Dry-run the worker command before executing it:

```bash
jcode queue run-next --worker-profile coder --dry-run
```

Execute the worker command synchronously in the foreground:

```bash
jcode queue run-next --worker-profile coder --execute
```

Start the worker command in the background:

```bash
jcode queue run-next --worker-profile coder --background
```

Inspect background activity and logs:

```bash
jcode queue active
jcode queue run-status <run-id>
jcode queue logs <run-id>
jcode queue refresh-runs
```

Inspect recorded runs:

```bash
jcode queue runs
```

Review completed worker tasks:

```bash
jcode queue review
```

Approve a task after review, or reopen it for another worker pass:

```bash
jcode queue approve <task-id>
jcode queue reopen <task-id>
```

## Task Statuses

- `backlog`: newly added work. This is the default status for `queue add`.
- `ready`: work that has been promoted for execution. `next`, `start-next`, and `run-next` consider `backlog` and `ready` tasks actionable.
- `running`: work currently claimed by a worker. `start-next` and `run-next --execute` move tasks here before execution.
- `review`: worker execution completed successfully and the task is waiting for human review.
- `done`: accepted work. `approve` moves review tasks here, and `finish --done` can mark a running task done directly.
- `blocked`: work that could not complete. A failing `run-next --execute` moves the task here.
- `cancelled`: work that should not be selected again.

Use `set-status` when you need to repair or manually move a task:

```bash
jcode queue set-status <task-id> ready
```

## Worker Profiles

Workers live in `.jcode/workers.toml`:

```toml
[workers.coder]
description = "Implements code changes from queue handoffs"
command = "codex exec <handoff_file>"

[workers.reviewer]
description = "Reviews task outputs and suggests fixes"
command = "codex exec <handoff_file>"

[workers.researcher]
description = "Researches sources and produces structured notes"
command = "opencode run <handoff_file>"
```

Worker command templates currently support these placeholders:

- `<handoff_file>`: path to the generated Markdown handoff file.
- `<task_id>`: selected queue task ID.

Queue Mode validates worker profile names when you pass `--worker-profile`. A task assigned to `coder` is only selected by worker-aware commands using `--worker-profile coder`.

### Safe Smoke-Test Worker

Use a no-op worker before wiring real agents:

```toml
[workers.smoke]
description = "Safe smoke-test worker"
command = "echo smoke worker ran task=<task_id> handoff=<handoff_file>"
```

Then run:

```bash
jcode queue add "Smoke test queue execution" --worker-profile smoke
jcode queue run-next --worker-profile smoke --dry-run
jcode queue run-next --worker-profile smoke --execute
jcode queue runs
jcode queue review --worker-profile smoke
```

## Recommended Workflow

1. Add small, reviewable tasks with `jcode queue add`.
2. Assign each task to a worker profile with `--worker-profile`.
3. Use `jcode queue dashboard` to inspect queue shape.
4. Use `jcode queue run-next --worker-profile <name> --dry-run` before every real run.
5. Use `jcode queue run-next --worker-profile <name> --execute` for a foreground run or `--background` for a background run.
6. Inspect artifacts with `jcode queue runs`, `jcode queue run <task-id> <timestamp>`, `jcode queue run-status <run-id>`, or `jcode queue logs <run-id>`.
7. Run `jcode queue refresh-runs` to reconcile completed background runs.
8. Review the inbox with `jcode queue review`.
9. Approve good work with `jcode queue approve <task-id>`.
10. Reopen incomplete work with `jcode queue reopen <task-id>`.

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

Add and run a task:

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

Background runs start as `running`. They write stdout and stderr to run files and write `exit_code.txt` when the process completes. `refresh-runs` reads completed background runs and moves successful tasks to `review`; non-zero exits move tasks to `blocked`.

## Cancellation Workflow

```bash
jcode queue run-next --worker-profile smoke --background
jcode queue active
jcode queue cancel-run <run-id>
jcode queue run-status <run-id>
jcode queue dashboard
```

`cancel-run` cancels running background runs and moves the task to `blocked` for review before retrying.

## Command Reference

### `jcode queue init`

Create `.jcode/workers.toml`, `.jcode/queue/queue.json`, `.jcode/queue/handoffs/`, and `.jcode/queue/runs/`.

```bash
jcode queue init
jcode queue init --force
```

`--force` overwrites `.jcode/workers.toml`. Existing queue state is left in place.

### `jcode queue add`

Add a task to the queue.

```bash
jcode queue add "Fix flaky login test"
jcode queue add "Fix flaky login test" \
  --description "Reproduce and fix the timeout in login retry tests" \
  --project crates/jcode-app-core \
  --priority high \
  --worker-profile coder \
  --output-path tests/login_retry.md
```

Options:

- `--description <text>`
- `--project <label-or-path>`
- `--priority low|normal|high|urgent`
- `--worker-profile <name>`
- `--output-path <path>`

### `jcode queue list`

List all queued tasks with status, priority, and task metadata.

```bash
jcode queue list
```

### `jcode queue status`

Show task counts by status.

```bash
jcode queue status
```

### `jcode queue show`

Show full details for one task.

```bash
jcode queue show <task-id>
```

### `jcode queue set-status`

Set a task status manually.

```bash
jcode queue set-status <task-id> ready
jcode queue set-status <task-id> blocked
```

Accepted statuses are `backlog`, `ready`, `running`, `review`, `done`, `blocked`, and `cancelled`.

### `jcode queue set-priority`

Set a task priority manually.

```bash
jcode queue set-priority <task-id> urgent
```

Accepted priorities are `low`, `normal`, `high`, and `urgent`.

### `jcode queue next`

Show the next actionable task without changing queue state.

```bash
jcode queue next
jcode queue next --worker-profile coder
```

`--worker-profile` filters selection to tasks assigned to that profile.

### `jcode queue start-next`

Mark the next actionable task as `running` without executing a worker command.

```bash
jcode queue start-next
jcode queue start-next --worker-profile coder
```

Use this for manual worker handoffs.

### `jcode queue finish`

Finish a running task manually.

```bash
jcode queue finish <task-id>
jcode queue finish <task-id> --done
jcode queue finish <task-id> --output-path path/to/result.md
```

Without `--done`, the task moves to `review`. With `--done`, it moves directly to `done`.

### `jcode queue workers`

List project-local worker profiles from `.jcode/workers.toml`.

```bash
jcode queue workers
```

### `jcode queue worker`

Show one worker profile.

```bash
jcode queue worker coder
```

### `jcode queue handoff`

Generate an agent-ready Markdown handoff for one task.

```bash
jcode queue handoff <task-id>
jcode queue handoff <task-id> --write
```

Without `--write`, the handoff is printed to stdout. With `--write`, it is saved to `.jcode/queue/handoffs/<task-id>.md`.

### `jcode queue handoff-next`

Generate a handoff for the next actionable task.

```bash
jcode queue handoff-next
jcode queue handoff-next --worker-profile coder --write
```

This is useful when you want Queue Mode to select the task but you want to run the worker manually.

### `jcode queue run-next`

Prepare or execute the next task for a worker profile.

```bash
jcode queue run-next --worker-profile coder --dry-run
jcode queue run-next --worker-profile coder --execute
jcode queue run-next --worker-profile coder --background
```

`run-next` requires `--worker-profile <name>` and exactly one of `--dry-run`, `--execute`, or `--background`.

Dry-run behavior:

- Selects the next actionable task for the worker profile.
- Writes the handoff file.
- Prints the command that would run.
- Does not change the task status.

Execute behavior:

- Selects the next actionable task for the worker profile.
- Writes the handoff file.
- Marks the task `running`.
- Executes the rendered command synchronously in the foreground.
- Writes run artifacts.
- Marks the task `review` on exit code `0`.
- Marks the task `blocked` on non-zero exit.

Background behavior:

- Selects the next actionable task for the worker profile.
- Writes the handoff file.
- Creates a run record and updates `.jcode/queue/runs/index.json`.
- Marks the task `running`.
- Starts the worker command in the background and returns immediately.
- Redirects stdout and stderr to run files.
- Writes `exit_code.txt` when the process completes.
- Requires `jcode queue refresh-runs` to move completed runs to `review` or `blocked`.

### `jcode queue active`

List background runs currently recorded as active.

```bash
jcode queue active
```

### `jcode queue run-status`

Show detailed state for one run ID.

```bash
jcode queue run-status <run-id>
```

### `jcode queue logs`

Print recorded stdout and stderr for one run. Non-UTF-8 bytes are displayed lossily instead of failing.

```bash
jcode queue logs <run-id>
```

### `jcode queue refresh-runs`

Reconcile completed background runs by reading their completion markers.

```bash
jcode queue refresh-runs
```

Exit code `0` marks the run `succeeded` and moves the task to `review`. A non-zero exit marks the run `failed` and moves the task to `blocked`.

### `jcode queue cancel-run`

Cancel a running background run.

```bash
jcode queue cancel-run <run-id>
```

On Windows, cancellation uses forced process-tree termination. Cancelled tasks move to `blocked`.

### `jcode queue runs`

List recent queue worker run artifacts.

```bash
jcode queue runs
jcode queue runs --task-id <task-id>
jcode queue runs --limit 5
```

### `jcode queue run`

Inspect one run artifact.

```bash
jcode queue run <task-id> <timestamp>
jcode queue run <task-id> <timestamp> --stdout
jcode queue run <task-id> <timestamp> --stderr
```

Use `jcode queue runs` to find the timestamp directory name.

### `jcode queue review`

List tasks waiting for review.

```bash
jcode queue review
jcode queue review --worker-profile coder
jcode queue review --limit 10
```

### `jcode queue approve`

Mark a review task as `done`.

```bash
jcode queue approve <task-id>
```

### `jcode queue reopen`

Move a task back for another worker pass.

```bash
jcode queue reopen <task-id>
```

Use this after review finds missing work or failed validation.

### `jcode queue dashboard`

Show a concise queue dashboard with status counts, the next actionable task, running tasks, review tasks, and blocked tasks.

```bash
jcode queue dashboard
jcode queue dashboard --worker-profile coder
jcode queue dashboard --limit 10
```

### `jcode queue board`

Show the grouped Queue Mode board using the same read-only board data used by the TUI scaffold.

```bash
jcode queue board
jcode queue board --worker-profile coder --limit 10
jcode queue board --json
jcode queue board --tui
```

`--tui` opens the standalone Kanban board for the current project-local queue state. It supports column/task navigation, task creation, background starts for selected actionable tasks, manual refresh, auto-refresh while the board is open, and approval of selected review tasks. It does not support drag-and-drop, task editing, selected-task logs/details, daemon scheduling, automatic task scheduling, or parallel/swarm scheduling. Quit with `q`.

### Main interactive app `/queue`

From the main interactive app launched with `jcode`, type:

```text
/queue
```

This opens a read-only project-local Queue Board overlay in the same terminal. It shows the Kanban columns and active runs, supports arrow or `h`/`j`/`k`/`l` navigation, and refreshes queue/run-index state with `r`. Press `Esc` or `q` to close the overlay and return to the main chat.

Current limitations: the main-app overlay does not create tasks, start selected tasks, approve/reopen/cancel tasks, reconcile completed background runs, auto-refresh, or run daemon/scheduler behavior. Use `jcode queue board --tui` for the existing standalone mutating board, and `jcode queue refresh-runs` for run reconciliation.

## Safety Notes

- Worker commands are project-local and come from `.jcode/workers.toml`.
- Always use `jcode queue run-next --worker-profile <name> --dry-run` before `--execute` or `--background`.
- `run-next --execute` runs synchronously in the foreground. `run-next --background` detaches and writes logs to run files.
- There is no background Queue Mode daemon.
- The standalone TUI auto-refreshes running tasks while open. In CLI workflows, run `jcode queue refresh-runs` manually.
- There is no parallel Queue Mode scheduler.
- The main `jcode` interactive app has a read-only `/queue` overlay; mutating board actions remain in the standalone TUI.
- Worker commands run through the local shell (`sh -c` on Unix, `cmd /C` on Windows), so quote paths and arguments carefully when adding complex commands.
- Queue Mode records process stdout, stderr, exit code, timestamps, and command metadata, but it does not validate the semantic quality of worker output. Keep human review in the loop.
- On Windows, `queue cancel-run` uses forced process-tree termination.
- `queue logs` may display lossy replacement characters for non-UTF-8 command output.
