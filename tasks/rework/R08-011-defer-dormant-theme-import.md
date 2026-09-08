# R08-011 — Quarantine the dormant theme-file import/watch infrastructure

## Status

`✅ Done`

## Owner

- Module: `themes`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: n/a (no wiring — that is the point)

## Goal

The user theme-file import/export/watch code in `labonair-theme` has one clear
disposition: **deferred, code retained under test, wired to nothing**, with a
named owner and activation/removal condition. No product doc or UI implies it
works.

## Scope

- In scope: doc comments on `theme/src/import.rs`,
  `ThemeStore::import_theme_file{,_variant}`, `ThemeStore::reload_user_themes`,
  `ThemeRegistry::load_user_themes`; a row in `docs/capabilities.md`'s
  deferred table.
- Out of scope: removing the code (it is tested and plausibly wanted);
  building the import UI (that is the activation task); the built-in
  colour/icon catalogs and preview/confirmation behaviour (unchanged and
  fully supported).

## Contracts and ownership

- Public domain values: `ThemeFile`, `ThemeFileVariant`, `COLOR_TOKENS`
  (unchanged; retained).
- Service traits or typed events: none.
- Registry contributions: none — deliberately unregistered.
- UI surface: none.
- Shared UI-kit components: none.

## Dependencies

- Existing edges removed: none.
- New edges: none.
- Dependency verifier change: none.

## Persistence and migration

- Settings: None.
- Storage: None — no themes directory is scanned or watched by the running
  app.
- Compatibility: existing user theme JSON files (if any exist on disk) remain
  parseable by the retained conversion code once a flow activates it.

## User-visible behavior

- None — confirmed: no palette action, Settings field, or file-watch reaches
  the import/user-theme paths (verified by `rg` across `shell`, `theme-ui`,
  `settings-ui`, `hosts-ui`).

## Implementation plan

1. Confirm no consumer (grep) → verify: no `import_theme_file` /
   `reload_user_themes` / `load_user_themes` call outside `crates/theme`
   tests.
2. Add "Deferred boundary (R08-011)" doc comments to the four entry points and
   the module → verify: `cargo doc`-shaped comments compile
   (`cargo check -p labonair-theme`).
3. Add the deferred-table row to `docs/capabilities.md` → verify:
   `python3 scripts/check_documentation.py`.

## Acceptance criteria

- [x] Current product docs say user theme-file import is deferred, code
      retained, not wired; the built-in catalogs and preview/confirmation are
      supported.
- [x] No active UI promises remote downloads, marketplace, or user theme-file
      import.
- [x] The retained dormant code has a named owner (`labonair-theme-ui`) and an
      explicit activation/removal condition.
- [x] `cargo check`, `cargo test --workspace`,
      `python3 scripts/check_documentation.py`, and `git diff --check` pass.

## Notes and follow-ups

If a user theme-file import flow is ever approved it becomes a bounded task
owned by `labonair-theme-ui` (palette action + confirmation, reusing the
retained conversion + `reload_user_themes`). If rejected, delete `import.rs`,
the `import_theme_file*` / `reload_user_themes` / `load_user_themes` methods,
and `ThemeFile*` in one commit.
