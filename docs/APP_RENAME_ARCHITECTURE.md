# App Rename Architecture

This document investigates a future rename path for Aymane's fork of jcode. It is documentation-only and does not implement a binary, config, environment, crate, package, protocol, or installer rename.

The current fork already has meaningful user-facing identity customization:

* telemetry removed
* network privacy docs
* app identity config
* terminal title config path
* customization v2
* built-in famous themes
* custom named themes
* theme commands
* top status bar
* configurable `top_bar_items`
* terminal-safe backgrounds
* project-local workspace config
* workspace commands
* parent-directory workspace discovery
* broader TUI color routing

That makes a soft rename practical before any hard rename is attempted.

## First Soft Branding Implementation Slice

The first soft branding implementation slice routes additional low-risk
user-facing labels through the existing `[app].name` identity setting.

Implemented surfaces now include:

* onboarding welcome title
* startup splash title fallback
* startup splash default subtitle
* top bar app item
* config visibility heading
* theme command headings
* terminal title hook through `[app].terminal_title`

This is intentionally still a soft branding pass. Missing, empty, or
whitespace-only app names continue to fall back to `jcode`, and hard rename work
remains deferred.

## Current Identity Layers

The implemented identity layers are configuration and presentation layers. They do not rename the installed binary, crates, packages, config directory, environment variables, release artifacts, or repository branding.

### `[app].name`

`[app].name` is the global installed app identity label. It is intended for user-facing UI surfaces where a custom build should identify itself without changing compatibility-sensitive names.

Current behavior:

* It is configured in global user config under `[app]`.
* Missing, empty, or whitespace-only values fall back to built-in jcode behavior.
* It can affect low-risk user-facing labels such as onboarding, startup splash,
  top bar, config visibility, and theme command headings.
* It does not change the `jcode` command, config directory, env vars, crates,
  packages, protocol labels, provider user-agent strings, or docs by itself.

### `[app].terminal_title`

`[app].terminal_title` customizes the active terminal/window title where the TUI already updates that title.

Current behavior:

* It is configured in global user config under `[app]`.
* Missing, empty, or whitespace-only values fall back safely.
* It does not rename the process, executable, shell command, or install channel.

### Startup Splash Title Fallback

The startup splash can be customized directly through `display.startup_splash_title`. If that field is missing or blank and `[app].name` is set to a non-blank value, the splash title can use `app.name`. Otherwise it falls back to the built-in `jcode` title.

This is a good soft-rename pattern because explicit display config wins, app identity is used as a secondary default, and built-in behavior remains stable.

### Top Bar App Name

The top status bar supports an `app` item through `display.top_bar_items`. That item is a natural place to display the resolved app identity.

For a soft rename, the top bar should prefer the configured app name where safe while keeping existing fallback behavior for missing or invalid config.

### Terminal Title Hook

The terminal title hook is already a user-facing app identity surface. It should remain a soft-rename surface: configurable, reversible, and independent of binary/config/env names.

This hook is especially useful when multiple terminals are open because it can communicate the custom build name without affecting scripts that still call `jcode`.

### Project-Local Workspace Identity

Project-local customization is stored in:

```text
.mercury/workspace.toml
```

Mercury compatibility also reads `.mercury/workspace.toml` during
parent-directory discovery. The existing `.jcode/workspace.toml` path remains
supported as a fallback. New workspace init/edit creation defaults now target
`./.mercury/workspace.toml`; edit still opens an existing current-directory
`./.jcode/workspace.toml` when no same-directory Mercury file exists.

The workspace identity field is:

```toml
[workspace]
name = "project label"
```

`workspace.name` identifies the repository or workspace. It should not replace `app.name`, rename the binary, rename config directories, or change installed channel behavior.

Project-local identity is useful for per-repo context, while `[app].name` is useful for the installed fork identity. Keeping those layers separate reduces migration risk.

## Soft Rename Vs Hard Rename

### Soft Rename

A soft rename changes user-facing labels while preserving compatibility-sensitive names.

Soft rename surfaces include:

* configurable `app.name`
* configurable terminal title through `app.terminal_title`
* startup splash title and fallback behavior
* startup splash default subtitle
* onboarding welcome title
* top bar app item
* config/theme command headings
* docs wording that describes this as Aymane's fork or custom build where appropriate
* generated examples that can show customization without implying command renames

