# Queue Main TUI Integration Plan

## Scope

This document maps how to add the Queue Mode Kanban board to the main interactive `jcode` terminal app opened by running:

```text
jcode
```

It is a planning document only. It intentionally does not implement the integration.

## Main App Entrypoint Map

### What happens when the user runs `jcode`

The root binary enters at `src/main.rs`.

Flow:

1. `src/main.rs`
   - Configures allocator/runtime setup.
   - Special-cases the macOS hotkey listener.
   - Builds a Tokio runtime.
   - Calls `jcode::run().await`.
2. `src/lib.rs`
   - Re-exports `jcode_tui::*`, which transitively exposes app-core/base modules.
   - Defines `pub async fn run() -> Result<()> { cli::startup::run().await }`.
3. `src/cli/startup.rs`
   - Installs panic/logging/setup hooks.
   - Parses CLI args with Clap.
   - Registers cross-layer callbacks such as config reload, permission notifications, session-list cache invalidation, and server spawning.
   - Calls `dispatch::run_main(args).await`.
4. `src/cli/dispatch.rs`
   - Dispatches explicit subcommands.
   - With no subcommand, falls through to `run_default_command(args).await`.
   - `run_default_command` checks for an existing server, may spawn one, then calls `tui_launch::run_tui_client(...)`.
5. `src/cli/tui_launch.rs`
   - `run_tui_client` initializes the terminal runtime with `init_tui_runtime`.
   - Creates the main TUI app with `tui::App::new_for_remote_with_options(...)`.
   - Runs the main interactive client with `app.run_remote(terminal).await`.

The default main app is therefore a remote TUI client connected to a local shared server. The CLI/root crate launches it, while most presentation behavior lives under `crates/jcode-tui`.

### Crates and modules involved

- `src/main.rs`: process entrypoint.
- `src/lib.rs`: root crate re-export layer and `jcode::run`.
- `src/cli/startup.rs`: startup wiring, Clap parsing, global callback registration.
- `src/cli/dispatch.rs`: command dispatch and no-subcommand default path.
- `src/cli/tui_launch.rs`: terminal setup and remote TUI client launch.
- `crates/jcode-tui/src/tui/app.rs`: main `App` state.
- `crates/jcode-tui/src/tui/app/run_shell.rs`: local and remote run loops.
- `crates/jcode-tui/src/tui/app/remote/*`: remote server connection, remote event handling, terminal input handling.
- `crates/jcode-tui/src/tui/ui.rs`: full-frame rendering and overlay rendering.
- `crates/jcode-tui/src/tui/mod.rs`: `TuiState` trait and shared TUI types.
- `crates/jcode-base/src/queue.rs`: Queue Mode domain data and storage helpers.

### Main TUI/event loop

The main interactive event loop is in `crates/jcode-tui/src/tui/app/run_shell.rs`.

Important functions:

- `App::run_remote(...)`
  - Used by the default `jcode` app.
  - Owns the terminal event stream, redraw interval, server connection, bus receiver, and remote event loop.
  - Draws frames via `crate::tui::render_frame(f, &self)`.
- `App::run(...)`
  - Local TUI mode, less relevant for the default `jcode` path.

The remote loop multiplexes:

- redraw ticks,
- status spinner ticks,
- remote server events,
- terminal events,
- native scroll commands,
- bus events.

### Keybindings and actions

Input handling is split between local and remote paths.

Remote/default path:

- `crates/jcode-tui/src/tui/app/remote/key_handling.rs`
  - Handles terminal key events while connected to the server.
  - Routes active overlays before normal chat input:
    - help overlay,
    - session picker overlay,
    - login picker overlay,
    - account picker overlay,
    - inline interactive picker.
  - Handles global shortcuts such as side panel toggle, diagram pane toggle, model/effort cycling, workspace navigation, typing scroll lock, and regular text input.

Local path:

- `crates/jcode-tui/src/tui/app/input.rs`
  - Contains `handle_key_event`, `handle_key_press_event`, and `handle_key_core`.
  - Routes modal keys, command suggestions, global shortcuts, inline picker keys, and normal text input.

The safest Queue integration should follow the overlay pattern already used by session/login/account pickers: route overlay keys first, then return to normal chat when the overlay closes.

