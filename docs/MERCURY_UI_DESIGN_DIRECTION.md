# Mercury UI Design Direction

This document defines a docs-only visual direction for Mercury's TUI before any
implementation work. The goal is to make Mercury feel like a premium AI cockpit
while preserving the speed, keyboard focus, and terminal reliability of the
current app.

## 1. Current UI Diagnosis

### What works

* The current interface is fast, keyboard-first, and functionally clear.
* The top bar already provides useful session context:
  `Mercury | session: chicken | theme: cursor | repo: jcode`.
* The startup block exposes valuable self-dev information such as server,
  client, model/provider, path, updates, MCP, skills, and connected clients.
* The slash command menu is discoverable after typing `/`.
* Existing terminal-safe themes and background styles provide a foundation for
  a more branded visual system.

### What feels unfinished

* The app still reads as a renamed terminal client instead of a Mercury-native
  product experience.
* Startup information is technically useful but visually ungrouped and debug
  heavy.
* The center startup block competes with the conversation area rather than
  acting like a compact branded status element.
* Most surfaces are lists or text blocks instead of intentionally grouped
  panels.
* There is not yet a strong hierarchy between product identity, session state,
  repo state, and implementation details.

### Branding inconsistencies

* Mercury is now the preferred product surface, but `jcode` still appears in
  repo context and compatibility language.
* Internal names such as server Fountain and client Chicken are useful for
  developers but should not dominate the first impression.
* The top bar says Mercury, but the rest of the opening screen still feels like
  a technical status dump.
* The product name should be the primary brand. Session, server, repo, and
  compatibility details should remain secondary.

### Homepage issues

* The startup screen currently behaves like a diagnostics page.
* Important information and debug information have similar visual weight.
* The large center block should be converted into a polished "Mercury Core"
  card or compact status surface.
* Self-dev information should remain available, but it should be grouped,
  labeled, and visually quieter.

### Command menu issues

* The command menu currently looks like a raw list:
  `/help Show help and keyboard shortcuts`,
  `/fix Recover when the model cannot continue`, and similar rows.
* Command names and descriptions are not visually separated enough.
* The selected row should have a clear accent treatment.
* There is no command-palette header, category grouping, or footer hint area.
* The surface feels functional but not premium.

### Input prompt issues

* The current `1>` prompt feels like an old REPL.
* It does not reinforce the Mercury identity.
* It gives little sense of a focused AI cockpit or active session.
* The prompt should remain compact and terminal-safe, especially at narrow
  widths.

## 2. Target Visual Direction

Mercury should feel like a premium AI cockpit: quiet, precise, and focused on
conversation, tools, and execution status. The visual language should suggest a
small operational console rather than a decorative cyberpunk terminal.

The base should remain a minimal dark terminal with restrained contrast,
intentional spacing, and a subtle Mercury identity. The UI should feel designed
without becoming noisy. The product should be recognizable through hierarchy,
accent color, status treatment, and compact branded elements.

Terminal-safe faux glass is welcome, but only through effects the TUI can
reliably render: dim panel backgrounds, soft borders, low-contrast separators,
accent lines, and simulated depth. Real blur, opacity, and CSS-style
glassmorphism are out of scope for the terminal.

The command menu should move toward a Raycast/Cursor-style command palette:
compact, searchable-feeling, structured, and clearly keyboard-first.

Avoid cyberpunk clutter. No heavy matrix effects by default, no noisy animated
backgrounds, no excessive neon, and no effects that distract while typing or
reading.

## 3. Design Principles

* Brand hierarchy: Mercury is the product; session, server, repo, and internal
  names are secondary.
* Calm by default, animated only when useful.
* Information should be grouped into cards or panels instead of raw technical
  blocks.
* Important status should stay visible; debug details should be hidden,
  collapsed, or visually deemphasized.
* The layout must work in small terminal widths.
* No distracting effects during typing or reading.
* Preserve keyboard-first operation.
* Prefer subtle, semantic status changes over decorative movement.
* Keep compatibility details visible where useful, but do not let them define
  the primary product impression.

## 4. Mercury Core Mini Animation

Replace or complement the large startup feel with a small branded strip near
the top of the conversation area. This should act like a compact Mercury Core
status element, not a hero banner.

Possible designs:

```text
☿ Mercury  ━━━●━━━━  ready
☿ Mercury Core  pulse: active
MERCURY  ━━━●━━━━━━━━
```

States:

* `idle`: calm, static, low contrast.
* `thinking`: subtle pulse or slow movement.
* `tool-running`: accent activity, optional tool label.
* `error`: restrained error color and short label.
* `done`: brief completion state before returning to idle.

The animation must stay subtle. It should degrade safely in terminals that do
not support the preferred glyphs or line characters. A plain ASCII fallback
should remain available:

```text
Mercury  ---*----  ready
```

## 5. Homepage / Startup Card Redesign

Convert the technical debug block into a "Mercury Core" card. The card should
make the app feel intentionally designed while keeping self-dev information
available.

Recommended visible fields:

* app/product: Mercury
* session: current session name, such as Chicken
* model/provider: concise active model and provider
* repo: current repository or workspace
* server/client status: connected, starting, degraded, or offline
* recent changes: compact update summary when relevant

Raw internal names should be deemphasized. For example, server Fountain and
client Chicken can appear as secondary metadata rather than the main headline.

The card should be useful for self-dev but polished for normal use. Debug-heavy
details such as MCP counts, skill counts, and connected server clients should
be visually lower priority or available through an expanded details mode later.

## 6. Command Palette Redesign

The slash command menu should become a compact command palette.

MVP structure:

* Header/title, such as `Mercury Commands`.
* Command name column, such as `/help`.
* Description column, such as `Show help and keyboard shortcuts`.
* Accent treatment for the selected row.
* Footer hints: `↑↓ move  Enter run  Esc close`.

Future categories:

* Core
* Git
* Session
* Debug
* Queue
* Model

The palette should remain keyboard-first and compact. It should not become a
large modal that obscures the conversation more than necessary. Filtering and
search behavior can evolve later, but the first slice should focus on
structure, spacing, selected-row polish, and command readability.

## 7. Input Prompt Redesign

Replace the raw `1>` feeling with a compact branded prompt.

Options:

```text
☿ ❯
Mercury ❯
Chicken ❯
```

Preferred MVP choice:

```text
☿ ❯
```

Rationale: it is short, branded, and keeps attention on the user's input. It
also works well with a premium cockpit style without adding extra text to every
line.

Fallback for narrow or glyph-limited terminals:

```text
> 
```

The prompt should not animate while the user is typing. Any activity indicator
should live in the Mercury Core strip, top bar, or status area instead.

## 8. Faux Glass / Terminal Effects

Real CSS glassmorphism is impossible in a terminal TUI because terminals do not
support real blur, backdrop filtering, layered alpha compositing, or CSS-style
opacity.

Terminal-safe alternatives:

* dim panel backgrounds
* soft borders
* accent lines
* low-contrast separators
* subtle patterns
* simulated opacity through darker or lower-contrast colors
* sparse highlight characters used as texture

These effects should be used sparingly. The goal is a premium, quiet interface,
not a busy visual layer. Avoid heavy animation, high-frequency patterns, noisy
backgrounds, or anything that reduces readability.

## 9. Background Effects

Existing background styles:

* none
* subtle-grid
* stars
* matrix

Future ideas:

* mercury-orbit
* soft-noise
* constellation
* scanlines
* radar

Recommended first new background style later: `mercury-orbit`.

`mercury-orbit` should be subtle and sparse: a few low-contrast orbital arcs or
points that imply Mercury identity without becoming a moving wallpaper. It
should be disabled or nearly invisible while typing, reading dense output, or
using command palettes.

## 10. Implementation Roadmap

Recommended order:

1. Design doc / no code.
2. Mercury Core mini animation MVP.
3. Command palette redesign MVP.
4. Homepage/status card redesign MVP.
5. Input prompt polish.
6. Top Bar V2.
7. Multi-session UI.

The first implementation slices should be small and reversible. They should
reuse existing TUI layout, theme palette, and background systems where possible
instead of introducing a large layout rewrite.

## 11. Non-Goals For V1

* No desktop app yet.
* No React/Tauri yet.
* No true blur/glass effect.
* No big layout rewrite.
* No multi-session implementation in this design slice.
* No hard internal rename.
* No Rust implementation in this documentation slice.
* No Cargo metadata changes in this documentation slice.
