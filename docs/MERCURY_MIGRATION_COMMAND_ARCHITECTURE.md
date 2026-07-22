# Mercury Migration Command Architecture

This document investigates a future safe migration command for Mercury users.
It is documentation-only and does not implement commands, change config
resolution, modify workspace discovery, edit Rust code, change Cargo metadata,
or alter Queue, server, or TUI behavior.

Mercury now supports the preferred new surfaces:

* `mercury` alias binary
* `MERCURY_HOME`
* `~/.mercury/config.toml`
* `.mercury/workspace.toml`

It also preserves legacy compatibility:

* `jcode` compatibility binary
* `JCODE_HOME`
* `~/.jcode/config.toml`
* `.jcode/workspace.toml`

The migration command should help existing users copy legacy config and
workspace files into Mercury locations without breaking older scripts or
removing fallback paths.

## Implementation Status

`mercury migrate config`, `mercury migrate workspace`, and
`mercury migrate all` are now implemented for both dry-run reporting and
copy-first migration. The implemented copy behavior remains conservative:
existing Mercury destinations are not overwritten, legacy jcode files are not
deleted or moved, and workspace writes are limited to the current directory.

## Proposed Command Shape

Recommended future command group:

```text
mercury migrate config
mercury migrate workspace
mercury migrate all
```

Recommended common option:

```text
--dry-run
```

Possible future overwrite option:

```text
--force
```

or:

```text
--overwrite
```

The default behavior should be conservative. The command should not overwrite
existing Mercury files unless the user explicitly asks for that behavior.
`--force` or `--overwrite` should be added only if there is clear user need and
after backup semantics are designed.

## Migration Rules

The migration command should be copy-first and non-destructive:

* Copy legacy files by default; do not move them.
* Never delete legacy `jcode` files.
* Never overwrite existing Mercury files unless an explicit force/overwrite
  option is provided.
* Show clear source and destination paths before writing.
* Preserve comments and formatting where possible by using raw file copy.
* Warn when both old and new files exist.
* Warn when `MERCURY_HOME` or `JCODE_HOME` changes the expected global config
  source or destination.

The command should treat migration as a convenience copy, not as a semantic
rewrite. It should not parse and reserialize TOML for the default path because
that could reorder keys, drop comments, or otherwise surprise users.

## Config Migration

Default no-env migration:

```text
~/.jcode/config.toml -> ~/.mercury/config.toml
```

Env-aware migration when relevant:

```text
JCODE_HOME/config.toml -> MERCURY_HOME/config.toml
```

Recommended precedence for choosing paths:

1. If `MERCURY_HOME` is set, the Mercury destination is
   `MERCURY_HOME/config.toml`.
2. If `MERCURY_HOME` is not set, the default Mercury destination is
   `~/.mercury/config.toml`.
3. If `JCODE_HOME` is set, the legacy source is `JCODE_HOME/config.toml`.
4. If `JCODE_HOME` is not set, the default legacy source is
   `~/.jcode/config.toml`.

When both `MERCURY_HOME` and `JCODE_HOME` are set, the command should show both
resolved paths and warn that env vars are controlling migration locations. It
should not silently copy from or to a surprising path.

When `MERCURY_HOME` is set but `JCODE_HOME` is not set, the destination changes
but the legacy source remains `~/.jcode/config.toml`. The command should print
that explicitly. When `JCODE_HOME` is set but `MERCURY_HOME` is not set, the
source changes but the destination remains `~/.mercury/config.toml`.

If the source file does not exist, the command should report that there is
nothing to migrate. If the destination already exists, the command should warn
and refuse to write by default.

## Workspace Migration

Default current-directory migration:

```text
./.jcode/workspace.toml -> ./.mercury/workspace.toml
```

The MVP should operate only in the current directory:

* Do not traverse parent directories for write targets.
* Do not write into a parent workspace directory.
* Do not migrate every workspace discovered in a tree.
* Do not shadow an existing `./.mercury/workspace.toml`.

This keeps the first workspace migration slice easy to reason about. Parent
directory discovery is useful for reading active workspace config, but migration
writes should be explicit and local at first.

If `./.jcode/workspace.toml` exists and `./.mercury/workspace.toml` does not
exist, the command may copy the file. If both exist, it should warn and refuse
to overwrite by default. If only `./.mercury/workspace.toml` exists, it should
report that no legacy current-directory workspace file needs migration.

## `migrate all`

`mercury migrate all` should combine the config and workspace plans in one
report. It should still apply the same safety rules independently to each
target:

* config migration may be skipped because the source is missing
* workspace migration may be skipped because the current directory has no
  legacy workspace file
* either migration may be blocked by an existing Mercury destination
* dry-run should report all intended actions without writing anything

The command should avoid making `all` more powerful than the individual
commands. It should not add parent traversal, deletion, network access, or
automatic conflict resolution.

## Safety Boundaries

Recommended safety behavior:

* Encourage `--dry-run` before writes.
* Default to no-overwrite.
* If overwrite is ever allowed, create a timestamped backup of the destination
  before writing.
* Do not print secrets or raw file contents.
* Do not contact the network.
* Do not emit telemetry.
* Do not change Cargo files.
* Do not change server, Queue, or TUI behavior.

The command output should focus on file paths, action status, and warnings. It
should not display config contents because provider settings, auth references,
or local paths may be sensitive.

## Validation Plan

Recommended future tests:

* Temp `HOME` tests for default `~/.jcode/config.toml` to
  `~/.mercury/config.toml` copying.
* `MERCURY_HOME` and `JCODE_HOME` precedence tests.
* Current-directory workspace tests for `./.jcode/workspace.toml` to
  `./.mercury/workspace.toml` copying.
* Conflict/no-overwrite tests when Mercury destination files already exist.
* Idempotency tests showing repeat runs do not rewrite or duplicate work.
* Dry-run tests proving no files are created or changed.
* Output tests showing source and destination paths are reported clearly while
  file contents are not printed.

Validation should use temporary directories and isolated environment variables.
It should not depend on a developer's real home directory or real workspace
config.

## Recommended Implementation Phases

Implement later in small slices:

### Phase A: Docs

Document the command architecture and safety rules before implementation.

### Phase B: Dry-Run Command Only

Add `mercury migrate config`, `mercury migrate workspace`, and
`mercury migrate all` as dry-run/reporting commands. They should resolve paths,
show intended actions, and write nothing.

### Phase C: Config Copy

Add non-destructive config copying. Preserve raw file bytes and refuse to write
when the destination already exists.

### Phase D: Workspace Copy

Add current-directory-only workspace copying. Preserve raw file bytes and refuse
to write when `./.mercury/workspace.toml` already exists.

Implemented for `migrate workspace`.

### Phase E: All Command

Enable `migrate all` to run the config and workspace copy flows together while
keeping their independent no-overwrite behavior.

Implemented for `migrate all`.

### Phase F: Force/Backup Only If Needed

Consider `--force` or `--overwrite` only after real user need appears. If added,
the command should create a backup of the existing Mercury destination before
overwriting it.

## Recommendation

Implement the migration command later as a copy-first, non-destructive helper.
Keep existing `jcode` files supported indefinitely through the compatibility
fallbacks. The command should make migration explicit, visible, reversible, and
safe for users who still rely on legacy paths or scripts.
