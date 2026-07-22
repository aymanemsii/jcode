# Mercury Core Mini Animation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a small branded Mercury Core status strip near the top of the
conversation area without rewriting the TUI layout.

**Architecture:** Implement the first slice as render-only TUI chrome. Add a
small helper that builds a theme-styled idle/ready Mercury Core line, reserve at
most one row above normal messages when the chat area is tall enough, and skip
the strip when it would crowd small terminals or onboarding takeover screens.

**Tech Stack:** Rust, Ratatui, existing `jcode-tui` theme helpers, existing TUI
unit/render tests.

## Global Constraints

* Do not implement Queue, server, protocol, migration, or Cargo changes.
* Do not add a new config field for the MVP unless implementation inspection
  proves the strip cannot be safely always-on.
* Use existing theme/accent colors.
* Prefer render-only first.
* Avoid a big layout rewrite.
* Keep the strip subtle and non-distracting while typing or reading.
* Provide safe fallback text for terminals that do not render special glyphs or
  line characters well.

---

## 1. Goal

The MVP should make Mercury feel more intentionally branded by adding a compact
conversation-area status strip such as:

```text
☿ Mercury  ━━━●━━━━  ready
```

The strip appears near the top of the conversation area, above normal messages.
It does not replace the top bar, does not take over the homepage, and does not
change command, Queue, server, protocol, migration, or Cargo behavior.

## 2. Architecture

Use a narrow render-only helper in the TUI layer. The helper should return a
single Ratatui line or paragraph for the idle/ready state and should apply
existing theme colors, likely accent for `Mercury` or the pulse marker and muted
foreground for decorative rails.

Recommended placement is inside `crates/jcode-tui/src/tui/ui.rs` where the chat
area is already split and `draw_messages` receives `messages_area`. Reserve one
row from the top of `messages_area` only when all of these are true:

* the terminal has enough height after top bar/input/status layout;
* onboarding is not taking over the chat area;
* the command/menu overlay does not need the row;
* the message area width can show at least a compact fallback.

For cleanliness, create a small module only if it keeps `ui.rs` readable. The
recommended module name is `ui_mercury_core.rs`; keep the interface private to
the TUI module.

Recommended MVP interface:

```text
build_mercury_core_line(width, glyph_mode) -> Line
draw_mercury_core_strip(frame, area, glyph_mode)
```

Use an enum or simple helper for glyph mode only if the existing codebase has a
terminal capability signal. If no reliable capability exists, choose width-based
fallback text and avoid config.

## 3. Files Likely To Inspect

* `crates/jcode-tui/src/tui/ui.rs`
  * Main render layout, top bar split, message area split, startup splash, and
    `draw_messages` call.
* `crates/jcode-tui/src/tui/ui_top_bar.rs`
  * Existing small branded chrome pattern, top bar field building, truncation,
    and tests.
* `crates/jcode-tui/src/tui/ui_theme.rs`
  * Existing theme color exports such as `accent_color`, `muted_color`,
    `panel_color`, and `foreground_color`.
* `crates/jcode-tui/src/tui/app/state_ui_maintenance.rs`
  * Existing processing/status maintenance, only if the MVP needs a reliable
    idle/processing signal later.
* `crates/jcode-tui/src/tui/ui_tests/mod.rs`
  * Test state, layout tests, startup splash helper tests, and access patterns
    for private UI helpers.
* `crates/jcode-config-types/src/lib.rs`
  * Inspect only to confirm no config field is needed for the MVP.
* `crates/jcode-base/src/config/default_file.rs`
  * Inspect only if a config field becomes necessary, which is not recommended
    for the first slice.
* `src/cli/commands.rs`
  * Inspect only to confirm no config visibility or CLI changes are needed.

## 4. Files Likely To Modify

Preferred smallest set:

* `crates/jcode-tui/src/tui/ui.rs`
  * Add the module import if a helper module is created.
  * Split one top row from `messages_area` for the Mercury Core strip.
  * Draw the strip before normal messages.
  * Skip the strip in cramped or takeover layouts.
* `crates/jcode-tui/src/tui/ui_mercury_core.rs`
  * New private helper module for building and drawing the strip.
  * Keep all text fallback, truncation, and style decisions here.
* `crates/jcode-tui/src/tui/ui_tests/mod.rs`
  * Add focused tests for text fallback and visibility behavior if existing
    test helpers make this practical.
* `crates/jcode-tui/src/tui/ui_tests/rendering.rs`
  * Alternative location for pure rendering helper tests if it matches existing
    patterns better.

Avoid modifying these files for the MVP:

* `crates/jcode-config-types/src/lib.rs`
* `crates/jcode-base/src/config/default_file.rs`
* `src/cli/commands.rs`
* `Cargo.toml`
* `Cargo.lock`

## 5. Test Strategy

Use targeted Rust tests during implementation, but do not run them during this
planning slice.

Recommended future tests:

* Unit test the Mercury Core text builder for normal width:
  * input width around 40;
  * expected content includes `Mercury` and `ready`;
  * expected display width is no greater than the target width.
* Unit test narrow fallback:
  * input width around 10-16;
  * expected content uses ASCII-safe or compact text;
  * expected display width is no greater than the target width.
* Unit test tiny width skip:
  * input width below the minimum;
  * expected helper returns no drawable line or the layout skips rendering.
