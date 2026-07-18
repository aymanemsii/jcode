# jcode Customization

This document describes the current customization system implemented in jcode customization v2.

## Config Location

jcode reads global customization settings from:

```text
~/.jcode/config.toml
```

When `JCODE_HOME` is set, jcode reads the config from:

```text
$JCODE_HOME/config.toml
```

Mercury compatibility also supports `MERCURY_HOME` as a higher-precedence
explicit config home:

```text
$MERCURY_HOME/config.toml
```

If both `MERCURY_HOME` and `JCODE_HOME` are set, `MERCURY_HOME` wins and
`jcode config show` reports that both env vars are set. When neither env var is
set, Mercury compatibility reads `~/.mercury/config.toml` before falling back to
`~/.jcode/config.toml`.

Customization is configured through TOML sections such as `[app]`, `[display]`, and `[themes.<name>]`.

## Project-Local Customization

jcode also supports a visual-only project-local workspace file:

```text
.mercury/workspace.toml
.jcode/workspace.toml
```

jcode searches upward from the current working directory and loads the nearest
`.mercury/workspace.toml` or `.jcode/workspace.toml`. It loads only one
workspace file and does not merge multiple parent workspace files. Distance is
the primary precedence rule; when both files exist in the same directory,
`.mercury/workspace.toml` wins.

Supported MVP shape:

```toml
[workspace]
name = "jcode"

[display]
theme = "cursor"
accent_color = "#0088FF"
startup_splash_title = "jcode dev mode"
startup_splash_subtitle = "local source build"
startup_splash_footer = "workspace customization enabled"
background_style = "subtle-grid"
background_opacity = 0.15
top_bar = true
top_bar_items = ["app", "session", "theme", "repo"]
```

Only these project-local fields are supported:

* `workspace.name`
* `display.theme`
* `display.accent_color`
* `display.startup_splash_title`
* `display.startup_splash_subtitle`
* `display.startup_splash_footer`
* `display.background_style`
* `display.background_opacity`
* `display.top_bar`
* `display.top_bar_items`

Project-local values override global config at field level for those fields
only.

Global config remains the source for `app.name`, `app.terminal_title`,
credentials, providers, auth, privacy/network settings, execution settings, and
everything else not listed above. Project-local config is intentionally not a
place for secrets or command execution behavior.

### Workspace Commands

Use:

```text
jcode workspace show
jcode workspace init
jcode workspace edit
```

`jcode workspace show` reports the discovered workspace file, whether it was
found in the current directory or a parent directory, whether the source is
`.mercury` or `.jcode`, `workspace.name`, and the supported project-local
display fields when present.

`jcode workspace init` creates `./.mercury/workspace.toml` with safe visual-only
defaults and refuses to overwrite an existing file:

```toml
[workspace]
name = "<current-directory-name>"

[display]
theme = "cursor"
top_bar = true
top_bar_items = ["app", "session", "theme", "repo"]
```

`jcode workspace edit` opens the current-directory `.mercury/workspace.toml`
when present, otherwise opens the current-directory `.jcode/workspace.toml`
when present. If neither file exists, it creates `./.mercury/workspace.toml`
and opens it in the editor selected by `VISUAL`, `EDITOR`, or Notepad on
Windows. On macOS and Linux, set `VISUAL` or `EDITOR` first.

`jcode workspace init` and `jcode workspace edit` are current-directory
commands. They do not edit a discovered parent workspace file. Existing
current-directory `./.jcode/workspace.toml` files remain supported, and
`jcode workspace init` refuses to create a duplicate Mercury workspace file
beside one.

## App Identity

Optional app identity settings live under `[app]`:

```toml
[app]
name = "AymaneCode"
terminal_title = "AymaneCode"
```

Both fields are optional. Missing, empty, or whitespace-only values fall back to the current jcode behavior.

`app.name` is used for safe user-facing identity labels, including the default startup splash title when `display.startup_splash_title` is missing or blank, the startup splash default subtitle, the top status bar app item, onboarding welcome title, and config/theme command headings. An explicit non-blank `display.startup_splash_title` always wins for the splash title.

`app.terminal_title` customizes the active TUI terminal/window title where jcode already updates that title. It does not rename the binary, config directory, `MERCURY_HOME`/`JCODE_HOME`, crates, packages, or documentation globally.

## Config Editing

Use:

