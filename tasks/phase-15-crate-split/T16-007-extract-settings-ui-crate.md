# T16-007: `labonair-settings-ui` extrahieren

## Status
✅ Done

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-002 (`labonair-ui-kit`), T16-004 (`labonair-command-palette`)

## Ziel
Die Settings-Fenster-UI (`crates/ui/src/settings.rs`, 5 957 Z.) in einen eigenen
Crate `labonair-settings-ui` lösen. In dieser Task **kein** inhaltlicher Umbau:
`FIELDS`, `SECTION_GROUPS`, `CATEGORIES`, die Sonder-Panes bleiben wie sie sind.
Der Zed-Style-Umbau (generierte UI, JSON-Editor) passiert in Phase 18
(T19-004 ff.) — der braucht diesen Crate als Ort.

**Zusatz-Scope (Thema 2 — Command-Palette-Host-Zugang).** T16-004 hat
`labonair-command-palette` schon extrahiert und ist `✅ Done`; die dort noch
fehlende Fähigkeit für den Host-Workflow (siehe
[`docs/architecture.md §8.1`](../../docs/architecture.md)) wird hier
nachgezogen, weil T16-004 nicht mehr geändert werden soll und T16-006 gerade
läuft. Rein additiv, keine bestehende Palette-Semantik brechen. Siehe
Abschnitt **„Zusatz: Palette-Host-Seite"** unten.

## Kontext
- Heute: `crates/ui/src/settings.rs` — `PreferencesStore` (Entity),
  `GlobalPreferences`, `SettingsView`, `SettingsTab`, `open_settings_window`,
  `settings_bounds`, `set_settings_deps`, `SettingsDeps`, `SettingsWindowRef`,
  `SettingsTarget`, `FIELDS`, `FieldDef`, `FieldKind`, `SECTION_GROUPS`,
  `CATEGORIES`, `apply_prefs_to_theme`, `capture_keybind`, `overwrite_keybind`,
  MCP-Pane.
- Backend-Gegenstück: `crates/backend/src/modules/settings/{mod.rs,
  preferences.rs,editor.rs,mcp.rs}` — bleibt in `labonair-backend`.
- Konsumenten: `app_shell.rs` (`open_settings_window`, `PreferencesStore`,
  `set_settings_deps`, `SettingsTab`, `apply_prefs_to_theme`),
  `menu.rs` (`SettingsTab::from_deep_link`), `command_palette`
  (`OpenSettings`-Command).
- Zed-Vorbild: `zed-refrence/zed/crates/settings_ui/` (eigenständiger Crate,
  getrennt von `settings/`).

## Anweisungen zur Umsetzung
1. **`crates/settings-ui/` anlegen** (`labonair-settings-ui`,
   `src/settings_ui.rs` Lib-Root). Datei ist groß → beim Move gleich in
   Module schneiden (nur mechanisch, kein Logik-Refactor):
   - `src/store.rs` — `PreferencesStore`, `GlobalPreferences`.
   - `src/window.rs` — `open_settings_window`, `settings_bounds`,
     `SettingsWindowRef`, `SettingsTarget`, `SettingsDeps`, `set_settings_deps`.
   - `src/fields.rs` — `FIELDS`, `FieldDef`, `FieldKind`, `SECTION_GROUPS`,
     `CATEGORIES`, `SettingsTab`, Deep-Link-Mapping.
   - `src/view.rs` — `SettingsView` + `Render`.
   - `src/panes/` — Theme-Grid, Shortcuts, AI, MCP-Bridge-Pane.
   - `src/apply.rs` — `apply_prefs_to_theme`, Keybind-Capture-Helfer.
   - Lib-Root re-exportiert die heute öffentlichen Symbole unverändert.
2. Dependencies: `gpui`, `gpui-component`, `labonair-ui-kit`,
   `labonair-gpui-ext`, `labonair-theme`, `labonair-command-palette`,
   `labonair-notifications`, `labonair-backend`. **Kein** `labonair-ui`,
   **kein** `labonair-shell`.
3. Workspace-`Cargo.toml`: Member + Dep-Eintrag.
4. `crates/ui`: `mod settings;` raus; `crate::settings::` →
   `labonair_settings_ui::…` in `app_shell.rs`, `menu.rs`.
   `crates/app/src/main.rs` — falls es `init`-artige Aufrufe gibt, anpassen.
5. `cargo run`: `Cmd+,` öffnet das Settings-Fenster; alle 10 Kategorien
   rendern; Feld-Änderungen persistieren; Deep-Links (`AiSettings…`,
   Command-Palette `OpenSettings`) springen zur richtigen Kategorie;
   Shortcuts-Capture + MCP-Pane unverändert.

## Zusatz: Palette-Host-Seite (Thema 2)

In `labonair-command-palette` (nicht im Settings-Crate — nur hier
mit-erledigt):

6. **Zeilen-Sekundäraktion.** Das Palette-Zeilenmodell (`Command` /
   `command_palette.rs`) bekommt ein optionales Feld
   `secondary: Option<SecondaryAction>` = `{ keystroke: "shift-enter",
   label: &'static str, run: … }`. `Shift+Enter` auf einer Zeile mit
   `secondary` führt diese aus statt der Primäraktion. Zeilen ohne `secondary`
   unverändert. Eine **Footer-Hinweisleiste** am unteren Palette-Rand zeigt
   die aktiven Bindings der markierten Zeile (`↵ …   ⇧↵ …`).
