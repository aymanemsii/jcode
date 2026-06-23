# Jcode Upstream UI Parity Diagnosis

Date: 2026-06-23

## Scope

This diagnosis inspects the local fork against `upstream/master` using read-only git commands. No upstream checkout, merge, rebase, cargo build, cargo fmt, or Rust source edit was performed for this document.

The upstream remote is:

- `upstream` -> `https://github.com/1jehuang/jcode.git`
- `refs/remotes/upstream/HEAD` -> `refs/remotes/upstream/master`

The key result is: the local fork does not currently differ from upstream in the main TUI crates, style crate, terminal image crate, TUI render crate, app-core, or normal TUI launch/config files inspected. The main visual mismatch is therefore more likely to come from binary path, runtime state, terminal configuration, provider/auth state, or misunderstanding which part of the README media is app-rendered.

## 1. Am I Running The Right Binary?

Run these in PowerShell from the same shell where you normally launch `jcode`:

```powershell
where.exe jcode
Get-Command jcode | Format-List *
Get-Item .\target\release\jcode.exe | Format-List FullName,Length,LastWriteTime
.\target\release\jcode.exe --version
jcode --version
```

Then compare:

- whether `where.exe jcode` lists this repo's `target\release\jcode.exe`, a launcher under `%LOCALAPPDATA%\jcode\bin\jcode.exe`, or another install entirely;
- whether `Get-Command jcode` resolves to an executable, alias, function, or script;
- whether the global `jcode --version` matches `.\target\release\jcode.exe --version`;
- whether the global executable timestamp is older than the local `target\release\jcode.exe`.

In this diagnostic shell, `where.exe jcode` and `Get-Command jcode` did not find `jcode` on PATH, while `target\release\jcode.exe` exists:

```text
C:\Users\Aymane's AI\AI-Lab\repos\jcode\target\release\jcode.exe
LastWriteTime: 2026-06-23 20:38
Length: 94956032
```

That means the user's normal shell may be different from this agent shell, and binary-path verification is the first required check. If `jcode` in PATH points to a stable install, an older build, or another fork, the UI can differ even when this repo's source matches upstream. It can also connect to an already-running background server from a different binary generation, so compare both the client executable and any running server/reload state.

## 2. How Upstream Renders The Expected UI

Commands used for orientation included:

```powershell
git remote -v
git branch -r
git symbolic-ref refs/remotes/upstream/HEAD
git ls-tree -r upstream/master --name-only
git show upstream/master:README.md
git show upstream/master:<path>
git grep -n -i <patterns> upstream/master -- <paths>
```

Relevant upstream code/docs:

- `src/main.rs`: binary entrypoint.
- `src/cli/dispatch.rs`: default command path.
- `src/cli/tui_launch.rs`: launches the remote interactive TUI client.
- `crates/jcode-tui/src/tui/ui_header.rs`: persistent header/provider/model/version lines.
- `crates/jcode-tui/src/tui/ui_input.rs`: prompt, status-adjacent notices, input line.
- `crates/jcode-tui/src/tui/ui_status.rs`: status wording.
- `crates/jcode-tui/src/tui/ui_viewport.rs`: viewport layout, centered/left alignment, info widget margins.
- `crates/jcode-tui/src/tui/info_widget*.rs`: right/left negative-space widgets.
- `crates/jcode-tui-style/src/theme.rs`: shared colors.
- `crates/jcode-tui-style/src/color.rs`: truecolor/256-color capability detection.
- `crates/jcode-terminal-image/src/display.rs`: terminal image protocol support.
- `crates/jcode-tui/src/tui/ui_inline_image.rs`: inline transcript image rendering.
- `terminal-capabilities.md`: terminal behavior notes.

Default `jcode` with no subcommand goes through `run_default_command` in `src/cli/dispatch.rs`. That path gathers setup hints, crash-resume hints, detects self-dev state, starts/connects to the server, and then launches the remote TUI through `tui_launch::run_tui_client`.

### README UI Evidence

The upstream README has a UI section titled "Side panels, Diagrams, Info Widgets, rendering, scrolling, alignment". It embeds a GitHub-hosted screenshot at `https://github.com/user-attachments/assets/6c7bec81-ef3f-434d-8a7b-d55f8a54e5cf`.