```text
jcode config edit
```

The command opens the global user config file in your editor. It creates the config directory and `config.toml` if they do not already exist.

Editor selection uses `$VISUAL` first, then `$EDITOR`. On Windows, jcode falls back to Notepad when neither variable is set. On macOS and Linux, set `VISUAL` or `EDITOR` before running the command.

## Built-In Themes

The active theme is selected with `display.theme`:

```toml
[display]
theme = "tokyonight"
```

Built-in theme names:

* `default`
* `dark`
* `high-contrast`
* `dracula`
* `tokyonight`
* `gruvbox`
* `nord`
* `catppuccin`
* `catppuccin-macchiato`
* `kanagawa`
* `everforest`
* `ayu`
* `one-dark`
* `matrix`
* `vercel`
* `cursor`

If `display.theme` is missing or invalid, jcode falls back to the default theme.

## Accent Color

`display.accent_color` overrides only the active theme accent color:

```toml
[display]
accent_color = "#8B5CF6"
```

Accepted formats:

* `#RRGGBB`
* `RRGGBB`

Invalid accent color values fall back safely. They do not prevent jcode from loading, and they do not invalidate the rest of the active theme.

## Custom Named Themes

Custom named themes live under the top-level `[themes]` table. Select a custom theme by setting `display.theme` to the custom theme name:

```toml
[display]
theme = "aymane"

[themes.aymane]
accent = "#8B5CF6"
user = "#7DD3FC"
assistant = "#C084FC"
tool = "#FBBF24"
system = "#94A3B8"
queued = "#38BDF8"
asap = "#F97316"
pending = "#A78BFA"

background = "#09090B"
foreground = "#E5E7EB"
muted = "#71717A"
border = "#27272A"
active_border = "#8B5CF6"
panel = "#111113"
input = "#18181B"
selection = "#312E81"
success = "#34D399"
warning = "#FBBF24"
error = "#FB7185"
```

Custom themes support the Theme Palette V2 fields shown above:

* Message and status colors: `accent`, `user`, `assistant`, `tool`, `system`, `queued`, `asap`, `pending`
* UI chrome colors: `background`, `foreground`, `muted`, `border`, `active_border`, `panel`, `input`, `selection`, `success`, `warning`, `error`

Theme Palette V2 now routes more visible TUI chrome through the active theme,
including low-risk input, status, panel/border, selection, notice, warning,
success, error, label, and muted/help text styling. Specialized renderers may
still keep local colors when their behavior needs separate review.

Missing or invalid custom theme color fields fall back safely at the field level.

## Theme Precedence

jcode resolves the active palette in this order:

1. A valid `display.accent_color` overrides the accent color only.
2. If `display.theme` matches a custom theme, jcode uses that custom theme.
3. If `display.theme` matches a built-in theme, jcode uses that built-in theme.
4. If no valid theme matches, jcode uses the default theme fallback.

## Reserved Theme Names

Built-in theme names are reserved. If a custom theme uses the same name as a built-in theme, the built-in theme wins.

This prevents local config from replacing the behavior of stable built-in theme names.

## Theme Commands

Use:

```text
jcode theme list
jcode theme current
jcode theme preview [theme-name]
```

`jcode theme list` prints the built-in theme names and any global custom themes
defined under `[themes.<name>]`. If the discovered project-local workspace file
overrides `display.theme` or `display.accent_color`, the command notes that
project-local workspace customization may affect the current theme or accent.

`jcode theme current` shows the active resolved theme from the same merged
global-plus-discovered-workspace config used by `jcode config show`. It
reports the active theme name, theme source, theme validity, active accent
color, whether a project-local workspace config is present, and whether
project-local `display.theme` or `display.accent_color` overrides are active.

`jcode theme preview` previews the active theme. `jcode theme preview
tokyonight` or another name previews that built-in or global custom theme. The
preview prints compact Theme Palette V2 hex values for semantic colors and
chrome fields including `accent`, `user`, `assistant`, `tool`, `system`,
`muted`, `success`, `warning`, `error`, `border`, `active_border`,
`background`, `panel`, and `input`.

Theme command headings use `app.name` when configured and fall back to `jcode`.

Unknown theme names return a non-zero error and print the available built-in and
global custom theme names. Theme commands are read-only; they do not set themes,
import/export themes, add project-local custom theme definitions, discover
parent directories, or configure wallpaper.

