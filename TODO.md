# TODO

## Task 32 - Add app identity config

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added optional app identity config through `[app]` with `name` and `terminal_title` fields.
* Added safe fallback behavior for missing or blank identity fields.
* Used `app.name` as a fallback startup splash title when no explicit splash title is configured.
* Updated config visibility and customization docs.
* Kept the slice limited to configurable identity without hard-renaming the binary, config directory, environment variables, crates, packages, Queue behavior, server protocol behavior, wallpaper, top bar, or broad app branding changes.

Validation:

* Manual validation will be run separately after this change.

## Task 30 - Add config edit command

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode config edit` for opening the global user config file in the user's preferred editor.
* Created the config directory/file when missing.
* Used `$VISUAL`, `$EDITOR`, and platform fallback behavior for editor selection.
* Updated customization docs with the edit command.
* Kept the command limited to global config editing without project-local customization, theme import/export, wallpaper, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.

## Task 1 - Investigate why `jcode.exe` stays alive after `/quit`

Task Type: Investigation

Status: Completed

Priority: High

Context:
When running `jcode` on Windows, using `/quit` exits the visible TUI/client, but a `jcode.exe` process remains alive. Process inspection shows the remaining process uses a command line like:

```text
jcode.exe --provider auto serve
```

This may be normal upstream server behavior, but we need to verify that from the code before deciding whether to change anything.

Investigation goal:
Find out exactly why the background `jcode.exe --provider auto serve` process remains alive after `/quit`.

Questions to answer:

* What starts the `serve` process?
* Is it intentionally detached from the TUI/client?
* Does `/quit` only close the client UI?
* Is there existing shutdown logic for the background server?
* Is the persistent server required for normal `jcode` behavior?
* Is this expected upstream behavior or a Windows-specific issue?
* Would changing this risk breaking provider/session behavior?

Evidence to collect:

* Relevant files/functions that start the server process.
* Relevant files/functions that handle `/quit`.
* Relevant lifecycle/shutdown behavior.
* Any comments, docs, or code patterns showing this is intentional.

Acceptance criteria:

* We understand whether this is expected behavior or a bug.
* We know which files/functions are involved.
* We can decide whether to leave it alone, document it, or create a small safe fix later.
* No code changes are made during this investigation task.

Investigation result:

* `jcode.exe --provider auto serve` is the shared background server.
* `/quit` exits only the TUI/client and does not request server shutdown.
* The server is intentionally spawned as a shared process that can outlive individual clients.
* Startup flow is mainly in `src/cli/dispatch.rs`, especially `run_default_command` and `spawn_server`.
* The `serve` command is defined through CLI args/dispatch.
* `/quit` handling is in `crates/jcode-tui/src/tui/app/remote/key_handling.rs` and sets `app.should_quit = true`.
* The remote TUI loop exits in `crates/jcode-tui/src/tui/app/run_shell.rs`.
* Server lifecycle logic is in `crates/jcode-app-core/src/server.rs`.
* Normally, the server should self-exit after about 5 minutes with zero clients.
* In debug-control/self-dev mode, that idle timeout is disabled.
* Launching from inside the `jcode` repo can enable self-dev and set `JCODE_DEBUG_CONTROL=1`, explaining why the server may stay alive indefinitely in this workspace.
* Manual shutdown exists through `jcode server stop --force`.

Decision:
Do not auto-stop the server on `/quit`. This could break expected shared server behavior, multi-client sessions, headless/swarm work, provider/session state, reconnect behavior, or session ownership.

Follow-up idea:
Later, consider documentation or UI wording that explains:

* `/quit` exits the client only.
* The shared server may remain alive.
* Use `jcode server stop --force` only when intentionally terminating the shared server.

Notes:
Before release builds on Windows, kill existing `jcode.exe` processes if the binary is locked:

```powershell
taskkill /IM jcode.exe /F 2>$null
```

## Task 2 - Investigate safest solution for intentional server shutdown

Task Type: Investigation / Proposal

Status: Completed

Priority: High

Context:
Task 1 confirmed that `/quit` should remain client-only. The remaining issue is user experience: if the shared background server stays alive, users need a safe and discoverable way to intentionally stop it.

Investigation result:

* Existing shutdown already exists through `jcode server stop --force`.
* The implementation is in `src/cli/commands.rs`.
* Without `--force`, the command refuses and warns that stopping the daemon can drop live headless/swarm sessions.
* With `--force`, it finds the server through the registry/socket.
* On Windows, it uses the existing platform termination logic rather than requiring manual `taskkill`.
* No existing graceful shutdown RPC/protocol command was found.
* Adding a direct TUI shutdown command would require more risky crate/layering changes.
* Changing `/quit` to stop the server is not recommended.
* Changing self-dev/debug-control idle timeout behavior is also risky.

Candidate solutions:

1. Document `jcode server stop --force`.
2. Add an informational TUI slash command such as `/server-stop` that explains the risk and shows the exact CLI command to run.
3. Add a confirmed TUI command that invokes the stop logic directly.
4. Add `/quit --shutdown`.
5. Change `/quit` behavior automatically.

Decision:
The safest immediate solution is documentation only.

Best future UX improvement:
Add `/server-stop` as an informational slash command only. It should not kill the server directly. It should explain that the shared server may outlive the TUI and tell the user to run:

```powershell
jcode server stop --force
```

Recommended wording:
`/server-stop` is better than `/shutdown`, `/quit-server`, or `/quit --shutdown` because it mirrors the existing CLI command and makes the target clear.

Risks / things to avoid:

* Do not make `/quit` stop the server.
* Do not add a shutdown RPC unless there is a broader need.
* Do not bypass the existing `--force` warning semantics.
* Do not rely on manual `taskkill` for normal UX.
* Do not hide that stopping the server can drop headless/swarm work.

Final recommendation:
For now, document the existing command. Later, if needed, implement `/server-stop` as an informational slash command that surfaces the existing safe manual shutdown path.

## Task 3 - Document shared server shutdown behavior

Task Type: Documentation

Status: Completed

Priority: High

Goal:
Document the current `/quit` and background server behavior so future users/developers understand that `/quit` exits the client only, while the shared `jcode.exe --provider auto serve` process may remain alive.

Planned documentation should explain:

* `/quit` exits the TUI/client.
* `/quit` does not stop the shared background server.
* The background server may outlive individual clients.
* In normal mode, the server may self-exit after an idle timeout.
* In self-dev/debug-control mode, idle shutdown may be disabled.
* To intentionally stop the server, use:

```powershell
jcode server stop --force
```

* Stopping the server can drop live headless/swarm sessions.
* Do not use manual `taskkill` as the normal user-facing shutdown path.
* Do not change `/quit` behavior.

Acceptance criteria:

* `TODO.md` clearly records the documentation task.
* `docs/SERVER_ARCHITECTURE.md` documents `/quit` as client-only behavior.
* `docs/SERVER_ARCHITECTURE.md` documents `jcode server stop --force` as the intentional shared-server shutdown path.
* No Rust/source/config files are changed.

Result:
Documented the shared server shutdown behavior in `docs/SERVER_ARCHITECTURE.md`.

## Task 4 - Investigate Queue storage foundation implementation

Task Type: Investigation / Implementation Planning

Status: Completed

Priority: High

Goal:
Investigate the cleanest, smallest implementation path for the first Queue Mode foundation slice.

Planned foundation scope:

* Project-local `.jcode/queue/tasks.json`
* Minimal task schema:

  * id
  * title
  * body
  * status
  * priority
  * created_at
  * updated_at
  * optional worker_profile
* CLI-only commands:

  * `jcode queue init`
  * `jcode queue add`
  * `jcode queue list`

Questions to answer:

* Where are existing CLI commands defined?
* Where should a new `queue` command group be added?
* Is there an existing pattern for subcommands similar to this?
* Where should queue storage code live?
* What crates/modules should be touched for the smallest safe implementation?
* What dependencies already exist for JSON serialization, timestamps, IDs, and filesystem paths?
* What should the first implementation slice include?
* What should be explicitly deferred?

Acceptance criteria:

* The investigation identifies the exact files/functions likely involved.
* The implementation plan is small and CLI-only.
* No TUI, worker execution, background runs, or visual/theme changes are included.
* No source code is changed during this task.

Investigation result:

* Existing CLI commands are defined in `src/cli/args.rs`.
* Runtime dispatch happens in `src/cli/dispatch.rs`.
* Command implementation mostly lives in `src/cli/commands.rs`, with larger commands split under `src/cli/commands/`.
* `--cwd` is applied before dispatch, so queue storage can use `std::env::current_dir()` safely.
* The recommended queue storage module location is `crates/jcode-base/src/queue.rs`.
* `crates/jcode-base/src/lib.rs` should re-export it with `pub mod queue;`.
* CLI-facing queue implementation should likely live in a small new `src/cli/commands/queue.rs`.
* Existing usable dependencies include `serde`, `serde_json`, `chrono`, `uuid` or `crate::id::new_id`, `std::fs`, `PathBuf`, `anyhow`, and existing storage helpers.
* Queue storage should be project-local at `./.jcode/queue/tasks.json`.
* First implementation slice should include only `jcode queue init`, `jcode queue add`, and `jcode queue list`.

Decision:
Implement Queue foundation as a small reusable `jcode-base` storage module plus thin CLI wiring. Keep the first slice CLI-only, project-local, JSON-backed, typed, and boring.

Deferred:

* TUI integration.
* Worker execution.
* Background runs.
* Server protocol changes.
* Debug socket support.
* Visual/theme changes.
* Task claiming/locking.
* Multi-project/global queue discovery.
* Reusing or modifying swarm/ambient/safety queues.
* Any implementation copied from old Queue work.

## Task 6 - Add informational `/server-stop` slash command

Task Type: Implementation / UX

Status: Completed

Priority: Medium

Result:

* Added `/server-stop` as an informational TUI slash command.
* The command does not stop the server directly.
* The command explains that `/quit` exits only the TUI/client.
* The command tells users to run `jcode server stop --force` to intentionally stop the shared server.
* The command warns that stopping the server can drop live headless/swarm sessions.

Validation:

* Manual validation will be run separately after this change.
* Do not record validation as passed unless validation was actually run outside Codex.

## Task 7 - Implement Queue CLI foundation

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Added project-local queue storage at `./.jcode/queue/tasks.json`.
* Added reusable queue storage module in `crates/jcode-base/src/queue.rs`.
* Re-exported queue module from `crates/jcode-base/src/lib.rs`.
* Added CLI parsing for `jcode queue init`, `jcode queue add`, and `jcode queue list`.
* Added thin CLI command handling.
* Wired queue command dispatch through CLI args/dispatch/command exports as needed.
* `queue init` creates storage if missing and preserves existing tasks.
* `queue add` auto-initializes storage if missing.
* `queue list` prints a simple readable task list.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 8 - Add Queue show command and list polish

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode queue show <id>` for inspecting a single queued task.
* Improved `jcode queue list` readability while keeping output simple.
* Kept Queue storage project-local at `./.jcode/queue/tasks.json`.
* Did not add execution, background workers, TUI, or server protocol changes.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 9 - Add Queue status command

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode queue status <id> <status>` for manually updating queued task status.
* Supported statuses: `ready`, `running`, `done`, and `failed`.
* Updated `updated_at` when task status changes.
* Kept Queue storage project-local at `./.jcode/queue/tasks.json`.
* Did not add execution, background workers, TUI, Queue Board, or server protocol changes.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 10 - Add Queue archive command

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode queue archive <id>` for preserving queued tasks while hiding them from the default active list.
* Added optional `archived_at` metadata to queue tasks.
* Added `jcode queue list --all` to include archived tasks.
* Updated `jcode queue show <id>` to display `archived_at` when present.
* Kept Queue storage project-local at `./.jcode/queue/tasks.json`.
* Did not add deletion, execution, background workers, TUI, Queue Board, or server protocol changes.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 11 - Add Queue next command

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode queue next` for displaying the next active ready task.
* Skipped archived tasks and tasks marked `running`, `done`, or `failed`.
* Kept `queue next` read-only; it does not claim, run, lock, or modify tasks.
* Kept Queue storage project-local at `./.jcode/queue/tasks.json`.
* Did not add execution, claiming, background workers, TUI, Queue Board, or server protocol changes.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 12 - Add Queue edit command

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode queue edit <id>` for editing queued task metadata.
* Supported editing title, body, priority, and worker profile.
* Added `--clear-worker-profile`.
* Updated `updated_at` when a task is edited.
* Kept Queue storage project-local at `./.jcode/queue/tasks.json`.
* Did not add execution, claiming, background workers, TUI, Queue Board, or server protocol changes.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 13 - Investigate Queue runner architecture