7. **Eine `Page::Hosts`** statt `Page::HostsSsh` + `Page::HostsSftp`
   (`command_palette.rs:416-417`). Jede Host-Zeile: Primär =
   `newSshTab(host)`, Sekundär (`Shift+Enter`) = `newSftpTab(host)`,
   Subtitle = `user@addr:port`. Placeholder „Search hosts…".
8. **Quick-Connect am Root**: die zuletzt verbundenen Hosts (Sortierung nach
   `last_connected_at`, Limit 5) als direkte Zeilen im Root-`Page` unter
   Sektion „Hosts" — gleiche Primär/Sekundär-Semantik. Port von
   `reference-src/src/modules/command-palette/hooks/useHostCommands.ts`.
9. **`ConnectSsh` / `OpenSftp`** (`command_palette.rs:587-588`) zeigen beide
   auf `Page::Hosts`. `Cmd+Shift+N` (heute Label „New SSH Connection",
   `command_palette.rs:157`) triggert `ConnectSsh` → `Page::Hosts` statt
   `OpenHostManager`.
10. **Host-Daten**: Die Palette bekommt Hostliste + `newSshTab`/`newSftpTab`
    weiterhin als injizierte Callbacks/Daten vom Shell (bestehendes
    `known_hosts`/`RegistryCallbacks`-Muster) — **keine** neue Crate-Kante
    `command-palette → hosts-ui` (Abhängigkeitsregel 9,
    `docs/architecture.md §3`).
11. `OpenHostManager` bleibt vorerst als Command bestehen (öffnet den
    Host-Tab), wird aber in T19-010 auf „Open Host Settings" umgestellt.

## Akzeptanzkriterien
- [ ] `crates/settings-ui/` ist Member; `crates/ui` hat keine `settings.rs`.
- [ ] Die Datei ist in ≥5 Module geschnitten, jedes < ~1 200 Z.; keine
      Logik-Zeile inhaltlich geändert (nur `mod`/`use`/`pub use`).
- [ ] `cargo tree -p labonair-settings-ui` zeigt keine Kante zu `labonair-ui`
      oder `labonair-shell`.
- [ ] `cargo run`: Settings-Fenster (Größe 860 px, `[580,900]`-Höhe),
      alle Kategorien, Persistenz, Deep-Links, Shortcuts, MCP — identisch.
- [ ] Bestehende Settings-UI-Tests (Feld-/Kategorie-/Keybind-Tests) laufen im
      neuen Crate.
- [ ] **Zusatz:** Palette hat **eine** `Page::Hosts`; `Enter` öffnet SSH,
      `Shift+Enter` SFTP; Footer-Hinweisleiste zeigt die Bindings;
      Quick-Connect-Zeilen am Root; `Cmd+Shift+N` → `Page::Hosts`.
- [ ] **Zusatz:** `cargo tree -p labonair-command-palette` zeigt **keine**
      Kante zu `labonair-hosts-ui`.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- **Zusatz-Scope-Begründung:** Der Palette-Teil gehört thematisch zu T16-004
  (bereits `✅ Done`) bzw. Phase 17/18, wird aber laut Absprache „alles
  ≤ T16-006 → in Task 7" hier mit-erledigt, weil T16-006 gerade läuft. Er ist
  klein (ein optionales Feld + eine Seite zusammenführen + Footer-Leiste) und
  additiv. Die Host-*Verwaltung* (CRUD/Credentials) kommt separat in
  **T19-010** (Settings › Hosts).
- Der `panes/`-Modulordner sollte so geschnitten werden, dass in Phase 18
  eine weitere Custom-Pane („Hosts", T19-010) ohne Umbau danebenpasst.
- `PreferencesStore` bleibt vorerst hier (er ist UI-nah: hält
  `GlobalPreferences`, notifiziert Views). In Phase 18 wandert die
  *Modell*-Seite in `labonair-settings-content` / `labonair-settings`; die
  Store-Entity kann dann dünner werden oder ganz in `labonair-settings`
  ziehen. Nicht jetzt.
- Die GPUI-0.2.2-Fensterlimits (kein always-on-top / max-size / parent) im
  Modul-Doc-Kommentar mitnehmen — sie gelten weiter.

## Warnungen
- ⚠️ `open_settings_window` baut ein eigenes `cx.open_window` mit `Root`. Der
  Import von `gpui_component::Root` muss im neuen Crate verfügbar sein
  (`gpui-component` als direkte Dep).
- ⚠️ `SettingsDeps`/`SettingsTarget`/`SettingsWindowRef` sind GPUI-Globals —
  die `set_global`/`observe_global`-Aufrufe müssen weiter genau von
  `AppShell::new` (bzw. `labonair-shell`) getriggert werden.

## Weiterführende Tasks
- [T16-008: Panel-Crates ausgliedern](./T16-008-split-panel-crates.md)
- [T19-004: Settings-UI aus Modell generieren](../phase-18-settings-core/T19-004-generated-settings-ui.md)
