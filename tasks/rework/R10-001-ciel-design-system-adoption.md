# R10-001 — Adopt the Ciel design system

## Status

`🔄 In Progress`

## Owner

- Modules: `theme` (tokens/themes), `ui-kit` (primitives), `settings`
  (`appearance.translucentSurfaces`), view crates (consumer sweep)
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair_theme_ui::apply_theme_metrics` /
  `window_background`

## Goal

Replace Labonair's visual system with Ciel's (`~/Developer/active/Ciel`,
`packages/app/DESIGN.md`): colors, radius, spacing/control scale, active-state
matrix, menus/dropdowns/dialogs, chrome heights. Clean-room: measured values
and rules, no source copied. Zed icon set stays.

## Scope

- In scope: built-in light/dark themes, `RadiusScale`, motion durations,
  ui-kit button/field/select/segmented/tab/list/tree/menu/popover/dialog/
  banner/toggle/checkbox geometry and states, titlebar/statusbar heights,
  command-palette selection style, opt-in translucency setting.
- Out of scope: icon-set swap, German UI copy, Ciel's page templates
  (`PageHeader`/`PageShell`), per-view layout redesign beyond what the new
  primitives force.

## Contracts and ownership

- Public domain values: `labonair-theme-tokens` (`SurfaceColors`
  `hover_strong/sunken/action*`, `BorderVariants` `strong/subtle`,
  `RadiusScale::ciel`, `VibrancyLayer::resolve`, `ThemeMetrics.translucent_surfaces`)
- UI surface: `labonair-ui-kit` (`Palette` gains `field_bg`, `field_ring`,
  `sunken`, `line_*`, `action*`, `scrim`, `popover_shadow()`;
  `ListItem::new(id, Palette)`; `ButtonVariant::Action`)
- Shared UI-kit components: all of the above; no new feature-local variants.

## Persistence and migration

- Settings: `appearance.translucentSurfaces` (bool, default `false`), UI row in
  Density & Motion; `docs/settings-inventory.md` + generated doc updated.
- Compatibility: user theme files without the new tokens inherit the built-in
  defaults (existing token-inheritance rule); no adapter needed.

## Implementation plan

1. Tokens + themes + translucency setting → verify: `cargo test -p labonair-theme*`
2. ui-kit primitives → verify: `cargo test -p labonair-ui-kit`, workspace check
3. Chrome heights, palette selection, docs → verify: native run (opaque dark
   confirmed), `check_documentation.py`
4. Remaining view sweep (hand-rolled radii/heights/accent tints in feature
   crates, `modal_overlay` dark scrim) → verify: Component Gallery in light and
   dark, Settings window, native screenshots

## Acceptance criteria

- [x] Tokens, themes, setting, and ui-kit primitives implemented.
- [x] `docs/design-system.md` describes the Ciel rules and token map.
- [ ] Light-theme and Settings/Gallery native visual check.
- [ ] View sweep complete (see Notes).
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` passes (currently
      blocked by a pre-existing `workspace.rs` lint, unrelated to this task).
- [ ] `cargo test --workspace` passes (two pre-existing
      `project_multibuffer` failures, unrelated).

## Notes and follow-ups

- Sweep leftovers: fixed-px radii/heights in feature views, `modal_overlay`
  scrim is still a fixed 30% (Ciel dark = 55%), SFTP/drag accent highlights,
  dialog content padding now doubled by the 24px `dialog_surface` padding in
  some owners, `Switch`/slider restyle, typography line-height 1.4.