### Slash commands and command palette

Slash-command input is handled in `crates/jcode-tui/src/tui/app/input.rs`.

Key path:

- `App::submit_user_input(...)` trims the input and checks a chain of command handlers:
  - `commands::handle_help_command`
  - `commands::handle_keys_command`
  - `commands::handle_ssh_command`
  - `commands::handle_session_command`
  - `commands::handle_dictation_command`
  - `commands::handle_config_command`
  - `commands::handle_log_command`
  - `commands::handle_diff_command`
  - `commands::handle_model_status_command`
  - `debug::handle_debug_command`
  - `model_context::handle_model_command`
  - `commands::handle_usage_command`
  - `productivity::handle_productivity_command`
  - `commands::handle_feedback_command`
  - `state_ui::handle_info_command`
  - `auth::handle_auth_command`
  - `tui_lifecycle_runtime::handle_dev_command`

Command suggestions are registered in `crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs`.

Important pieces:

- `RegisteredCommand`
- `REGISTERED_COMMANDS`
- `App::command_candidates`
- `App::rank_suggestions`
- `App::get_suggestions_for`

The "command palette" is the slash-command suggestion UI shown when typing `/...`, not a separate application-level command registry. A new `/queue` or `/board` command must be added both to command handling and to `REGISTERED_COMMANDS` if it should be discoverable.

### Screens, views, panels, and rendering

The app does not appear to have a general screen router for arbitrary full-screen views. It has:

- full-frame rendering via `crates/jcode-tui/src/tui/ui.rs`,
- state access through the `TuiState` trait in `crates/jcode-tui/src/tui/mod.rs`,
- side-panel and diagram panes,
- inline interactive UI above the input,
- modal overlays.

Overlay render path in `crates/jcode-tui/src/tui/ui.rs`:

1. Draw/clear base area.
2. If changelog/help/model-status overlay is active, render it and return.
3. If session picker overlay is active, render it and return.
4. If login picker overlay is active, render it and return.
5. If account picker overlay is active, render it and return.
6. Otherwise render the normal chat/panes/input frame.

Existing overlay-like state lives in `App` fields such as:

- `session_picker_overlay`
- `login_picker_overlay`
- account picker overlay state
- help/changelog/model-status scroll state
- inline interactive state

Recommended integration should add a Queue Board overlay rather than trying to run the standalone TUI loop from inside `App::run_remote`.

## Current Queue TUI Map

### CLI parsing

`jcode queue board --tui` is parsed in `src/cli/args.rs`.

Relevant shape:

```text
Command::Queue(QueueCommand::Board {
    worker_profile,
    limit,
    json,
    tui,
})
```

The `Board` subcommand options are:

- `--worker-profile <name>`
- `--limit <usize>` with default `20`
- `--json`
- `--tui`

### Dispatch

Dispatch happens in `src/cli/dispatch.rs`.

The queue branch maps:

```text
QueueCommand::Board { worker_profile, limit, json, tui }
  -> commands::run_queue_board_command(worker_profile.as_deref(), limit, json, tui)
```

### Command implementation

`run_queue_board_command` lives in `src/cli/commands.rs`.

It:

1. Rejects `--json` plus `--tui`.
2. Normalizes and validates `worker_profile`.
3. Loads queue state with `crate::queue::load()`.
4. Builds board data with `crate::queue::build_queue_board(...)`.
5. If `--tui`:
   - loads run index with `crate::queue::load_run_index()`,
   - filters active runs,
   - initializes terminal runtime,
   - calls `queue_board_tui::run_read_only_queue_board(...)`,
   - finishes terminal runtime.
6. Else if `--json`, prints JSON.
7. Else prints text board output.

### Queue Board TUI renderer

The standalone board lives in `src/cli/commands/queue_board_tui.rs`.

Key functions/types:

- `run_read_only_queue_board(...)`
  - Owns a blocking crossterm event loop.
  - Draws the board.
  - Handles keyboard input.
  - Performs auto-refresh.
- `QueueBoardTuiOptions`
  - Holds `worker_profile` and `limit`.
- `draw_queue_board(...)`
  - Renders header, Kanban columns, active runs panel, and footer.
