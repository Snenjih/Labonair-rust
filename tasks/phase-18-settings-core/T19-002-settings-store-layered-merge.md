# T19-002: `SettingsStore` + Layer-Merge + `Settings`-Trait

## Status
📋 Geplant

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-001 (`labonair-settings-content`)

## Ziel
Der Crate `labonair-settings` mit dem `SettingsStore`: er lädt alle Schichten,
merged sie in der festen Reihenfolge zu einem effektiven `SettingsContent`,
überwacht die Dateien live, und stellt feature-lokale, typisierte
Settings-Slices über ein `Settings`-Trait + Registrierung bereit. Die
`Preferences`-Brücke aus T19-001 bleibt vorerst als Kompatibilitäts-Layer.

## Kontext
- T19-001: `SettingsContent` (Baum, `Option<T>`), `MergeFrom`, `defaults()`,
  `assets/settings/default.json`, `parse`.
- Heute: `PreferencesStore` (Entity in `labonair-settings-ui`), `preferences_load`
  / `preferences_save` (in `labonair-backend`), `GlobalPreferences`-Global,
  `cx.observe`/`observe_global` in den Modulen.
- Zed-Vorbild:
  `zed-refrence/zed/crates/settings/src/settings_store.rs` — `SettingsStore`,
  `register_setting::<T>()`, `T::get(location, cx)`, `set_user_settings`,
  `recompute` (Merge), `SettingsLocation`, `LocalSettingsKind`.
  `zed-refrence/zed/crates/settings/src/settings.rs` — `trait Settings`
  (`fn from_settings(content: &SettingsContent) -> Self`, `fn register(cx)`),
  `settings_macros::RegisterSetting`, `inventory`.
  `zed-refrence/zed/crates/settings/src/settings_file.rs` — fs-Watch.

## Anweisungen zur Umsetzung
1. **`crates/settings/` anlegen** (`labonair-settings`, `src/settings.rs`
   Lib-Root). Deps: `labonair-settings-content`, `labonair-settings-macros`,
   `gpui` (für den Store als Global + `App`-Zugriff), `serde_json`,
   `notify`/`fs`-Watch, `inventory`. **Kein** UI-Crate.
2. **Schicht-Modell**: `enum SettingsLayer { Default, User, Os, Profile,
   Project(WorktreeId), Language(String) }` mit fester Merge-Reihenfolge
   (Default zuerst, Sprache zuletzt). `struct SettingsStore { raw: HashMap<
   SettingsLayer, SettingsContent>, merged: SettingsContent, ... }`.
   - `Default` = `SettingsContent::defaults()` aus `assets/settings/default.json`.
   - `User` = `~/<config_dir>/labonair/labonair-settings.json` (der bestehende
     Ort; der `preferences`/`editor`/`mcp`-Verbund wird von T19-009 in den
     flachen `SettingsContent` migriert).
   - `Os` / `Profile` — Struktur vorsehen, aber nur aktiv, wenn Keys vorhanden
     (`platform_overrides`, `profiles` wie bei Zed).
   - `Project` — leer bis T19-003.
   - `Language` — leer/Platzhalter (Labonair ist kein LSP-Editor; Feld
     vorsehen für Editor-per-Sprache-Overrides, kein Muss jetzt).
3. **`recompute()`**: `merged = defaults(); for layer in order { merged.merge_from(&layer_content) }`.
   Nach jedem `recompute` die abgeleiteten Slices neu berechnen + Observer
   benachrichtigen.
4. **`trait Settings`** (Port):
   - `fn from_settings(content: &SettingsContent) -> Self;`
   - `fn register(cx: &mut App)` → trägt sich im `SettingsStore` ein.
   - `fn get(cx: &App) -> &Self` (Store hält den berechneten Wert je Typ).
   - `#[derive(RegisterSetting)]` in `labonair-settings-macros` (kleiner
     proc-macro; Port aus `zed/crates/settings_macros`).
   - `inventory::collect!` für Compile-Zeit-Registrierung.
