# Settings Design Contract

**Status:** Normative. Binding for every settings page from `T19-001` onward, and
for any settings-related change to existing pages. Established by `T19-000`.

## Why this document exists

The Tauri-era settings UI (`reference-src/src/modules/settings/`, and its early
Rust port `crates/ui/src/settings.rs`, 212 KB) drifted: a flat 10-entry
`CATEGORIES` list, a hand-maintained `FIELDS` table (131 entries, chronically
out of sync with ~170 `Preferences` fields), a separate `SECTION_GROUPS` table,
and several special-cased panes (theme grid, shortcut capture, AI provider,
MCP) each with their own layout conventions. Settings existed that had no
backing field, and fields existed that no UI ever exposed. This document is the
fix: a small, closed set of rules that every settings page — present and
future — must satisfy, written down *before* `T19-001` cuts the
`SettingsContent` tree, so the tree is shaped with these rules in mind rather
than retrofitted to them.

This is the contract that `T19-001`–`T19-010` implement against. A reviewer
should be able to point at a settings PR and say "this violates point N" — the
points below are written to make that possible, not to specify pixels, widget
choices, or spacing (that is `T19-004` + visual review, not this document; see
Non-Goals).

## The 9 rules

### 1. One navigation model

Left: top-level categories, in a fixed order. Right: the selected category
rendered as a page with **collapsible section headers** (`SectionHeader`,
disclosure widgets) and scroll-spy jump anchors for those sections. Large
categories may additionally have **sub-pages** (`SubPageLink`, with a back
arrow) for content too large for a single scrolling page.

No category deviates from this shape. There is no category-specific chrome,
no category with its own window, no category that replaces the section-header
list with something else "because it's different." If a category feels like
it needs a different shape, that is a signal to split it into sections/
sub-pages within this model, not to invent a new model (see Rule 4 for the one
sanctioned exception, and Rule 9 for why a second window is never the answer).

### 2. Every setting is a typed field, not UI-only

Every setting lives in the `SettingsContent` tree (`T19-001`) as a typed field
with metadata: `title`, `description`, and where applicable `unit`, `range`,
`placeholder`, `requires_restart`. No setting exists only as a UI control with
no backing field — if it's not in `SettingsContent`, it is not a setting. No
parallel `FIELDS`-style array duplicates what the type already declares.

Consequence: adding a setting means adding a field (with metadata) to
`SettingsContent`. The UI appears automatically (Rule 3). There is no second
place to register it.

### 3. Field UI is generated from the Rust type

The field renderer (`T19-004`) maps Rust types to widgets mechanically:

| Rust type | Widget |
|---|---|
| `bool` | Switch |
| `enum` | Dropdown, human-readable labels (not the Rust variant name verbatim) |
| numeric (`u32`, `f32`, …) | NumberField, using `range`/`step` from metadata |
| `String` | TextInput |
| anything else | a metadata marker selects a registered special renderer |

No bespoke hand-built toggle, row, or input is written for a type this table
already covers. A field that needs something outside this table needs a new
renderer-registry entry (extending the table), not a one-off widget inlined
into a page.

### 4. Custom panes are sanctioned, not a workaround

Some UI is not a field grid — a theme gallery, a shortcut-capture recorder, an
AI provider list, MCP server management. These get a first-class path:
`SettingsPage { kind: Custom(render_fn) }`. A custom pane may be registered as
a **top-level category** exactly like a field-based one (e.g. Themes today;
Hosts, Shortcuts, AI, MCP going forward).

The one constraint that makes this not a loophole: a custom pane still renders
**inside the standard page chrome** — same header, same search integration,
same origin badges — only the content area below the header is custom. A new
custom top-level category is one registration entry (page kind, slug, render
fn); it must never require touching the field registry, and it must never
grow its own header/search/window (that would violate Rule 1 and Rule 9).

### 5. Origin + reset on every field

Every rendered field shows, unobtrusively, which layer is currently effective
for it — Default / User / Project (`SettingsStore::source_of`, `T19-002`). If
the effective value differs from the default, the field offers "reset to
default." This applies uniformly; a field does not get to skip it because its
renderer is unusual.

### 6. Search is global

Every page — including custom panes — feeds searchable keywords into the
global settings search (`T19-007`). A custom pane's `render_fn` alone is not
enough; it must also register the keywords a user would type to find it (pane
title, and any of its major inner concepts). The search index must be able to
find every field and every custom pane, not just field-based pages.

### 7. Deep links everywhere

Every category and every section has a stable slug. `settings://<category>/
<section>` navigates directly to it. This is what the command palette,
"show origin" affordances, and `menu.rs` entry points use to jump into
settings — they never reimplement navigation logic against the settings
window internals.

### 8. Copy rules

- Labels: sentence style, short. Not a restatement of the description.
- Descriptions: imperative, one sentence.
- Units belong in the field/metadata (`unit`), never appended to the label
  text.
- No redundancy between label and description — if the description just
  repeats the label with punctuation, delete the description.
- English only (repo-wide Language Protocol; also — see CLAUDE.md).

### 9. No window sprawl

One settings window, one store, one navigation tree. A new settings area is a
new category or section hung off the existing tree (or a custom top-level
category per Rule 4) — never a new window, overlay, or parallel settings
surface. If a feature seems to need its own dedicated window, that is a
product decision to make explicitly and document as a deviation (see below),
not a default outcome of "settings got complicated."

## Non-Goals

This contract fixes structure and process, not visuals. It does **not**
specify: exact pixel spacing, corner radii, animation timing, or which exact
widget component renders a `bool` (only that *a* Switch-shaped widget does).
Those choices are `T19-004`'s to make, verified by user-visible review against
`reference-src/src/styles/globals.css` (per `CLAUDE.md` Critical Rule 3), not
by this document.

## Deviation process

A page or field that cannot satisfy one of the 9 rules as written is not
automatically wrong, but it cannot be silently exempted either:

1. State which rule is being deviated from and why the standard shape
   (including the custom-pane path in Rule 4) genuinely does not fit.
2. Record the deviation and its rationale in `docs/architecture.md` (the
   settings section, §8.3, or a new entry near it) before merging the code
   that relies on it — not after.
3. A deviation is scoped to the specific page/field it was justified for; it
   is not precedent for unrelated pages to skip the same rule.

In practice this should be rare: Rule 4 (custom panes) already covers every
known non-field UI need (themes, shortcuts, AI providers, MCP). A proposed
deviation that turns out to just be "a custom pane, but let it also replace
the header/search/chrome" is not a deviation — it is a Rule 4 violation and
should be rejected, not documented.
