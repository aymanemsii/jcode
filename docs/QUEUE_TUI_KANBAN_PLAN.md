# Queue Mode TUI/Kanban Plan

## Goal

Build a TUI/Kanban board that makes Queue Mode easier to operate visually without changing the proven CLI workflow. The board should expose queue tasks, active runs, review state, and task details in one place while keeping existing commands and project-local storage as the operational foundation.

## Current Checkpoint

Standalone Queue Mode TUI/Kanban is implemented and launched with:

```text
jcode queue board --tui
```

It displays `backlog`, `ready`, `running`, `review`, `blocked`, `done`, and `cancelled` columns using project-local `.jcode/` queue storage and run state.

Keyboard controls:

- `Left`/`Right`: move between columns.
- `Up`/`Down` or `j`/`k`: move within a column.
- `n`: create a new task; prompts for title and worker profile.
- `x`: run the selected actionable task in the background.
- `r`: manually refresh board and run status.
- `a`: approve the selected review task.
- `q`: quit.

Safe workflow:

1. Open `jcode queue board --tui`.
2. Press `n`.
3. Enter a task title.
4. Enter a worker profile, such as `planner`.
5. Select the task.
6. Press `x` to run it in the background.
7. Wait for auto-refresh to move it to `review`.
8. Press `a` to approve it to `done`.

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

## Phase 3C: Read-Only TUI Board

- Completed foundation: `jcode queue board --tui` opens a standalone terminal board from the same board data used by `jcode queue board`.
- It renders project-local queue state, shows the canonical columns, supports 2D navigation, creates tasks, starts selected actionable tasks in the background, manually refreshes with `r`, auto-refreshes running tasks while open, approves selected review tasks, and exits with `q`.
- Its refresh action reloads queue/run state and reuses the existing `refresh-runs` reconciliation logic.

Remaining richer TUI work:

- Add a minimal Queue Mode screen in the TUI.
- Read queue state using the same logic as the CLI board command.
- Render columns with stable ordering.
- Render active runs.
- Render selected task details.
- Support keyboard navigation across columns and tasks.

## Phase 3D: Safe TUI Actions

Initial safe actions are implemented in the standalone board:

- Create task.
- Run selected actionable task in the background.
- Refresh/reconcile run status.
- Approve selected review task.

Potential later standalone actions:

- Reopen a task.
- Maybe cancel a run with explicit confirmation.
- Maybe open logs or task details from the selected task/run.

Each action should call the same logic used by existing CLI commands and should be introduced one at a time with focused tests.

## Later, Not Now

- Daemon.
- Automatic task scheduling.
- Parallel or swarm scheduling.
- Drag-and-drop.
- Complex agent orchestration.
- Provider, model, or session integration.

## Risks

- TUI code can cause broad changes if data shaping and rendering are mixed too early.
- Avoid formatting or refactoring unrelated TUI files.
- Keep each change branch tiny.
- Prefer board data helpers before UI rendering changes.
- Treat TUI mutations as a later layer over already-tested CLI behavior.

## Recommended Next Implementation Step

Next, inspect and map the main `jcode` interactive TUI integration path, then integrate Queue Board into the main terminal app safely.
