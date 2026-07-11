# Wallpaper Support Architecture

## Current State

jcode currently has a terminal-safe background MVP. The supported display
configuration is intentionally limited to text-cell effects:

* `display.background_style = "none" | "subtle-grid" | "stars" | "matrix"`
* `display.background_opacity`
* project-local overrides through `./.jcode/workspace.toml`
* safe rendering only in empty/startup areas

The current implementation does not load image files, does not support PNG or
JPG wallpaper, does not use terminal image protocols, and does not make network
requests. The background is drawn as normal TUI cells and is limited to safe
empty areas where it will not compete with active transcript, input, dialogs,
onboarding, or other readability-critical UI.

## Real Wallpaper Goals

The eventual goal is to support local image wallpaper while preserving the
reliability of the terminal UI:

* support image wallpaper when the user explicitly opts in;
* support opacity, intensity, or sampling controls;
* avoid reducing text readability;
* avoid breaking terminals that do not support images;
* keep terminal-safe background styles as the default and fallback path;
* preserve existing safe empty-area rendering behavior until broader routing is
  proven.

Real wallpaper should be treated as progressive enhancement. The app must remain
usable when image support is unavailable, disabled, blocked by a multiplexer, or
visually poor in the current terminal.

## Terminal Support Questions

Terminal image support is fragmented. There is no single portable "draw image
behind this Ratatui application" capability across supported environments.

### Windows Terminal

Windows Terminal can display a native background image through its own profile
settings, but that setting belongs to the terminal emulator, not to the
alternate-screen TUI application. jcode should not assume it can set or manage
that background at runtime.

Terminal image protocols are not a dependable baseline for Windows Terminal and
ConPTY paths. Even where inline image techniques work in some configurations,
they are not equivalent to a persistent wallpaper layer behind Ratatui cells.
Resize behavior, clearing, and redraws are likely to expose artifacts.

### PowerShell and cmd Hosts

Classic console hosts and shell-host combinations should be treated as plain
terminals unless proven otherwise. They may support ANSI color but not graphics
protocols. Any image-specific path must fail closed to terminal-safe backgrounds
without changing command behavior or rendering unreadable content.

### WSL Terminal Behavior

WSL behavior depends on the outer terminal emulator, not just the Linux process
inside WSL. A WSL session in Windows Terminal, WezTerm, Alacritty, Kitty, or
another host may expose different capabilities. Capability detection from inside
WSL can also be incomplete because environment variables, terminal names,
multiplexer passthrough, and remote sessions may not describe the actual image
path accurately.

### macOS Terminal and iTerm2

Apple Terminal should be treated as a plain terminal for wallpaper purposes.
iTerm2 has a proprietary image protocol and can render inline images, but that
does not provide a portable persistent background behind TUI text cells. It also
creates terminal-specific behavior that must be opt-in and carefully isolated if
it is ever added.

### Linux Terminals

Linux terminal support varies widely. Kitty supports its graphics protocol.
Some terminals support Sixel. Some support neither. Some terminals run inside
tmux, screen, SSH, or nested shells that block or transform image escape
sequences.

The Linux desktop environment may also allow emulator-native background images,
but those settings are outside jcode's portable TUI rendering model.

### SSH Sessions

SSH should be considered high risk for image wallpaper. The terminal emulator is
local, the app is remote, and protocol support depends on passthrough, terminal
identity, latency, scrollback behavior, and security policy. jcode should not
send image protocol payloads automatically in SSH sessions.

### Plain Terminals Without Image Protocols

Plain terminals remain the baseline. They can render text, ANSI styles, and
cell background colors. They cannot render PNG/JPG content directly. The current
terminal-safe simulated backgrounds are the correct fallback for this class.

## Possible Technical Approaches

### Terminal-Safe Simulated Backgrounds

This is the current approach. It uses normal Ratatui cells and terminal-safe
characters, so it works anywhere the rest of the TUI works. Opacity is simulated
with pattern density and theme-aware color blending rather than real alpha.

This remains the safest default because it is portable, cheap to render, and
compatible with empty-area-only drawing.

### Unicode, Block, or ANSI Art Generated From Images

A local image could be converted into low-resolution Unicode, half-block, or ANSI
art. This avoids terminal graphics protocols and can work in plain terminals, but
it is not true wallpaper. It consumes cells, has limited color fidelity, and can
hurt readability unless it is heavily dimmed and restricted to empty areas.

This is the most realistic first image-based experiment because it degrades like
normal TUI content and can reuse existing background routing.

### Sixel

Sixel can render raster images in supporting terminals, especially some Unix-like
terminal emulators. It is not universal, is often disabled or unsupported, and
can behave poorly through tmux, SSH, or Windows console paths. Sixel also renders
images into the terminal surface, not into a standardized background layer under
Ratatui cells.

