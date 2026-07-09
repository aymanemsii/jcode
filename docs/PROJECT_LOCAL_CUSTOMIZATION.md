# Project-Local Customization

This document investigates project-local workspace customization for jcode.

The current customization system is global. jcode reads user configuration from:

```text
~/.jcode/config.toml
```

When `JCODE_HOME` is set, jcode reads:

```text
$JCODE_HOME/config.toml
```

Project-local customization would let a repository carry a small, safe workspace identity layer without changing the user's global defaults.

## Why Project-Local Customization Is Useful

Different repositories often need different visual context. A user may work on the jcode source tree, a client project, an internal tool, and a personal app in the same day. Project-local customization can make those contexts easier to distinguish.

Useful outcomes include:

* Making the startup splash and top bar identify the current repository.
* Letting a project choose a preferred theme or accent color without changing every other project.
* Reducing accidental context confusion when multiple terminals are open.
* Giving teams a lightweight way to document the intended workspace identity for a repo.
* Preserving global user preferences while allowing targeted project overrides.

The first version should stay visual and informational. It should not add project-local behavior that affects credentials, provider selection, network behavior, command execution, or auth.

## Candidate File Path

Recommended project-local path:

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

## Proposed Config Shape

Use a small TOML shape that mirrors existing customization concepts but avoids global identity and secret-bearing settings:

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

The example in shorthand form:

```toml
[workspace]
name = "jcode"

[display]
theme = "cursor"
accent_color = "#0088FF"
startup_splash_title = "jcode dev mode"
top_bar = true
```

`workspace.name` should be the project-local label used by workspace-aware UI surfaces. It should not replace `app.name`, rename the binary, rename config directories, or change installed channel behavior.

`display.*` project-local values should use the same validation and fallback behavior as global display customization. Invalid colors, invalid theme names, or blank splash strings should fail closed to the next layer rather than preventing startup.

## Proposed Precedence

Recommended precedence:

1. Project-local workspace customization from `./.jcode/workspace.toml`
2. Global user config from `~/.jcode/config.toml` or `$JCODE_HOME/config.toml`
3. Built-in defaults

This order lets a repository override only the small set of project-safe fields. Missing or invalid project-local fields should fall through to the global user config. Missing or invalid global fields should fall through to built-in defaults.

Precedence should be field-level, not whole-file. For example, a project-local `display.theme` should not erase the global `display.startup_splash_footer` unless the workspace file explicitly sets that footer.

## Project-Local Fields For The First Slice

Allow only visual and workspace identity fields first:

* `workspace.name`
* `display.theme`
* `display.accent_color`
* `display.startup_splash_title`
* `display.startup_splash_subtitle`
* `display.startup_splash_footer`
* `display.top_bar`

These fields are appropriate because they are visible, low-risk, and already connected to the customization work in this fork. They help identify the current repository without changing model behavior, authentication, network access, or command execution.

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

There are two plausible discovery strategies:

* Current directory only: read `./.jcode/workspace.toml` from the process current working directory.
* Upward discovery: start at the current working directory and walk upward until a `./.jcode/workspace.toml` file is found.

Recommended MVP:

```text
current directory only
```

Current-directory-only loading is safer and easier to explain. It avoids surprising users when jcode is launched from a subdirectory and silently loads a parent repository's settings. It also avoids extra filesystem traversal during startup.

Upward discovery can be considered later, but it should be explicit, well-documented, and ideally paired with CLI visibility so users can see exactly which workspace file was loaded.

## CLI Visibility

Later, `jcode config show` should make layered values visible. It should report both global and project-local sources where applicable.

Useful output concepts:

* Global config path.
* Whether a project-local workspace file was found.
* Project-local workspace config path when loaded.
* Effective value for each supported workspace/display field.
* Source for each effective value: project, global, or default.
* Ignored or invalid project-local values with safe summaries.

`jcode config show` should remain safe and avoid printing secrets. Since project-local customization should not include secrets, this should be straightforward for workspace fields.

## Future Commands

Future workspace-focused commands could make the feature easier to understand:

```text
jcode workspace show
jcode workspace init
jcode workspace edit
```

`jcode workspace show` should show the loaded workspace file, effective workspace values, and source layering.

`jcode workspace init` should create `./.jcode/workspace.toml` with a minimal safe template.

`jcode workspace edit` should open the project-local workspace file in the user's editor. It should create only the local `.jcode` directory and `workspace.toml`, not modify global config.

## Risks

Important risks:

* Confusing precedence between project and global config.
* Accidentally loading untrusted repo config.
* Config drift between global and project-local shapes.
* Making startup slower with filesystem discovery.
* Users expecting all global config fields to work project-locally.
* Teams committing local-only visual preferences that not every contributor wants.

The MVP should reduce these risks by using a distinct filename, a small allowlist of fields, current-directory-only loading, and clear CLI visibility before broadening the feature.

## Recommended First Implementation Slice

Recommended first implementation after this investigation:

1. Add a workspace config model that only contains `workspace.name` and the allowed `display.*` fields.
2. Load `./.jcode/workspace.toml` from the current working directory only.
3. Merge workspace values over global config at the field level for the allowlisted fields.
4. Reuse existing validation and fallback behavior for themes, accent colors, splash text, and `top_bar`.
5. Update `jcode config show` to report whether project-local config was loaded and show project/global/default sources for the allowlisted fields.
6. Add documentation and focused unit tests for precedence, invalid values, and absent workspace files.

Do not include upward discovery, secrets, provider/model settings, auth settings, network/privacy settings, automatic execution, workspace commands, Queue integration, server protocol changes, or Cargo version changes in the first implementation slice.
