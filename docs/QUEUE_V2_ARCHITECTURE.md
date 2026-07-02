# Queue v2 Architecture

Queue v2 should evolve Queue v1 into a safer execution system without jumping directly to background automation.

Queue v1 is intentionally project-local, manual, foreground-only, explicit-id-only, and does not include claiming, locks, background workers, retries, scheduling, Queue Board/TUI, worker profile mapping, or multi-agent execution. Queue v2 should preserve the useful parts of that model while adding coordination in small, testable layers.

## Goals

Queue v2 should eventually support:

* `queue run-next`
* safe task claiming
* cross-platform queue locks
* stale running and stale claim recovery
* worker profiles that can map to provider, model, tool, and permission configuration
* background worker design
* daemon design
* retries
* scheduling
* parallel workers
* Queue Board/TUI
* multi-agent execution

These are architecture goals, not first-slice requirements.

## Core Principle

Queue v2 should not start with background workers.

The safe path is to evolve Queue v1 in layers:

1. Claim one task.
2. Run one claimed task.
3. Release or finalize the claim.
4. Add foreground `queue run-next`.
5. Add retry and scheduling metadata.
6. Add explicit foreground worker loops.
7. Add background workers only after claim, lock, and recovery behavior are proven.
8. Add Queue Board/TUI after state metadata stabilizes.
9. Add multi-agent orchestration last.

Each layer should work in the foreground first. Background execution magnifies ambiguity in locking, recovery, identity, logging, and user control, so it should come after those behaviors are already boring.

## Task State Model

Queue v1 statuses are:

```text
ready
running
done
failed
```

These statuses are probably enough for the first Queue v2 slices. Queue v2 should prefer metadata over status expansion unless a new status represents a durable user-visible state that cannot be expressed clearly otherwise.

Likely v2 metadata:

* `claimed_by`
* `claimed_at`
* `claim_expires_at`
* `run_attempts`
* `max_attempts`
* `last_error`
* `scheduled_at`
* `locked_by`
* `lock_token`

The preferred model is:

* `ready` means the task can be claimed if it is not archived, not scheduled for the future, and has no active unexpired claim.
* `running` means execution has started for a claimed task or manually selected task.
* `done` means execution completed successfully.
* `failed` means the latest execution attempt failed and no automatic retry should happen unless retry semantics are explicitly enabled.

Avoid adding statuses such as `claimed`, `scheduled`, `retrying`, or `locked` until metadata has proven insufficient. Status explosion makes the CLI, recovery commands, and future TUI harder to reason about.

## Claiming Architecture

A future claim should reserve one task for one worker before execution starts.

Claiming rules:

* Only non-archived `ready` tasks can be claimed.
* A task with an active unexpired claim cannot be claimed by another worker.
* A task scheduled for the future cannot be claimed until `scheduled_at` has passed.
* The claim selector must be explicit: either first ready task in JSON order or a documented priority ordering. The first v2 implementation should choose one and document it before adding `run-next`.
* The claim operation must be atomic enough for local filesystem use.
* Each claim should include a unique token.
* Claimed task metadata should include worker id, claim timestamp, and lease expiry.
* A claimed task should not be runnable by another worker unless the claim is expired, released, or reset by an explicit recovery command.

Claim identity should not rely only on process id. A worker id plus unique claim token is easier to display, audit, and recover.

The first claiming slice should expose storage helpers and a foreground command before any background worker exists. A command such as `queue claim-next` can make the behavior observable without coupling it to model execution.

## Locking Architecture

Queue v2 needs a queue-level lock before `run-next`, workers, retries, scheduling, or daemon behavior are safe.

Locking requirements:

* Work on Windows, macOS, and Linux.
* Keep lock state project-local under `./.jcode/queue/`.
* Avoid relying only on best-effort advisory locking without fallback behavior.
* Include owner pid, hostname if available, `created_at`, command, and unique token in the lock file.
* Treat stale lock detection conservatively.
* Provide a manual recovery command before background workers are added.
* Do not add lock-dependent Queue behavior until the lock design is validated on Windows.

The lock protects short critical sections such as loading `tasks.json`, selecting a task, writing claim metadata, and saving the updated task index. Long model execution should not hold the queue lock unless a later design proves that it must.

Stale lock recovery should be explicit and cautious. If pid checks, hostname checks, or timestamps are uncertain, the command should prefer refusing automatic cleanup and telling the user how to inspect or reset the lock.