If used later, it should be opt-in per terminal capability, never the default.

### iTerm2 Image Protocol

iTerm2's protocol is useful for inline images on iTerm2-compatible terminals.
It is terminal-specific and unsuitable as a broad default. It also does not solve
z-ordering behind text, persistent redraw, or non-iTerm environments.

This should remain deferred unless there is a dedicated terminal-specific image
feature separate from wallpaper.

### Kitty Graphics Protocol

The Kitty graphics protocol is capable and well suited to some inline image use
cases. It is still emulator-specific and does not provide a portable wallpaper
layer for all terminals. Multiplexers, SSH, alternate screen behavior, image
placement, cleanup, and resize handling all require careful testing.

This should remain an advanced opt-in protocol path, not the MVP.

### Terminal Emulator Native Background Settings

Some terminal emulators can set a profile background image or transparency.
That is outside Ratatui and outside jcode's portable runtime control. jcode
should document that users can configure native terminal backgrounds themselves,
but should not attempt to mutate terminal profile settings as part of the TUI.

### External Preview Panes or Companion UI

A companion UI, side pane, browser preview, or desktop wrapper could show images
without forcing the terminal to become a graphics surface. This is more reliable
for rich media but is a different product surface from terminal wallpaper. It
also should not be required for the core TUI.

## Ratatui and TUI Constraints

Ratatui renders a grid of terminal cells. Each cell has content and style, but
there is no portable retained graphics layer behind those cells.

Important constraints:

* cell-based rendering limits image fidelity to character-sized samples unless a
  terminal graphics protocol is used;
* z-ordering is explicit draw order, not true compositing behind arbitrary text;
* terminal opacity is not portable real alpha blending;
* resizing requires recomputing layout, sampled art, and any protocol placement;
* high-resolution conversion or protocol payloads may be expensive during
  redraws;
* terminal scrollback and alternate-screen behavior can expose stale image
  fragments or unexpected clearing;
* messages, input, dialogs, onboarding, top bar, side panes, and command
  surfaces need strong contrast and should generally cover or suppress
  background effects;
* broad background routing would require auditing every UI surface that clears,
  resets, overlays, or scrolls content.

The current empty/startup-area restriction is the right boundary until these
constraints are addressed by focused implementation and visual testing.

## Security and Privacy

The wallpaper path should be local file only for any MVP. Remote image fetching
should not be supported in the first implementation.

Rules for a future implementation:

* no remote URLs;
* no network access for wallpaper loading;
* no automatic loading of untrusted project-local image paths unless explicitly
  allowed by a later design;
* project-local wallpaper paths should be treated carefully because they can
  point outside the repository, reveal local filesystem structure, or trigger
  expensive file processing;
* invalid paths, unsupported formats, permission failures, and decode failures
  must fall back to terminal-safe styles;
* image decoding should be bounded by size and memory limits if dependencies are
  later added;
* config display should avoid leaking sensitive absolute paths in places where
  logs or shared output are expected.

Project-local visual configuration is useful, but project-local image loading is
a higher-risk class than the existing text-only background fields.

## Proposed MVP If Implementation Is Later Approved

The safest future implementation path is:

1. Keep terminal-safe backgrounds as the default.
2. Add an optional local file path only after explicit global config by the user.
3. Do not support remote URLs.
4. Do not support animation.
5. Convert the local image to ANSI, half-block, or static sampled pattern output
   if feasible.
6. Render only in empty/startup areas first.
7. Apply a conservative intensity control that biases toward readability.
8. Gracefully fall back to `background_style` when the file is missing,
   unsupported, too large, or unsuitable for the terminal.
9. Keep terminal-specific image protocols out of the first slice.

This path treats the image as a source for terminal-safe cell art, not as a
terminal graphics layer. It avoids protocol fragmentation while still letting
users experiment with personal wallpaper-like visuals.

## Deferred Features

The following should remain deferred:

* animated wallpaper;
* remote wallpaper URLs;
* terminal-specific image protocols by default;
* full-screen image rendering behind messages;
* automatic project-local image loading;
* broad background routing across all TUI surfaces;
* terminal emulator profile mutation;
* per-terminal protocol negotiation beyond existing capability diagnostics.

## Recommendation

Do not implement true terminal image wallpaper soon. The portability, z-ordering,
opacity, resize, and security issues are larger than the user-visible gain for a
terminal-first application.

The next safest implementation slice is to keep improving terminal-safe
backgrounds first: add more text-cell styles, improve intensity/readability
controls, and keep drawing limited to empty/startup areas. If image wallpaper is
approved after that, start with explicit local-file configuration that converts
an image into a static ANSI/block-art background for empty areas only, with
strict fallback to the existing terminal-safe styles.
