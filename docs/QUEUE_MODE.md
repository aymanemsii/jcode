# Queue Mode CLI

Queue Mode is a project-local task queue for handing work to command-line workers. It stores tasks in the current repository, renders each selected task into a Markdown handoff, and can run a configured worker command against that handoff.

The current MVP is CLI-only. It does not start a background daemon, run tasks in parallel, or provide a TUI queue board yet.

## Why It Exists

Queue Mode gives agent-heavy development a simple control loop:

1. Capture work as explicit tasks.
2. Assign tasks to worker profiles such as `coder`, `reviewer`, or `researcher`.
3. Preview the exact worker command before it runs.
4. Execute one foreground worker task at a time.
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
```

- `.jcode/workers.toml` defines project-local worker profiles and the command each worker runs.
- `.jcode/queue/queue.json` stores queue tasks, statuses, priorities, worker assignments, and timestamps.
- `.jcode/queue/handoffs/` stores generated Markdown handoff files such as `.jcode/queue/handoffs/<task-id>.md`.
- `.jcode/queue/runs/` stores run artifacts under `.jcode/queue/runs/<task-id>/<timestamp>/`.

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
5. Use `jcode queue run-next --worker-profile <name> --execute` to run one selected task.
6. Inspect artifacts with `jcode queue runs` and `jcode queue run <task-id> <timestamp>`.
7. Review the inbox with `jcode queue review`.
8. Approve good work with `jcode queue approve <task-id>`.
9. Reopen incomplete work with `jcode queue reopen <task-id>`.

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
```

`run-next` requires `--worker-profile <name>` and exactly one of `--dry-run` or `--execute`.

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

## Safety Notes

- Worker commands are project-local and come from `.jcode/workers.toml`.
- Always use `jcode queue run-next --worker-profile <name> --dry-run` before `--execute`.
- `run-next --execute` runs synchronously in the foreground. It does not detach.
- There is no background Queue Mode daemon yet.
- There is no parallel Queue Mode execution yet.
- There is no Queue Mode TUI board yet.
- Worker commands run through the local shell (`sh -c` on Unix, `cmd /C` on Windows), so quote paths and arguments carefully when adding complex commands.
- Queue Mode records process stdout, stderr, exit code, timestamps, and command metadata, but it does not validate the semantic quality of worker output. Keep human review in the loop.
