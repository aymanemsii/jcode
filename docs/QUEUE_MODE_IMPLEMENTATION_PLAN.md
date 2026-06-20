# Jcode Queue Mode & Kanban Implementation Plan

This document outlines a phased implementation strategy for adding a **Queue Mode / Kanban Mode** to the `jcode` TUI and backend. This plan ensures that the feature is built incrementally, prioritizing stability and reusing existing abstractions.

---

## 1. Implementation Workflow & Guardrails

To introduce this feature safely without disrupting the existing codebase, developers must adhere to the following guardrails:

### What to Implement First (Core Backend)
1. **Core Data Models & Storage**: Define serialization models and persistent file formats in [session.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/session.rs) or a new `queue` module within `jcode-base`.
2. **Server-Side Queue Manager**: Build the scheduler loop on the server that monitors the task list and executes queue items.
3. **API Protocols**: Define message channels in [wire.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-protocol/src/wire.rs) to synchronize client-server queue state.
4. **Backend Integration Tests**: Verify enqueueing, running, pausing, and restarting tasks using a mock provider without drawing any UI.

### What NOT to Touch Early (Avoid Churn)
- **TUI Drawing Pipeline**: Do not modify [ui.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui.rs) or [app.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/app.rs) until backend models, storage, and protocols are completed and fully tested.
- **Provider Translation Layers**: Do not change provider-specific code (`jcode-provider-openai`, `jcode-provider-gemini`, etc.). Queue Mode should execute prompts via the existing unified `Provider` trait.
- **CLI Core Dispatch**: Keep the new commands isolated. Avoid altering startup logic in [startup.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/startup.rs) until the TUI launcher integration phase.

---

## 2. Proposed Queue Data Model

We propose adding a `QueueItem` struct to represent tasks within the queue:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueStatus {
    Pending,
    Running,
    AwaitingReview,
    Completed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: Uuid,
    pub title: String,
    pub prompt: String,
    pub status: QueueStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    
    // Links to execution environment
    pub session_id: Option<String>,         // ID of the worker session executing the task
    pub assigned_agent_id: Option<String>,  // Swarm worker ID if assigned
    pub working_dir: Option<String>,        // Isolation directory (e.g. git worktree path)
    
    // Outputs & review artifacts
    pub outcome_summary: Option<String>,    // Summary returned by the agent upon completion
    pub changeset_preview: Option<Vec<String>>, // List of files modified during execution
    pub rejection_feedback: Option<String>, // User review feedback if task was rejected
}
```

---

## 3. Proposed Queue Storage Strategy

- **Location**: `~/.jcode/queue.json`
- **Mechanism**: The queue state will be persisted in a single JSON file under the standard `jcode_dir` calculated by [lib.rs (jcode-storage)](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-storage/src/lib.rs).
- **Atomic Operations**: All writes must use `write_json` from `jcode-storage` which writes to a temporary file (`.queue.json.tmp`) and executes an atomic rename. This prevents corruption during system crashes or power losses.
- **Locking**: The background daemon will hold a lock file `~/.jcode/queue.lock` during mutations to prevent concurrent client writes if multiple TUI instances are open.

---

## 4. Proposed Kanban TUI Layout

Kanban Mode will be implemented as a separate full-screen state in `crates/jcode-tui`.

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ TUI Header (System Status, Provider Info)                                   │
├─────────────────────────────────────────────────────────────────────────────┤
│ KANBAN DASHBOARD                                                            │
│                                                                             │
│ [TO DO]            [IN PROGRESS]         [AWAITING REVIEW]     [DONE]       │
│ ┌──────────────┐   ┌──────────────┐      ┌───────────────┐     ┌──────────┐ │
│ │ Task #12     │   │ Task #8      │      │ Task #5       │     │ Task #1  │ │
│ │ Fix lint     │   │ Swarm run    │      │ Re-exec test  │     │ Setup    │ │
│ └──────────────┘   └──────────────┘      └───────────────┘     └──────────┘ │
│ ┌──────────────┐                                                            │
│ │ Task #14     │                                                            │
│ │ Refactor lib │                                                            │
│ └──────────────┘                                                            │
├─────────────────────────────────────────────────────────────────────────────┤
│ ACTIVE AGENTS PANEL (Right Rail or Bottom Band)                             │
│ 🦊 Task #8: running git diff (12s ago)                                       │
│ 🐺 Task #5: idle, awaiting verification                                     │
└─────────────────────────────────────────────────────────────────────────────┘
```

- **Layout Grid**: Use `Layout::horizontal` from `ratatui` to split the viewport into 4 columns.
- **Card Widget**: Define a custom `KanbanCard` widget in `crates/jcode-tui-render` to draw boxed items with borders representing status highlights.
- **Keybindings**:
  - `Tab` / `Shift+Tab`: Move focus between columns.
  - `Up` / `Down` arrows: Select card within the focused column.
  - `Enter`: Open the Details / Review modal for the selected card.
  - `Space`: Promote the selected card to the next phase (e.g. Pending -> In Progress).
  - `a`: Add a new task card (opens prompt input).
  - `d`: Delete the selected task.

---

## 5. Proposed Active Agents Panel

- **Implementation**: Create a panel (re-using the grid render logic in `jcode-tui-render::swarm_gallery` / [swarm_tiles.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui-render/src/swarm_tiles.rs)) located in the bottom or right sidebar.
- **Data Source**: Subscribe to `BusEvent::SubagentStatus` and `BusEvent::BatchProgress` via [bus.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/bus.rs).
- **Details Displayed**:
  - Worker animal icon + name (e.g. `🦊 fox`).
  - Active task title and step execution time.
  - CPU/Memory indicators and the last few lines of the worker's stdout stream.

