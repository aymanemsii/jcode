# Mercury Compatibility Rename Architecture

This document investigates the safest compatibility rename layer for the Mercury fork. It is documentation-only and does not implement new config paths, environment variables, workspace paths, migration commands, protocol names, package names, crate names, or installer behavior.

Implementation has since added the `mercury` alias binary, `MERCURY_HOME`,
`~/.mercury/config.toml`, and `.mercury/workspace.toml` as preferred Mercury
surfaces/defaults. This is still not a hard rename: `jcode`, `JCODE_HOME`,
`~/.jcode/config.toml`, and `.jcode/workspace.toml` remain supported
compatibility paths.

## Goal

Mercury should feel like its own app while preserving existing `jcode` compatibility.

The rename layer should support new Mercury-specific paths and environment variables later without breaking users who already rely on `jcode` names, paths, scripts, and project-local customization.

The immediate goal is not a hard rename. The safe path is a compatibility layer that can read new Mercury locations while continuing to honor existing `jcode` locations for a long compatibility window.

## Compatibility Targets

Mercury-compatible surfaces include:

* `MERCURY_HOME`
* `~/.mercury/config.toml`
* `.mercury/workspace.toml`

Legacy compatibility is preserved for:

* `JCODE_HOME`
* `~/.jcode/config.toml`
* `.jcode/workspace.toml`

These are compatibility aliases and preferred new defaults, not destructive
replacements for the legacy `jcode` surfaces.

## Config Resolution Strategy

Recommended global config precedence:

1. Explicit `MERCURY_HOME`
2. Explicit `JCODE_HOME`
3. `~/.mercury/config.toml`
4. `~/.jcode/config.toml`
5. Fallback/default config

Rationale:

* An explicit Mercury environment variable should be the strongest signal that the user wants Mercury-specific configuration.
* Existing `JCODE_HOME` users should remain supported and should not be broken by introducing Mercury names.
* A present `~/.mercury/config.toml` should beat the legacy default location because it is the newer app-specific default.
* Existing `~/.jcode/config.toml` should remain a stable fallback.
* Built-in defaults should be used only when no configured or discovered file exists.

If both `MERCURY_HOME` and `JCODE_HOME` are set, Mercury should use strict documented precedence rather than fail hard. The recommended behavior is:

* use `MERCURY_HOME`
* warn that both env vars are set
* include the ignored `JCODE_HOME` path in diagnostic output where appropriate

Failing hard would be safer for detecting misconfiguration, but it is likely too disruptive for a compatibility layer. Silent precedence is also risky because users can end up editing the wrong file. A warning gives clear behavior while keeping existing workflows running.

When commands create or edit config, write-target behavior is explicit. If `MERCURY_HOME` is set, write there. If only `JCODE_HOME` is set, continue writing there. If neither env var is set and no config exists, new global config creation defaults to `~/.mercury/config.toml`.

## Workspace Resolution Strategy

Parent-directory workspace discovery should support both:

* `.mercury/workspace.toml`
* `.jcode/workspace.toml`

Recommended discovery behavior:

1. Walk from the current directory upward toward the filesystem root.
2. At each directory, prefer `.mercury/workspace.toml` over `.jcode/workspace.toml` when both exist in that same directory.
3. Stop at the first directory that contains either workspace file.
4. If the nearest matching directory contains only `.jcode/workspace.toml`, use it.
5. If a parent contains `.mercury/workspace.toml` but a nearer child contains `.jcode/workspace.toml`, use the nearer child `.jcode/workspace.toml`.

This makes distance the primary project-local signal and file name the tie-breaker within the same directory.

The alternative, globally preferring `.mercury` even when `.jcode` is nearer, would make a parent Mercury config unexpectedly override a closer project-local legacy config. That would be surprising in monorepos and nested checkouts. Nearest workspace should win because parent-directory discovery is fundamentally about local project scope.

Same-directory precedence should prefer `.mercury/workspace.toml` because it is the new app-specific location. `workspace show` and `config show` report the resolved path/source so users can see which file is active.

Workspace init/edit now default brand-new current-directory workspace files to
`./.mercury/workspace.toml`. Existing current-directory `.jcode/workspace.toml`
files remain supported and are not migrated, moved, or copied automatically.

## Migration Strategy

Migration should be optional, conservative, and non-destructive.

Recommended approach:

* Do not automatically move, delete, or rewrite existing config or workspace files.
* Consider an optional future command such as `mercury migrate config`.
* Copy files by default; do not move them by default.
* Warn before overwriting any destination file.
* Preserve old paths as fallbacks after migration.
* Make the copied destination explicit in command output.
* Avoid automatically migrating credentials, auth material, or provider-specific secrets without a separate security review.