The same section says info widgets use negative space and "will get out of the way if there isn't any." It also says jcode is left-aligned by default and can switch to centered mode with `Alt+C`, `/alignment`, or config.

Handterm is referenced for smooth native scroll API work. The README says normal-terminal scrolling is still implemented. Handterm is not presented as a required dependency for a photographic background.

### Background / Black-Hole / Space Visual

I did not find evidence that upstream jcode renders a persistent black-hole or space wallpaper as part of the TUI.

Findings:

- Upstream has terminal image support, but it is for displaying image content in the terminal: Kitty graphics, iTerm2 inline images, and Sixel.
- `crates/jcode-terminal-image/src/display.rs` detects terminal image protocols and displays specific image files when requested.
- `crates/jcode-tui/src/tui/ui_inline_image.rs` is for images attached to the conversation, read/generated images, screenshots, and inline transcript images.
- The TUI is Ratatui/crossterm text rendering over terminal cells. The inspected code styles spans, prompts, headers, side panels, diagrams, and inline image regions.
- I did not find a code path that paints a full-screen photographic background behind the TUI.

Upstream assets include demos and screenshots such as:

- `assets/demos/*.mp4`
- `assets/demos/jcode-vs-claude-code.png`
- `assets/niri-screenshot.png`
- `assets/readme/100-sessions-spawn-demo.gif`
- `docs/images/*.png`
- `tests/desktop-gallery-golden/*.png`

These are README/docs/test assets, not an obvious bundled black-hole TUI wallpaper asset. The primary README UI image is hosted externally through GitHub user attachments.

### Terminal Image Protocols

Upstream terminal image support exists and detects:

- Kitty graphics protocol for Kitty/Ghostty-like environments.
- iTerm2 inline images.
- Sixel for terminals such as xterm/foot/mlterm/WezTerm when conversion support is available.

This matters for inline screenshots/generated images in the transcript. It does not by itself imply that the app draws a background image. Windows Terminal is listed in `terminal-capabilities.md` as truecolor-capable and generally TUI-capable, but not as Kitty image protocol capable. Its own settings can provide acrylic/transparency/background images outside jcode.

### Windows Terminal

Windows Terminal can plausibly reproduce an atmospheric look through terminal-level settings:

- acrylic/transparency;
- background image;
- theme/color scheme;
- font and opacity settings.

That would be outside jcode's Ratatui render path. If the expected upstream screenshot includes a visible black-hole/space field, verify whether that visual is from the terminal/window manager/compositor or from the app. The inspected upstream code supports styled text UI and inline images, not a persistent wallpaper renderer.

## 3. How This Fork Differs From Upstream

Command:

```powershell
git diff --name-status upstream/master...HEAD -- crates/jcode-tui crates/jcode-tui-style crates/jcode-terminal-image crates/jcode-tui-render crates/jcode-app-core src/cli README.md docs Cargo.toml
```

Observed differences:

```text
M  README.md
A  docs/ARCHITECTURE_MAP.md
A  docs/AYMANE_ROADMAP.md
A  docs/JCODE_LAB_NOTES.md
A  docs/JCODE_UI_VISUAL_DIAGNOSIS.md
A  docs/QUEUE_BACKGROUND_RUNNER_PLAN.md
A  docs/QUEUE_MAIN_TUI_INTEGRATION_PLAN.md
A  docs/QUEUE_MODE.md
A  docs/QUEUE_MODE_IMPLEMENTATION_PLAN.md
A  docs/QUEUE_TUI_KANBAN_PLAN.md
M  src/cli/args.rs
M  src/cli/args/tests.rs
M  src/cli/commands.rs
A  src/cli/commands/queue_board_tui.rs
M  src/cli/commands_tests.rs
M  src/cli/dispatch.rs
M  src/cli/proctitle.rs
```

Command:

```powershell
git diff --name-only upstream/master...HEAD -- crates/jcode-tui crates/jcode-tui-style crates/jcode-terminal-image crates/jcode-tui-render crates/jcode-app-core crates/jcode-base/src/config src/cli/tui_launch.rs src/cli/terminal.rs
```