Task Type: Investigation

Status: Completed

Priority: High

Result:

* Investigated the safest architecture for future manual Queue task execution.
* Documented recommended scope for a future `jcode queue run <id>` or `jcode queue run-next` slice.
* Identified status transitions, storage/logging considerations, and safety boundaries.
* Explicitly deferred background workers, claiming, Queue Board, TUI, server protocol changes, scheduling, retries, and parallel execution.

Validation:

* Documentation-only change.
* Manual review will be run separately after this change.

## Task 14 - Add manual Queue run command

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Added `jcode queue run <id>` for manually running one queued task in the foreground.
* Kept execution explicit-id-only and current-process.
* Updated task status from `ready` to `running`, then to `done` or `failed`.
* Rejected archived tasks and non-ready tasks.
* Kept Queue storage project-local at `./.jcode/queue/tasks.json`.
* Did not add claiming, locks, background workers, Queue Board, TUI, server protocol changes, scheduling, retries, or `run-next`.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 15 - Add Queue run history command

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added `jcode queue runs <id>` for listing recorded run metadata for one queue task.
* Kept the command read-only.
* Displayed run id, status, started_at, finished_at or unfinished, and error summaries when present.
* Reused project-local Queue run metadata under `./.jcode/queue/runs/<task-id>/`.
* Did not change Queue execution, claiming, locks, background workers, Queue Board, TUI, server protocol, scheduling, retries, or `run-next`.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 16 - Add Queue reset-running command

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Added `jcode queue reset-running <id>` for recovering interrupted manual Queue runs.
* Reset only non-archived tasks currently in `running` back to `ready`.
* Preserved task content, priority, worker_profile, created_at, archived_at, and run metadata.
* Rejected missing, archived, and non-running tasks with clear errors.
* Did not change Queue execution, run history, claiming, locks, background workers, Queue Board, TUI, server protocol, scheduling, retries, rerun, or `run-next`.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 17 - Document Queue v1 workflow

