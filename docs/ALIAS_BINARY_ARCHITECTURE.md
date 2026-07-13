# Alias Binary Architecture

This document tracks the alias or wrapper command path for Aymane's fork of jcode. The initial MVP now implements a compatibility-safe Cargo alias binary named `mercury` while preserving the existing `jcode` binary and all compatibility-sensitive names.

## MVP Implementation Status

Implemented:

* added a second Cargo bin target named `mercury`
* pointed `mercury` at the same `src/main.rs` startup source as `jcode`
* kept `jcode` as the canonical command
* preserved existing config directories, environment variables, project-local `.jcode` paths, provider user-agent constants, package/crate names, Queue behavior, server protocol behavior, and hard rename deferrals

The only intended behavioral difference for the MVP is the unavoidable executable name / `argv[0]` surface when users launch the app as `mercury`.

Installer, release artifact, shell completion, config directory, environment variable, and hard rename work remain deferred.

## Goal

The goal is to allow a custom command name later while keeping the existing `jcode` command compatible.

Possible future command names can remain placeholders for now, such as:

* `mercury`
* `<custom-name>`

The alias should be treated as a compatibility layer, not as a hard rename. Existing users, scripts, docs, config, workspaces, and release workflows should continue to work through `jcode`.

## Possible Approaches

### Second Cargo Bin Target

A second Cargo binary target can compile another executable that calls the same main entrypoint as `jcode`.

MVP shape:

* keep the existing `jcode` binary
* add a tiny alias binary named `mercury`
* share the real startup path instead of duplicating command logic
* keep config, environment variables, server behavior, and docs identical at first

The current Cargo structure supports this cleanly through explicit bin targets and `autobins = false`. The MVP uses the same `src/main.rs` path for both `jcode` and `mercury`, avoiding copied startup logic.

### Wrapper Script Installed Next To `jcode`

A small shell, PowerShell, batch, or platform-specific launcher script could live next to the installed `jcode` binary and forward arguments.

Potential behavior:

```text
<custom-name> <args...> -> jcode <args...>
```

This avoids changing Rust binary layout, but introduces platform-specific quoting, PATH, executable extension, and installer maintenance concerns.

### Shell Alias Or Function

Users can define a local shell alias or function:

```text
alias <custom-name>=jcode
```

This is the lowest-risk option for individual users, but it is not a real installed command. It does not help scripts on machines without the alias, shell completions may not follow automatically, and Windows shells require separate guidance.

### Installer-Level Alias

Installers could create a second launcher, symlink, shim, copy, or shortcut during install.

This keeps Rust source changes minimal, but pushes complexity into install and release scripts. It also needs careful handling across Unix, Windows, package managers, self-dev installs, stable installs, immutable versioned installs, and PATH ordering.

### Renamed Binary With Compatibility Symlink Or Copy

The primary binary could eventually be renamed, while `jcode` remains available as a compatibility symlink, shim, or copied executable.

This is closer to a hard rename and should not be the first step. It affects release artifact names, install paths, documentation, support expectations, and user scripts. It should only be considered after an alias has proven safe and compatibility behavior is documented.

## Repo Surfaces To Inspect

Before implementing any alias path, inspect these surfaces:

* Cargo bin targets and workspace package structure
* `src/main.rs` and whether the real entrypoint can be shared
* CLI args, dispatch, and help output
* process title and process identity assumptions
* terminal title behavior and `app.terminal_title`
* shell completions and generated command names
* installer and release scripts
* docs, README examples, and generated examples
* Windows executable naming, `.exe` behavior, launcher shims, and PATH behavior
* release artifact names and packaging expectations

Special care is needed anywhere the current executable path or `argv[0]` is used to spawn a server, find resources, print help, install completions, or locate updates.

## Compatibility Strategy

The first alias step should preserve compatibility:

* keep the `jcode` binary working
* make the alias binary share config initially
* do not rename `~/.jcode` yet
* do not rename `JCODE_HOME` yet
* do not rename `.jcode/workspace.toml` yet
* keep `app.name` as the user-facing brand layer

The alias should behave like another way to launch the same app, not like a new app with separate state.

Initial examples should continue to show `jcode` unless the example is specifically about the alias. This prevents docs from implying that the alias is required or that existing installs are obsolete.

## Risks

The main risks are compatibility drift and install complexity:

* duplicated binaries if two compiled executables contain separate startup logic
* confusing help output if the alias changes command names inconsistently
* install and release complexity across stable, current, canary, and versioned builds
* Windows executable naming, launcher, `.exe`, and PATH issues
* shell completions drift between `jcode` and the alias
* upstream merge friction if package or source layout changes too early
* server spawning bugs if a client launched through the alias expects a different binary name
* docs drift if examples mix `jcode`, `aymanecode`, and `<custom-name>` without clear rules
* support confusion if the alias appears to imply separate config or app state

## Recommended Implementation Path

The MVP follows the narrow path:

1. Add a tiny second bin target because the Cargo structure supports it cleanly.
2. Reuse the shared `src/main.rs` startup path.
3. Keep behavior identical except for unavoidable process `argv[0]` or binary-name differences.
4. Keep `jcode` as the canonical compatible command.
5. Keep config, state, env vars, server protocol, and workspace paths unchanged.
6. Update docs after implementation to explain the alias as optional.
7. Defer installer and release changes until the code-level alias is proven safe.

If a clean shared entrypoint is not possible without broader refactoring, prefer documenting a shell alias or installer-level experiment before changing Cargo or duplicating startup logic.

## What Not To Do Yet

Do not do these in the alias-binary investigation or first implementation slice:

* no hard crate rename
* no config dir rename
* no env var rename
* no package metadata rename
* no server protocol rename
* no global search/replace
* no release artifact rename
* no installer rewrite
* no Queue changes
* no forced migration from `jcode` examples

The safest next step is an optional alias that launches the same app and preserves all existing names that carry compatibility risk.