Observed output: empty.

Interpretation:

- The fork changes relevant to this comparison are Queue Mode CLI/docs work plus a README note.
- No upstream-vs-local diff was found in the main interactive TUI crate, TUI style crate, terminal-image crate, TUI render crate, app-core, config files checked, `src/cli/tui_launch.rs`, or `src/cli/terminal.rs`.
- Queue Mode files are not expected to affect the normal interactive `jcode` UI unless dispatch/wiring accidentally changes the no-subcommand path. The inspected diff list shows `src/cli/dispatch.rs` changed, so later review should confirm the default command branch remains upstream-equivalent except for Queue subcommand wiring.
- The interrupted Queue Board restyle edit was restored before this document was created. `git status --short` was clean before the documentation edit.

## 4. Config And Runtime Causes

### Alignment

Upstream defaults:

- `[display] centered = false`
- keybinding `centered_toggle = "alt+c"`
- CLI replay flags support `--centered` and `--no-centered`
- `/alignment` can show and persist centered vs left alignment

Left-aligned mode plus a wide terminal naturally leaves negative space. Info widgets are designed to occupy that space. A right-side info widget is therefore normal upstream behavior, not necessarily a fork regression.

### Info Widget

The README explicitly describes info widgets as using negative space. The code has `info_widget*.rs` modules and viewport margin logic. Seeing a right-side info widget locally is consistent with upstream.

### Theme / Color

Upstream theme colors live in `crates/jcode-tui-style/src/theme.rs` and are re-exported by `crates/jcode-tui/src/tui/ui_theme.rs`. Color capability detection lives in `crates/jcode-tui-style/src/color.rs`.

The terminal can affect the result:

- truecolor vs 256-color fallback;
- font/glyph width;
- terminal background color;
- Windows ConPTY quirks;
- tmux/screen filtering or color downgrades.

### Terminal Capability

`terminal-capabilities.md` lists Windows Terminal as truecolor-capable with Unicode/emoji and alt-screen support, but notes ConPTY can add latency and occasionally drop rapid escape sequences. It also notes background color bleed on resize and bold/bright-color mapping surprises.

If running under tmux, screen, VS Code Terminal, or another terminal layer, compare against Windows Terminal directly.

### Provider / Login / Timeout State

Provider/login state can strongly affect the startup screen. If the provider is unauthenticated, unavailable, timing out, or still loading model/catalog state, the transcript may be sparse and dominated by status messages, setup hints, onboarding, or retry notices.

That can look very different from an upstream README screenshot captured during a healthy, content-rich session with side panels/diagrams/info widgets visible. Fixing auth/provider timeout should be part of visual parity verification before changing UI code.

## 5. Queue Board Implication

Do not restyle Queue Board yet.

Queue Board is a local fork feature implemented at `src/cli/commands/queue_board_tui.rs` and launched by `jcode queue board --tui`. It does not exist on upstream `master`.

Because the main `jcode` UI parity question is still unresolved at the runtime/binary/terminal level, Queue Board styling should wait until:

1. the exact binary being launched is verified;
2. the provider/auth timeout is resolved or intentionally reproduced;
3. the terminal/config requirements for the expected upstream look are understood;
4. the normal interactive UI is compared from the same terminal against this fork's built binary.

Only after that should Queue Board be restyled against the confirmed native jcode visual language.

## 6. Recommended Next Steps

1. Verify binary path:
   - run `where.exe jcode`;
   - run `Get-Command jcode | Format-List *`;
   - compare global `jcode --version` with `.\target\release\jcode.exe --version`;
   - run `.\target\release\jcode.exe` directly from this repo.

2. Verify server/client generation:
   - ensure the interactive client is not attaching to an old background server;
   - use the repo-local binary directly for the comparison.

3. Verify terminal/config:
   - run in Windows Terminal, not a nested terminal first;
   - check font, truecolor, opacity/acrylic/background image settings;
   - avoid tmux/screen/VS Code Terminal for the first parity check.

4. Verify alignment:
   - run `/alignment`;
   - try `Alt+C`;
   - test left-aligned and centered modes.

