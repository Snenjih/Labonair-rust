# R08-007 — Delete the pre-migration ShortcutId / SHORTCUTS keymap model

## Status

`✅ Done`

## Owner

- Module: `keymap`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::commands` / `::keymap_loader`

## Goal

`CommandId` + owner `CommandDescriptor` default bindings are the sole source
for command rows, defaults, and keymap-management data. The
reference-derived `ShortcutId` / `SHORTCUTS` cheat-sheet model — inert since
the runtime migration — is physically gone.

## Scope

- In scope: `crates/keymap/src/lib.rs` (delete `SHORTCUTS`, `ShortcutGroup`,
  `Shortcut`, `shortcut*()`, `find_conflict`, `resolve_conflict`,
  `effective_binding(s)`, `Conflict`, `KeybindMap`, `RESERVED_ACCELERATORS`,
  keep `normalize_keystrokes` + `keystroke_tokens`); the
  `CommandDescriptor.shortcut` field + `with_shortcut()` +
  `command_for_shortcut()` in `command-palette-core`; every `.with_shortcut()`
  call site (workspace / editor / settings / keymap / command-palette-core
  providers, `shell::commands::command_descriptor`); the `Command.shortcut`
  field in `palette.rs`; the `command_palette.rs` re-export block; and the now
  dead `labonair-interaction-contracts` crate.
- Out of scope: the v1→v2 keybind-slug alias table in
  `settings::legacy_migrations::migrate_v2` (string data, genuine migration
  compat); the keymap runtime / file / management modules (unchanged).

## Contracts and ownership

- Public domain values: `CommandId` (`command-palette-core`) is the single
  command identity. `DefaultBinding` metadata carries owner defaults.
- Service traits or typed events: unchanged.
- Registry contributions: unchanged (`ThemeCommandProvider`,
  `WorkspaceCommandProvider`, … keep their `with_default_binding` metadata).
- UI surface: none.
- Shared UI-kit components: none.

## Dependencies

- Existing edges removed: `command-palette-core → interaction-contracts`,
  `keymap → interaction-contracts`, `editor → interaction-contracts`,
  `settings → interaction-contracts`; the `labonair-interaction-contracts`
  crate is deleted.
- New edges: none.
- Dependency verifier change: drop the `labonair-interaction-contracts`
  allow-list entry and remove it from the four consumer sets.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: the runtime already resolved from `CommandDescriptor`
  defaults, so no default binding changed. Old `keymap.json` files are
  unaffected (they are action-name keyed, not slug keyed).

## User-visible behavior

- None — the removed metadata was never rendered.

## Implementation plan

1. Strip `.with_shortcut()` from every provider + the shell helper; drop the
   `shortcut` fields and `command_for_shortcut` → verify: `cargo check --workspace`.
2. Rewrite `keymap/src/lib.rs` down to the two keystroke-string helpers; trim
   the `command_palette.rs` re-exports to `keystroke_tokens` → verify:
   `cargo test -p labonair-keymap -p labonair-command-palette`.
3. Delete `crates/interaction-contracts`, update the manifests, the verifier,
   and the normative docs (`architecture.md`, `registries.md`,
   `architecture-inventory.md`); regenerate the crate graph → verify:
   `bash scripts/check-crate-deps.sh`, `python3 scripts/check_documentation.py`,
   `cargo test --workspace`.

## Acceptance criteria

- [x] New palette rows and keymap UI do not read `SHORTCUTS` (it no longer
      exists).
- [x] `ShortcutId` is referenced nowhere in `crates/` (the type and its crate
      are deleted).
- [x] No default binding is defined twice — `with_default_binding` is the only
      source; the parallel `SHORTCUTS` binding strings are gone.
- [x] Current documentation describes `CommandId` as the single command
      identity and does not present `ShortcutId` as an authority.
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`,
      `python3 scripts/check_documentation.py`, and `git diff --check` pass
      (55 crates, 224 edges).

## Notes and follow-ups

Net ~810 deletions / ~405 additions across 25 files, almost all deletion.
`normalize_keystrokes` (used by `keymap::runtime` / `::management`) and
`keystroke_tokens` (used by `keymap-ui` and the palette) stay in
`keymap/src/lib.rs`. Next task: R08-008 (P1.1, `ui-kit → theme`).
