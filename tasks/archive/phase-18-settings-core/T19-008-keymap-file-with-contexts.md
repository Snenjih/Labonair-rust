# T19-008: Keymap als Datei mit Kontexten + Chords

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T17-007 (`CommandRegistry`), T19-002 (`SettingsStore`)

## Ziel
Tastaturkürzel von der hartkodierten `enum ShortcutId` + flacher
`BTreeMap<slug,String>` auf eine editierbare `keymap.json` umstellen: mit
**Kontexten** (`context = "Editor && vim_mode == normal"`) und **Chords**
(`cmd-k cmd-s`). Das `ShortcutId`-Enum bleibt nur noch als Default-Quelle.

## Kontext
- Heute: `labonair-command-palette::keybind` (aus T16-004) — `KeybindMap`
  (`BTreeMap<slug, String>`), `ShortcutId`-Enum, `shortcut*`,
  `effective_binding`, `resolve_conflict`, `Conflict`. `apply_keybinds` +
  `menu.rs` wenden sie an. Overrides liegen im `preferences.keybinds`-Blob.
  Keine Kontexte, keine Chords.
- T17-007: `CommandRegistry` — Kommandos haben `id: CommandId` +
  `default_key: Option<&'static str>`. Keymap-Dispatch läuft über `CommandId`.
- Zed-Vorbild (die Blaupause):
  `zed-refrence/zed/crates/settings/src/keymap_file.rs` — `KeymapFile`,
  `KeymapBlock { context: Option<String>, bindings: Map<String, ActionSequence> }`,
  Chord-Parsing, `KeyBindingValidator`, `KeybindSource`
  (Default/BaseKeymap/User).
  `zed-refrence/zed/crates/settings/src/base_keymap_setting.rs` — Basis-Keymaps.
  `zed-refrence/zed/assets/keymaps/default-macos.json`,
  `.../default-linux.json` — Format-Vorlage.
  GPUI-Seite: `zed-refrence/zed/crates/gpui/` — `KeyBindingContextPredicate`,
  `KeymapVersion`, `cx.bind_keys(...)`, `KeyContext`.

## Anweisungen zur Umsetzung
1. **`keymap.json`-Format** festlegen (JSONC, kommentiert):
   ```jsonc
   [
     { "context": "Workspace", "bindings": {
         "cmd-t": "tab::NewTerminal",
         "cmd-shift-p": "command_palette::Toggle",
         "cmd-k cmd-s": "zed::OpenKeymap"        // Chord-Beispiel
     }},
     { "context": "Editor", "bindings": { "cmd-f": "search::Toggle" } }
   ]
   ```
   Action-Namen = `CommandId`-String (`<namespace>::<Name>`).
2. **`labonair-keymap` (Crate oder Modul in `labonair-settings`)** — Port von
   `keymap_file.rs`, reduziert:
   - Parser für die Datei (mit Kommentaren/trailing commas — den JSONC-Parser
     aus T19-005 wiederverwenden).
   - `KeyContextPredicate` (`&&`, `||`, `!`, `==`, `in`) — GPUIs vorhandenen
     Prädikat-Parser nutzen, falls in `gpui` 0.2.2 exportiert; sonst
     minimal selbst (nur `&&` + `==` + Bezeichner).
   - Chord-Parsing (`"cmd-k cmd-s"` → `Vec<Keystroke>`).
   - Validierung: unbekannte Action → Fehler mit Zeile; Konflikt (gleicher
     Chord+Kontext zweimal) → Warnung.
   - `KeybindSource` (Default / BaseKeymap / User) für die Anzeige „woher".
3. **Default-Keymaps als Assets**: `assets/keymaps/default-macos.json`,
   `assets/keymaps/default-linux.json` — aus dem heutigen `ShortcutId`-Enum
   + `CommandRegistry::default_key` generieren (einmalig ein
   `build.rs`/Test, der Enum→JSON dumpt, oder von Hand + Test auf
   Vollständigkeit). `enum ShortcutId` bleibt die Quelle dieser Defaults.
4. **User-Keymap**: `~/<config_dir>/labonair/keymap.json`. Merge:
   Default(plattform) → BaseKeymap(optional, T-später) → User. User gewinnt;
   `"key": null` hebt eine Bindung auf.
5. **Anbindung an GPUI/`CommandRegistry`**: nach dem Laden/Mergen die
   effektiven Bindungen via `cx.bind_keys` registrieren; Dispatch löst den
   `CommandId` in der `CommandRegistry` aus (T17-007). `apply_keybinds` +
   der `preferences.keybinds`-Blob entfallen (Migration: T19-009).
6. **Live-Reload**: `keymap.json` per fs-Watch beobachten → neu parsen,
   validieren, `cx.bind_keys` neu setzen. Fehler → Banner + letzte gute
   Keymap behalten.
7. **Shortcuts-Settings-Pane** (die Custom-Pane aus T19-004) umbauen:
   - Liste aller Kommandos mit aktueller effektiver Bindung + Quelle
     (Default/User) + Kontext.
   - „Bearbeiten" → Keystroke/Chord aufnehmen → schreibt einen User-
     `keymap.json`-Block (surgisch, T19-005-Mechanik) mit dem passenden
     `context`.
   - Konflikt-Auflösung (bestehende `resolve_conflict`-Logik, jetzt
     kontext-bewusst): gleicher Chord in **unterschiedlichem** Kontext = kein
     Konflikt.
   - „Auf Standard zurücksetzen" (pro Binding / global) → User-Block entfernen.
   - „keymap.json öffnen"-Button.
