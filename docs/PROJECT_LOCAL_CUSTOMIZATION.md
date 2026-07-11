# Project-Local Customization

This document describes the project-local workspace customization MVP in jcode
and records the remaining deferred work.

The current customization system is global. jcode reads user configuration from:

```text
~/.jcode/config.toml
```

When `JCODE_HOME` is set, jcode reads:

```text
$JCODE_HOME/config.toml
```

Project-local customization lets a repository carry a small, safe workspace
identity layer without changing the user's global defaults.

## Why Project-Local Customization Is Useful

Different repositories often need different visual context. A user may work on the jcode source tree, a client project, an internal tool, and a personal app in the same day. Project-local customization can make those contexts easier to distinguish.

Useful outcomes include:

* Making the startup splash and top bar identify the current repository.
* Letting a project choose a preferred theme or accent color without changing every other project.
* Reducing accidental context confusion when multiple terminals are open.
* Giving teams a lightweight way to document the intended workspace identity for a repo.
* Preserving global user preferences while allowing targeted project overrides.

The first version should stay visual and informational. It should not add project-local behavior that affects credentials, provider selection, network behavior, command execution, or auth.

## File Path

jcode reads the project-local workspace file only from the current working
directory:

```text
./.jcode/workspace.toml
```

This path keeps project-local workspace identity separate from the global user config file:

```text
~/.jcode/config.toml
$JCODE_HOME/config.toml
```

`./.jcode/workspace.toml` is preferable to reusing `./.jcode/config.toml` for the MVP because `config.toml` already means user-level jcode configuration. Reusing the same filename inside a repository could imply that every global option is valid project-locally, including sensitive provider, auth, privacy, and network settings. A distinct `workspace.toml` name makes the scope clearer: this file describes the local workspace, not the user's account, credentials, or global app behavior.

Only reuse `./.jcode/config.toml` if there is a strong future reason to support a full layered config system and the loader can clearly separate project-safe keys from global-only keys.

For the MVP, jcode does not search parent directories.

## Config Shape

The supported TOML shape mirrors existing visual customization concepts but
avoids global identity and secret-bearing settings:

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
```

The example in shorthand form:

```toml
[workspace]
name = "jcode"