Task Type: Documentation

Status: Completed

Priority: High

Result:

* Added `docs/QUEUE.md` documenting the Queue v1 workflow, commands, storage layout, run metadata, manual execution model, and recovery flow.
* Documented that Queue v1 is manual, foreground-only, explicit-id-only, and project-local.
* Documented deferred Queue v2 work including run-next, claiming, locks, background workers, Queue Board/TUI, server protocol changes, scheduling, retries, worker_profile mapping, and multi-agent execution.

Validation:

* Documentation-only change.
* Manual review will be run separately after this change.

## Task 18 - Document Queue v2 architecture

Task Type: Investigation

Status: Completed

Priority: High

Result:

* Added `docs/QUEUE_V2_ARCHITECTURE.md` documenting the safe Queue v2 architecture direction.
* Documented future claiming, locking, run-next, worker profiles, retries, scheduling, background workers, Queue Board/TUI, and multi-agent execution.
* Recommended a phased implementation order that avoids adding background workers or run-next before claim and lock semantics are designed.
* Kept Queue v2 as architecture only with no implementation changes.

Validation:

* Documentation-only change.
* Manual review will be run separately after this change.

## Task 19 - Investigate jcode customization architecture

Task Type: Investigation

Status: Completed

Priority: High

Result:

* Added `docs/JCODE_CUSTOMIZATION_ARCHITECTURE.md` documenting a safe architecture direction for jcode customization.
* Investigated theme, visual styling, wallpaper feasibility, configuration options, multi-session UI direction, and future multi-agent/agent mixture surface direction.
* Recommended a phased implementation order that starts with small configuration and theme slices before broad TUI changes.
* Kept this as documentation only with no implementation changes.

