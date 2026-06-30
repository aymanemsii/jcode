# Queue Runner Architecture Investigation

This document records the recommended architecture for the next Queue phase: manually running a queued task. It is intentionally an investigation note only; it does not define a background worker or implement queue execution.

## Current State

The project-local Queue CLI already supports queue storage and metadata management:

```text
jcode queue init
jcode queue add "Task title"
jcode queue list
jcode queue list --all
jcode queue show <id>
jcode queue status <id> <status>
jcode queue archive <id>
jcode queue next
jcode queue edit <id> ...
```

Queue storage is project-local:

```text
./.jcode/queue/tasks.json
```

The current implementation is deliberately small:

* `src/cli/args.rs` defines `QueueCommand`.
* `src/cli/dispatch.rs` maps parsed commands into CLI command handlers.
* `src/cli/commands/queue.rs` handles Queue CLI output and calls storage helpers.
* `crates/jcode-base/src/queue.rs` owns the JSON schema and storage operations.

The existing Queue storage schema includes `ready`, `running`, `done`, and `failed`, but current commands only change status manually. No code claims, runs, locks, schedules, retries, or spawns queue workers.

## Recommendation

The safest next slice is a foreground-only manual runner:

```text
jcode queue run <id>
```

`jcode queue run-next` can wait until after the explicit-id path works. Running by id is safer because it avoids accidentally introducing claiming semantics, task selection races, and hidden queue mutation rules.

The first runner should stay CLI-only and should not use the server protocol, TUI, Queue Board, daemon, swarm, or headless worker paths. It should reuse the same execution shape as the existing foreground `jcode run` command: initialize a provider and tool registry, create an agent, convert the queued task into a prompt, run it in the current process, and stream or print output in the current terminal.

This keeps execution visible, cancellable by the user, and bounded to a single process. It also preserves the current Queue mental model: a project-local task list controlled by explicit CLI commands.

## Why Manual And Foreground-Only

Manual foreground execution is the lowest-risk bridge from queue metadata to real work:

* The user explicitly chooses the task with `jcode queue run <id>`.
* No process survives after the command exits.
* Ctrl+C or terminal closure has obvious process semantics.
* Logs and status updates can be tied to one visible command invocation.
* No new scheduler, daemon, worker pool, claim protocol, or server RPC is needed.
* Existing provider/model/tool initialization behavior can be reused from `jcode run`.
* Failures are easier to reason about because there is one task and one process.

The main safety boundary is that `queue run` should mean "run this task now in this terminal", not "enqueue for a worker" or "start processing the queue".

## Where The Runner Should Live

The command surface should follow the existing Queue CLI split:

* Add parsing in `src/cli/args.rs` under `QueueCommand`.
* Map the parsed command in `src/cli/dispatch.rs`.
* Put CLI orchestration in `src/cli/commands/queue.rs`.
* Add only small storage helpers in `crates/jcode-base/src/queue.rs` if needed.

If the run implementation needs reusable execution helpers, prefer extracting a small helper from the existing `jcode run` implementation in `src/cli/commands.rs` rather than routing through the shared server. That helper should remain a foreground CLI helper, not a queue worker abstraction.

Likely files for the next implementation slice:

* `src/cli/args.rs`
* `src/cli/dispatch.rs`
* `src/cli/commands/queue.rs`
* `src/cli/commands.rs`
* `crates/jcode-base/src/queue.rs`

Files to avoid for the first slice:

* `crates/jcode-tui/**`
* visual/theme crates
* server protocol files
* swarm/headless/background worker files

## Execution Path

A future `jcode queue run <id>` should:

1. Resolve the project directory from the current process, matching existing Queue commands.
2. Load `./.jcode/queue/tasks.json`.
3. Find the task by id and reject archived tasks.
4. Require the task to be `ready` unless a later explicit `--rerun` or `--force` option is designed.
5. Create a run record before execution starts.
6. Set status to `running`.
7. Convert the queued task into a normal prompt.
8. Run the prompt through the foreground agent execution path.
9. On success, set status to `done`.
10. On error, set status to `failed`.
11. Write run metadata with session id, timestamps, exit state, and result pointers.

The prompt should be deterministic and simple. A reasonable first format is:

```text
Queued task:

Title: <title>
Priority: <priority>
Worker profile: <worker_profile or none>

<body if present>
```

If `body` is empty, the title should still be enough to run. The task should be converted into the same kind of user prompt that `jcode run` already accepts, rather than inventing a separate task execution protocol.

## CLI Versus Server/Session Reuse

The first runner should be CLI-only in the sense that it should not add server protocol messages or background server behavior. It can still reuse the core session and agent machinery that `jcode run` uses.

Recommended:

* Reuse provider initialization, tool registry setup, MCP registration behavior, agent creation, session persistence, and output modes from the existing foreground run path.
* Record the resulting session id in queue run metadata.
* Let existing session storage remain the authoritative transcript for model/tool interaction.

Not recommended for the first slice:

* Attaching to the shared server.
* Sending a queued task over a new RPC.
* Running through TUI remote-client code.
* Reusing swarm, ambient, or background task infrastructure.
* Starting a detached child process.

