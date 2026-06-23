# Jcode UI Visual Diagnosis

Date: 2026-06-23

## Scope

This diagnosis compares the local fork against the original upstream repository through git inspection only. Upstream default branch is `master`; `upstream/main` is not present.

Commands used for orientation included:

- `git remote -v`
- `git branch -r`
- `git symbolic-ref refs/remotes/upstream/HEAD`
- `git ls-tree -r upstream/master --name-only`
- `git show upstream/master:README.md`
- `git show upstream/master:<path>`

No upstream checkout, merge, rebase, cargo build, or cargo fmt was performed.

## 1. What Launches When Running `jcode`

The binary entrypoint is `src/main.rs`. It configures allocator/runtime details, handles the macOS hotkey listener special case, builds a Tokio runtime, then calls `jcode::run().await`.

The CLI dispatcher is `src/cli/dispatch.rs`. With no subcommand, `run_default_command` is used. That path:

- resolves startup hints and crash-resume hints;
- detects whether the current directory is the jcode repo and may enable self-dev mode;
- checks whether the server is already running;
- spawns a background `jcode serve` process if no server is available;
- launches the interactive remote TUI client through `src/cli/tui_launch.rs`.

The main interactive TUI entrypoint is `tui_launch::run_tui_client`. It initializes the terminal runtime, sets the terminal title, creates `tui::App::new_for_remote_with_options(...)`, applies startup hints, then calls `app.run_remote(terminal).await`.

Startup and status rendering is split across the native TUI modules under `crates/jcode-tui/src/tui/`, especially:

- `ui_header.rs`: persistent centered header, provider/model/version/auth summary lines.
- `ui_input.rs`: prompt/composer, command suggestions, queued-message preview, status-adjacent notices.
- `ui_status.rs`: debug/status wording for idle, connecting, streaming, thinking, tool-running states.
- `info_widget*.rs`: right/left negative-space widgets for model/context/git/todos/usage/tips/swarm/etc.
- `ui_viewport.rs` and `info_widget_layout.rs`: message viewport and placement of info widgets into available negative space.

Provider/model/status widgets are native jcode UI. The right-side info widget seen locally is expected behavior when there is enough negative space. Provider timeout or login/setup state can make the screen look sparse because the transcript contains little content and the app is emphasizing status, onboarding, or retry messages rather than an active conversation. That is a normal startup/needs-attention state, not evidence that the app failed to load a richer visual skin.

## 2. Why Local UI May Look Different From Upstream Screenshots

The upstream README is on `upstream/master`. It embeds several remote release assets and GitHub user-attachment screenshots. The polished demo media in the README is not proof that jcode itself renders a wallpaper.

I did not find a jcode code path that paints a black-hole/space bitmap background in the TUI. The TUI is Ratatui/crossterm text rendering over terminal cells. Native code styles foreground/background colors for spans, input, headers, widgets, and panels, but it does not draw a full-screen photographic background.

I also did not find an obvious in-repo black-hole or space wallpaper asset. Upstream assets include demo videos/images such as `assets/demos/*.mp4`, `assets/demos/jcode-vs-claude-code.png`, `assets/niri-screenshot.png`, `assets/readme/100-sessions-spawn-demo.gif`, docs images, and desktop gallery golden screenshots. The README's primary media is hosted through GitHub release assets or user attachments.

The black-hole/space look is therefore most likely one of:

- terminal transparency showing a desktop wallpaper or compositor background;
- a specific terminal emulator/window-manager/recording setup;
- README/demo production context rather than a jcode-rendered surface.

Handterm is mentioned in the README for native scroll API work. The README says normal-terminal scrolling is still implemented. Handterm appears related to terminal capabilities and demo/runtime quality, not a required dependency for the space/black-hole background.

Windows Terminal can plausibly reproduce a similar atmosphere outside jcode by configuring acrylic/transparency or a background image. That should be tested separately from jcode because jcode itself is not responsible for drawing a photographic background.

Jcode config does control UI alignment. The README and code indicate jcode is left-aligned by default, and centered mode can be toggled with `Alt+C`, `/alignment`, or `[display] centered = true/false` in config. CLI flags also support `--centered` and `--no-centered` for relevant paths. Alignment changes layout and negative-space distribution; it does not add a wallpaper or demo background.