- `refresh_board_state(...)`
  - Reloads queue and run state.
  - Reconciles completed background runs through `super::refresh_queue_runs`.
- board selection helpers:
  - `BoardSelection`
  - `select_first_available_task`
  - `preserve_selection_after_refresh`
  - `move_selection_within_column`
  - `move_selection_to_column`
- mutation helpers:
  - `add_task_from_prompt`
  - `approve_selected_review_task`
  - `run_selected_task_in_background`

Despite the current function name `run_read_only_queue_board`, the standalone TUI is now interactive and mutating: `n` creates tasks, `x` starts selected tasks in the background, and `a` approves review tasks.

### Queue/domain helpers

Queue domain data and storage are in `crates/jcode-base/src/queue.rs`.

Reusable domain pieces:

- `TaskStatus`
- `TaskPriority`
- `Task`
- `QueueState`
- `QueueBoard`
- `QueueBoardColumn`
- `QueueBoardTask`
- `RunStatus`
- `RunState`
- `RunIndex`
- `WorkerProfile`
- `load()`
- `save()`
- `load_run_index()`
- `save_run_index()`
- `list_active_runs()`
- `load_worker_profiles()`
- `load_worker_profiles_from_path()`
- `worker_profiles_file_path()`
- `build_queue_board(...)`

CLI helper pieces in `src/cli/commands.rs` that are useful but riskier to reuse directly from `jcode-tui`:

- worker profile validation and normalization,
- run refresh/reconciliation,
- approve/reopen task logic,
- start selected queue task in background,
- board JSON/text formatting.

Because those helpers currently live in the CLI/root crate, directly calling them from `crates/jcode-tui` would likely violate dependency direction. For native main-TUI integration, reusable non-CLI logic should be moved or duplicated carefully into a lower crate only on a future implementation branch.

### Parts reusable for main app integration

Low-risk reusable pieces:

- `crates/jcode-base/src/queue.rs` board model and storage.
- Existing status ordering and sorting in `build_queue_board`.
- Existing run index model for active-run display.
- The standalone board's rendering layout as a reference.
- Selection/navigation logic from `queue_board_tui.rs`, after extraction to a TUI-owned module.

Not directly reusable without refactor:

- `run_read_only_queue_board`, because it owns a blocking event loop and terminal runtime.
- Direct calls from `jcode-tui` to CLI-only helpers in `src/cli/commands.rs`.
- Mutating actions that start background workers from inside the main app.

## Recommended Integration Point

The smallest safe path is a slash command that opens a read-only Queue Board overlay inside the main TUI:

```text
/queue
```

Suggested aliases after the first implementation works:

```text
/board
/kanban
```

This fits the existing app because slash commands are already discoverable through the slash-command suggestion UI, and overlays already support returning to the main chat with `Esc`.

### Option comparison

| Option | Files likely touched | Risk | Return to main app? | Notes |
| --- | --- | --- | --- | --- |
| Slash command `/queue` | `crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs`, `crates/jcode-tui/src/tui/app/input.rs` or a new `app/commands_queue.rs`, `crates/jcode-tui/src/tui/app.rs`, `crates/jcode-tui/src/tui/ui.rs`, new Queue overlay module | Low/medium | Yes, if implemented as overlay | Best first integration. Discoverable and consistent with `/resume`, `/usage`, `/info`, `/help`. |
| Slash aliases `/board`, `/kanban` | Same as `/queue` plus aliases in command suggestions/handler | Low | Yes | Good later once primary `/queue` behavior is proven. Avoid adding too many names in the first patch. |
| Command palette action | Same as slash command registry because command palette is the slash suggestion UI | Low/medium | Yes | There is no separate command-palette action registry found. This collapses into slash-command work. |
| Keybinding | keybinding config/types, local and remote key handlers, help docs/tests | Medium/high | Yes, if overlay | Premature. Adds conflict risk and discoverability burden. Prefer after `/queue` exists. |
| Menu/screen route | app state, render routing, key routing, possibly new screen enum | High | Maybe | No general screen router was found. Building one for Queue would be a broad TUI refactor. |
| CLI flag fallback | `src/cli/args.rs`, `src/cli/dispatch.rs`, `src/cli/tui_launch.rs` | Medium | No, if it launches standalone board instead of chat | Useful fallback only if embedded overlay proves too risky. It does not satisfy the native main-app feel. |

