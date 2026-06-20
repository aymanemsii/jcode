# Jcode Workspace Architecture Map

This document maps the high-level codebase architecture of the `jcode` repository. It outlines crate boundaries, key entry points, UI rendering layouts, session lifecycles, daemon communications, and reusable abstractions to guide the implementation of a future Queue / Kanban Mode.

---

## 1. High-Level Workspace & Crate Structure

The repository is structured as a cargo workspace containing a root CLI crate and modular library crates under the `crates/` directory.

### Workspace Members & Roles
- **[jcode (Root CLI Crate)](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/lib.rs)**: Parses CLI arguments, manages global initializations (logging, allocations, panic hooks), and delegates execution to `jcode-tui` and `jcode-app-core`.
  - Binary Entry Point: [main.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/main.rs)
  - Library Entry Point: [lib.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/lib.rs)
  - CLI Parsing & Dispatch: [args.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/args.rs) and [dispatch.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/dispatch.rs)
- **[crates/jcode-tui](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/lib.rs)**: The terminal UI presentation crate. Manages the crossterm event loops, ratatui frame drawing, input processing, and offline session replays.
- **[crates/jcode-tui-render](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui-render/src/lib.rs)**: A dependency-free presentation library containing shared render modules (e.g. status grids, memory tiles, and swarm galleries).
- **[crates/jcode-app-core](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/lib.rs)**: The core execution logic. It hosts the background daemon (server), agent runtime orchestration, ambient execution, and updater services.
- **[crates/jcode-base](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/lib.rs)**: Shared core data models, including the session structure, persisted message formats, and the system-wide event bus.
- **[crates/jcode-protocol](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-protocol/src/lib.rs)**: The wire format definition (JSON serialization types) for communications between the background server daemon and TUI clients.
- **[crates/jcode-storage](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-storage/src/lib.rs)**: Manages platform-aware configuration directory resolution (`~/.jcode`), file security locks, and process ID directories.
- **[crates/jcode-agent-runtime](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-agent-runtime/src/lib.rs)**: Lower-level system tools for executing processes, managing command timeouts, and queueing/delivering soft interrupts.
- **[crates/jcode-provider-*](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/Cargo.toml)**: Dedicated translation crates matching LLM provider schemas (Gemini, Anthropic, OpenAI, OpenRouter, Bedrock, etc.) to the unified execution traits.

---

## 2. Where CLI Commands Are Defined

CLI commands and argument schemas are built using the `clap` derive parser.

- **Definition**: [args.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/args.rs) defines the primary `Command` enum and its nested subcommands (such as `ServerCommand`, `SessionCommand`, `AuthCommand`).
- **Orchestration & Dispatch**: [dispatch.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/dispatch.rs) matches the parsed `Command` variant and invokes target run routines.
- **Subcommand Implementation**: [commands.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/commands.rs) hosts logic for administrative operations (like listing sessions, checking usage, stopping the daemon, or executing headless runs).

---

## 3. Where TUI Screens & Components Are Implemented

The terminal interface utilizes `ratatui` for drawing panels.

- **Main TUI App State**: [app.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/app.rs) acts as the central client-side controller, storing active session details, stream buffers, and onboarding flows. Split app-methods reside under `crates/jcode-tui/src/tui/app/`.
- **TUI Drawing Pipeline**: [ui.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui.rs) drives the visual draw sequence, dividing the terminal screen into chat headers, messages viewports, input bars, and sidebar panels.
- **Sidebar & Panels**:
  - [ui_pinned.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui_pinned.rs) handles the sidebar drawer displaying pinned logs, todos, and file details.
  - [ui_diagram_pane.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui_diagram_pane.rs) renders generated Mermaid diagrams.
  - [ui_messages.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui_messages.rs) manages transcript message list wrapping and syntax highlighting.
- **Onboarding & Picker Screen**:
  - [session_picker.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/session_picker.rs) implements the start-up visual launcher allowing users to select or resume sessions.
  - [ui_onboarding.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/ui_onboarding.rs) manages first-run welcome and default LLM checks.

---

## 4. Where Session State Is Created, Resumed, and Tracked

- **Data Structure**: [session.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/session.rs) contains the `Session` struct, representing a conversation's history, title, models, token consumption, and status flags.
- **Client Session Launching**: [tui_launch.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/tui_launch.rs) constructs the client connection handle and resumes or initiates target sessions.
- **Server Session Management**: [runtime.rs (server)](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/runtime.rs) manages active background session instances in a process-wide `sessions` map (`HashMap<String, Arc<Mutex<Agent>>>`).

---

## 5. Where Server/Client Protocol Messages Are Defined

Communication between client and daemon is defined inside `jcode-protocol`.

- **Wire Types**: [wire.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-protocol/src/wire.rs) defines:
  - `Request`: Client commands (e.g., `Message`, `Cancel`, `Subscribe`, `Rewind`, `SoftInterrupt`).
  - `ServerEvent`: Streamed notifications (e.g., `TextDelta`, `ToolStart`, `ToolDone`, `BatchProgress`, `ServerEvent::Notification`).