Soft rename intentionally does not change:

* binary name
* config directory
* environment variable names
* project-local `.jcode` directory
* crate names
* package names
* release artifact names
* command names

Recommendation: continue with the soft rename first. It gives the fork its own visible identity with low migration risk and minimal upstream merge friction.

Hard rename remains deferred. It should still be treated as a separate
migration project with compatibility aliases, fallback order, docs, tests, and
installer behavior designed before implementation.

The next hard-rename path should be a compatibility rename layer, not an
immediate destructive rename. Mercury-specific names such as future config
paths and environment variables should be introduced additively while existing
`jcode` paths and env vars remain supported.

## Alias Binary MVP

A compatibility-safe alias binary MVP now exists:

* `jcode` remains the canonical binary.
* `mercury` is available as a second Cargo bin target.
* `mercury` uses the same startup source path as `jcode`.
* Config directories, environment variables, project-local `.jcode` paths,
  provider user-agent constants, package/crate names, Queue behavior, server
  protocol behavior, installers, release artifacts, and hard rename work remain
  unchanged.

This is Phase B's first code-level alias slice, not a hard rename.

### Hard Rename

A hard rename changes compatibility-sensitive names and should happen only after compatibility aliases and migration behavior are designed.

Hard rename surfaces include:

* binary name
* config directory
* environment variables
* command/help output
* docs
* package/build metadata
* internal constants
* provider user-agent strings
* paths
* installer and release artifact names, if any
* crate names
* workspace package names

Hard rename should be treated as a migration project, not a search-and-replace. Many surfaces are externally observable and may be used by users, scripts, tests, installers, or packaging.

## Rename Surfaces To Audit

These surfaces should be audited before any hard rename implementation.

### CLI And Process Identity

* Binary name: `jcode`
* Command examples using `jcode`
* Commands and help text
* Shell completions, if any
* Server/process title strings
* Any spawned process command lines such as `jcode ... serve`
* Terminal title fallback

### Config, Workspace, State, And Paths

* Global config directory: `~/.jcode`
* Windows global config/install equivalents under local app data
* Env vars such as `JCODE_HOME`
* Env vars with `JCODE_NO_*` names
* Project-local directory: `.jcode`
* Project-local workspace file: `.mercury/workspace.toml`, with legacy compatibility for `.jcode/workspace.toml`
* Logs, cache, and state paths
* Generated default config comments
* Install notes and launcher paths

### Documentation And User-Facing Text

* README references
* `docs/` references
* CLI examples
* Network privacy docs
* Customization docs
* Workspace docs
* Release docs
* Historical telemetry references that may still mention old naming, even though telemetry behavior has been removed from the fork
* GitHub URLs and repository branding

### Build, Package, And Source Metadata

* Package metadata
* Cargo package names
* Crate names
* Workspace package names
* Build scripts
* Installer names
* Release artifact names
* Any package manager metadata

### Runtime Constants And Protocol-Adjacent Text

* Internal constants that contain `jcode`
* Provider user-agent constants
* Server names and process labels
* Protocol-visible labels, if any
* Tests and snapshots that assert command names, paths, help output, or generated config text

Protocol-visible strings need special care. Even if the eventual rename is hard, server/client protocol compatibility should not be broken casually.

## Risks

The main risks are compatibility and drift:

* Breaking existing user configs under `~/.jcode`
* Migration complexity across config, logs, cache, state, auth, and install paths
* Old scripts expecting the `jcode` command
* Env var compatibility for `JCODE_HOME` and `JCODE_NO_*`
* Config path compatibility for `~/.jcode`
* Project-local compatibility for `.jcode/workspace.toml`
* Upstream merge friction if crates, package names, and broad internal names diverge too early
* Cargo package and crate renaming complexity
* Windows path and installer weirdness, especially launcher paths and local app data directories
* Documentation drift between soft-branded docs and hard-coded command examples
* Tests and snapshots failing because they assert old strings
* Provider or server behavior changing if user-agent/process strings are renamed without review

## Recommended Phased Rename Plan