## `queue run-next`

`queue run-next` should not be added until claiming and locking are designed and implemented.

Expected future foreground behavior:

1. Acquire the queue lock.
2. Select one claimable task using the documented ordering rule.
3. Mark the task claimed and then either mark it `running` immediately or transition to `running` just before execution. The exact order should be chosen deliberately.
4. Save the updated task index.
5. Release the queue lock before long model execution if the claim lease makes that safe.
6. Execute the task in the current foreground process first.
7. Finalize the task as `done` or `failed`.
8. Preserve run metadata under the existing run history model.
9. Release, clear, or archive claim metadata according to the finalized state.

The first `run-next` should be foreground-only. It should not start a daemon, spawn hidden workers, or run multiple tasks.

## Worker Profiles

Queue v1 `worker_profile` is metadata only. Queue v2 should keep that true until a separate mapping layer exists.

Future worker profile mapping might include:

* provider
* model
* approval mode
* tool profile
* MCP profile
* sandbox policy
* system prompt or worker instructions
* cost and latency preferences

Do not prescribe the exact config format yet. Reasonable future locations include:

```text
./.jcode/queue/profiles.json
```

or integration with existing jcode configuration if that produces less duplication and a clearer user model.

Profile resolution should be observable before it affects execution. A read-only command that explains which provider/model/tool policy would be selected for a task is safer than silently changing runtime behavior.

## Background Workers And Daemon

Background execution should be phased:

1. Foreground `queue run-next`.
2. Single foreground loop with an explicit command, such as `queue worker --once` or `queue worker`.
3. Background process only after lock, claim, and recovery behavior are proven.

A daemon should not be considered safe until it has:

* `status` UX
* `stop` UX
* log discovery
* visible worker identity
* stale claim recovery
* stale lock recovery
* clear behavior around crashes and interrupted model execution

The daemon should be a coordinator, not a shortcut around the claim and lock model. It should use the same storage helpers and recovery paths as the foreground commands.

## Retries And Scheduling

Failed tasks should not be automatically retried in the first Queue v2 implementation.

Safe retry semantics:

* Retry should increment `run_attempts`.
* Retry should record `last_error`.
* `max_attempts` should limit automatic retry behavior once automatic retries exist.
* `scheduled_at` should prevent claiming or running before the scheduled time.
* Retry backoff can wait until after basic retry metadata is stable.
* Manual retry should come before automatic retry.

Scheduling should start as metadata plus filtering in claim selection. It should not require a daemon in the first slice.

## Queue Board / TUI

Queue Board should come after stable Queue v2 metadata.

It should display:

* task list
* active claims and workers
* active and recent runs
* run history
* stale running or stale claim warnings
* reset and retry actions

No TUI should be built before the state model stabilizes. A TUI built too early would either hide important recovery states or force state-model decisions for presentation reasons.

## Multi-Agent Execution

Multi-agent and mixture-of-agents execution should be last.

It requires:

* worker profiles
* claims
* locks
* per-worker identity
* safe parallelism
* task isolation
* clear logs and run history
* stop and recovery controls

Parallel execution without these foundations risks duplicate runs, lost updates, confusing logs, and tasks stuck in states that are hard to recover from.

## Recommended Implementation Order

Small future slices:

1. Add this Queue v2 architecture document.
2. Investigate claim metadata and compatibility with existing `tasks.json`.
3. Investigate a queue lock prototype across Windows, macOS, and Linux.
4. Add a read-only stale running and stale claim detector.
5. Add claim and release storage helpers.
6. Add foreground `queue claim-next`.
7. Add foreground `queue run-claimed <id>` or carefully integrate claimed execution with existing `queue run <id>`.
8. Add foreground-only `queue run-next`.
9. Add retry metadata.
10. Add worker profile config mapping.
11. Add a single-process worker loop.
12. Add Queue Board/TUI.
13. Add background daemon.
14. Add multi-agent orchestration.

This order keeps every early slice inspectable from the CLI and avoids invisible background behavior until the queue can recover from interrupted work.

## Deferred From First V2 Implementation

The first Queue v2 implementation should explicitly defer:

* background daemon
* parallel workers
* `run-next` without locks
* automatic retries
* scheduling
* TUI Queue Board
* multi-agent execution
* remote/server queue protocol

The first implementation should prove claim and lock semantics before adding more automation.