### Recommended shape

Do not invoke `jcode queue board --tui` from inside the main app. That would nest terminal runtimes/event loops and would make returning to chat fragile.

Instead:

1. Add a `QueueBoardOverlay`/`QueueBoardView` state object inside `jcode-tui`.
2. Load `QueueState` and `RunIndex` through `crates/jcode-base/src/queue.rs`.
3. Build `QueueBoard` with `build_queue_board`.
4. Render it as an overlay from `ui.rs`.
5. Route overlay keys before normal chat input:
   - `Esc`/`q`: close overlay and return to chat.
   - arrows or `h`/`j`/`k`/`l`: navigate.
   - `r`: reload read-only state, if safe.
6. Keep the first version read-only:
   - no `n`,
   - no `x`,
   - no `a`,
   - no task editing,
   - no background execution.

This keeps the first integration inside the presentation layer plus queue base storage and avoids provider/session/server internals.

## Proposed Smallest Implementation Step

Status: implemented as a read-only `/queue` overlay in the main interactive app.

Next branch goal:

```text
queue-main-tui-readonly-overlay
```

Scope:

- Add a discoverable `/queue` slash command in the main interactive app.
- `/queue` opens a read-only Kanban overlay for the current working directory's project-local queue.
- Overlay displays:
  - `backlog`, `ready`, `running`, `review`, `blocked`, `done`, `cancelled`,
  - per-column task cards with id/title/priority/worker profile,
  - active runs if available,
  - footer with `Esc/q close`, navigation, and `r refresh`.
- Overlay supports:
  - `Esc` or `q` to return to the main chat,
  - 2D navigation,
  - optional read-only `r` refresh.
- Overlay does not mutate queue state in the first branch.

Implementation constraints:

- Reuse `QueueBoard`, `RunIndex`, and `build_queue_board` from `jcode-base`.
- Copy or extract only rendering/navigation pieces needed for an embedded overlay.
- Do not call the standalone `run_read_only_queue_board` from the main app.
- Do not call CLI/root `src/cli/commands.rs` helpers from `crates/jcode-tui`.
- If run refresh requires CLI-only helpers, skip auto-refresh in the first branch and document that `jcode queue refresh-runs` remains the manual CLI fallback.

Expected user flow:

1. User runs `jcode`.
2. User types `/queue`.
3. Queue Board overlay opens.
4. User browses columns/tasks.
5. User presses `Esc` or `q`.
6. The normal chat/app is restored in the same terminal.

Implemented behavior:

- `/queue` is registered in slash-command suggestions and the help overlay.
- The overlay is embedded in the main TUI render/key path and returns cleanly to chat with `Esc` or `q`.
- It reuses `QueueBoard`, `RunIndex`, and `build_queue_board` from `jcode-base`.
- It is intentionally read-only: navigation and `r` reload are supported, but task creation, background starts, approvals, and run reconciliation are not wired into the main app.

Next step for richer behavior:

- Extract queue mutation/run-refresh helpers out of CLI-only modules into a lower shared queue service before adding safe main-app mutations or auto-refresh.

If returning cleanly is hard:

- Do not ship a half-embedded terminal loop.
- Fallback to a slash command that prints a system message:
  - `Queue Board is available with: jcode queue board --tui`
  - optionally include a read-only text summary from `build_queue_board`.
- Keep standalone `jcode queue board --tui` as the reliable path until overlay state/key routing is solved.

## Boundaries And Risks

Do not touch yet:

- provider/model/session internals,
- server/sidecar,
- app-core agent internals,
- memory internals,
- daemon/scheduler design,
- automatic execution from the main app,
- background worker launch from the main app,
- task editing,
- drag/drop,
- broad `App`/`TuiState` refactors,
- CLI command behavior except where absolutely needed for documentation or a future narrow entrypoint.

Main risks:

- The standalone Queue TUI is blocking and owns its own terminal runtime; embedding it directly would conflict with `App::run_remote`.
- The main app's default mode is remote-client based; local filesystem Queue reads from the client working directory may not always match remote/server working directory expectations.
- Queue mutating actions need careful ownership boundaries because many helper functions live in the CLI/root crate.
- Auto-refresh currently uses CLI run-refresh logic from `src/cli/commands.rs`; moving that into a lower crate may be needed before native auto-refresh is clean.
- Adding a new overlay touches central TUI state and render/key routing, so tests should cover close/return behavior.
- Command naming could conflict with user expectations: `/queue` is broad, `/board` is generic, `/kanban` is specific but less obvious.

## Concrete File List For Next Implementation

Required:

- `crates/jcode-tui/src/tui/app/state_ui_input_helpers.rs`
  - Add `/queue` to `REGISTERED_COMMANDS`.
- `crates/jcode-tui/src/tui/app/input.rs`
  - Route `/queue` to a Queue overlay opener, or delegate to a new queue command handler.
- `crates/jcode-tui/src/tui/app/remote/key_handling.rs`
  - Route active Queue overlay keys before normal chat input in the remote/default app path.
- `crates/jcode-tui/src/tui/app.rs`
  - Add Queue overlay state field and expose it through `TuiState`.
- `crates/jcode-tui/src/tui/mod.rs`
  - Add a `TuiState` accessor for Queue overlay state, unless rendering can access it through an existing generic overlay path.
- `crates/jcode-tui/src/tui/ui.rs`
  - Render Queue overlay before normal chat/panes and return afterward.
- New file, likely `crates/jcode-tui/src/tui/queue_board.rs`
  - Embedded Queue Board overlay state, render function, navigation, close/refresh actions.

Optional:

- `crates/jcode-tui/src/tui/app/commands_queue.rs`
  - Keep `/queue` parsing/handling separate from the large generic command file.
- `crates/jcode-tui/src/tui/app/tests/...`
  - Add focused tests for `/queue` opening the overlay and `Esc`/`q` closing it.
- `docs/QUEUE_MODE.md`
  - Update after implementation to mention main-app `/queue`.
- `docs/QUEUE_TUI_KANBAN_PLAN.md`
  - Mark read-only main overlay checkpoint complete after implementation.

Risky:

- `src/cli/commands.rs`
  - Avoid for the read-only overlay. Moving shared Queue mutation/refresh helpers out of CLI belongs in a later branch.
- `src/cli/commands/queue_board_tui.rs`
  - Avoid direct modification unless extracting pure render/navigation code. The file currently owns standalone terminal-loop behavior.
- `crates/jcode-base/src/queue.rs`
  - Safe for small pure helpers, but avoid changing storage semantics in the overlay branch.
- `crates/jcode-app-core/src/server/*`
  - Do not touch for read-only overlay.
- Provider/session/agent files
  - Do not touch for read-only overlay.

## Open Questions

- Should `/queue` read queue state from the client process current working directory, the active session working directory, or a server-provided workspace path? The Queue storage helper currently resolves from `std::env::current_dir()`.
- In remote/shared-server mode, is the client working directory guaranteed to be the project directory the user expects?
- Should the first overlay show a soft error if `.jcode/queue/queue.json` does not exist, or should it call `queue::load()` and create empty queue storage like the CLI does?
- Should read-only refresh call only `queue::load()`/`queue::load_run_index()`, or should it also reconcile completed background runs? Reconciliation currently depends on CLI helper code.
- Should active runs display include `RunStatus::Unknown`, matching `RunIndex::active_runs()`, or only `Running` for a simpler first view?
- Is `/queue` the final user-facing command name, or should `/kanban` be the precise entry and `/queue` later become a broader Queue Mode menu?
- Should the overlay support a worker-profile filter in the first version, and if so, what syntax should be accepted: `/queue`, `/queue coder`, or `/queue --worker-profile coder`?
- Should the read-only overlay live in `jcode-tui`, or should a small reusable Queue board widget crate be introduced after the first native integration proves useful?

## Verification For This Documentation Branch

Required command:

```text
git diff --check
```

Do not run:

- `cargo fmt`
- `cargo build`
- broad test/build commands

## Rollback

This branch only adds documentation. To roll back:

```text
git restore docs/QUEUE_MAIN_TUI_INTEGRATION_PLAN.md
```

If the file has already been committed, revert that commit instead of touching application code.