- **Channel Transport**: Communication happens over Unix domain sockets on macOS/Linux and Named Pipes on Windows, handled in [socket.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/socket.rs).

---

## 6. Where Storage/Persistence Is Handled

Persistence directories and standard write protocols are owned by `jcode-storage`.

- **Path Resolvers**: [lib.rs (jcode-storage)](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-storage/src/lib.rs) calculates platform-specific directories:
  - `~/.jcode/sessions/`: Conversation transcripts stored as JSON.
  - `~/.jcode/logs/`: Daily logs (e.g. `jcode-YYYY-MM-DD.log`).
  - `~/.jcode/servers.json`: Lock registration for active background daemons.
- **Atomic Serialization**: Utilizes temporary file writes and atomic renames to avoid partial-write corruptions during daemon crashes.

---

## 7. Where Self-Development & Reload Behavior Are Implemented

- **Trigger Handler**: [tui_lifecycle_runtime.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/app/tui_lifecycle_runtime.rs) intercepts `/reload` or `/rebuild` inputs, requests server updates, and flags hot-reload requests.
- **Server Execution Swap**: [reload.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/reload.rs) manages swapping the executing server binary. On a reload request, the server invokes `execve` (or `replace_process` on Windows) to launch the updated binary, retaining the active Unix socket.
- **Self-Dev Canary Management**: [selfdev.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/src/cli/selfdev.rs) enables canary building, pulls down recent git updates, compiles the test binaries, and connects to the shared server with elevated `canary` flags.

---

## 8. Where Active Sessions & Agents Are Tracked

- **On-Disk PID Mapping**: [active_pids.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-storage/src/active_pids.rs) writes active session IDs and their corresponding process IDs (PIDs) to `~/.jcode/active_pids/`. This directory is polled for crash recovery and session lists.
- **Daemon Active Context**: [client_lifecycle.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/client_lifecycle.rs) records live websocket connections, tracks subscription requests, and manages active client-to-session bounds.

---

## 9. Crate & File Relevance for Future Queue Mode

Implementing Queue / Kanban Mode will require changes/additions in the following key locations:

```
┌───────────────────────────┐
│     jcode-base            │ ──▶ Queue models and events
│     └─ session.rs         │
│     └─ bus.rs             │
└─────────────┬─────────────┘
              ▼
┌───────────────────────────┐
│     jcode-protocol        │ ──▶ Client-server Queue API requests
│     └─ wire.rs            │
└─────────────┬─────────────┘
              ▼
┌───────────────────────────┐
│     jcode-app-core        │ ──▶ Daemon execution queue, worker threads,
│     └─ server/queue.rs    │     and scheduler
└─────────────┬─────────────┘
              ▼
┌───────────────────────────┐
│     jcode-tui-render      │ ──▶ Kanban columns, cards, and tile grid
│     └─ queue_render.rs    │
└─────────────┬─────────────┘
              ▼
┌───────────────────────────┐
│     jcode-tui             │ ──▶ Kanban dashboard screen, inputs, and
│     └─ tui/queue_view.rs  │     review modals
└───────────────────────────┘
```

1. **[crates/jcode-base/src/bus.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/bus.rs)**: Create new event channels `BusEvent::QueueUpdated` and `BusEvent::TaskCompleted`.
2. **[crates/jcode-protocol/src/wire.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-protocol/src/wire.rs)**: Define requests to append to the queue (`Request::Enqueue`), retrieve the current queue, and manage item reviews.
3. **[crates/jcode-app-core/src/server/](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server)**: Create `queue.rs` or `scheduler.rs` to maintain the memory state of the queue and dispatch tasks to parallel swarm workers.
4. **[crates/jcode-tui-render/src/](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui-render/src)**: Implement Kanban drawing grids and tiles.
5. **[crates/jcode-tui/src/tui/](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui)**: Add `queue_view.rs` containing the dashboard layout, keyboard focus handlers, and interactive review interfaces.

---

## 10. Reusable Abstractions

To minimize code duplication and maintain system stability, the following existing abstractions must be reused:

- **Global Event Bus ([bus.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-base/src/bus.rs))**: Use to broadcast queue changes from the backend scheduler to all connected client screens.
- **Swarm Orchestrator ([swarm.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-app-core/src/server/swarm.rs))**: Reuse swarm agent spawn pathways to run queue tasks in isolated worker subprocesses.
- **Soft Interrupts ([soft_interrupt.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-agent-runtime/src/lib.rs))**: Use to pause running workers when they encounter issues, require reviews, or trigger safety checks.
- **Split Views Layout ([split_view.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/app/split_view.rs))**: Reuse multi-pane terminal layouts to display the Kanban columns side-by-side.
- **Swarm Status Gallery ([info_widget_swarm_gallery.rs](file:///C:/Users/Aymane's%20AI/AI-Lab/repos/jcode/crates/jcode-tui/src/tui/info_widget_swarm_gallery.rs))**: Adapt for the active agents sidebar panel to display live CPU/memory stats and streaming text snippets from executing workers.
