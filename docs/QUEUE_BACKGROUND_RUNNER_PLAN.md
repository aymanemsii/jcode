# Queue Background Runner / Active Agent Mode Plan

This document is a Phase 2 design plan for Queue Mode background execution. It is not an implementation status document and does not describe completed functionality.

Queue Mode CLI MVP is complete and smoke-tested for foreground execution. Phase 2 should add the smallest safe background runner surface so `jcode` can start worker commands, record durable run state, and let users inspect or cancel active and past runs later.

## Current MVP Baseline

The current Queue Mode MVP supports:

- `jcode queue init`
- `jcode queue add`
- `jcode queue list`
- `jcode queue status`
- `jcode queue show`
- `jcode queue next`
- `jcode queue start-next`
- `jcode queue finish`
- `jcode queue workers`
- `jcode queue worker`
- `jcode queue handoff`
- `jcode queue run-next --dry-run`
- `jcode queue run-next --execute`
- `jcode queue runs`
- `jcode queue run`
- `jcode queue review`
- `jcode queue approve`
- `jcode queue reopen`
- `jcode queue dashboard`

The MVP stores queue state in project-local `.jcode/queue/queue.json`, writes handoffs under `.jcode/queue/handoffs/`, and writes run artifacts under `.jcode/queue/runs/`.

Execution is synchronous foreground only. `run-next --execute` starts one configured worker command, waits for it to exit, records the result, and updates the task status before returning.

## Phase 2 Goal

Phase 2 should allow `jcode` to start a configured worker command in the background and return control to the user immediately. The user should be able to inspect active runs, inspect historical run state, read stdout and stderr logs, and request cancellation by run ID.

The design should preserve the existing Queue Mode shape:

- Queue state remains project-local.
- Worker commands still come from `.jcode/workers.toml`.
- Handoffs remain explicit artifacts.
- `--dry-run` remains the safest preview path.
- Completed background work still flows through the review inbox before approval.

The first implementation should favor durable state and inspectability over automation. A background process that cannot be inspected, recovered, or cancelled is not acceptable for this phase.

## Non-Goals for Phase 2

Phase 2 should not add:

- A full TUI or Kanban board.
- A daemon or OS service unless process behavior proves it is absolutely required.
- Parallel swarm mode.
- Scheduled or recurring queue execution.
- Provider/model internals or provider selection changes.
- A complex permissions system.
- Cross-project or global queue storage.
- Automatic task approval after command success.

## Proposed Commands

### `jcode queue run-next --worker-profile <name> --background`

Purpose: start the next actionable task for the worker profile and return immediately.

Behavior:

- Select the next `backlog` or `ready` task matching `--worker-profile`.
- Generate or refresh the task handoff.
- Resolve the worker command template.
- Create a new run directory and write initial run metadata before spawning.
- Spawn the command as a background process.
- Redirect stdout and stderr to files in the run directory.
- Mark the task `running`.
- Print the run ID, task ID, PID, run directory, and log paths.

Expected output:

```text
Started background queue run
run_id: run_20260622_163000_ab12cd
task_id: task_abc123
worker_profile: coder
pid: 12345
status: running
stdout: .jcode/queue/runs/task_abc123/20260622-163000/stdout.log
stderr: .jcode/queue/runs/task_abc123/20260622-163000/stderr.log
```

State changes:

- Task status changes to `running`.
- `run.json` is written with `status: running`.
- `runs/index.json` is updated if an index is introduced.
- `command.txt`, `stdout.log`, and `stderr.log` are created in the run directory.

Safety rules:

- Require explicit `--background`; do not make background execution the default.
- Preserve `--dry-run`; `--dry-run --background` should preview the background command and planned run paths without spawning.
- Refuse to run if the worker profile has no command.
- Refuse to start a second active task for the same worker profile unless a later explicit concurrency flag or setting allows it.
- Never use global queue storage.

### `jcode queue active`

Purpose: list background runs that are currently believed to be active.

Behavior:

- Read project-local run state.
- Show runs with `status: running` or `status: unknown`.
- Check whether tracked PIDs appear to still exist when the platform supports that check.
- Do not silently mark tasks succeeded or done.
- If a PID is missing, display the run as stale or `unknown` and recommend recovery.

Expected output:

```text
Active queue runs

run_20260622_163000_ab12cd  task_abc123  coder  running  pid=12345  started=2026-06-22T16:30:00Z
run_20260622_164512_ef34gh  task_def456  reviewer  unknown  pid=23456  started=2026-06-22T16:45:12Z
```

State changes:

- None by default.
- It may refresh a run from `running` to `unknown` only if the process is clearly gone and no exit code was recorded. This should be conservative and documented in output.

Safety rules:

- Listing active runs must not start or cancel processes.
- Stale detection must not mark a task `done`.

### `jcode queue run-status <run-id>`

Purpose: show detailed state for one run.

Behavior:

- Read `run.json` by run ID, either through `runs/index.json` or by scanning local run directories.
- Print task ID, worker profile, command path reference, PID, status, timestamps, exit code, run directory, stdout path, and stderr path.
- If the run is still marked `running`, perform a conservative PID existence check and report whether the process appears alive.

Expected output:

```text
run_id: run_20260622_163000_ab12cd
task_id: task_abc123
worker_profile: coder
status: running
pid: 12345
process_alive: yes
started_at: 2026-06-22T16:30:00Z
ended_at: -
exit_code: -
run_dir: .jcode/queue/runs/task_abc123/20260622-163000
stdout: .jcode/queue/runs/task_abc123/20260622-163000/stdout.log
stderr: .jcode/queue/runs/task_abc123/20260622-163000/stderr.log
```

State changes:

- None unless stale detection is explicitly implemented for this command.

Safety rules:

- Do not infer success from missing processes.
- Do not expose hidden global state; all paths should resolve under the project queue directory.

### `jcode queue cancel-run <run-id>`

Purpose: request cancellation of an active background run.

Behavior:

- Load run state and verify it belongs to the current project queue.
- Refuse cancellation if the run is already `succeeded`, `failed`, or `cancelled`.
- Check the tracked PID.
- Send a termination request appropriate to the platform.
- Update run status to `cancelled` only after the process is known to have exited or after a clearly documented forced termination path succeeds.
- Record `ended_at` and any cancellation note.
- Move the task to `blocked` by default so the user reviews what happened before retrying.

Expected output:

```text
Cancelled queue run
run_id: run_20260622_163000_ab12cd
task_id: task_abc123
previous_pid: 12345
task_status: blocked
```

State changes:

- Run status becomes `cancelled`.
- `ended_at` is set.
- Task status becomes `blocked`.

Safety rules:

- Do not kill arbitrary PIDs without verifying the run record and project-local ownership.
- Prefer terminating only the direct child process for Phase 2. Process-tree termination should be a later explicit design if needed.
- If the PID no longer exists, mark the run `unknown` or `cancelled` only with a clear note; do not mark the task done.

### `jcode queue logs <run-id>`

Purpose: inspect stdout and stderr for a recorded run.

Behavior:

- Print the last N lines of stdout and stderr by default.
- Support `--stdout`, `--stderr`, and `--tail <n>`.
- Optionally support `--follow` after basic log reading works.

Expected output:

```text
== stdout: .jcode/queue/runs/task_abc123/20260622-163000/stdout.log ==
...

== stderr: .jcode/queue/runs/task_abc123/20260622-163000/stderr.log ==
...
```

State changes:

- None.

Safety rules:

- Read only log files recorded in project-local run metadata.
- Do not execute shell commands to read logs.
- Handle missing or still-empty log files without failing the whole command.

### `jcode queue recover`

Purpose: reconcile persisted run state after terminal exits, crashes, or interrupted `jcode` invocations.

Behavior:

- Scan local run metadata for `running` runs.
- Check whether tracked PIDs still appear alive.
- For missing PIDs with no recorded exit code, mark the run `unknown` and move the task to `blocked`.
- For known completed runs where an exit code can be collected by a parent process, update to `succeeded` or `failed`. Without a daemon, this may not be possible after the parent exits.
- Print every state change.

Expected output:

```text
Recovered queue runs
run_20260622_163000_ab12cd task_abc123: running -> unknown, task running -> blocked
```

State changes:

- Stale `running` runs may become `unknown`.
- Associated tasks may become `blocked`.

Safety rules:

- Recovery must be conservative.
- Recovery must never mark a task `done`.
- Recovery should not delete artifacts.

## Run State Model

The minimal persisted run record should include:

```text
run_id: string
task_id: string
worker_profile: string
command: string
pid: integer | null
status: running | succeeded | failed | cancelled | unknown
started_at: timestamp
ended_at: timestamp | null
exit_code: integer | null
run_dir: path
stdout_path: path
stderr_path: path
```

Recommended additions if they are cheap:

- `handoff_path`
- `command_path`
- `platform`
- `cwd`
- `created_by_jcode_version`
- `status_note`

`command` is useful for inspection, but `command.txt` should also be written so the exact expanded command can be reviewed without parsing JSON.

## Storage Structure

