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
