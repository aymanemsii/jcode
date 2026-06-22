# Aymane's jcode Fork Roadmap

Goal:
Turn jcode into a personal AI workday manager.

Core direction:
I want jcode to become a tool where I can give it tasks, assign/customize agents per project, see active agents, and review work through a queue/kanban-style TUI.

Planned features:
1. Project-local custom workers - CLI MVP implemented
2. Queue mode - CLI MVP implemented
3. Kanban-style TUI view
4. Active agents panel
5. Review inbox
6. Sequential task runner
7. Parallel task execution later

## Queue Mode CLI MVP Status

Implemented and smoke-tested:
- Queue Mode CLI MVP is implemented.
- Project-local storage was fixed and validated.
- Smoke test passed for `queue init`, worker config, task add, dashboard, dry-run, execute, local `queue.json`, local handoffs/runs, review, and approve.

Current limitations:
- Execution is synchronous foreground only.
- No background daemon yet.
- No parallel agents yet.
- No TUI/Kanban yet.

Important rule:
Do not rush into source-code changes. First understand the architecture, then add features in small safe steps.