## Startup Splash

The startup splash is configured under `[display]`:

```toml
[display]
startup_splash = true
startup_splash_title = "jcode // Aymane"
startup_splash_subtitle = "Build fast. Break nothing."
startup_splash_footer = "custom mode enabled"
```

`startup_splash` must be `true` for the splash to appear.

Blank title, subtitle, or footer values fall back safely to the built-in splash text. Missing fields also use built-in fallback text.

If `startup_splash_title` is missing or blank and `[app] name` is set to a non-blank value, the splash title uses `app.name`. Otherwise it falls back to the built-in `jcode` title.

The splash only appears on the empty startup screen when the onboarding welcome is inactive. It does not replace active conversation content.

## Background / Wallpaper MVP

Terminal-safe background customization is configured under `[display]`:

```toml
[display]
background_style = "subtle-grid"
background_opacity = 0.15
```

Supported MVP styles:

* `none`
* `subtle-grid`
* `stars`
* `matrix`

Missing `background_style` behaves as `none`. Invalid or empty styles also fall
back safely to `none` and do not prevent startup.

`background_opacity` is a numeric intensity control from `0.0` to `1.0`.
Missing values fall back to `0.15`. Out-of-range values are clamped to the valid
range. Invalid numeric values fall back safely. Because terminals do not provide
portable real alpha blending for text cells, opacity is simulated with pattern
density and a Theme Palette V2 color blend toward the active theme background.

The MVP renders only terminal text-cell patterns. It does not load image files,
does not support PNG/JPG wallpapers, does not use terminal image protocols, and
does not make network requests.

Rendering is intentionally limited to the empty startup/chat area when no
messages are present and onboarding is inactive. The optional startup splash is
drawn over the background panel. jcode does not draw the background over
messages, input text, dialogs, command palettes, onboarding, or active
conversation content.

Project-local `.mercury/workspace.toml` or `./.jcode/workspace.toml` can override
`display.background_style` and `display.background_opacity` because they are
visual-only fields.

## Top Status Bar

The optional top status bar is configured under `[display]`:

```toml
[display]
top_bar = true
top_bar_items = ["app", "session", "theme", "repo"]
```

When enabled, jcode renders one line at the top of the TUI:

```text
AymaneCode | session: main | theme: dracula | repo: jcode
```

The bar shows only safe MVP fields: app name, session name with a `main` fallback, active theme name with a `default` fallback, and the current repo/current-directory basename when available.

`display.top_bar_items` is optional. When it is missing, jcode preserves the
default order: `app`, `session`, `theme`, `repo`.

Supported MVP item names are:

* `app`
* `session`
* `theme`
* `repo`

An explicitly empty list renders no items while `display.top_bar = true` still
reserves the top bar line. Unknown item names are ignored safely and reported by
`jcode config show` as ignored items.

Project-local `.mercury/workspace.toml` or `./.jcode/workspace.toml` can
override `display.top_bar_items` because it is visual-only.

Token usage, multi-session controls, Queue integration beyond the current
session label, wallpaper, and split-pane controls are deferred.

## Config Visibility

Use:

```text
jcode config show
```

The command reports safe display/customization visibility details, including:

* Active theme
* Theme validity
* Theme source
* Custom invalid color count
* Active accent color
* App identity fields
* Startup splash fields
* Background style and opacity fields
* Top status bar setting
* Top status bar item order and ignored item names
* Project-local workspace config path, source, location, workspace name, and project-local display overrides

The heading uses `app.name` when configured and falls back to `jcode`.

This command is read-only and intended for diagnosing which customization settings are active.

## Implemented

The current customization system includes:

* Accent color override
* Built-in themes
* Custom named themes
* Theme Palette V2
* App identity config
* Startup splash personalization
* Terminal-safe background MVP
* Optional top status bar
* Configurable top status bar items
* Project-local visual workspace customization
* Workspace show/init/edit commands
* Parent-directory workspace discovery
* Config visibility
* Config editing command

## Deferred

The following customization areas are intentionally deferred:

* Real image wallpaper/background files
* Animated wallpaper/backgrounds
* Terminal image protocol wallpaper
* Project-local app identity or terminal title
* Theme import/export
* Full TUI recolor sweep
* Broad layout redesign
* Multi-session UI
* Multi-agent UI
* Token usage in the top status bar