5. **Konkrete `Settings`-Structs** für die Hauptverbraucher — mind.:
   `ThemeSettings`, `TerminalSettings`, `EditorSettings`, `AiSettings`,
   `WorkspaceSettings`, `PersonalizationSettings`. Jede `from_settings` liest
   ihren Teilbaum mit `defaults()`-Fallback. Die Module (Terminal/Editor/
   Theme/Panels) stellen schrittweise von `GlobalPreferences` auf
   `XSettings::get(cx)` um — **in dieser Task mindestens `ThemeSettings` +
   `TerminalSettings` real umgestellt**, Rest als Folge (T20-007 u.a.).
6. **Live-fs-Watch**: `labonair-settings.json` (+ später Projekt-Dateien)
   beobachten; bei Änderung: parsen, `User`-Layer setzen, `recompute`. Debounce
   (~150 ms). Fehlerhafte Datei → alte `merged` behalten + Fehler-Toast/Log
   (nicht crashen; `.bak` wie bisher).
7. **Kompat-Brücke**: `GlobalPreferences(Preferences)` weiter publizieren
   (aus `From<&SettingsContent>`), damit noch nicht migrierte Module laufen.
   `PreferencesStore` (settings-ui) delegiert Lesen/Schreiben an den
   `SettingsStore` (schreiben = User-Layer patchen + persistieren; die
   surgische JSON-Schreibweise kommt in T19-005).
8. **Init**: `labonair_settings::init(cx)` in `main.rs`/`bootstrap` — vor dem
   ersten Render, nach dem Migrator (T19-009).
9. **Tests**: Merge-Reihenfolge (user schlägt default, projekt schlägt user);
   `Settings::get` liefert gemergten Wert; fs-Watch löst `recompute` +
   Observer aus (mit GPUI-Test-Executor); kaputte Datei ⇒ letzter guter Wert.

## Akzeptanzkriterien
- [ ] `crates/settings/` + `crates/settings-macros/` existieren, ohne
      UI-Deps.
- [ ] `SettingsStore` merged Default → User (→ Os/Profile/Project/Language)
      in fester Reihenfolge; `recompute` benachrichtigt Observer.
- [ ] `trait Settings` + `#[derive(RegisterSetting)]` + `inventory`
      funktionieren; ≥6 konkrete `Settings`-Structs registriert.
- [ ] `ThemeSettings` und `TerminalSettings` werden von den echten Modulen
      über `XSettings::get(cx)` konsumiert (nicht mehr über
      `GlobalPreferences`).
- [ ] Live-fs-Watch: externe Änderung an `labonair-settings.json` wirkt ohne
      Neustart; kaputte Datei crasht nicht.
- [ ] `GlobalPreferences`-Brücke bleibt für nicht-migrierte Module aktuell.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- `inventory` auf macOS **und** Linux verifizieren (Linker-Sektionen) — früh,
  nicht am Ende.
- Der volle Umzug aller Module weg von `GlobalPreferences` ist bewusst NICHT
  Teil dieser Task — nur die zwei Referenz-Konsumenten. Der Rest folgt
  inkrementell (Folge-Tickets / T20-007).

## Warnungen
- ⚠️ Store-als-Global + GPUI: `SettingsStore::update_global` / `observe_global`
  wie bei Zed; niemals den Store vom Tokio-Thread mutieren (fs-Watch-Callback
  über `cx.spawn` auf den Foreground bringen).
- ⚠️ Merge-Reihenfolge ist normativ (`docs/architecture.md`) — Default zuerst,
  Sprache zuletzt. Nicht umdrehen.
- ⚠️ Schreiben passiert in dieser Task noch „dumm" (ganzer User-Layer neu
  serialisiert) — das ist ok, T19-005 macht es surgisch. Aber `.bak` + atomarer
  `rename` schon jetzt.

## Weiterführende Tasks
- [T19-003: Projekt-/Ordner-Settings](./T19-003-project-folder-settings.md)
- [T19-004: Settings-UI aus Modell generieren](./T19-004-generated-settings-ui.md)
- [T19-005: Rohe `settings.json` editierbar](./T19-005-raw-json-settings-editor.md)
