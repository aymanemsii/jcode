# jcode Customization Architecture

Status: Investigation

This document records a safe architecture direction for future jcode customization features. It is documentation only; it does not implement customization.

The goal is to make jcode feel more personal through themes, accent colors, layout preferences, session surfaces, and future multi-agent views without making broad risky TUI, theme, server, or protocol changes before the architecture is clear.

## Current Customization and Theme Areas

### TUI visual styling

The terminal UI is built on Ratatui/crossterm and is spread across a few presentation crates:

* `crates/jcode-tui/src/tui/` owns the main app UI, rendering orchestration, layout choices, overlays, input area, message rendering, side panels, status bars, session picker, and app-local UI state.
* `crates/jcode-tui/src/tui/ui_theme.rs` is a thin adapter that re-exports theme functions from `jcode-tui-style`.
* `crates/jcode-tui-style/src/theme.rs` contains the current hard-coded semantic colors, including user/assistant/tool colors, accent color, dim color, system message color, queued/asap/pending colors, prompt-entry animation colors, header colors, and animated tool colors.
* `crates/jcode-tui-style/src/color.rs` handles terminal color capability detection and maps RGB colors to truecolor or xterm-256 colors. It also contains glyph-safe behavior for fragile terminals.
* `crates/jcode-tui-render/src/chrome.rs`, `layout.rs`, `memory_tiles.rs`, `swarm_gallery.rs`, and `swarm_tiles.rs` contain shared presentation/layout helpers for chrome, right rails, memory tiles, and swarm gallery tiles.

Styling today is not a user-configurable theme system. Most visual choices are functions returning `ratatui::style::Color` or local `Style` values. A future theme system should preserve those semantic call sites and replace the backing palette gradually.

### Themes and colors

Current color definitions are semantic but hard-coded. Examples:

* `accent_color()`
* `user_color()`
* `ai_color()`
* `tool_color()`
* `system_message_color()`
* `queued_color()`
* `asap_color()`
* `pending_color()`
* `user_text()`
* `user_bg()`
* `header_icon_color()`
* `header_name_color()`
* `header_session_color()`

The useful part is that many call sites already ask for semantic colors rather than raw RGB values. The risky part is that some modules still create local `Color::Rgb(...)` or `rgb(...)` values directly, especially for specialized UI like the swarm gallery, tiles, and borders. Theme customization should start by routing the existing central semantic functions through a typed theme object, then later migrate direct local colors only where needed.

### Layout and rendering

The main rendering surface is in `crates/jcode-tui/src/tui/ui.rs`, with submodules such as:

* `ui_header.rs`
* `ui_input.rs`
* `ui_messages.rs`
* `ui_overlays.rs`
* `ui_pinned.rs`
* `ui_status.rs`
* `ui_inline_image.rs`
* `ui_layout.rs`
* `ui_viewport.rs`

User-visible layout preferences already exist in configuration, especially under `DisplayConfig`:

* centered content
* diff mode
* diagram mode and pane position
* markdown spacing
* image pinning/inline image behavior
* compact notifications
* native scrollbars
* animation/performance settings

This means layout personalization should begin as small additions to the existing display/config model, not as a rewrite of the renderer.

### Terminal background image support

The current terminal UI does not support a true full-screen wallpaper behind the TUI.

jcode does support inline and pane image rendering for content through `jcode-tui-mermaid` and `ratatui-image`. The existing image path is designed for bounded image regions in the transcript or side/pinned areas, not for a persistent terminal background layer.

Relevant areas:

* `crates/jcode-tui/src/tui/ui_inline_image.rs`
* `crates/jcode-tui/src/tui/mermaid.rs`
* `crates/jcode-tui-mermaid/src/`
* `terminal-capabilities.md`

The image pipeline can use terminal image protocols such as Kitty, iTerm2, Sixel, or halfblock fallback depending on terminal support. That is not the same as portable wallpaper. TUI cells still occupy a grid, and Ratatui renders foreground/background cell styles. Most terminals do not expose a portable "draw image behind alternate-screen app cells" protocol.

### Meaning of "wallpaper"

For jcode, "wallpaper" should be split into separate feature classes:

* True image wallpaper: an image behind the terminal UI. This is not portable today and should remain deferred until terminal support is proven.
* Terminal-emulator-specific image layer: possible only in some terminals and multiplexers, probably fragile in alternate screen, resizing, tmux, SSH, and Windows/ConPTY.
* TUI background color: portable and feasible. This means configurable cell background colors for the main viewport, input/status bars, panels, and overlays.
* Gradient-like TUI background: possible as colored cells or bands, but this can be noisy, expensive, and sensitive to 256-color terminals. Treat as an experiment.
* ASCII/ANSI art background or splash: portable if it is rendered as normal text/cells, but it competes with transcript space and should be limited to empty/startup states.
* Branded startup splash: feasible as a first low-risk personalization experiment, especially before the first user prompt.

## Existing Configuration Model

The primary config system is global TOML:

```text
~/.jcode/config.toml
```