### Phase A: Complete The Soft Rename

Keep the binary as `jcode`.

Use app identity config everywhere user-facing where it is safe:

* startup splash fallback
* top bar app label
* terminal title
* low-risk status labels
* generated examples or docs that describe Aymane's fork/custom build

Docs should be explicit that command examples still use `jcode` unless a compatibility alias exists.

### Phase B: Add Alias Or Wrapper Binary If Feasible

Investigate adding a new alias/wrapper binary while keeping the `jcode` binary for compatibility.

The alias should forward to the same behavior and preserve existing config/env defaults at first. This gives users a new command name without immediately moving config directories or breaking automation.

Initial MVP status: implemented as a second Cargo bin target named `mercury` that shares `src/main.rs` with `jcode`. Installer and release packaging changes remain deferred.

### Phase C: Add New Config Dir And Env Var Aliases

Introduce new config directory and environment variable aliases only after the command alias is stable.

Compatibility should be preserved:

* `MERCURY_HOME` is now supported as a higher-precedence explicit config home.
* Continue supporting `JCODE_HOME`.
* Continue reading `~/.jcode`.
* Add a new home/env alias only with documented precedence.
* Keep project-local `.jcode/workspace.toml` working while also reading `.mercury/workspace.toml`.

When both `MERCURY_HOME` and `JCODE_HOME` are set, `MERCURY_HOME` takes precedence and `config show` reports the conflict. When neither env var is set, Mercury compatibility reads `~/.mercury/config.toml` before falling back to `~/.jcode/config.toml`.

### Phase D: Optional Hard Rename

Only after compatibility aliases exist, consider a hard rename of crates, package metadata, binary names, release artifacts, docs, and internal constants.

This should be optional. If upstream mergeability remains valuable, stopping at Phase B or Phase C may be the better long-term tradeoff.

## Migration Strategy

Migration should prefer compatibility over forced moves.

Recommended fallback order for global config:

1. Explicit new app home env var, if one is introduced later.
2. `JCODE_HOME`, for existing compatibility.
3. New app config directory, if one is introduced later and exists.
4. Existing `~/.jcode`.
5. Built-in defaults.

Recommended fallback behavior:

* Keep reading `~/.jcode` initially.
* Optionally support a new config directory later.
* Prefer compatibility aliases over breaking renames.
* Do not break existing `.jcode/workspace.toml` immediately.
* Document whether new config directories are read-only fallbacks, write targets, or migration destinations.
* Avoid automatically moving auth or credential material without a separate security review.
* Keep generated default config comments clear about both old and new names during migration.

Project-local workspace migration should be even more conservative. Many repositories may already contain `.jcode/workspace.toml`; changing that path would break repository-local identity with little immediate benefit. If a new project-local directory is ever introduced, `.jcode/workspace.toml` should remain supported for a long deprecation window.

## Recommended Next Implementation Slice

The first app identity docs and branding pass has started. Future soft branding
slices should stay similarly narrow and use `app.name` only where config is
already loaded and the label is clearly user-facing.

Scope:

* Continue auditing low-risk user-facing labels where `app.name` can be used as
  a fallback without changing commands, paths, env vars, protocols, provider
  user-agent constants, package metadata, or Queue behavior.
* Update customization and network/privacy docs to describe Aymane's fork/custom build where appropriate.
* Keep command examples as `jcode`.
* Make docs clear that `[app].name` and `[app].terminal_title` are soft identity, not hard rename controls.

Alternative small slice: terminal title polish. Verify that all existing terminal title updates use the configured title consistently and fall back cleanly.

Another small investigation slice: alias command/binary feasibility. Audit Cargo/bin layout, installer scripts, shell completions, and release packaging before writing code.

## What Not To Do Yet

Do not do these until a compatibility plan exists:

* no immediate crate rename
* no immediate config dir rename
* no immediate env var rename
* no breaking command rename
* no broad search/replace
* no package/release artifact rename
* no project-local `.jcode` path rename
* no server protocol label changes without protocol review
* no automatic migration of credentials or auth files

The fork can feel like Aymane's app through soft identity first. A hard rename should remain a deliberate migration after aliasing, fallback order, docs, tests, and installers are ready.