5. Fix provider/auth runtime state:
   - resolve login or timeout issues;
   - run provider auth checks if needed;
   - compare a healthy active session, not only a timeout screen.

6. Compare source-built fork vs upstream expectation:
   - if main TUI still differs after binary/runtime/terminal checks, inspect the exact screenshot target and identify whether it is a terminal/compositor setup, a desktop app screenshot, or a TUI state.

7. Only then restyle Queue Board:
   - create a narrow Queue Board visual branch after native UI parity is understood.

## 7. Concrete File List For Later Work

Required inspection files:

- `src/main.rs`
- `src/cli/dispatch.rs`
- `src/cli/tui_launch.rs`
- `src/cli/terminal.rs`
- `crates/jcode-tui/src/tui/ui.rs`
- `crates/jcode-tui/src/tui/ui_header.rs`
- `crates/jcode-tui/src/tui/ui_input.rs`
- `crates/jcode-tui/src/tui/ui_status.rs`
- `crates/jcode-tui/src/tui/ui_viewport.rs`
- `crates/jcode-tui/src/tui/info_widget*.rs`
- `crates/jcode-tui-style/src/theme.rs`
- `crates/jcode-tui-style/src/color.rs`
- `crates/jcode-terminal-image/src/display.rs`
- `crates/jcode-tui/src/tui/ui_inline_image.rs`
- `crates/jcode-base/src/config/default_file.rs`
- `crates/jcode-base/src/config/env_overrides.rs`
- `terminal-capabilities.md`
- `README.md`

Optional inspection files:

- `crates/jcode-tui-render/src/chrome.rs`
- `crates/jcode-tui/src/tui/ui_inline.rs`
- `crates/jcode-tui/src/tui/ui_diagram_pane.rs`
- `crates/jcode-tui/src/tui/ui_pinned*.rs`
- `crates/jcode-tui/src/tui/app/commands.rs`
- `crates/jcode-tui/src/tui/app/input.rs`
- `crates/jcode-tui/src/tui/app/remote/*.rs`
- `docs/KEYMAP_CONFLICTS.md`
- `docs/SPAWN_HOOK.md`

Risky files to change later:

- `crates/jcode-tui/src/tui/*`
- `crates/jcode-tui-style/src/*`
- `crates/jcode-terminal-image/src/*`
- `crates/jcode-app-core/**`
- `crates/jcode-base/src/provider/**`
- `src/cli/dispatch.rs`
- `src/cli/tui_launch.rs`

Queue Board files to defer:

- `src/cli/commands/queue_board_tui.rs`
- `docs/QUEUE_MODE.md`
- `docs/QUEUE_TUI_KANBAN_PLAN.md`

## 8. Manual Verification Checklist

- Run global `jcode`.
- Run `.\target\release\jcode.exe` from this repo.
- Compare `jcode --version` vs `.\target\release\jcode.exe --version`.
- Check `where.exe jcode`.
- Check `Get-Command jcode | Format-List *`.
- Stop/reload any old background server if the client may be attaching to stale code.
- Run in Windows Terminal directly.
- Avoid nested terminal layers for the first comparison.
- Try `/alignment`.
- Try `Alt+C`.
- Log in or fix the provider timeout.
- Test a healthy active session with real transcript content.
- Only if the expected screenshot appears to rely on terminal-level visuals, try Windows Terminal transparency/acrylic/background image settings.
- Compare against the upstream README UI screenshot after controlling binary, provider state, alignment, and terminal.

## Bottom Line

The local fork's main interactive TUI source appears upstream-equivalent in the inspected paths. The current mismatch is most plausibly from one or more of:

- running a different binary or old launcher/server;
- provider/login timeout causing a sparse startup screen;
- normal upstream info-widget negative-space behavior;
- left-aligned default layout;
- terminal settings/background/transparency outside jcode;
- expecting README/demo/window-manager visuals to be fully app-rendered.

Recommended next branch/scope:

- `aymane/jcode-main-ui-parity-diagnosis`
- documentation and verification only;
- no Queue Board restyle until main UI parity is confirmed.
