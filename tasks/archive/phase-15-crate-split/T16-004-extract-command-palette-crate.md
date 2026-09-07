# T16-004: `labonair-command-palette` extrahieren

## Status
✅ Done

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-002 (`labonair-ui-kit`), T16-003 (`labonair-notifications`)

## Ziel
Die Command-Palette samt Shortcut-/Keybind-Modell aus `crates/ui` in einen
eigenen Crate `labonair-command-palette` lösen. Dieser Crate wird später (Phase
16 / 18) die Basis für die `CommandRegistry` und die `keymap.json`. In dieser
Task: reiner Move, API stabil.

## Kontext
- Heute: `crates/ui/src/command_palette.rs` — `CommandPalette` (View),
  `CommandId`, `PaletteChoice`, `PaletteData`, `PaletteEvent`, dazu das
  Keybind-/Shortcut-Modell: `effective_binding`, `resolve_conflict`,
  `shortcut`, `shortcut_slug`, `shortcuts`, `Conflict`, `KeybindMap`,
  `ShortcutId`, `Fuzzy`/`match_score` (Fuzzy-Matcher).
- Nutzer: `app_shell.rs` (`CommandPalette`, `PaletteEvent`,
  `build_palette_data`, `pending_commands`), `settings.rs` (importiert
  `effective_binding`, `resolve_conflict`, `shortcut*`, `Conflict`,
  `KeybindMap`, `ShortcutId` für die Shortcuts-Settings-Seite),
  `menu.rs` (`apply_keybinds`).
- Zed-Vorbild: `zed-refrence/zed/crates/command_palette/`,
  `zed-refrence/zed/crates/command_palette_hooks/`,
  `zed-refrence/zed/crates/fuzzy/` (eigener Fuzzy-Crate).

## Anweisungen zur Umsetzung
1. **`crates/command-palette/` anlegen** (`labonair-command-palette`,
   `src/command_palette.rs` Lib-Root).
2. Inhalt aufteilen:
   - `src/palette.rs` — `CommandPalette`-View + `PaletteData`/`PaletteChoice`/
     `PaletteEvent`/`CommandId`.
   - `src/keybind.rs` — `KeybindMap`, `ShortcutId`, `shortcut*`,
     `effective_binding`, `resolve_conflict`, `Conflict`.
   - `src/fuzzy.rs` — der `Fuzzy`/`match_score`-Matcher (falls hier definiert;
     sonst dort lassen wo er liegt und nur re-exportieren).
   - Lib-Root re-exportiert alle heute öffentlichen Symbole unverändert.
3. Dependencies: `gpui`, `labonair-ui-kit`, `labonair-gpui-ext`,
   `labonair-theme`. **Kein** Rückbezug auf `crates/ui`.
4. Workspace-`Cargo.toml`: Member + Dep-Eintrag.
5. `crates/ui`: `mod command_palette;` raus; `crate::command_palette::` →
   `labonair_command_palette::` in `app_shell.rs`, `settings.rs`, `menu.rs`.
6. `ai_composer.rs` nutzt laut Handshake `palette Fuzzy match_score` — Import
   dort ebenfalls anpassen.
7. `cargo run`: `Cmd+Shift+P` öffnet die Palette wie bisher; Shortcut-Capture
   in den Settings funktioniert; Menü-Keybinds (`apply_keybinds`) wirken.

## Akzeptanzkriterien
- [ ] `crates/command-palette/` ist eigener Member; `crates/ui` hat keine
      `command_palette.rs` mehr.
- [ ] Alle heute öffentlichen Symbole sind unter `labonair_command_palette::`
      erreichbar, gleiche Namen.
- [ ] `settings.rs`, `menu.rs`, `app_shell.rs`, `ai_composer.rs` kompilieren
      nur mit geändertem Import-Pfad.
- [ ] `cargo run`: Palette öffnen/filtern/auswählen, Shortcut-Neubelegung inkl.
      Konflikt-Dialog, Menü-Accelerator — alle unverändert.
- [ ] Bestehende Palette-/Keybind-/Fuzzy-Tests laufen im neuen Crate.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- `ShortcutId` bleibt hier vorerst der zentrale Enum. Erst T19-008 macht daraus
  „nur noch Default-Quelle" neben der `keymap.json`.
- Die spätere `CommandRegistry` (T17-007) baut auf `CommandId`/`PaletteData`
  auf — deshalb Namen jetzt nicht umbenennen.

## Warnungen
- ⚠️ Der Fuzzy-Matcher wird an mehreren Stellen genutzt (Palette, `@`-File-Picker
  in `ai_composer`, Settings-Suche). Sicherstellen, dass er nach dem Move für
  alle drei importierbar ist.

## Weiterführende Tasks
- [T16-005: `labonair-panel` Contracts-Crate](./T16-005-panel-contracts-crate.md)