A future migration command should be idempotent where possible. If the Mercury destination already exists, it should either refuse, offer an explicit overwrite flag, or show a clear diff/backup workflow before writing.

## Repo Surfaces To Inspect Before Implementation

Before implementing the compatibility layer, inspect these surfaces:

* config path logic
* workspace path discovery
* workspace init/edit/show
* config show/edit
* docs/examples
* tests
* CLI output wording
* Windows path behavior
* env var handling

The implementation should also check any snapshots or tests that assert paths, environment variables, generated config text, command output, or workspace discovery behavior.

## Risks

Main risks:

* breaking existing `JCODE_HOME` users
* confusing dual config locations
* project-local override ambiguity
* workspace parent discovery edge cases
* Windows path quirks
* docs drift
* upstream merge friction

Specific compatibility risks:

* Users may edit `~/.mercury/config.toml` while the app is still reading `~/.jcode/config.toml`, or the reverse.
* Scripts may set `JCODE_HOME` and expect it to remain authoritative.
* A nested `.jcode/workspace.toml` could be shadowed accidentally if `.mercury` is given global priority instead of nearest-directory priority.
* Windows home and local-app-data conventions may not map cleanly to Unix-style examples.
* Tests and docs may drift if examples mix `jcode`, `mercury`, `.jcode`, and `.mercury` without explaining precedence.
* Broad internal renames can increase upstream merge conflicts before user-facing compatibility is proven.

## Recommended Implementation Phases

### Phase A: Docs-Only Investigation

Document the compatibility rename layer and agree on precedence before code changes.

### Phase B: Add `MERCURY_HOME` Support While Preserving `JCODE_HOME`

Status: implemented.

`MERCURY_HOME` is now supported as the higher-precedence explicit config home environment variable. `JCODE_HOME` remains supported as a fallback when `MERCURY_HOME` is not set. If both are set, `MERCURY_HOME` wins and `config show` reports that both env vars are set.

The default config directory remains unchanged in this phase: when neither env var is set, Mercury/jcode still uses the existing `~/.jcode` global config home. This phase does not add `~/.mercury/config.toml`, `.mercury/workspace.toml`, migration behavior, package/crate renames, provider user-agent changes, Queue changes, or server protocol changes.

### Phase C: Add `~/.mercury/config.toml` Support While Preserving `~/.jcode/config.toml`

Status: implemented.

Mercury default config discovery now reads `~/.mercury/config.toml` before the
legacy `~/.jcode/config.toml` fallback when neither explicit home env var is
set.

### Phase E1: Default New Global Configs To `~/.mercury/config.toml`

Status: implemented.

When neither `MERCURY_HOME` nor `JCODE_HOME` is set and no existing global
config file is found, brand-new global config creation now targets
`~/.mercury/config.toml`. Existing `~/.jcode/config.toml` remains a fallback,
and explicit env vars still take precedence. `config show` reports the resolved
config path and source; `config edit` creates or edits the resolved path.

### Phase D: Add `.mercury/workspace.toml` Support While Preserving `.jcode/workspace.toml`

Status: implemented.

Parent-directory workspace discovery now supports both
`.mercury/workspace.toml` and `.jcode/workspace.toml`. Discovery uses
nearest-directory precedence, with `.mercury` as the same-directory tie-breaker.
The implementation preserves the workspace allowlist and field-level merge
behavior, and `config show` / `workspace show` report the resolved workspace
path and source. Workspace init/edit write-target changes were intentionally
handled separately in Phase E2.

### Phase E2: Default New Workspace Configs To `./.mercury/workspace.toml`

Status: implemented.

Brand-new current-directory workspace config creation now targets
`./.mercury/workspace.toml`. `workspace init` creates the Mercury path when no
current-directory workspace config exists, and refuses an existing
current-directory `.mercury/workspace.toml` or `.jcode/workspace.toml` instead
of shadowing it. `workspace edit` opens an existing current-directory
`.mercury/workspace.toml` first, then an existing current-directory
`.jcode/workspace.toml`; if neither exists, it creates and opens
`./.mercury/workspace.toml`.

Parent-directory discovery remains unchanged: nearest workspace directory wins,
with `.mercury/workspace.toml` preferred over `.jcode/workspace.toml` only as a
same-directory tie-breaker. No automatic migration, copying, moving, command
rename, package/crate rename, provider user-agent rename, Queue change, server
protocol change, or Cargo change is included in this phase.

## What Not To Do Yet

Do not do these in the compatibility investigation or first compatibility layer:

* no hard crate rename
* no package rename
* no server protocol rename
* no provider user-agent rename
* no automatic migration
* no destructive moving of config files
* no removing jcode compatibility

The safest rename path is additive: introduce Mercury names as aliases and preferred new defaults while preserving `jcode` paths, env vars, and behavior.
