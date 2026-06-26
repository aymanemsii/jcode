# TODO

## Task 1 - Investigate why `jcode.exe` stays alive after `/quit`

Task Type: Investigation

Status: Not started

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

Notes:
Before release builds on Windows, kill existing `jcode.exe` processes if the binary is locked:

```powershell
taskkill /IM jcode.exe /F 2>$null
```