Provider timeout/login state can explain a sparse, ugly local first impression. If the app is waiting on a provider, showing a timeout, or in a first-run/deferred-auth state, there may be little transcript content, making the terminal's plain black background and right-side widget dominate the screen.

## 3. Native Jcode UI Inventory

Core native colors live in `crates/jcode-tui-style/src/theme.rs` and are re-exported for the main TUI through `crates/jcode-tui/src/tui/ui_theme.rs`.

Important color/style constants include:

- `user_color`: light blue.
- `ai_color`: green.
- `tool_color`: muted gray.
- `file_link_color`: pale blue.
- `dim_color`: dark gray for receding metadata.
- `accent_color`: purple accent.
- `system_message_color`: pink.
- `queued_color`: yellow.
- `asap_color`: cyan.
- `pending_color`: gray.
- `user_text`, `user_bg`, `ai_text`.
- `header_icon_color`, `header_name_color`, `header_session_color`.

Text styling patterns:

- muted metadata via `dim_color`;
- active/selected/action states through restrained hue changes rather than large blocks;
- bold sparingly for emphasis;
- no heavy always-visible chrome around normal transcript content.

Borders/panels/widgets:

- the main chat is mostly sparse text and negative space;
- right-side info widgets are placed into unused margin space rather than fixed dashboard panels;
- reusable right-rail chrome exists in `crates/jcode-tui-render/src/chrome.rs` through a left border and compact title/content split;
- overlays such as account/usage pickers use framed areas, but normal status content does not become a full dashboard.

Footer/help/status patterns:

- status notices are short and transient;
- key hints are compact;
- prompt/status lines stay close to the composer instead of becoming a large command legend.

Input prompt styling:

- `ui_input.rs` uses `> ` with `user_color` for normal chat;
- `$ ` with shell-mode green for shell mode;
- an ellipsis prompt with `queued_color` while processing;
- an accent prompt when a skill is active;
- command suggestions use compact inline color and dim descriptions.

Selection/highlight patterns:

- native UI tends to use subtle color, bold, dimming, and narrow indicators;
- it does not rely on broad table-like highlight bands as a primary visual identity.

Sparse/negative-space patterns:

- message content is allowed to breathe;
- info widgets fill only available negative space and move out of the way;
- left-aligned mode leaves a small inset and often more right-side free space;
- centered mode creates symmetric margins and lets widgets use either side.

Reusable components or patterns Queue Board should use:

- palette from `jcode-tui-style::theme`;
- `dim_color`, `queued_color`, `asap_color`, `pending_color`, `accent_color`, `user_color`, `ai_color`, `tool_color`;
- right-rail chrome pattern from `jcode-tui-render/src/chrome.rs` if a detail pane is needed;
- prompt treatment from `ui_input.rs`;
- compact status/footer wording from `ui_status.rs` and `ui_input.rs`;
- negative-space/detail-widget thinking from `info_widget_layout.rs`.

## 4. Queue Board Visual Gap

The standalone Queue Board is implemented in the local fork at `src/cli/commands/queue_board_tui.rs`. The command is wired from:

- `src/cli/args.rs`: parses `jcode queue board --tui`;
- `src/cli/dispatch.rs`: dispatches `QueueCommand::Board`;
- `src/cli/commands.rs`: loads queue/run state and calls `queue_board_tui::run_read_only_queue_board(...)`.

This Queue Board work does not exist on `upstream/master`. Upstream has the native TUI style crate and info widgets, but no fork Queue Mode board implementation to copy.

Current Queue Board rendering approach:

- owns a standalone Ratatui terminal event loop;
- renders a full-width header block with `Borders::ALL`;
- splits every queue status into equal-width columns;
- renders each column as a bordered block;
- renders active runs in another bordered block;
- puts a single-line footer/help legend at the bottom;
- supports navigation, refresh, auto-refresh, add task, approve, and start selected task.

What makes it feel non-native:

- too many full boxes and borders compared with the main jcode transcript;
- equal-width Kanban columns make it feel like a generic dashboard;
- every status is visible even when empty or low-value, increasing chrome and visual noise;
- active runs are a separate bordered panel rather than a compact status/detail surface;
- selection is board/table-like instead of jcode-like subtle emphasis;
- footer is a command legend rather than the native compact prompt/status idiom;
- it does not use the shared native palette directly;
- it does not use the right-rail/detail-widget language used by the main TUI.

What should not be copied from the GitHub screenshot:

