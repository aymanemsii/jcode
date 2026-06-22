# Queue Mode TUI/Kanban Plan

## Goal

Build a TUI/Kanban board that makes Queue Mode easier to operate visually without changing the proven CLI workflow. The board should expose queue tasks, active runs, review state, and task details in one place while keeping existing commands and project-local storage as the operational foundation.

## Principles

- CLI remains the source of truth.
- Project-local `.jcode/` storage remains the source of truth.
- TUI should reuse existing `QueueState` and `RunIndex` logic.
- First TUI version must be read-only.
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

- Add a minimal Queue Mode screen in the TUI.
- Read queue state using the same logic as the CLI board command.
- Render columns with stable ordering.
- Render active runs.
- Render selected task details.
- Support keyboard navigation across columns and tasks.
- Do not mutate queue state, run state, files, reviews, or processes yet.

## Phase 3D: Safe TUI Actions

Only after the read-only board works:

- Refresh queue and run state.
- Approve a review task.
- Reopen a task.
- Maybe cancel a run with explicit confirmation.
- Maybe open logs or task details from the selected task/run.

Each action should call the same logic used by existing CLI commands and should be introduced one at a time with focused tests.

## Later, Not Now

- Daemon.
- Automatic polling.
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

First implement `jcode queue board` as a CLI/data foundation. Once that grouped board representation is working and tested, implement a read-only TUI board that renders the same data.