---

## 6. Proposed Review Inbox

When an agent completes a task, the task moves to the `AwaitingReview` column. Pressing `Enter` opens the Review Inbox.

- **Outcome Details**: Displays the final markdown response from the worker describing what was fixed or constructed.
- **Visual Diff Viewer**: Uses the existing diff rendering from [ui_diff.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui_diff.rs) and [ui_file_diff.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui_file_diff.rs) to display the modifications.
- **Actions**:
  - **Accept (`a` or `/approve`)**: Merges the changeset into the main workspace, closes the worker session, and moves the card to `Done`.
  - **Reject (`r` or `/reject`)**: Opens an input prompt where the user provides feedback (e.g. "The test fails on edge case X"). The task moves back to `Pending` or `In Progress` with the feedback appended to the session history so the agent can resume and self-correct.

---

## 7. Connecting Queue Mode to Existing Sessions & Agents

Queue Mode runs on top of the existing multi-session backend:

```
                  ┌───────────────────────┐
                  │ Server Queue Manager  │
                  └───────────┬───────────┘
                              │
                    Spawns Swarm Worker
                              ▼
                  ┌───────────────────────┐
                  │     Worker Agent      │
                  │ (Under Session ID)    │
                  └───────────┬───────────┘
                              │
                 Publishes Completion Event
                              ▼
                  ┌───────────────────────┐
                  │       Event Bus       │ ──▶ Moves Task to
                  │   (BusEvent Channel)  │     AwaitingReview
                  └───────────────────────┘
```

1. **Task Launching**: The Server Queue Manager monitors the task queue. When it detects a `Pending` task, it spins up a new worker instance.
2. **Worker Spawning**: It reuses the swarm core launcher in [swarm.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/swarm.rs) to spawn a worker agent under a unique, hidden session ID (e.g. `queue-task-UUID`).
3. **Workspace Isolation**: If checked, the swarm manager provisions a git worktree for the task, setting `working_dir` inside the worker's agent configuration.
4. **Completion Tracking**: Once the agent produces its final response (fulfilling its completion report policy), it publishes a `BusEvent::BackgroundTaskCompleted` on the global [bus.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/bus.rs). The Queue Manager listens to this event, saves the summary/changeset, and flags the task status to `AwaitingReview`.

---

## 8. Risks and Rollback Notes

| Risk Scenario | Impact | Mitigation / Rollback Action |
|---------------|--------|------------------------------|
| **Daemon Crash during execution** | Tasks stuck in `Running` state indefinitely. | On daemon startup, check [active_pids.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-storage/src/active_pids.rs). If a task's worker session PID is no longer running, mark the task as `Failed` (with "crashed/interrupted" notice). |
| **Workspace Merge Conflicts** | Overlapping file edits across parallel tasks. | Restrict concurrent executions to different subdirectories, or enforce git worktree isolation. If a conflict occurs during review approval, abort the merge and prompt the user to resolve or run `/reject` to auto-rebase the task. |
| **Out-of-Memory (OOM)** | Multiple concurrent LLM model contexts exhaust resources. | Implement a strict scheduler queue limit (e.g. `MAX_CONCURRENT_QUEUE_TASKS = 2`). Pause pending tasks when memory usage crosses critical thresholds. |
| **Corrupted Queue File** | Inability to launch or loss of task list. | Write atomically using `.tmp` and atomic rename. Implement automatic backup (`queue.json.bak`) on every successful load/save cycle. |

---

## 9. Milestone Order

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│ Milestone 1  │ ──▶ │ Milestone 2  │ ──▶ │ Milestone 3  │ ──▶ │ Milestone 4  │ ──▶ │ Milestone 5  │
│ Models &     │     │ Server Wire  │     │ Execution &  │     │ Kanban TUI   │     │ Review Inbox │
│ Persistence  │     │ Protocol     │     │ Scheduler    │     │ Dashboard    │     │ & Integration│
└──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘     └──────────────┘
```

### Milestone 1: Models & Persistence
- Define `QueueItem` and `QueueStatus` structures in `jcode-base`.
- Implement load/save serialization logic using `jcode-storage` helpers.
- Write unit tests for file serialization, locking, and crash backups.

### Milestone 2: Server Wire Protocol
- Add queue request enums (`Request::EnqueueTask`, `Request::DeleteTask`, `Request::UpdateTask`) to [wire.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-protocol/src/wire.rs).
- Define server events (`ServerEvent::QueueUpdated`) to notify connected clients.
- Implement server-side client connection endpoints to synchronize active client states.

### Milestone 3: Execution & Scheduler
- Create the server Queue Manager loop inside `jcode-app-core`.
- Integrate task launching with [swarm.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/swarm.rs) to spawn workers.
- Handle completion event interception and auto-save the task outcomes.

### Milestone 4: Kanban TUI Dashboard
- Implement custom Kanban widgets in `jcode-tui-render`.
- Build the vertical column split layout inside `crates/jcode-tui/src/tui/`.
- Wire navigation controls, card selections, and list state scrolling.

### Milestone 5: Review Inbox & Final Polish
- Integrate diff rendering panels into the card details modal.
- Implement Accept (merge changeset, mark Done) and Reject (feedback resume loop) controls.
- Polish visual states, status indicators, and keyboard focus changes.