Validation:

* Documentation-only change.
* Manual review will be run separately after this change.

## Task 20 - Add global accent color customization

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Added a minimal global accent color configuration path.
* Routed the existing semantic `accent_color()` path through the configured accent color when valid.
* Preserved the existing default accent color when no config is set or the configured value is invalid.
* Kept the change narrow and did not add named themes, wallpaper, project-local customization, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 21 - Add customization config visibility

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added a read-only config visibility path for safe display/customization settings.
* Displayed configured `display.accent_color`, whether it is valid, the active accent color, and fallback/default behavior.
* Avoided printing secrets or raw full configuration.
* Did not add config editing, named themes, wallpaper, project-local customization, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 22 - Add named theme config MVP

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added a minimal `display.theme` configuration field with built-in `default`, `dark`, and `high-contrast` theme names.
* Used the active theme only to choose the default accent color fallback when `display.accent_color` is missing or invalid.
* Preserved explicit valid `display.accent_color` as the highest-priority override.
* Updated `jcode config show` to display theme validity, active theme, accent validity, active accent color, and fallback behavior.
* Did not add full palette theming, wallpaper, project-local customization, config editing, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 23 - Add theme palette MVP

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added a small built-in theme palette path for central semantic TUI colors.
* Preserved the default theme behavior and kept explicit `display.accent_color` as the highest-priority accent override.
* Applied `default`, `dark`, and `high-contrast` themes only through existing central semantic color functions.
* Avoided broad TUI recoloring, direct color migration, wallpaper, project-local customization, config editing, Queue changes, server protocol changes, and layout refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 24 - Add startup splash MVP

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added an opt-in `display.startup_splash` configuration field.
* Added a minimal theme-aware TUI startup splash/background panel for the empty/startup state when enabled.
* Kept default behavior unchanged when `display.startup_splash` is missing or false.
* Updated `jcode config show` to display `display.startup_splash`.
* Did not implement true wallpaper, terminal image backgrounds, animation, layout refactors, Queue changes, server protocol changes, multi-session, or multi-agent behavior.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 25 - Add famous built-in themes

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added famous built-in theme presets for `display.theme`, including Dracula, Tokyo Night, Gruvbox, Nord, Catppuccin, Kanagawa, Everforest, Ayu, One Dark, Matrix, Vercel, and Cursor-inspired themes.
* Kept the themes limited to the existing central semantic palette.
* Preserved default theme behavior and kept explicit valid `display.accent_color` as the highest-priority accent override.
* Did not add custom theme files, wallpaper, project-local customization, config editing, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 26 - Add custom named themes MVP

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Added custom named theme support through a `[themes]` config table.
* Supported custom semantic palette fields for accent, user, assistant, tool, system, queued, asap, and pending colors.
* Preserved built-in themes and kept built-in names reserved.
* Preserved explicit valid `display.accent_color` as the highest-priority accent override.
* Handled missing and invalid custom theme color fields with safe field-level fallback behavior.
* Updated `jcode config show` to report active theme source and custom theme fallback details.
* Did not add theme import/export, project-local customization, config editing, wallpaper, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 27 - Add Theme Palette V2 fields

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Expanded the theme palette with UI chrome fields for background, foreground, muted, border, active_border, panel, input, selection, success, warning, and error colors.
* Added custom theme config support for the new optional fields.
* Added safe field-level fallback behavior for missing and invalid custom colors.
* Added reasonable UI chrome colors for built-in themes.
* Routed a small, low-risk set of UI chrome styling through the centralized theme palette.
* Preserved built-in theme behavior, custom named themes, reserved built-in names, and explicit valid display.accent_color as the highest-priority accent override.
* Did not add wallpaper, project-local customization, import/export, config editing, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.
* Do not mark validation as passed unless it was actually run outside Codex.

