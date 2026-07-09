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

Customization is configured through TOML sections such as `[app]`, `[display]`, and `[themes.<name>]`.

## Project-Local Customization

jcode also supports a visual-only project-local workspace file in the current
working directory:

```text
./.jcode/workspace.toml
```

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
top_bar = true
```

Only these project-local fields are supported:

* `workspace.name`
* `display.theme`
* `display.accent_color`
* `display.startup_splash_title`
* `display.startup_splash_subtitle`
* `display.startup_splash_footer`
* `display.top_bar`

Project-local values override global config at field level for those fields
only. jcode does not search parent directories for this MVP.

Global config remains the source for `app.name`, `app.terminal_title`,
credentials, providers, auth, privacy/network settings, execution settings, and
everything else not listed above. Project-local config is intentionally not a
place for secrets or command execution behavior.

## App Identity

Optional app identity settings live under `[app]`:

```toml
[app]
name = "AymaneCode"
terminal_title = "AymaneCode"
```

Both fields are optional. Missing, empty, or whitespace-only values fall back to the current jcode behavior.

`app.name` is currently used as the default startup splash title only when `display.startup_splash_title` is missing or blank. An explicit non-blank `display.startup_splash_title` always wins.

`app.terminal_title` customizes the active TUI terminal/window title where jcode already updates that title. It does not rename the binary, config directory, `JCODE_HOME`, crates, packages, or documentation globally.

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

## Top Status Bar

The optional top status bar is configured under `[display]`:

```toml
[display]
top_bar = true
```

When enabled, jcode renders one line at the top of the TUI:

```text
AymaneCode | session: main | theme: dracula | repo: jcode
```

The bar shows only safe MVP fields: app name, session name with a `main` fallback, active theme name with a `default` fallback, and the current repo/current-directory basename when available.

Token usage, multi-session controls, Queue integration, project-local customization, wallpaper, and split-pane controls are deferred.

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
* Top status bar setting
* Project-local workspace config path, workspace name, and project-local display overrides

This command is read-only and intended for diagnosing which customization settings are active.

## Implemented

The current customization system includes:

* Accent color override
* Built-in themes
* Custom named themes
* Theme Palette V2
* App identity config
* Startup splash personalization
* Optional top status bar
* Project-local visual workspace customization
* Config visibility
* Config editing command

## Deferred

The following customization areas are intentionally deferred:

* Wallpaper/image background
* Workspace init/edit commands
* Parent-directory workspace discovery
* Project-local app identity or terminal title
* Theme import/export
* Full TUI recolor sweep
* Broad layout redesign
* Multi-session UI
* Multi-agent UI
* Token usage in the top status bar