- no fake wallpaper/background rendering;
- no black-hole image inside the TUI;
- no attempt to reproduce a recording/window-manager scene in app code;
- no styling that depends on a specific terminal emulator background.

## 5. Recommended Queue Board Visual Direction

Restyle the standalone board to feel like native jcode text UI, not like a separate dashboard.

Recommended direction:

- sparse layout with fewer always-visible columns;
- minimal columns for the operational states users act on most often, with secondary states collapsed or de-emphasized when empty;
- subtle separators rather than boxes around every region;
- native palette from `jcode-tui-style`;
- selected task indicated by narrow marker, brighter text, or restrained accent/bold;
- dim metadata and low-priority states with `dim_color`;
- use `queued_color`/`pending_color`/`asap_color`/`ai_color` semantically where they match queue states;
- compact native footer/help style;
- native input prompt style when adding a task;
- optional right-side detail/info rail for the selected task or active run details;
- keep empty space empty instead of filling it with bordered panels;
- preserve behavior and keybindings unless a later branch explicitly changes interaction design.

Avoid:

- fake wallpaper or background image rendering;
- overdesigned borders;
- thick dashboard/table styling;
- copying demo recording atmosphere into app rendering;
- integrating into the main `jcode` app before the standalone board has native visual language.

## 6. Next Implementation Branch

Recommended next branch:

`aymane/queue-tui-native-restyle`

Recommended scope:

- restyle standalone `jcode queue board --tui` only;
- preserve all current behavior;
- keep state loading, refresh, auto-refresh, add, approve, and run-selected behavior intact;
- do not integrate into the main `jcode` TUI yet;
- do not add new Queue features;
- keep the patch narrow and visual only.

## 7. Concrete File List

Required implementation files:

- `src/cli/commands/queue_board_tui.rs`: main standalone board layout/rendering and local tests.

Optional implementation files:

- `src/cli/commands_tests.rs`: only if text output expectations or command-level behavior tests need adjustment.
- `docs/QUEUE_MODE.md`: update screenshots/wording only after restyle behavior is stable.
- `docs/QUEUE_TUI_KANBAN_PLAN.md`: update implementation notes if the plan should reflect the new native visual direction.

Potentially useful but should be approached carefully:

- `crates/jcode-tui-style/src/theme.rs`: risky for this branch because it affects the entire native TUI; avoid unless a missing semantic color is truly needed.
- `crates/jcode-tui-render/src/chrome.rs`: risky/shared; prefer using existing patterns rather than changing them.
- `crates/jcode-tui/src/tui/ui_input.rs`, `ui_status.rs`, `ui_header.rs`, `info_widget*.rs`: risky/shared; inspect and mimic patterns, but do not modify for the standalone restyle unless absolutely necessary.
- `src/cli/args.rs`, `src/cli/dispatch.rs`, `src/cli/commands.rs`: risky for a pure visual branch; should not change unless wiring is already broken.
- `crates/jcode-base/src/queue.rs`: risky and out of scope for visual styling.
- `crates/jcode-app-core`, provider crates, memory, server, sidecar, image, and agent internals: out of scope.

## 8. Manual Checks

Run these manually when evaluating the next styling branch:

1. Run `jcode`.
2. Observe startup with the current provider/account state.
3. Try `/alignment` if supported in the running session.
4. Try `Alt+C` if the terminal passes it through.
5. Run `jcode queue board --tui`.
6. Compare Queue Board visual language against the main jcode TUI: palette, spacing, prompt/status treatment, selection, and negative-space use.
7. Test Windows Terminal acrylic/transparency/background image separately from jcode to confirm the atmospheric background is terminal-level.
8. Compare provider-error/timeout startup state against an active authenticated session.
9. Test Queue Board empty state.
10. Test Queue Board with running tasks.
11. Test Queue Board with review tasks.
12. Test Queue Board with blocked tasks.
13. Test Queue Board with active runs.
14. Test narrow and wide terminal sizes.

Do not use the upstream README/demo background as the target for Queue Board styling. Use native jcode text UI patterns as the target.

## Bottom Line

The local plain black background is likely normal terminal background plus sparse startup/status content. The polished upstream GitHub look is probably demo media shown through terminal/window-manager/recording context, not a jcode-rendered black-hole background.

Queue Mode should be styled as native jcode text UI: sparse, palette-consistent, low-chrome, selection-focused, and detail-oriented. It should not attempt to render or imitate a wallpaper.
