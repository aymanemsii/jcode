# Jcode Customization Map

Date: 2026-06-24

This map is based on repository inspection only. No Rust source was changed, no build was run, no formatter was run, and no commit was made.

## 1. What Jcode Can Customize Natively

Jcode has real native customization surfaces, but it is not currently a fully themeable app. The main user-facing configuration lives in `~/.jcode/config.toml`, represented by `crates/jcode-config-types/src/lib.rs` and loaded through `crates/jcode-base/src/config/config_file.rs`.

### Alignment

Relevant code:

- `crates/jcode-config-types/src/lib.rs`
- `crates/jcode-base/src/config/default_file.rs`
- `crates/jcode-tui/src/tui/app/commands.rs`
- `crates/jcode-tui/src/tui/keybind.rs`
- `crates/jcode-tui/src/tui/ui_viewport.rs`

Controls:

- `[display].centered = false` by default.
- `JCODE_DISPLAY_CENTERED=true|false` overrides config.
- `Alt+C` toggles centered mode for the current session.
- `/alignment` shows current and saved alignment.
- `/alignment centered` and `/alignment left` apply immediately and persist through `Config::set_display_centered`.
- Some replay/export paths expose `--centered` and `--no-centered`.

Behavior:

- Left-aligned mode is the default.
- Centered mode changes the content block and margin distribution.
- Info widgets and inline images use the current margin model, so alignment affects where negative-space UI can appear.
- Alignment does not add a wallpaper, background image, or full-screen skin.

### Display and UI Config

`[display]` currently exposes these appearance-affecting fields:

- `diff_mode = "off" | "inline" | "full-inline" | "pinned" | "file"`
- `queue_mode = true|false`
- `auto_server_reload = true|false`
- `mouse_capture = true|false`
- `debug_socket = true|false`
- `centered = true|false`
- `show_thinking = true|false`
- `reasoning_display = "off" | "full" | "current"`
- `diagram_mode = "none" | "margin" | "pinned"`
- `markdown_spacing = "compact" | "document"`
- `pin_images = true|false`
- `idle_animation = true|false`
- `prompt_entry_animation = true|false`
- `disabled_animations = ["donut", ...]`
- `diff_line_wrap = true|false`
- `performance = "auto" | "full" | "reduced" | "minimal"`
- `animation_fps = 1..120`
- `redraw_fps = 1..120`
- `prompt_preview = true|false`
- `copy_badge_alt_label = "" | "Alt" | "Option" | ...`
- `[display.native_scrollbars].chat = true|false`
- `[display.native_scrollbars].side_panel = true|false`

Environment overrides include `JCODE_DIFF_MODE`, `JCODE_PIN_IMAGES`, `JCODE_DISPLAY_CENTERED`, `JCODE_DIFF_LINE_WRAP`, `JCODE_QUEUE_MODE`, `JCODE_MOUSE_CAPTURE`, `JCODE_DEBUG_SOCKET`, `JCODE_REASONING_DISPLAY`, `JCODE_MARKDOWN_SPACING`, `JCODE_IDLE_ANIMATION`, `JCODE_PROMPT_ENTRY_ANIMATION`, `JCODE_DISABLED_ANIMATIONS`, `JCODE_PERFORMANCE`, `JCODE_ANIMATION_FPS`, `JCODE_REDRAW_FPS`, `JCODE_COPY_BADGE_ALT_LABEL`, `JCODE_CHAT_NATIVE_SCROLLBAR`, and `JCODE_SIDE_PANEL_NATIVE_SCROLLBAR`.

### Slash Commands Related to UI

Relevant code: `crates/jcode-tui/src/tui/app/commands.rs`.

- `/alignment`: show or persist centered/left layout.
- `/reasoning`: show or persist reasoning display mode.
- `/diff`: cycle or set diff display mode.
- `/config`: show current config summary.
- `/config init` or `/config create`: generate the default config file.
- `/config edit`: open the config in `$EDITOR` or platform editor.
- `/queue`: open the main-app read-only Queue Board overlay.
- `/btw`: route an answer into the side panel.
- `/dictate`: run configured dictation command.

There are many provider/session/auth commands that affect header/status content indirectly, especially `/model`, `/account`, `/auth`, `/login`, `/usage`, and provider-specific account settings.

### Keybindings Related to UI

Relevant code:

- `crates/jcode-config-types/src/lib.rs`
- `crates/jcode-base/src/config/default_file.rs`
- `crates/jcode-tui/src/tui/keybind.rs`
- `crates/jcode-tui-core/src/keybind.rs`

Config-backed defaults:

- `scroll_up = "ctrl+k"`
- `scroll_down = "ctrl+j"`
- `scroll_page_up = "alt+u"`
- `scroll_page_down = "alt+d"`
- `model_switch_next = "ctrl+tab"`
- `model_switch_prev = "ctrl+shift+tab"`
- `effort_increase = "alt+right"`
- `effort_decrease = "alt+left"`
- `centered_toggle = "alt+c"`
- `scroll_prompt_up = "ctrl+["`
- `scroll_prompt_down = "ctrl+]"`
- `scroll_bookmark = "ctrl+g"`
- `workspace_left/down/up/right = "alt+h/j/k/l"`
- `side_panel_toggle = "alt+m"`
- `copy_selection_toggle = "alt+y"`
- `diagram_pane_toggle = "alt+t"`
- `typing_scroll_lock_toggle = "alt+s"`
- `diff_mode_cycle = "alt+g"`
- `info_widget_toggle = "alt+i"`
- `session_picker_enter = "current-terminal" | "new-terminal"`

The parser supports `ctrl`, `alt`/`option`/`meta`, `cmd`/`super`/`win`, `shift`, function keys, arrows, page keys, `home`, `end`, `insert`, `delete`, `backspace`, `tab`, `enter`, and comma-separated aliases for workspace navigation. `none`, `off`, or `disabled` disables optional bindings.

### Theme, Style, and Colors

Relevant code:

- `crates/jcode-tui-style/src/theme.rs`
- `crates/jcode-tui-style/src/color.rs`
- `crates/jcode-tui/src/tui/ui_theme.rs`

Native palette is currently code-defined, not config-defined. Important color functions include `user_color`, `ai_color`, `tool_color`, `file_link_color`, `dim_color`, `accent_color`, `system_message_color`, `queued_color`, `asap_color`, `pending_color`, `user_text`, `user_bg`, `ai_text`, `header_icon_color`, `header_name_color`, and `header_session_color`.

There is no discovered `[theme]` table, no wallpaper field, and no user-configurable palette file. A future theme extraction would require code changes.

### Header, Status, Input, Selection, and Widgets

Relevant code:

- `crates/jcode-tui/src/tui/ui_header.rs`
- `crates/jcode-tui/src/tui/ui_status.rs`
- `crates/jcode-tui/src/tui/ui_input.rs`
- `crates/jcode-tui/src/tui/ui_viewport.rs`
- `crates/jcode-tui/src/tui/info_widget*.rs`
- `crates/jcode-tui/src/tui/ui_pinned*.rs`
- `crates/jcode-tui/src/tui/ui_inline_image.rs`

Native surfaces:

- Header renders provider/model/version/auth/session information.
- Status text handles idle, connecting, thinking, streaming, network retry, and running-tool states.
- Input prompt style changes for chat, slash command, shell mode, queued input, and processing.
- Slash-command suggestions use fuzzy highlight spans.
- Info widgets use negative space and can show model, overview, usage, todos, tips, git, memory, swarm, background, and stability information.
- Side panel supports managed pages, markdown, diffs, images, and diagram/image placement.
- Copy/selection mode has persisted inline image visibility in app config.

## 2. Terminal Customization and Setup

Terminal appearance is a major part of perceived visual parity. Jcode controls terminal cell content, colors, raw mode, mouse capture, bracketed paste, keyboard enhancement, image placement, and alt-screen usage. It does not control terminal font, window opacity, acrylic, background image, compositor wallpaper, or window-manager effects.

### Windows Setup Hints

Relevant code:

- `crates/jcode-setup-hints/src/windows_setup.rs`
- `crates/jcode-setup-hints/src/lib.rs`

Windows detection:

- `WT_SESSION` means Windows Terminal.
- `WEZTERM_EXECUTABLE` or `WEZTERM_PANE` means WezTerm.
- `ALACRITTY_WINDOW_ID` means Alacritty.

Windows setup can:

- prompt every third launch, capped by `MAX_TERMINAL_NUDGES`;
- suggest Alacritty as the fastest terminal;
- install Alacritty through `winget install -e --id Alacritty.Alacritty --accept-source-agreements`;
- create an `Alt+;` global hotkey listener;
- create startup shortcut files under `~/.jcode/hotkey/`;
- use Alacritty for hotkey launches when installed, otherwise Windows Terminal;
- create a Windows desktop shortcut.

