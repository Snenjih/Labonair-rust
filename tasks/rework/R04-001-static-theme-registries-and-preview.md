# R04-001 — Make themes static, registry-owned, and previewable

## Status

`⏳ Planned`

## Owner

- Module: `themes`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Dependencies

- `R03-001-command-palette-provider-registry`
- `R03-002-keymap-runtime-and-editor`

## Goal

Provide one coherent theme capability for color themes and icon themes. The
initial catalog is explicit and built into the application; moving through the
palette previews immediately, while Enter commits the selection.

## Scope

- In scope: built-in color/icon definitions, typed theme IDs, registry
  snapshots, preview/commit/cancel lifecycle, palette submenus, persistence,
  and UI-kit integration.
- Out of scope: GitHub fetching, marketplace downloads, extension hosting,
  user-authored theme packages, and feature-specific theme selection logic.

## Contracts and ownership

- Public values and registry: `labonair-theme` owns IDs, definitions, active
  selection, preview state, and persistence contracts.
- UI: the palette owns filtering and navigation; the theme module owns preview
  application and commit behavior.
- Shared UI-kit: searchable list, list row, key hints, scroll container, and
  preview/status indicators.

## Dependencies

- Existing edges removed: feature-local theme catalogs and remote-download
  placeholders.
- New edges: palette consumes immutable theme-provider snapshots; feature views
  consume semantic tokens only.
- Dependency verifier change: update only for the typed provider edge; no theme
  module may depend on a feature UI crate.

## Persistence and migration

- Settings: persist selected color/icon theme IDs as values; Settings does not
  render or own the management workflow.
- Storage: built-in definitions require no external catalog database.
- Compatibility: unknown persisted IDs fall back to the default and publish a
  diagnostic notification without blocking startup.

## User-visible behavior

- Canonical entry point: titlebar global menu → Themes or Icon Themes, then a
  searchable command-palette submenu.
- Notifications: invalid selection or persistence failures use structured
  notifications; preview is reversible and emits no passive message.
- Inline errors/toasts: none.

## Implementation plan

1. Define static catalogs and transactional selection state → verify IDs,
   ordering, fallback, preview, commit, and cancel behavior.
2. Connect palette providers and semantic token consumers → verify filtering,
   keyboard navigation, and no feature-local catalog.
3. Remove remote-fetch placeholders and old Settings management paths → verify
   migration fixtures, dependency graph, and visual previews.

## Acceptance criteria

- [ ] Color and icon themes each have one registry-backed palette submenu.
- [ ] Moving selection previews immediately; Enter commits; cancellation
      restores the previous selection.
- [ ] The initial catalog is static and deterministic with no network request.
- [ ] Settings contains only selected values, not theme management UI.
- [ ] Feature views consume semantic tokens and shared UI-kit controls.
- [ ] Focused tests and all repository verification gates pass.
- [ ] Normal, filtered, long-list, preview, and empty states are visually
      checked.

## Removal condition

This task is complete only when no feature or Settings pane defines a second
theme catalog or selection path and no remote theme download code is required
for the supported workflow.

## Notes and follow-ups

An extension/download system may be proposed later, but it requires a new
product decision, owner, security model, and persistence contract.
