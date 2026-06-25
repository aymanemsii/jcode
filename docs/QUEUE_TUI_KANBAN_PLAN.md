# Queue Mode TUI/Kanban Plan

## Goal

Build a TUI/Kanban board that makes Queue Mode easier to operate visually without changing the proven CLI workflow. The board should expose queue tasks, active runs, review state, and task details in one place while keeping existing commands and project-local storage as the operational foundation.

## Queue TUI v0.4 Checkpoint

Standalone Queue Mode TUI/Kanban is implemented and launched with:

```text
jcode queue board --tui
```

It displays `backlog`, `ready`, `running`, `review`, `blocked`, `done`, and `cancelled` columns using project-local `.jcode/` queue storage and run state. This checkpoint supports a complete human-controlled AI Kanban loop: create work, run a selected actionable task in the background, watch it move to review or blocked, inspect details/log previews, approve finished work, reopen work for another pass, or cancel a running task.

Keyboard controls:

- `Left`/`Right`: move between columns.
- `Up`/`Down` or `j`/`k`: move within a column.
- `n`: create a rich task with title, description, worker profile, priority, project, and output path.
- `x`: run the selected actionable task in the background.
- `r`: manually refresh board and run status.
- `a`: approve the selected review task to `done`.
- `o`: reopen the selected `review`, `done`, or `blocked` task to `ready`.
- `c`: cancel the selected running task to `blocked`.
- `q`: quit.

Safe workflow:

1. Open `jcode queue board --tui`.
2. Press `n`.
3. Create a task with title, description, worker profile, priority, project, and output path.
5. Select the task.
6. Press `x` to run it in the background.
7. Watch auto-refresh move it to `review` or `blocked`.
8. Inspect the details panel, latest run summary, and stdout/stderr log preview.
9. Press `a` to approve a review task to `done`, `o` to reopen a review/done/blocked task to `ready`, or `c` to cancel a running task to `blocked`.

Worker profile behavior:

- `.jcode/workers.toml` controls valid worker profiles when present.
- The TUI discovers and shows available profiles from `.jcode/workers.toml`.
- Task creation validates the selected worker profile against that file.
- If `.jcode/workers.toml` is missing, arbitrary profile names are allowed.

Run lifecycle:

- `backlog`/`ready` -> `running` when a selected actionable task starts.
- `running` -> `review` when the worker exits successfully.
- `running` -> `blocked` when the worker fails or the task is cancelled.
- `review` -> `done` when approved.
- `review`/`done`/`blocked` -> `ready` when reopened.

Current limitations:

- The mutating Kanban workflow is standalone only; it is not integrated into the main `jcode` app yet.
- No drag-and-drop.
- No task editing.
- No daemon or scheduler.
- No parallel/swarm mode.
- No full log viewer in the TUI beyond the preview.

Next recommended phases:

1. Native visual restyle for the standalone board.
2. Main `jcode` integration.
3. Security and hardening.
4. Daemon/scheduler later.

## Principles

- CLI remains the source of truth.
- Project-local `.jcode/` storage remains the source of truth.
- TUI should reuse existing `QueueState` and `RunIndex` logic.
- No daemon at first.
- No provider, model, or session internals.
- No automatic swarm scheduler.
- No broad refactors.

## Proposed TUI Layout

Primary board columns:

- Backlog
- Ready
- Running
- Review
- Blocked
- Done

Supporting panels:

- Active runs panel: current background or synchronous runs, matching existing active-run data.
- Selected task details panel: task id, title, status, priority, worker profile, handoff/review state, and relevant paths.
- Help/status footer: navigation keys, refresh hint, selected task summary, and errors.

## Phase 3A: Design And Data Foundation

- Define a board view model that is independent of terminal rendering.
- Reuse queue task status and priority sorting.
- Reuse dashboard and active-run information instead of inventing parallel state.
- Add a CLI/logic-level board representation before touching TUI code.
- Keep the board model small enough to test with existing queue fixtures and command output checks.

## Phase 3B: CLI Board Command

Add a small command before TUI work:

```text
jcode queue board
```

Optional later:

```text
jcode queue board --json
```

Purpose:

- Produce the same grouped data the TUI will render.
- Let board grouping, sorting, and active-run joins be tested without touching TUI internals.
- Provide a fallback view for users who do not use the TUI.

The command should group tasks into the proposed columns, include active-run summaries, and preserve existing Queue Mode semantics.

## Phase 3C: Standalone TUI Board

- Completed foundation: `jcode queue board --tui` opens a standalone terminal board from the same board data used by `jcode queue board`.
- It renders project-local queue state, shows the canonical columns, supports 2D navigation, creates rich tasks, starts selected actionable tasks in the background, manually refreshes with `r`, auto-refreshes running tasks while open, shows selected-task details and log previews, approves selected review tasks, reopens review/done/blocked tasks, cancels selected running tasks, and exits with `q`.
- Its refresh action reloads queue/run state and reuses the existing `refresh-runs` reconciliation logic.

## Phase 3D: Safe TUI Actions

Initial safe actions are implemented in the standalone board:

- Create rich task.
- Run selected actionable task in the background.
- Refresh/reconcile run status.
- Approve selected review task.
- Reopen selected review/done/blocked task.
- Cancel selected running task.

Each action should call the same logic used by existing CLI commands and should be introduced one at a time with focused tests.

## Later, Not Now

- Daemon.
- Automatic task scheduling.
- Parallel or swarm scheduling.
- Drag-and-drop.
- Task editing.
- Full in-TUI log viewer.
- Complex agent orchestration.
- Provider, model, or session integration.

## Risks

- TUI code can cause broad changes if data shaping and rendering are mixed too early.
- Avoid formatting or refactoring unrelated TUI files.
- Keep each change branch tiny.
- Prefer board data helpers before UI rendering changes.
- Treat TUI mutations as a later layer over already-tested CLI behavior.

## Recommended Next Implementation Steps

1. Native visual restyle for the standalone board.
2. Main `jcode` integration.
3. Security and hardening.
4. Daemon/scheduler later.