Setup state is persisted in `~/.jcode/setup_hints.json`.

### Terminal Capability Detection

Truecolor detection lives in `crates/jcode-tui-style/src/color.rs`:

- `COLORTERM=truecolor` or `COLORTERM=24bit` enables truecolor.
- `TERM_PROGRAM=ghostty|iTerm.app|wezterm|warp|alacritty|hyper` enables truecolor.
- `GHOSTTY_RESOURCES_DIR`, `GHOSTTY_BIN_DIR`, `WEZTERM_EXECUTABLE`, or `WEZTERM_PANE` enables truecolor.
- `TERM` containing `kitty`, `ghostty`, or `alacritty` enables truecolor.
- `TERM` containing `256color` falls back to 256-color.
- Otherwise the app uses 256-color.

The color helper maps RGB colors to xterm-256 when truecolor is unavailable.

### Terminal Runtime Behavior

Relevant code: `src/cli/terminal.rs`.

Jcode TUI startup requires interactive stdin/stdout. It initializes ratatui/crossterm, installs Mermaid and markdown hooks, enables bracketed paste, optionally enables focus-change events, optionally enables mouse capture, and may enable keyboard enhancement. Cleanup disables those modes and restores the terminal unless the process is intentionally execing into reload/update/rebuild.

This means terminal state can affect:

- text selection versus mouse wheel scrolling (`mouse_capture`);
- keybinding delivery;
- bracketed paste behavior;
- alt-screen rendering;
- cursor/raw-mode recovery after crashes.

### Terminal Image Protocols

Relevant code:

- `crates/jcode-terminal-image/src/display.rs`
- `crates/jcode-tui/src/tui/ui_inline_image.rs`
- `crates/jcode-tui/src/tui/mermaid.rs`
- `crates/jcode-tui-markdown/src/lib.rs`

Supported terminal image protocols:

- Kitty graphics protocol for Kitty/Ghostty-like environments.
- iTerm2 inline image protocol.
- Sixel for xterm/foot/mlterm/WezTerm/mintty/contour when ImageMagick `convert` is available.
- Graceful placeholder when no protocol is available.

Detection uses `KITTY_WINDOW_ID`, `TERM`, `TERM_PROGRAM`, `LC_TERMINAL`, and ImageMagick availability. Windows Terminal is truecolor-capable but is not detected as a Kitty/iTerm/Sixel inline image protocol target in this low-level display path.

Mermaid rendering is currently gated. `JCODE_ENABLE_MERMAID=1` is required in markdown rendering paths, and the fallback module reports Mermaid rendering disabled when the feature is not active.

### Background Color and Terminal Workarounds

`terminal-capabilities.md` documents terminal-specific issues:

- background color erase problems;
- Windows Terminal/ConPTY rapid redraw and resize quirks;
- tmux/screen truecolor and escape filtering issues;
- emoji/double-width misalignment;
- alternate-screen restoration problems;
- kitty keyboard protocol cleanup risks;
- truecolor guidance through `COLORTERM`.

Jcode itself does not provide terminal transparency or background image settings. Use the terminal emulator for acrylic, opacity, font, ligatures, background image, and window theme.

## 3. Config Files and Locations

### Main Config

Primary file:

- `~/.jcode/config.toml`

Implementation:

- `crates/jcode-storage/src/lib.rs`
- `crates/jcode-base/src/config/config_file.rs`
- `crates/jcode-base/src/config/default_file.rs`
- `crates/jcode-config-types/src/lib.rs`

`JCODE_HOME` redirects `~/.jcode` behavior to a sandbox directory. With `JCODE_HOME` set, `Config::path()` becomes `$JCODE_HOME/config.toml`.

Default config generation is via `/config init`, `/config create`, or `Config::create_default_config_file()`.

### App-Owned Config Directory

`storage::app_config_dir()` resolves to the platform config directory plus `jcode`, for example `~/.config/jcode` on Linux. With `JCODE_HOME`, it becomes `$JCODE_HOME/config/jcode`.

Known app config files include:

- `ui_preferences.json`: persisted inline image visibility.
- provider env files created by login/provider setup flows.

On Windows, the platform config dir normally maps under `%APPDATA%` or the platform-specific config location returned by the `dirs` crate. Installation binaries are under `%LOCALAPPDATA%`, which is separate from config.