Keep all artifacts project-local:

```text
.jcode/
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
```

`runs/index.json` is optional but useful once commands need to find a run by `run_id` without scanning every task directory. If introduced, it should be treated as a convenience index that can be rebuilt from per-run `run.json` files.

Writes should be atomic where practical:

- Write `run.json` through a temp file and rename.
- Update `queue.json` through the same existing safe write path used by Queue Mode.
- If `index.json` write fails after `run.json` succeeds, the run should still be recoverable by scanning.

## Task Status Transitions

Background execution should follow these task status transitions:

- Background start: `backlog` or `ready` -> `running`.
- Process success: `running` -> `review`.
- Process failure: `running` -> `blocked`.
- Cancel: `running` -> `blocked`.
- Unknown or stale process: `running` -> `blocked` with run status `unknown`.

Rationale:

- Successful worker execution still requires human review; it should not become `done`.
- Failed and cancelled work needs inspection before retry, so `blocked` is safer than automatically returning to `ready`.
- Unknown process state must not be silently marked complete.

A later explicit retry command can move blocked tasks back to `ready` or create a new run from the same task.

## Windows Considerations

Windows behavior needs specific handling before process spawning is implemented:

- Worker command execution should be explicit about shell semantics. If Queue Mode stores commands as strings, Windows spawning will likely need `cmd /C <command>` for parity with foreground execution.
- PID tracking should record the direct child PID returned by the spawn API. If that PID is `cmd.exe`, the actual worker may be a child process; Phase 2 should document this limitation in `cancel-run`.
- A background process may outlive the terminal that started `jcode`. Logs must be redirected to files from the start so output is not lost when the terminal closes.
- Cancellation should be careful. Killing a PID on Windows can terminate the shell wrapper without terminating grandchildren, or it can terminate an unrelated process if the PID has been reused. The implementation should verify the run record and prefer conservative failure over broad process-tree killing.
- Exit code collection after the original parent exits may be limited without a daemon. `recover` should mark stale runs `unknown` instead of inventing success or failure.
- Paths in `run.json` should be serialized consistently and should not assume Unix separators.
- Log inspection must handle files that are locked, still being written, or encoded with platform defaults.

## Safety Rules

Phase 2 should enforce these rules:

- Require explicit `--background`.
- Preserve `--dry-run` for command preview and planned artifact paths.
- Never background-run without a configured worker command.
- Never use global queue storage.
- Do not run multiple tasks for the same worker profile unless explicitly allowed later.
- Keep execution artifacts local to the project.
- Do not mark background runs successful without an observed process exit code of zero.
- Do not mark tasks `done` from background execution.
- Do not delete run artifacts during cancellation or recovery.
- Treat worker commands as configured executable actions; make the expanded command visible before execution through `--dry-run`.

## Implementation Phases

Phase 2 should be split into small branches:

1. Background runner design doc
   - Add this plan.
   - No source changes.

2. Run state/index foundation
   - Add run state types and read/write helpers.
   - Add scanning and optional index rebuild support.
   - No process spawning yet.

3. Background start command
   - Add `run-next --background`.
   - Create run artifacts before spawn.
   - Redirect stdout and stderr to files.
   - Mark task `running`.

4. Active runs command
   - Add `queue active`.
   - List `running` and `unknown` runs.
   - Add conservative PID existence checks.

5. Log inspection command
   - Add `queue logs <run-id>`.
   - Support stdout/stderr selection and tailing.
   - Add `--follow` only if basic reading is stable.

6. Cancellation command
   - Add `queue cancel-run <run-id>`.
   - Implement direct child termination.
   - Move cancelled tasks to `blocked`.

7. Recovery and stale-run handling
   - Add `queue recover`.
   - Reconcile stale `running` records.
   - Keep unknown outcomes explicit.

8. Optional worker concurrency limits
   - Add per-worker active-run limits after single-worker background execution is stable.
   - Do not add swarm behavior in this phase.

Each branch should include targeted tests for state transitions and file layout. Process-spawning tests should use a safe smoke worker and should cover Windows behavior where possible.

## First Recommended Implementation Branch

The first implementation branch after this documentation should be:

```text
aymane/run-state-foundation
```

Scope:

- Add run state and index helpers only.
- Define serialization for the minimal run model.
- Add read/write tests for `.jcode/queue/runs/<task-id>/<timestamp>/run.json`.
- Add index rebuild or scan-by-run-id behavior.
- Do not spawn processes.
- Do not add `--background` yet.

This keeps the riskiest part, process management, out of the first code branch while establishing the durable state shape that later commands can share.