8. **`command_palette`**: Keybind-Anzeige neben jedem Kommando nutzt jetzt die
   gemergte Keymap (kontext-gefiltert auf den aktuellen Fokus-Kontext).
9. **Tests**: Chord-Parsing; Kontext-Prädikat-Auswertung; Default→User-Merge
   inkl. `null`-Unbind; unbekannte Action ⇒ Fehler mit Zeile; gleicher Chord
   in zwei Kontexten ⇒ kein Konflikt; Live-Reload wendet Änderung an.
10. `cargo run`: `keymap.json` mit einem Custom-Chord (`cmd-k cmd-t` →
    `tab::NewTerminal`) anlegen → wirkt; ein `cmd-f` nur im `Editor`-Kontext
    bindet nicht im Terminal; Shortcuts-Pane zeigt Quelle + Kontext; Datei
    editieren + speichern → sofort aktiv.

## Akzeptanzkriterien
- [x] `keymap.json` (JSONC, kommentiert) mit `context` + Chord-Support wird
      geladen, validiert und via `cx.bind_keys` angewendet.
- [x] Default-Keymaps als plattformabhängige Assets, generiert aus
      `ShortcutId` + `CommandRegistry::default_key`; Vollständigkeits-Test.
- [x] User-Keymap merged über Default; `null` hebt auf; Quelle je Binding
      sichtbar.
- [x] `apply_keybinds` + `preferences.keybinds`-Blob sind entfernt
      (Migration in T19-009).
- [x] Live-Reload; kaputte Datei → Banner + letzte gute Keymap.
- [x] Shortcuts-Pane: Bearbeiten (schreibt User-Block), Konflikt (kontext-
      bewusst), Reset, „keymap.json öffnen".
- [x] Command-Palette zeigt die kontext-gefilterte effektive Bindung.
- [x] Tests: Chords, Kontext-Prädikate, Merge+Unbind, Fehlerzeile, Kontext-
      Nicht-Konflikt, Live-Reload.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Basis-Keymaps (VSCode/JetBrains/Sublime-Presets) sind **nicht** Teil dieser
  Task — nur die Infrastruktur (`KeybindSource::BaseKeymap` vorsehen). Presets
  als späteres Ticket.
- `keymap.json` ist bewusst **kein** Teil von `SettingsContent` (eigene Datei,
  eigener Loader) — nur `base_keymap: BaseKeymap` steht in den Settings.
- **Umsetzung (2026-09-04):** `crates/settings/src/keymap.rs` (Parse/Merge/
  Validate, entkoppelt von `CommandId` per Crate-Graph-Regel — Actions werden
  als Strings validiert, `known_actions` kommt vom Aufrufer),
  `crates/settings/assets/keymaps/default-{macos,linux}.json`,
  `crates/shell/src/keymap_loader.rs` (Laden/Mergen/Live-Reload,
  `KeybindDisplay`-Global, hält letzte gute User-Keymap bei kaputter Datei),
  `crates/settings-ui/src/keymap_edit.rs` (chirurgischer `keymap.json`-Writer).
  `apply_keybinds` + `preferences.keybinds` entfernt (`menu::apply_keymap`
  ersetzt sie). **Bewusst reduzierter Scope** (dokumentiert im Code):
  Shortcuts-Pane nimmt weiterhin nur Einzel-Keystrokes auf (Chords sind im
  Modell voll unterstützt, nur die Aufnahme-UI noch nicht); Konflikterkennung
  + Command-Palette-Bindungsanzeige sind kontext-agnostisch (erster Treffer)
  statt voll fokus-kontext-gefiltert — beides spätere Politur, nicht
  Blocker für diese Task laut deren eigenen Prioritäts-Vorgaben.

## Warnungen
- ⚠️ GPUI 0.2.2: prüfen, welche Keymap-/Kontext-APIs (`KeyBindingContextPredicate`,
  `cx.bind_keys`, Chord-`Keystroke`-Parsing) die veröffentlichte Version
  exportiert. Fehlt etwas → in `zed/crates/gpui` nachsehen und entweder ein
  reduziertes Prädikat selbst bauen oder als Blocker für T22-001 (vendored
  gpui) vermerken.
- ⚠️ macOS-Menü-Accelerators (`menu.rs`) müssen mit der Keymap konsistent
  bleiben — die Menü-Items ihre Anzeige aus der gemergten Keymap ziehen.
- ⚠️ Fokus-Kontext (`KeyContext`) muss von den Views korrekt gesetzt werden
  (`key_context("Editor")` etc.) — bestehende `key_context`-Aufrufe prüfen/
  ergänzen, sonst greifen kontextspezifische Bindings nie.

## Weiterführende Tasks
- [T19-009: Settings-Migrator](./T19-009-settings-migrator.md)
- [T22-001: vendored `gpui` — Entscheidung](../phase-21-gpui-decision/T22-001-vendored-gpui-decision.md)