[display]
theme = "cursor"
accent_color = "#0088FF"
startup_splash_title = "jcode dev mode"
background_style = "stars"
top_bar = true
```

`workspace.name` should be the project-local label used by workspace-aware UI surfaces. It should not replace `app.name`, rename the binary, rename config directories, or change installed channel behavior.

`display.*` project-local values should use the same validation and fallback behavior as global display customization. Invalid colors, invalid theme names, blank splash strings, invalid background styles, or out-of-range background opacity values should fail closed to a safe fallback rather than preventing startup.

## Precedence

Effective display customization is resolved at field level:

1. Project-local workspace customization from `./.jcode/workspace.toml`
2. Global user config from `~/.jcode/config.toml` or `$JCODE_HOME/config.toml`
3. Built-in defaults

This order lets a repository override only the small set of project-safe fields. Missing or invalid project-local fields should fall through to the global user config. Missing or invalid global fields should fall through to built-in defaults.

Precedence should be field-level, not whole-file. For example, a project-local `display.theme` should not erase the global `display.startup_splash_footer` unless the workspace file explicitly sets that footer.

## Project-Local Fields

The MVP allows only visual and workspace identity fields:

* `workspace.name`
* `display.theme`
* `display.accent_color`
* `display.startup_splash_title`
* `display.startup_splash_subtitle`
* `display.startup_splash_footer`
* `display.background_style`
* `display.background_opacity`
* `display.top_bar`

These fields are appropriate because they are visible, low-risk, and already connected to the customization work in this fork. Background style and opacity are visual-only terminal text-cell settings; they do not introduce image files, terminal image protocols, network access, or command execution. These fields help identify the current repository without changing model behavior, authentication, network access, or command execution.

## Global-Only Fields For Now

Keep these fields global-only for now:

* `app.name`
* `app.terminal_title`
* Provider and model credentials
* Auth and secrets
* Privacy and network settings

`app.name` and `app.terminal_title` should remain global because they describe the installed application identity and terminal/window title policy, not just the current repository. Provider credentials, auth tokens, secrets, privacy controls, and network behavior must not be read from untrusted repository files.

## Security And Safety

Project-local config should be treated as untrusted input because repositories can come from third parties.

Safety rules for the MVP:

* Do not store secrets in project-local config.
* Do not load provider credentials or auth tokens from project-local config.
* Do not allow automatic command execution from project-local config.
* Do not allow project-local config to enable new network behavior.
* Limit the first slice to visual and workspace identity fields.
* Validate every field and fall back safely on invalid values.
* Keep unknown keys ignored or reported as ignored; do not silently expand behavior.

The most important safety boundary is that a cloned repo should not be able to change how jcode authenticates, which providers it calls, what network requests it makes, or what commands it runs.

## Loading Strategy

jcode currently uses:

```text
current directory only
```

Current-directory-only loading is safer and easier to explain. It avoids surprising users when jcode is launched from a subdirectory and silently loads a parent repository's settings. It also avoids extra filesystem traversal during startup.

Upward discovery remains deferred. If added later, it should be explicit,
well-documented, and paired with CLI visibility so users can see exactly which
workspace file was loaded.

## CLI Visibility

`jcode config show` reports project-local customization compactly:

* Project-local workspace config path when loaded, or `not found`.
* `workspace.name` when present.
* Which project-local display fields override global config.

`jcode config show` should remain safe and avoid printing secrets. Since project-local customization should not include secrets, this should be straightforward for workspace fields.

## Workspace Commands

Workspace-focused commands make the current-directory project-local file easier
to inspect and manage:

```text
jcode workspace show
jcode workspace init
jcode workspace edit
```

`jcode workspace show` reports whether `./.jcode/workspace.toml` exists in the
current directory. When present, it prints the file path, `workspace.name`, and
the supported project-local `display.*` fields. When missing, it prints the path
that would be used and suggests `jcode workspace init` or `jcode workspace edit`.

`jcode workspace init` creates `./.jcode/workspace.toml`, creating `./.jcode/`
first when needed. It does not overwrite an existing file. The generated file is
visual-only:

```toml
[workspace]
name = "<current-directory-name>"

[display]
theme = "cursor"
top_bar = true
```

`jcode workspace edit` opens `./.jcode/workspace.toml` in the user's editor. If
the file is missing, it creates the same safe default first. Editor selection
matches `jcode config edit`: `VISUAL`, then `EDITOR`, then Notepad on Windows,
with a helpful error on macOS/Linux if no editor is configured.

All workspace commands are current-directory only. They do not search parent
directories, modify global config, add secrets, or change provider, auth,
network, execution, Queue, or server protocol behavior.

## Risks

Important risks:

* Confusing precedence between project and global config.
* Accidentally loading untrusted repo config.
* Config drift between global and project-local shapes.
* Making startup slower with filesystem discovery.
* Users expecting all global config fields to work project-locally.
* Teams committing local-only visual preferences that not every contributor wants.

The MVP should reduce these risks by using a distinct filename, a small allowlist of fields, current-directory-only loading, and clear CLI visibility before broadening the feature.

## Implemented MVP

The implemented MVP:

* Adds a workspace config model containing `workspace.name` and the allowed `display.*` fields.
* Loads only `./.jcode/workspace.toml` from the current working directory.
* Merges workspace values over global config at the field level for the allowlisted fields.
* Reuses existing validation and fallback behavior for themes, accent colors, splash text, terminal-safe background fields, and `top_bar`.
* Updates `jcode config show` to report the loaded workspace path, workspace name, and project-local display overrides.
* Adds `jcode workspace show`, `jcode workspace init`, and `jcode workspace edit`.
* Rejects non-allowlisted project-local sections instead of treating them as config.

The MVP does not include upward discovery, secrets, provider/model settings,
auth settings, network/privacy settings, automatic execution, workspace
Queue integration, server protocol changes, or Cargo version changes.