or, when redirected:

```text
$JCODE_HOME/config.toml
```

Relevant files:

* `crates/jcode-base/src/config.rs`
* `crates/jcode-base/src/config/config_file.rs`
* `crates/jcode-base/src/config/default_file.rs`
* `crates/jcode-base/src/config/env_overrides.rs`
* `crates/jcode-config-types/src/lib.rs`

`Config::load()` reads the TOML file and applies environment overrides. `Config::save()` writes the same file. `jcode-config-types` defines typed config sections such as `DisplayConfig`, `KeybindingsConfig`, `AgentsConfig`, `TerminalConfig`, and provider/tool/safety-related config.

The default generated config already documents `[display]`, `[keybindings]`, `[features]`, `[tools]`, and other sections. `/config` is referenced in the default config comments as the user-facing way to see current settings.

I did not find an existing general project-local config loader for UI customization. Project-local state exists for some features, such as Queue v1 under `./.jcode/queue/`, but that is feature storage rather than a global/project config merge model.

## Wallpaper Feasibility

### True image wallpaper

True wallpaper is not currently portable in terminal TUI architecture.

Reasons:

* Ratatui renders a grid of cells, not layers.
* Terminal image protocols draw images in terminal coordinates, but they are not a standard background layer under text cells.
* Alternate screen, resizing, scrolling, tmux passthrough, SSH, Windows ConPTY, and terminal-specific image caches make persistent image layers fragile.
* jcode already has rendering guidance in `terminal-capabilities.md` that background color erase behavior can be a source of white blocks/stale cells. A wallpaper layer would multiply those risks.

This should stay deferred until a dedicated terminal feasibility spike proves a narrow supported path.

### Terminal-emulator-specific protocols

jcode already has image protocol detection and rendering for inline content. The plausible future experiment would be:

* detect Kitty/iTerm2/Sixel support using the existing image protocol picker;
* draw a bounded image only in an empty startup/splash area, not behind the entire transcript;
* clear it explicitly on redraw/resize;
* avoid tmux unless passthrough is proven;
* fall back silently to normal TUI colors.

This should be treated as an experiment, not a core customization primitive.

### Color themes and panels

Portable customization should start with colors:

* main background color
* transcript background color
* input/status bar colors
* side panel background/border colors
* user/assistant/tool/system/status colors
* accent color

This aligns with Ratatui's model and the existing `jcode-tui-style` semantic functions.

### Startup splash, ASCII art, and branded background

A startup splash is feasible because it can be normal TUI content:

* shown only before the first prompt or while loading;
* rendered as text/ANSI art or a bounded image when image protocol support exists;
* removed once conversation content occupies the viewport;
* controlled by `[display]` or future `[customization]` config.

This is safer than true wallpaper because it does not need to persist behind active UI.

### TUI area background color

Area background color customization is the most realistic "wallpaper-like" first step. It should mean explicit cell background colors in known areas:

* app background
* chat viewport
* prompt/input area
* status line
* side/pinned panels
* overlays/modals

Implementation should be careful about erase behavior: areas should set intended background colors before clearing or resetting cells, and tests should cover resize/clear behavior.

## Theme Customization Design

Theme customization should be semantic, typed, and narrow at first.

### Named themes

Support a small built-in set of named themes before arbitrary imports:

* default
* dark
* high-contrast
* maybe light later, only after render paths are audited for background assumptions

Named themes should map to a typed palette, not a bag of raw ad hoc strings.

### Accent color

Accent color is the safest first user-facing customization because there is already an `accent_color()` semantic function. The first implementation slice can allow a single global accent color while leaving the rest of the palette unchanged.

The accent should feed:

* primary highlights
* selected/focused borders where appropriate
* command/status affordances
* future session/agent badges only after readability is checked

### Palette

A future palette should separate base colors from status colors:

* background
* foreground
* muted foreground
* border
* panel background
* input background
* selection/focus
* user
* assistant
* tool
* file/link
* system
* success
* warning
* error
* pending/queued/running

Use semantic names rather than call-site-specific names where possible. Keep backward compatibility by having existing functions read from the active palette.

### Command and status bars

Command/status bar colors should be explicit theme fields because they are high-frequency UI and readability-critical:

* status foreground
* status background
* status accent
* command input foreground
* command input background
* placeholder/muted text

Do not infer these blindly from accent color. Contrast should be deliberate.

### User config location

Use the existing global config model first:

```toml
[display]
# or future [customization]
```

Do not invent a final large format yet. A minimal first step could be one or two fields in the existing display/customization area, such as a theme name and accent color. The exact field names should be chosen during implementation after checking current config naming conventions and migration risk.

### Project-local vs global customization

Recommended precedence, if project-local customization is introduced later:

1. command-line or environment override
2. project-local `.jcode/config.toml` or a scoped project customization file
3. global `~/.jcode/config.toml`
4. built-in defaults