* Render/layout test if existing TUI tests support it cleanly:
  * with enough chat height, the strip is visible above the first message;
  * with very small height, normal messages/input are not crowded by the strip.

Suggested future commands for the implementer:

```text
cargo test -p jcode-tui mercury_core -- --nocapture
cargo test -p jcode-tui tui::ui_tests -- --nocapture
```

If those targets are not valid after inspection, use the narrowest equivalent
test invocation that covers the new helper and any layout integration test.

## 6. MVP Scope

In scope:

* One idle/ready Mercury Core strip.
* Render near the top of the conversation area.
* Theme-aware styling using existing palette helpers.
* Width-safe truncation or fallback.
* No user-facing config.
* No dynamic states beyond idle/ready unless the implementation can reuse an
  existing status signal without extra state plumbing.

Preferred MVP text:

```text
☿ Mercury  ━━━●━━━━  ready
```

Preferred ASCII fallback:

```text
Mercury  ---*----  ready
```

Minimum narrow fallback:

```text
Mercury ready
```

## 7. Non-Goals

* No command palette redesign.
* No homepage/status card redesign.
* No input prompt redesign.
* No Top Bar V2.
* No multi-session UI.
* No Queue changes.
* No server or protocol changes.
* No migration changes.
* No new Cargo dependencies.
* No `Cargo.toml` or `Cargo.lock` edits.
* No desktop, React, or Tauri work.
* No true blur/glass effect.
* No animated wallpaper or background redesign.

## 8. Step-By-Step Implementation Tasks

### Task 1: Add A Pure Mercury Core Text Helper

**Files:**

* Create: `crates/jcode-tui/src/tui/ui_mercury_core.rs`
* Modify: `crates/jcode-tui/src/tui/ui.rs`
* Test: `crates/jcode-tui/src/tui/ui_tests/rendering.rs` or
  `crates/jcode-tui/src/tui/ui_tests/mod.rs`

**Interfaces:**

* Produces: a private helper that builds a width-safe idle/ready line for the
  Mercury Core strip.
* Consumes: existing theme helpers from `ui_theme.rs`.

Steps:

* [ ] Inspect existing truncation helpers in `ui.rs` and top bar truncation in
  `ui_top_bar.rs`.
* [ ] Write a failing unit test for normal-width strip text.
* [ ] Write a failing unit test for narrow fallback text.
* [ ] Add `ui_mercury_core.rs` with a pure line/text builder and no app state.
* [ ] Use existing theme helpers for accent and muted styling.
* [ ] Keep the MVP state label fixed as `ready`.
* [ ] Run only the targeted helper tests.

### Task 2: Integrate The Strip Into The Chat Layout

**Files:**

* Modify: `crates/jcode-tui/src/tui/ui.rs`
* Test: `crates/jcode-tui/src/tui/ui_tests/mod.rs`

**Interfaces:**

* Consumes: the Mercury Core draw helper from Task 1.
* Produces: one optional row above normal chat messages when the terminal area is
  large enough.

Steps:

* [ ] Find the existing `messages_area` creation before the `draw_messages`
  call.
* [ ] Split `messages_area` into `mercury_core_area` and remaining chat content
  only when height and width are sufficient.
* [ ] Draw the Mercury Core strip into `mercury_core_area`.
* [ ] Pass the reduced message area to `draw_messages`.
* [ ] Skip the strip when onboarding takes over or the available chat area is
  too small.
* [ ] Run the narrowest layout/render test that proves normal messages still
  render.

### Task 3: Add Visibility And Small-Terminal Tests

**Files:**

* Modify: `crates/jcode-tui/src/tui/ui_tests/mod.rs`
* Modify if needed: `crates/jcode-tui/src/tui/ui_tests/rendering.rs`

**Interfaces:**

* Consumes: integrated Mercury Core layout behavior from Task 2.
* Produces: regression coverage that prevents the strip from crowding cramped
  layouts.

Steps:

* [ ] Add a test state with no messages or one short message.
* [ ] Render at a normal terminal size and assert the output contains Mercury
  Core text above the message area.
* [ ] Render at a cramped terminal size and assert the strip is skipped or does
  not hide the input/message area.
* [ ] If full-frame text extraction is not already ergonomic, test the pure
  helper and layout split function instead of adding a brittle snapshot.
* [ ] Run only the targeted TUI tests for these cases.

### Task 4: Final Verification And Review

**Files:**

* Inspect: `git diff --stat`
* Inspect: `git diff -- Cargo.toml Cargo.lock`
* Inspect: `git diff -- src crates docs`

**Interfaces:**

* Consumes: implementation and test changes from Tasks 1-3.
* Produces: a small reviewed implementation slice ready for manual build/check
  according to repository workflow.

Steps:

* [ ] Confirm no Queue, server, protocol, migration, or Cargo files changed.
* [ ] Confirm the implementation did not add a config field.
* [ ] Confirm the strip uses existing theme colors.
* [ ] Confirm the strip is one row and does not replace the top bar.
* [ ] Confirm targeted tests pass.
* [ ] Leave broader build validation for the repository's normal post-change
  build step.

## Recommended First Implementation Task

Start with Task 1: add the pure Mercury Core text helper and focused tests. That
keeps the first code change independent from the main layout and gives a clear
fallback contract before the strip is inserted into `ui.rs`.