The existing server/session paths are powerful but carry semantics the queue runner should not inherit yet: reconnects, live clients, swarm state, soft interrupts, reload behavior, and background lifecycle concerns.

## Status Transitions

The basic lifecycle should be:

```text
ready -> running -> done
ready -> running -> failed
```

Recommended first-slice rules:

* `queue run <id>` accepts only `ready` tasks.
* `running` is written immediately before agent execution starts.
* `done` is written only after the foreground run returns success.
* `failed` is written when the foreground run returns an error and the process can still update storage.
* Archived tasks are not runnable.
* Existing manual `queue status` remains available for operator repair.

Interrupted runs need a conservative rule. If the process receives an ordinary error path, mark the task `failed` and record the error. If the process is killed, crashes, loses power, or is terminated before cleanup runs, the task may remain `running`. That is acceptable for the manual first slice because there is no worker claiming yet, and the operator can inspect sidecar run metadata and repair status manually.

Do not add a new `interrupted` status in the first runner slice unless there is a broader status migration. Keeping the existing four statuses avoids expanding queue behavior while execution semantics are still manual.

## Logs And Results

Do not store full logs, transcripts, or large model output inside `tasks.json`. Keep `tasks.json` as the compact task index and mutable status file.

Recommended storage split:

```text
./.jcode/queue/tasks.json
./.jcode/queue/runs/<task-id>/<run-id>.json
./.jcode/queue/runs/<task-id>/<run-id>.stdout.log
./.jcode/queue/runs/<task-id>/<run-id>.stderr.log
```

The sidecar JSON should be small metadata:

* `run_id`
* `task_id`
* `started_at`
* `finished_at`
* `status`
* `session_id`
* `provider`
* `model`
* `exit_code` or error summary
* pointers to stdout/stderr logs if captured

The full conversation transcript should remain in existing session storage under `~/.jcode/sessions/`, because that is already the system of record for session history. Queue run metadata should point to that session id instead of duplicating transcript data in project-local queue files.

For the first slice, terminal streaming can be enough. Capturing stdout/stderr sidecars is still valuable because it gives `queue show` or future tooling a stable result pointer without inflating `tasks.json`.

## `worker_profile`

`worker_profile` should remain metadata for now.

Do not make it automatically select a provider, model, tool profile, permission set, or swarm profile in the first manual runner. The term is intentionally future-facing, and prematurely binding it to existing provider/model/profile concepts would make the first runner behave like a scheduler.

Safe first behavior:

* Include `worker_profile` in the generated prompt for visibility.
* Copy it into run metadata.
* Optionally print it before running.
* Do not interpret it.

A later design can map `worker_profile` to named provider profiles, model selections, tool profiles, sandbox policy, or worker classes once there is a clear configuration model.

## Locking

Do not add locking before the manual `queue run` slice.

Locking is important once there are multiple workers, `run-next`, retries, or background execution. For explicit foreground `queue run <id>`, the minimum safe implementation can rely on:

* explicit task id selection,
* a `ready` precondition,
* immediate `running` status update,
* operator-visible terminal execution,
* manual status repair for abnormal interruption.

This does leave a known race if two terminals run the same ready task at the same time. That risk is acceptable for the first manual slice and should be documented in command output or help text if needed. Adding locks now would introduce cross-platform filesystem semantics before the feature has proven its shape.

## Risks And Open Questions

Risks:

* A killed process can leave a task in `running`.
* Two manual invocations can race without locking.
* Existing `jcode run` auto-poke behavior may be surprising for queued tasks if reused unchanged.
* Provider/model selection for `worker_profile` is undefined.
* Result sidecars need a stable naming scheme and should not corrupt existing queue storage.
* Session storage is global while queue storage is project-local, so run metadata must preserve the session id clearly.

Open questions for the implementation slice:

* Should `queue run` default to plain output, JSON, NDJSON, or mirror `jcode run` options?
* Should `JCODE_RUN_AUTO_POKE` remain enabled by default for queued runs?
* Should a failed run leave enough error detail in sidecar metadata even when stdout/stderr capture is disabled?
* Should `queue run-next` require locking from day one, or wait until background workers exist?

## Minimum Safe Next Slice

The next implementation slice should be:

```text
jcode queue run <id>
```

Scope:

* explicit id only;
* foreground current-process execution only;
* ready-only precondition;
* archived-task rejection;
* status updates `ready -> running -> done/failed`;
* sidecar run metadata under `./.jcode/queue/runs/`;
* session id recorded in run metadata;
* no lock file;
* no claiming;
* no background child process;
* no server protocol changes;
* no TUI changes.

Implementation should first extract or reuse enough of the existing `jcode run` path to avoid duplicating agent/provider setup. The Queue command should remain a thin orchestrator around storage status changes, prompt construction, and foreground execution.

## Explicitly Deferred

The following are deliberately deferred:

* daemon/background worker;
* Queue Board/TUI;
* server protocol;
* task claiming;
* locks;
* parallel execution;
* scheduling;
* retries;
* `queue run-next`;
* swarm/headless integration;
* provider/model/tool-profile mapping from `worker_profile`;
* queue-wide automation;
* task dependency handling;
* result browsing UI.

These features should be designed only after a manual foreground runner proves the task-to-prompt flow, status updates, and run metadata layout.