However, project-local UI customization should not be added first. It creates questions about trust, repository portability, accidental team-wide visual changes, and config merge semantics. Start with global user preference. Add project-local only after there is a clear use case, such as per-repo accent colors or workspace identity.

## Configuration Recommendation

Recommended path:

1. Extend `jcode-config-types` with a small typed customization/display theme section only when implementation begins.
2. Load it through the existing `Config::load()` path.
3. Keep env overrides optional and minimal.
4. Keep unknown/missing values lenient.
5. Avoid a separate customization file until the built-in model is insufficient.

Do not add broad import/export, marketplaces, or arbitrary theme files yet.

## Multi-Session UI Direction

The existing `docs/MULTI_SESSION_CLIENT_ARCHITECTURE.md` already proposes a future model where the server owns sessions and a client can host one or many session surfaces.

Customization should fit that model rather than inventing a separate session UI architecture.

Potential UX:

* session list: build on the existing `/resume` session picker and preview model;
* switch session: treat a session as a selectable surface within one client;
* pin/favorite sessions: store small user preference metadata separately from core session history;
* split/pane/tab: start with a list/workspace switcher before implementing panes; terminal panes inside one Ratatui app are complex because each surface has scroll/input/focus/render state;
* workspace map: reuse the direction in `MULTI_SESSION_CLIENT_ARCHITECTURE.md` and existing workspace navigation keybindings;
* relationship to session storage: session history remains server/storage-owned; UI favorites/pins should be small client metadata keyed by session id or working directory;
* relationship to TUI state: each visible surface needs isolated viewport, input focus, scroll offsets, side-panel state, and transient render caches.

Safe direction: first improve session list/switch affordances, then add persistent pins/favorites, then experiment with workspace surfaces. Avoid split-pane multi-session rendering until state isolation is explicit.

## Agent Mixtures and Multi-Agent Surface Direction

There is already UI direction for multiple agents through swarm/inline gallery code:

* `crates/jcode-tui-render/src/swarm_gallery.rs`
* `crates/jcode-tui-render/src/swarm_tiles.rs`
* `crates/jcode-tui/src/tui/info_widget_swarm_gallery.rs`
* `docs/SWARM_ARCHITECTURE.md`
* `docs/QUEUE_V2_ARCHITECTURE.md`

Queue v2 also documents future worker profiles that may map to provider, model, tool, and permission configuration. That maps naturally to an eventual "agent mixture" surface.

Possible future UI:

* worker/agent list with name, role/profile, status, current task, model/provider, and last activity;
* active agents area showing running/thinking/blocked/completed states;
* agent profiles shown as durable named capabilities, not just transient session ids;
* per-agent status badges using theme status colors;
* live output tiles for active agents, reusing swarm gallery layout where possible;
* relationship to Queue v2 worker profiles: Queue `worker_profile` should remain metadata until a mapping layer exists; once mapped, the UI can show which profile claimed or is running a task;
* relationship to Queue Board/TUI: the Queue Board should be a separate future surface after Queue v2 state semantics stabilize, not a prerequisite for basic theme customization.

Safe direction: keep agent mixture UI read-only at first. Show what exists before adding controls that start/stop/reassign agents.

## Safe Implementation Order

Recommended future slices:

1. Keep this customization architecture document as the baseline.
2. Inspect direct color/style call sites in `crates/jcode-tui`, `crates/jcode-tui-render`, and `crates/jcode-tui-style` more deeply.
3. If useful, add a read-only `jcode config show` or extend existing `/config` visibility for display/theme settings.
4. Add a global accent color config field and route only `accent_color()` through it.
5. Add named built-in theme config with no arbitrary imports.
6. Apply the theme object to the central semantic color functions.
7. Migrate high-value direct colors, especially status and border colors, after visual review.
8. Add TUI area background color customization for well-bounded areas.
9. Add a startup splash/background panel experiment.
10. Investigate true wallpaper separately with a terminal protocol spike.
11. Investigate multi-session UI pins/favorites and switcher improvements.
12. Investigate agent mixture UI as a read-only worker/agent list.

The first implementation slice should be global accent color configuration routed through the existing semantic `accent_color()` function. It has a small blast radius, exercises the config path, and avoids changing layout, protocol, server behavior, or image rendering.

## Explicitly Deferred

* true wallpaper implementation until terminal feasibility is confirmed
* broad TUI refactor
* multi-session implementation
* multi-agent implementation
* server protocol changes
* Queue Board
* background worker UI
* theme marketplace/import/export
* arbitrary project-local theme execution or untrusted theme loading
* terminal-emulator-specific wallpaper as a default experience

## Architecture Decisions

* Treat customization as a client/TUI concern first.
* Reuse the existing global TOML config system before introducing new files.
* Add customization through typed config and semantic palette functions.
* Prefer portable color/theme customization over terminal-specific image wallpaper.
* Keep project-local customization deferred until merge/trust semantics are designed.
* Keep multi-session and multi-agent UI as future surfaces that build on existing session, swarm, and Queue v2 architecture.
* Avoid server protocol changes for early theme and accent features.
