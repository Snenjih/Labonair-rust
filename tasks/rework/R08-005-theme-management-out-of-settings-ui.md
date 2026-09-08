# R08-005 — Move Theme management policy out of Settings UI

## Status

`✅ Done`

## Owner

- Module: `themes`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Goal

`labonair-settings-ui` is a values-only editor. Theme preview, activation,
cancel/revert, persistence, and settings→`ThemeStore` application live with the
Themes owner. `labonair-theme` keeps no `labonair-settings` dependency.

## Scope

- In scope: new sibling crate `crates/theme-ui` (`labonair-theme-ui`) holding
  `apply_prefs_to_theme`, `apply_theme_metrics`, `theme_metrics_from_settings`,
  `font_overrides_from_settings`, `apply_stored_theme_variant`,
  `preview_app_theme`, `activate_app_theme`, and the `command_provider`
  `"themes"` palette-action handler. Rewire `settings-ui`, `shell/bootstrap`.
- Out of scope: `labonair-theme`'s runtime store, catalogs, and preview
  mechanics (unchanged); the deterministic-catalog product decision (P2.6);
  the `char_of` keystroke helper stays in `settings-ui/apply.rs`.

## Contracts and ownership

- Public domain values: none new.
- Service traits or typed events: none — `theme-ui` exposes free functions
  taking `&Entity<ThemeStore>` + `&mut App`, called by the composition root.
- Registry contributions: `theme-ui::command_provider::register_palette_action_handlers`
  registers the `"themes"` owner on `PaletteActionHandlerRegistry` (moved from
  `settings-ui`). `labonair-theme::ThemeCommandProvider` still contributes the
  descriptors/submenus.
- UI surface: none (policy crate).
- Shared UI-kit components: none.

## Dependencies

- Existing edges removed: `settings-ui`'s ownership of theme policy (the code
  moved; `settings-ui → theme-ui` replaces it as a narrow call edge).
- New edges: `labonair-theme-ui → {labonair-theme, labonair-settings,
  labonair-command-palette-core, labonair-command-palette-runtime}`;
  `labonair-settings-ui → labonair-theme-ui`;
  `labonair-shell → labonair-theme-ui`.
- Dependency verifier change: add the `labonair-theme-ui` allow-list entry;
  add it to the `settings-ui` and `shell` sets.

## Persistence and migration

- Settings: None (same `appearance.*` / `general.theme` / `editor.editorTheme`
  keys, same writes — just from a different crate).
- Storage: None.
- Compatibility: None.

## User-visible behavior

- Canonical entry point: unchanged (titlebar global menu → Themes / Icon
  Themes → palette submenu; Settings appearance fields).
- Notifications: unchanged.
- Inline errors/toasts: none.

## Implementation plan

1. Create `labonair-theme-ui` and move the bridge + palette handler verbatim →
   verify: `cargo test -p labonair-theme-ui`.
2. Strip the moved code from `settings-ui`; `SettingsView::sync_theme_from_prefs`
   now calls `labonair_theme_ui::apply_prefs_to_theme` → verify:
   `cargo test -p labonair-settings-ui`.
3. Rewire `bootstrap` (`apply_prefs_to_theme`, `apply_theme_metrics`, the
   palette handler registration) and the dependency verifier → verify:
   `bash scripts/check-crate-deps.sh`, `cargo test --workspace`.

## Acceptance criteria

- [x] Settings UI has no theme preview/activation/persistence policy and does
      not own `ThemeStore` policy (only calls the `theme-ui` bridge to refresh
      after a value write).
- [x] Theme preview/cancel-revert/activate/persist and the settings→store
      application live in `labonair-theme-ui`; `labonair-theme` has no
      `labonair-settings` dependency.
- [x] Theme palette actions are registered by the Themes owner
      (`labonair-theme-ui::command_provider`).
- [x] Settings remains a values-only management surface.
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`, and
      `git diff --check` pass.
- [ ] Theme preview/confirm/cancel visual states are recorded natively.
      *Deferred into the R07-001 visual matrix.*

## Notes and follow-ups

`labonair-theme-ui` currently exposes free functions rather than a struct; a
future task may wrap them in a typed `ThemeApplication` contract if a second
consumer appears. Next task: R08-006 (P2.4, remove the `AppComposition`
`Deref`).