### Runtime and Data

`storage::jcode_dir()` defaults to `~/.jcode` or `JCODE_HOME`.

Common files and directories:

- `~/.jcode/logs/`
- `~/.jcode/sessions/`
- `~/.jcode/servers.json`
- `~/.jcode/active_pids/`
- `~/.jcode/setup_hints.json`
- `~/.jcode/mcp.json`
- `~/.jcode/mcp-schema-cache.json`
- `~/.jcode/pending-login/`
- `~/.jcode/hotkey/`
- `~/.jcode/selfdev-build-requests/`

Runtime sockets/ephemeral state use `JCODE_RUNTIME_DIR`, then `XDG_RUNTIME_DIR`, then macOS `TMPDIR`, then a temp fallback like `jcode-<user>`.

### Project-Local `.jcode/`

Project-local `.jcode/` is real but currently scoped. It is not a general project-local override for `[display]`.

Known project-local files:

- `.jcode/mcp.json`: project MCP servers.
- `.jcode/skills/<name>/SKILL.md`: project-local skills.
- `.jcode/workers.toml`: Queue Mode worker profiles.
- `.jcode/queue/queue.json`: project queue state.
- `.jcode/queue/handoffs/*.md`: task handoffs.
- `.jcode/queue/runs/`: run logs and indexes.

README also documents `.claude/mcp.json` as a compatibility fallback.

### MCP Config Shape

MCP config is separate from `config.toml`.

Primary paths:

- `~/.jcode/mcp.json`
- `.jcode/mcp.json`

Compatibility/import paths:

- `.claude/mcp.json`
- first-run import from `~/.claude/mcp.json`
- first-run import from `~/.codex/config.toml`

Shape:

```json
{
  "servers": {
    "filesystem": {
      "command": "/path/to/mcp-server",
      "args": ["--root", "/workspace"],
      "env": {},
      "shared": true
    }
  }
}
```

Shared MCP servers are pooled by the daemon. `shared = false` servers are session-owned.

### Provider Config Shape

Named providers live under `[providers.<name>]` in `~/.jcode/config.toml`.

Fields include:

- `type`
- `base_url`
- `api`
- `auth`
- `auth_header`
- `api_key_env`
- `api_key`
- `env_file`
- `default_model`
- `requires_api_key`
- `provider_routing`
- `model_catalog`
- `allow_provider_pinning`
- `extra_body`
- `supports_reasoning_effort`
- `[[providers.<name>.models]]` with `id`, `context_window`, and `input`

Provider defaults live under `[provider]`, including `default_model`, `default_provider`, OpenAI/Anthropic reasoning effort, OpenAI transport/service tier, native compaction settings, reasoning preservation, failover behavior, Copilot premium mode, and stream idle timeout.

## 4. Self-Dev Workflow

Relevant code:

- `src/cli/selfdev.rs`
- `src/cli/dispatch.rs`
- `crates/jcode-build-support/src/paths.rs`
- `crates/jcode-app-core/src/tool/selfdev/*.rs`
- `docs/UNIFIED_SELFDEV_SERVER_PLAN.md`
- `tests/test_selfdev_reload.py`

### Repo Detection

`build::get_repo_dir()` checks:

1. `JCODE_REPO_DIR`, if it points to a valid jcode repo.
2. Compile-time `CARGO_MANIFEST_DIR` ancestors.
3. Current executable assumed under `repo/target/<profile>/<binary>`.
4. Current working directory ancestors.

`is_jcode_repo()` requires:

- `Cargo.toml` exists;
- `.git` exists as a directory or gitdir file;
- `Cargo.toml` contains `name = "jcode"`.

If no repo is found, `self-dev` can clone `https://github.com/1jehuang/jcode.git` into `~/.jcode/source/jcode`.

### What Self-Dev Mode Does

`run_self_dev()`:

- sets `CLIENT_SELFDEV_ENV` to request self-dev client behavior;
- sets `JCODE_REPO_DIR`;
- creates or resumes a session;
- marks the session as canary/self-dev;
- optionally builds with the selfdev profile;
- publishes a local current build for the source state;
- resolves the best target binary;
- starts or connects to the shared server;
- launches the TUI client for the self-dev session.

Self-dev build command:

- uses `scripts/dev_cargo.sh build --profile selfdev -p jcode --bin jcode` when available;
- can include `jcode-desktop` when changed files suggest desktop impact;
- infers TUI, desktop, or all from `git status`.

Self-dev tool actions include launch, setup, status, reload, build queue, and queued background build coordination. Build requests are stored under `~/.jcode/selfdev-build-requests/`.

### Server/Client Behavior in Self-Dev

Client update candidate order:

1. `current` channel binary.
2. self-dev repo binary from `target/selfdev` or `target/release`.
3. canary channel.
4. launcher path.
5. stable channel.
6. current executable.

Shared server update candidate is more conservative:

- self-dev sessions can use the `shared-server` channel;
- normal sessions use `shared-server` only when its version marker matches stable or current;
- otherwise stable is preferred;
- this avoids dirty local builds accidentally replacing the shared daemon for every client.

Reload candidate prefers newer repo binaries when appropriate.

### How to Disable or Avoid Self-Dev

Practical controls found from code/config:

- Do not run `jcode self-dev`.
- Do not set `JCODE_REPO_DIR` to this checkout.
- Launch official installed `jcode` from outside the repo if you want normal installed behavior.
- Use `[display].auto_server_reload = false` or `JCODE_AUTO_SERVER_RELOAD=false` to prevent automatic remote server reload behavior.
- Ensure PATH resolves to the official launcher rather than `target/release/jcode.exe`.

### Safe Self-Dev Practice

Safe workflow:

1. Verify `where.exe jcode` / `Get-Command jcode` and `jcode --version`.
2. Keep official installed jcode available as the stable launcher.
3. Use a branch per experiment.
4. Prefer docs/config experiments before touching TUI source.
5. Avoid changing server/app-core/provider/memory/sidecar/image/agent internals for UI experiments.
6. If a UI experiment needs code, isolate it to TUI rendering or standalone Queue Board files.
7. Use `git diff --check` before handing off.
8. Build only when explicitly ready; this document intentionally did not build.

Files to avoid during risky UI experiments unless the branch is explicitly scoped there:

- `crates/jcode-app-core/**`
- provider crates
- memory crates
- server/session internals
- sidecar/image/provider internals
- shared protocol crates
- install/update/reload code
- broad config schema changes

## 5. GitHub Screenshot Visual Parity

I did not find evidence that jcode paints a persistent black-hole/space bitmap background.

Findings:

- README UI media is hosted through GitHub release assets or user attachments.
- Repo assets include demo videos, screenshots, desktop gallery golden images, icons, and an AVIF/MP4 demo, but no obvious bundled black-hole/space wallpaper asset used by the TUI.
- Terminal image support is for explicit inline images, generated/read images, screenshots, side-panel images, and diagrams.
- TUI rendering is Ratatui/crossterm cell rendering over the terminal background.
- Terminal transparency, acrylic, background image, blur, and wallpaper are terminal/window-manager settings.

For GitHub visual parity:

- First verify the same binary and server generation.
- Then use a high-quality terminal with truecolor.
- Then tune terminal font, opacity/background image/transparency.
- Then enable centered mode if the screenshot uses balanced margins.
- Then compare a healthy provider session with side panels/info widgets active.

## 6. Installation and Launcher Behavior

Relevant code:

- `crates/jcode-build-support/src/paths.rs`
- `scripts/install.ps1`
- `scripts/install.sh`
- `scripts/install_release.sh`
- `AGENTS.md`

Important paths:

- Unix launcher: `~/.local/bin/jcode`
- Unix current channel: `~/.jcode/builds/current/jcode`
- Unix stable channel: `~/.jcode/builds/stable/jcode`
- Unix immutable versions: `~/.jcode/builds/versions/<version>/jcode`
- Windows launcher: `%LOCALAPPDATA%\jcode\bin\jcode.exe`
- Windows stable channel: `%LOCALAPPDATA%\jcode\builds\stable\jcode.exe`
- Windows immutable versions: `%LOCALAPPDATA%\jcode\builds\versions\<version>\jcode.exe`

The launcher directory defaults to:

- `JCODE_INSTALL_DIR`, if set;
- `$JCODE_HOME/bin`, if `JCODE_HOME` is set;
- `%LOCALAPPDATA%\jcode\bin` on Windows;
- `~/.local/bin` on Unix.

PATH decides which binary runs when typing `jcode`. On Unix, `~/.local/bin` should appear before `~/.cargo/bin`. On Windows, verify with:

```powershell
where.exe jcode
Get-Command jcode | Format-List *
jcode --version
.\target\release\jcode.exe --version
```

Running `target/release/jcode.exe` directly bypasses the installed launcher/channel layout and may attach to an already-running shared server. Official installed jcode can feel faster because it uses the installed channel/server path and avoids repo/self-dev resolution or stale local build state.

## 7. Practical Customization Checklist

1. Install official jcode and verify `jcode --version`.
2. Confirm PATH resolves to `%LOCALAPPDATA%\jcode\bin\jcode.exe` on Windows or `~/.local/bin/jcode` on Unix when you want the installed channel.
3. Launch in the recommended terminal. On Windows, test Alacritty and Windows Terminal separately.
4. Run `jcode setup-hotkey` on Windows if you want `Alt+;` launcher behavior.
5. Create config with `/config init` if missing.
6. Set `[display].centered = true` or use `/alignment centered` if desired.
7. Verify provider/login health with `/auth`, `/account`, `/login`, `/model`, and `jcode auth-test`.
8. Test side panel behavior with `/btw` or by asking the agent to write side-panel content.
9. Test info widgets with `Alt+I`.
10. Test diff modes with `Alt+G` or `/diff`.
11. Test inline images and diagrams in a terminal that supports an image protocol. For Mermaid, set `JCODE_ENABLE_MERMAID=1` while experimenting.
12. Tune terminal font, background, opacity/transparency, color scheme, and acrylic/background image outside jcode.
13. Compare against GitHub screenshots only after provider state is healthy and the same binary/server generation is confirmed.
14. Use self-dev mode only on a branch and keep experiments scoped.

## 8. Future Native UI Customization Roadmap

### Easy

- Add documentation and starter terminal configs for Windows Terminal and Alacritty.
- Add a config/style inventory command or doc page.
- Add status text polish using existing `ui_status.rs` patterns.
- Add more `/config` display detail for effective UI settings.
- Add Queue Board styling docs that distinguish app styling from terminal styling.

### Medium

- Top bar with date/time.
- Active provider/model/session indicator refinement.
- Current project/repo indicator in header.
- Queue Mode task count indicator.
- Active background run indicator.
- Better side-panel/info-widget status polish.
- Extract more semantic style helpers in `jcode-tui-style`.

### Risky

- General theme config or palette file.
- Full top/status bar redesign that affects layout, viewport, and info widget margins.
- Main TUI Queue Board integration with mutating actions.
- Global rendering changes in `ui_viewport.rs`, `info_widget_layout.rs`, or shared chrome.
- Any reload/server/session behavior changes tied to self-dev UI.
- Terminal image protocol changes.

## 9. Queue Board Implications

Queue Board styling should be split between app code and terminal settings.

Handle in jcode code:

- board layout density;
- status columns and grouping;
- selected-task highlight;
- native palette usage;
- detail rail;
- compact footer/status wording;
- task count and run indicators;
- Queue Mode overlay integration;
- keyboard/navigation behavior;
- empty-state handling.

Handle in terminal settings:

- background image or transparency;
- font and glyph rendering;
- acrylic/blur;
- color scheme base background;
- window padding;
- ligatures;
- shell profile and startup command.

Do not put a fake GitHub screenshot wallpaper into Queue Board code. If the goal is visual parity with a screenshot that has a space/black-hole background, reproduce that in the terminal emulator first.

## 10. Recommended Next Branch

Recommended next branch:

```text
aymane/jcode-terminal-setup-docs
```

Scope:

- document verified Windows Terminal and Alacritty setups;
- include PATH/binary/server-generation checks;
- include centered mode and provider-health checks;
- include terminal background/transparency instructions;
- avoid Rust changes;
- avoid Queue Board restyle until the baseline installed visual setup is reproducible.

After that, use:

```text
aymane/topbar-date-ui-experiment
```

Scope for that later branch:

- TUI-only experiment;
- add a compact top bar/date-time indicator;
- avoid app-core/provider/server/memory changes;
- do not combine with Queue Board work.

Queue Board restyle should wait until terminal visual parity is documented, unless the user explicitly prioritizes Queue Board next.

## 11. Rollback

This change is documentation-only. To remove it:

```powershell
git restore docs/JCODE_CUSTOMIZATION_MAP.md
```

If the file is already committed later, revert that commit instead:

```powershell
git revert <commit>
```
