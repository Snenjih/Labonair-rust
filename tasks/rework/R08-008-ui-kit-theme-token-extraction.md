# R08-008 — Remove the `labonair-ui-kit → labonair-theme` dependency

## Status

`✅ Done`

## Owner

- Module: foundation (`ui-kit` / new `theme-tokens`) + `themes`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: n/a (crate-graph change)

## Goal

`labonair-ui-kit` renders against a foundation-level design-token contract, not
the Themes *feature* crate. `crates/ui-kit/Cargo.toml` has no `labonair-theme`
dependency.

## What shipped

- **New leaf crate `crates/theme-tokens` (`labonair-theme-tokens`)** — deps:
  `gpui`, `palette`, `serde`, `serde_json` only. Modules:
  - `color`, `tokens`, `contrast` (moved verbatim from `labonair-theme`)
  - `metrics` (was `theme_settings.rs`: `ActiveTheme`, `GlobalActiveTheme`,
    `ThemeMetrics`, `UiDensity`)
  - `icons` (`IconThemeContent` + `DirectoryIcons` / `ChevronIcons` /
    `IconDefinition` + their lookup methods + the `DEFAULT_FILE_*` tables +
    `BUILTIN_ICON_THEME_{ID,NAME}`)
  - `ui_theme` (the `UiTheme` trait + `ActiveThemeExt` + `impl ActiveThemeExt
    for App`, moved out of `ui-kit/src/theme.rs`)
  - `font_families` (the 4 font-family / fallback consts `tokens.rs` needs)
- **`labonair-theme`** — `dep: labonair-theme-tokens`; `lib.rs` re-exports
  every moved type under its existing name so no downstream crate changed.
  Deleted `color.rs` / `tokens.rs` / `contrast.rs` / `theme_settings.rs`;
  `icon_theme.rs` reduced to the runtime registry. New `ui_theme_impl.rs`
  holds `impl UiTheme for ThemeStore` (orphan rule — `ThemeStore` is local
  here). `import.rs`'s inherent `impl Theme { from_theme_file* / to_theme_file
  }` became the extension trait `ThemeFileConversion` (`Theme` is foreign
  now); `store.rs` brings it into scope.
- **`labonair-ui-kit`** — `dep: labonair-theme` → `labonair-theme-tokens`.
  `theme.rs` is a one-line re-export shim (`pub use
  labonair_theme_tokens::{UiTheme, ActiveThemeExt}`). `palette.rs` /
  `icon.rs` / `test_support.rs` repointed. `mod gallery` removed;
  `pub use context_menu::menu_card_preview` added under the gallery cfg.
- **`labonair-shell`** — `gallery.rs` moved here (it holds
  `Entity<ThemeStore>`), `#[cfg(any(debug_assertions, feature = "gallery"))]
  pub mod gallery;`; `commands.rs` calls `crate::gallery::open_gallery_window`.
- **`scripts/check_crate_deps.py`** — `labonair-theme-tokens` leaf entry;
  `ui-kit` allow-list → `{theme-tokens, gpui-ext}`; `theme` gains
  `theme-tokens`; `labonair-theme` added to `forbidden_for_ui_kit` (the
  regression the audit asked for).

## Dependencies

- Existing edges removed: `labonair-ui-kit → labonair-theme`.
- New edges: `ui-kit → theme-tokens`, `theme → theme-tokens`,
  `shell → theme-tokens` (via the moved gallery — shell already deps `theme`).
- Dependency verifier change: as above.

## Persistence and migration

None — pure code move, no logic change; every moved test moved with its code.

## User-visible behavior

None.

## Acceptance criteria

- [x] `crates/ui-kit/Cargo.toml` has no `labonair-theme` dependency.
- [x] `rg "labonair_theme|ThemeStore|IconThemeContent" crates/ui-kit/src`
      finds no production product references (only `labonair_theme_tokens` +
      the re-export shim; the `//! ... labonair-theme tokens` doc comments are
      prose).
- [x] Reusable components still render from the foundation token contract; no
      visual change (pure move).
- [x] The component gallery is reachable only through the documented
      `labonair-shell` debug integration.
- [x] A dependency-verifier regression rejects a `labonair-theme` dependency
      from `ui-kit` (`forbidden_for_ui_kit`).
- [x] `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace --no-fail-fast` (0 failures),
      `scripts/check-crate-deps.sh` (56 crates, 225 edges),
      `cargo check -p labonair-shell --features gallery`, and
      `git diff --check` pass.
- [ ] Native visual spot-check of the primitives — folds into R07-001.

## Notes and follow-ups

Blocked mid-task by a full `/` volume (Rust `target/` had grown to 140 GB);
recovered with `rm -rf target`. Next task: R08-012 (P1.2, `workspace →
hosts-ui`).
