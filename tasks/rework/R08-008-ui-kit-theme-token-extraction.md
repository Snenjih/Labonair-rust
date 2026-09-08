# R08-008 — Remove the `labonair-ui-kit → labonair-theme` dependency

## Status

`⏳ Planned`

## Owner

- Module: foundation (`ui-kit`) + `themes`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: n/a (crate-graph change)

## Why deferred

This is a load-bearing foundation refactor: it moves the entire visual-token
layer (`Theme`, `RadiusScale`, `ThemeMetrics`, `ActiveTheme`, colour structs,
`IconThemeContent`, the `UiTheme` trait) out of `labonair-theme` into a new
leaf crate that both `ui-kit` and `theme` depend on. It touches ~20 files
across the crate that every surface renders from, and its correctness is
partly visual. It is being handed off for review rather than executed blind at
the end of a long batch. The `your-task-2.md` acceptance
(`crates/ui-kit/Cargo.toml` has no `labonair-theme` dependency) is only met by
this extraction, so it stays `Planned`, not silently dropped.

The violation is confirmed: `crates/ui-kit/Cargo.toml` depends on
`labonair-theme` (a feature module per `docs/architecture.md`), and
`ui-kit/src/{theme,palette,icon,gallery}.rs` import `ThemeStore`,
`GlobalActiveTheme`, `IconThemeContent`, and concrete token types. The code's
own doc comment already states the *intent* ("must not depend on the runtime
theme store ... mirrors Zed's `ui` / `theme` split").

## Scope (when activated)

- New leaf crate `crates/theme-tokens` (`labonair-theme-tokens`, deps: `gpui`,
  `serde`, `serde_json`) holding: `color.rs`, `tokens.rs`, `contrast.rs`, the
  metric layer (`ActiveTheme`, `GlobalActiveTheme`, `ThemeMetrics`,
  `UiDensity`), the `IconThemeContent` / `DirectoryIcons` / `ChevronIcons` /
  `IconDefinition` data types, and the `UiTheme` + `ActiveThemeExt` traits
  (moved from `ui-kit/src/theme.rs`).
- `labonair-theme`: depend on `theme-tokens`, re-export every moved type under
  its current name (zero downstream churn), keep `impl UiTheme for ThemeStore`
  here (now legal), keep the store/registry/preview/import.
- `labonair-ui-kit`: swap the `labonair-theme` dependency for
  `labonair-theme-tokens`; `theme.rs` becomes a thin re-export of `UiTheme` /
  `ActiveThemeExt` for existing `labonair_ui_kit::UiTheme` consumers.
- `gallery.rs` (the debug component gallery holding `Entity<ThemeStore>`):
  move to `crates/shell` behind `#[cfg(debug_assertions)]`
  (`shell::open_gallery_window`), or to `labonair-theme-ui`.
- `scripts/check_crate_deps.py`: add the `labonair-theme-tokens` leaf entry;
  `ui-kit` allow-list becomes `{labonair-theme-tokens, labonair-gpui-ext}`;
  add a regression assertion that `labonair-ui-kit` may not depend on any
  feature module.
- Regenerate `docs/assets/crate-graph.{dot,svg}`; update `architecture.md`
  (§ layering + crate table), `capabilities.md`, `architecture-inventory.md`.

## Acceptance criteria (when activated)

- [ ] `crates/ui-kit/Cargo.toml` has no `labonair-theme` dependency.
- [ ] `rg "labonair_theme|ThemeStore|IconThemeContent" crates/ui-kit/src`
      finds no production product references.
- [ ] Reusable components still render from the injected/foundation token
      contract; no visual change.
- [ ] The component gallery is reachable only through the documented debug
      integration.
- [ ] A dependency-verifier regression rejects a feature-module dependency
      from `ui-kit`.
- [ ] Full verification suite passes; native visual spot-check of the
      primitives (folds into the R07-001 matrix).

## Notes and follow-ups

Cheaper alternative if extraction is rejected: reclassify the `labonair-theme`
*token* surface as foundation-consumable in `docs/architecture.md` and
document the `ui-kit → theme` edge as an approved exception with the reason
("theme is the single design-token source"). That does **not** satisfy the
`your-task-2.md` acceptance but is an honest, low-risk resolution if the team
decides token ownership belongs with `labonair-theme`.
