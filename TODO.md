# TODO

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