## Task 28 - Add startup splash personalization

Task Type: Implementation

Status: Completed

Priority: Medium

Result:

* Added optional startup splash title, subtitle, and footer customization through display config.
* Preserved existing startup splash visibility behavior.
* Added safe fallback behavior for missing or blank splash text fields.
* Updated config visibility and default config examples.
* Kept the slice limited to startup splash personalization without wallpaper, project-local customization, import/export, config editing, Queue changes, server protocol changes, or broad TUI refactors.

Validation:

* Manual validation will be run separately after this change.

## Task 29 - Document customization v2 workflow

Task Type: Documentation

Status: Completed

Priority: Medium

Result:

* Added `docs/JCODE_CUSTOMIZATION.md` documenting customization v2 configuration, built-in themes, custom named themes, Theme Palette V2 fields, accent override precedence, startup splash personalization, config visibility, fallback behavior, and deferred work.
* Kept the slice documentation-only.

Validation:

* Documentation-only change.
* Manual review will be run separately after this change.

## Task 31 - Remove telemetry and audit network behavior

Task Type: Implementation

Status: Completed

Priority: High

Result:

* Removed or disabled telemetry behavior for this fork.
* Removed the user-facing telemetry notice/banner.
* Ensured telemetry collection and sending are inactive by default.
* Audited outbound network-capable paths and documented the fork's network/privacy policy.
* Added `docs/NETWORK_PRIVACY.md` explaining that external requests should be limited to configured AI providers and explicit user-enabled tools/actions.
* Did not change AI model/provider behavior, Queue behavior, server protocol behavior, Cargo files, broad app identity, or rename behavior.

Validation:

* Manual validation will be run separately after this change.
