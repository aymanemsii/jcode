# jcode Lab Notes

Setup notes:
- Fork cloned successfully.
- origin points to my fork.
- upstream points to original jcode repo.
- Release build works.
- Debug help command caused a stack overflow on Windows, so for now use target/release/jcode.exe.
- cargo test appeared to hang on some provider/auth/Ollama-related tests, so use cargo check as the basic setup check for now.

Current safe command:
cargo build --release

Current usable binary:
.\target\release\jcode.exe

## Queue Mode CLI MVP

Status:
- Queue Mode CLI MVP is implemented.
- Project-local storage was fixed and validated.
- Smoke test passed for `queue init`, worker config, task add, dashboard, dry-run, execute, local `queue.json`, local handoffs/runs, review, and approve.

Current limitations:
- Execution is synchronous foreground only.
- No background daemon yet.
- No parallel agents yet.
- No TUI/Kanban yet.
