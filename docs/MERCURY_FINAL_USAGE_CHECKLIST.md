# Mercury Final Usage Checklist

This checklist summarizes the practical Mercury workflow after the rename and
migration compatibility track.

## 1. What Mercury Is Now

* `mercury` is the preferred command for new usage.
* `jcode` remains available as a compatibility command for existing scripts,
  habits, docs, and installs.
* A hard internal crate/package rename is not required yet. The current rename
  layer is intentionally compatibility-first and user-facing.

## 2. New-User Paths

New Mercury users should use the Mercury paths by default:

```text
~/.mercury/config.toml
./.mercury/workspace.toml
```

Use `~/.mercury/config.toml` for global user config.
Use `./.mercury/workspace.toml` for project-local visual/workspace config.

## 3. Legacy Compatibility

Existing jcode paths continue to work:

* `JCODE_HOME` still works when `MERCURY_HOME` is not set.
* `~/.jcode/config.toml` still works as the legacy global config fallback.
* `./.jcode/workspace.toml` still works as the legacy workspace config fallback.
* Migration never deletes legacy files.

Compatibility is additive. Mercury prefers Mercury names for new files, while
legacy files remain valid fallbacks for existing users and scripts.

## 4. Migration Commands

Dry-run commands:

```text
mercury migrate config --dry-run
mercury migrate workspace --dry-run
mercury migrate all --dry-run
```

Copy commands:

```text
mercury migrate config
mercury migrate workspace
mercury migrate all
```

`migrate config` copies the legacy global config to the Mercury global config
path when safe.

`migrate workspace` copies the current-directory legacy workspace config to the
current-directory Mercury workspace config path when safe.

`migrate all` runs the config and current-directory workspace migration flows
together, with the same no-overwrite behavior for each target.

## 5. Recommended Safe Migration Flow

1. Run the dry-run first:

   ```text
   mercury migrate all --dry-run
   ```

2. Inspect the reported source and destination paths.
3. Run the copy migration:

   ```text
   mercury migrate all
   ```

4. Confirm the resolved global config:

   ```text
   mercury config show
   ```

5. Confirm the resolved workspace config:

   ```text
   mercury workspace show
   ```

6. Keep the legacy `jcode` files until you are comfortable with the Mercury
   paths and have verified your normal workflow.

## 6. What Migration Will Not Do

Migration is intentionally conservative:

* no overwrite
* no delete
* no move
* no force
* no backup behavior yet
* no parent traversal workspace writes
* no network access or telemetry

Migration copies files only when the destination is missing and the operation is
safe. It does not rewrite TOML, delete old paths, or discover parent workspace
files as write targets.

## 7. Installation and Storage Checklist

Build the preferred Mercury binary when working from source:

```text
cargo build --bin mercury
```

After build/source work, `cargo clean` is optional if you need to reclaim local
disk space.

The installed Mercury binary should not take 28GB. Large disk usage usually
comes from `target/` build artifacts, incremental compilation state, dependency
caches, and other source-build outputs rather than the installed binary itself.

## 8. Final Safe Tags and Checkpoints

Latest relevant safe checkpoints:

* `safe-after-mercury-workspace-default`
* `safe-after-alias-aware-cli-hints`
* `safe-after-docs-default-to-mercury`
* `safe-after-mercury-migration-architecture`
* `safe-after-mercury-migrate-dry-run`
* `safe-after-mercury-migrate-config-copy`
* `safe-after-mercury-migrate-workspace-copy`
* `safe-after-mercury-migrate-all-copy`

These tags/checkpoints mark the incremental Mercury rename and migration slices
through the final copy-based `migrate all` implementation.
