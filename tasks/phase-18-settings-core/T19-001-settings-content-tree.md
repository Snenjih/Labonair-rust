# T19-001: `labonair-settings-content` — typisierter Settings-Baum + `MergeFrom`

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T16-001 (ADR & Ziel-Crate-Graph), T16-007 (`labonair-settings-ui` extrahiert), T19-000 (Settings-Design-Kontrakt)

## Ziel
Das Fundament des neuen Settings-Systems: ein Crate `labonair-settings-content`
mit dem vollständig typisierten `SettingsContent`-Baum (alle Bereiche) und dem
`MergeFrom`-Trait, mit dem sich Schichten (default, user, projekt, …) sauber
übereinanderlegen lassen. Ersetzt langfristig die flache `Preferences`-Struct.

## Kontext
- Heute: `crates/backend/src/modules/settings/preferences.rs` —
  `struct Preferences` (~170 Felder, `#[serde(default, rename_all = "camelCase")]`,
  Kategorien als Kommentar-Blöcke), plus `editor.rs` (`"editor"`-Key),
  `mcp.rs` (`"mcp"`-Key). Alles in `labonair-settings.json`.
- Zed-Vorbild (die Blaupause):
  `zed-refrence/zed/crates/settings_content/src/settings_content.rs` —
  `struct SettingsContent { ... }` (Baum), Submodule `editor.rs`,
  `terminal.rs`, `theme.rs`, `project.rs`, `workspace.rs`, `title_bar.rs`,
  `agent.rs`, `language.rs`.
  `zed-refrence/zed/crates/settings_content/src/merge_from.rs` —
  `trait MergeFrom { fn merge_from(&mut self, other: &Self); }` + Impls für
  `Option`, Maps, Vecs, Primitives; `#[derive(MergeFrom)]`.
  `zed-refrence/zed/crates/settings_content/src/fallible_options.rs` —
  `FallibleOption` (Feld ungültig ⇒ Default statt Datei-Abbruch).

## Anweisungen zur Umsetzung
1. **`crates/settings-content/` anlegen** (`labonair-settings-content`,
   `src/settings_content.rs` Lib-Root). Deps minimal: `serde`, `serde_json`,
   `schemars` (für T19-006 vorbereitet). **Kein** GPUI, **kein** UI-Crate,
   **kein** `labonair-backend`.
2. **`SettingsContent`-Baum** definieren — Bereiche als eigene Structs,
   Feldnamen 1:1 aus der heutigen `Preferences` (camelCase serde), gruppiert:
   - `general: GeneralContent` (theme, restore_window_state, startup_tab,
     startup_terminal_count, autostart, credential_encryption,
     notify_on_errors, confirm_quit_with_ssh, check_for_updates,
     session_restore)
   - `appearance: AppearanceContent` (app_theme, theme_variant_overrides,
     app_font_size, app_line_height, app_font_family, reduce_motion,
     app_corner_radius, background_*, tabs_location …)
   - `terminal: TerminalContent`
   - `editor: EditorContent` (den heutigen `"editor"`-Key hier eingliedern)
   - `file_manager: FileManagerContent`
   - `connections: ConnectionsContent` (Explorer/Host-Availability-Polling +
     die im `subagent-2.md` als fehlend markierten Felder — jetzt modellieren;
     **die eigentlichen Host-Einträge/Credentials NICHT hier**, siehe `hosts`)
   - `hosts: HostsContent` — **eigener Top-Level-Bereich** (nicht unter
     `connections`), Vorbild „Themes ist eigener Bereich"
     (`docs/architecture.md §8.1`). Enthält: `entries: Vec<HostEntry>`
     (Name, Adresse, Port, User, Auth-Methode, Jump-Host-Ref, Tunnel-Liste,
     `last_connected_at`, Gruppe/Tag), `default_shell`, `keepalive`,
     `ssh_config_import`-Optionen. **Credentials/Secrets** werden **nicht** in
     `SettingsContent`/`settings.json` serialisiert — nur eine Keyring-Referenz
     (`credential_ref: Option<String>`); die Secrets bleiben im OS-Keychain
     (Critical Rule: keine Secrets in SQLite/JSON). Feldnamen 1:1 aus dem
     heutigen `backend::modules::ssh`/`hosts`-Modell ableiten.
   - `workspace: WorkspaceContent` (command-palette, bookmarks, source-control,
     dock-layout-Referenz)
   - `ai: AiContent` (defaults/providers/behaviour/agents/directives)
   - `mcp: McpContent` (der heutige `"mcp"`-Key)
   - `personalization: PersonalizationContent`
     (statusBarItemPlacements, panelToggleVisibility — aus Phase 17)
   - `keymap` bleibt **separat** (eigene `keymap.json`, T19-008) — hier nur
     ein `base_keymap: BaseKeymap`-Feld (VSCode/JetBrains/…), kein Bindings-Baum.
   - Jedes Feld `Option<T>` (für Schicht-Merge: „nicht gesetzt" ≠ „Default").
   - `#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema, MergeFrom)]`.
3. **`MergeFrom`** implementieren (Port aus Zed):
   - `trait MergeFrom` + Blanket-/Basis-Impls (`Option<T>`: `other.is_some()`
     gewinnt und merged rekursiv; `Vec`: ersetzen oder anhängen — je Feld
     via Attribut; `BTreeMap`: key-weise mergen).
   - `#[derive(MergeFrom)]` via `labonair-settings-macros` (kleiner
     proc-macro-Crate — hier gleich mit anlegen, minimal).
4. **`DEFAULTS`**: `SettingsContent::defaults()` — der voll ausgefüllte Baum
   mit allen heutigen `Preferences`-Default-Werten. Zusätzlich als
   **dokumentiertes JSON-Asset** `assets/settings/default.json` (Vorbild
   `zed-refrence/zed/assets/settings/default.json` — jeder Key mit
   Kommentar). `defaults()` und `default.json` müssen übereinstimmen (Test).
5. **`FallibleOption`**-Äquivalent: eine `parse(json) -> (SettingsContent,
   Vec<FieldError>)`-Funktion, die pro Feld fehlertolerant ist.
6. **Ableitung von `Preferences`**: `impl From<&SettingsContent> for Preferences`
   (mit `.unwrap_or_default()` je Feld über die `defaults()`), damit der
   bestehende Code (Terminal/Editor/Theme lesen `Preferences`) unverändert
   weiterläuft, bis T19-002 den Store umstellt.
7. **Kategorie-Metadaten** (Vorbereitung T19-004): eine kleine, deklarative
   Liste `AREAS: &[AreaMeta]` mit `{ key, title, slug, kind }`, wobei
   `kind ∈ { Generated, Custom }`. `Generated` = Felder werden aus dem Typ
   gerendert (General, Appearance, Terminal, Editor, File Manager, Connections,
   Workspace). `Custom` = Sonder-Pane als **Top-Level-Kategorie** (Themes,
   **Hosts**, Shortcuts, AI, MCP, Personalisierung). Diese Liste ist die
   einzige Stelle, an der eine neue Top-Level-Kategorie registriert wird —
   „Custom-Top-Level" ist damit ein **sanktionierter Sonderfall**, kein Hack
   (`docs/settings-guidelines.md` Punkt 4). Kein UI hier, nur die Daten.
8. **Tests**: `defaults()` == geparste `default.json`; `MergeFrom` (user über
   default, projekt über user); `parse` mit einem kaputten Feld ⇒ Default +
   gemeldeter Fehler, restliche Felder intakt; `SettingsContent → Preferences`
   Round-Trip der Default-Werte; `hosts.entries`-Round-Trip **ohne** Secrets
   (nur `credential_ref`); jede `AREAS`-`key` trifft ein reales Feld/Submodul.

## Akzeptanzkriterien
- [x] `crates/settings-content/` + `crates/settings-macros/` existieren,
      Workspace-Members, ohne GPUI-/UI-/backend-Deps.
- [x] `SettingsContent` deckt alle heutigen `Preferences`-Felder ab **plus**
      die in `subagent-2.md` als fehlend markierten (Connections, Bookmarks,
      Statusbar-Toggles) **plus** `hosts: HostsContent` als eigener
      Top-Level-Bereich — Feldnamen camelCase-serde wie bisher.
- [x] `hosts`-Einträge serialisieren **keine** Secrets, nur `credential_ref`
      (Test).
- [x] `AREAS` listet die Top-Level-Kategorien mit `kind` (Generated/Custom);
      „Themes", „Hosts", „Shortcuts", „AI", „MCP", „Personalisierung" sind
      `Custom`.
- [x] `MergeFrom` + `#[derive(MergeFrom)]` funktionieren; Schicht-Merge
      getestet.
- [x] `SettingsContent::defaults()` und `assets/settings/default.json` sind
      inhaltsgleich (Test erzwingt das); `default.json` ist kommentiert.
- [x] `parse` ist feld-fehlertolerant (Test) — Granularität pro Top-Level-Area
      dokumentiert (siehe `## Notizen`).
- [x] `impl From<&SettingsContent> for Preferences` erlaubt dem bestehenden
      Code, unverändert weiterzulaufen (`crates/backend/src/modules/settings/
      content_bridge.rs`; dafür bekommt `labonair-backend` einen neuen,
      dokumentierten Edge auf `labonair-settings-content`, siehe
      `docs/architecture.md` §8.15).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Diese Task ändert **noch nicht** das Laufzeitverhalten — sie baut nur das
  Modell + die `default.json`. T19-002 schaltet den Store darauf um.
- `zed-refrence/zed/crates/settings_content/` ist fast 1:1 die Vorlage — dort
  die Struktur der Submodule + `merge_from.rs` genau lesen und übernehmen,
  Zed-spezifische Bereiche (collab, dap, vim-tiefe) weglassen.
- **`parse`-Granularität:** Zeds `FallibleOption` erholt sich pro **Blatt-Feld**.
  Dieser Port (`fallible.rs`) erholt sich pro **Top-Level-Area** (ein
  `SettingsContent`-Feld, z. B. `"terminal"`): ein kaputtes Feld defaultet die
  ganze Area und meldet einen `FieldError`; alle anderen Areas parsen
  unabhängig weiter. Erfüllt das Akzeptanzkriterium ("ein kaputtes Feld
  defaultet, der Rest bleibt intakt") ohne einen Pro-Feld-Derive für ~150
  Felder zu bauen; echte Blatt-Granularität kann später nachgerüstet werden,
  ohne die Funktions-Signatur zu ändern.
- **`labonair-backend`-Edge:** `impl From<&SettingsContent> for Preferences`
  (Anweisung #6) lebt in `labonair-backend` (`modules::settings::
  content_bridge`), nicht in `labonair-settings-content` — sonst müsste
  `labonair-settings-content` auf `labonair-backend` zurück-referenzieren
  (verboten). `labonair-backend` bekommt dafür einen neuen, einseitigen Edge
  auf `labonair-settings-content`; als Abweichung von der bisherigen
  „`labonair-backend` → leaf"-Grafik in `docs/architecture.md` §8.15
  dokumentiert.
- `hosts.entries` ist ein **neues** Modell (`HostAuthMethod`/`HostEntry`/
  `HostTunnel`), keine 1:1-Wiederverwendung von
  `labonair-backend::modules::hosts::db::Host` — die SQLite-Zeile bleibt der
  autoritative Laufzeit-Store; die Zusammenführung ist `T19-010`.

## Warnungen
- ⚠️ `Option<T>` überall macht den Zugriff im restlichen Code umständlich —
  deshalb die `Preferences`-Ableitung als Brücke. Nicht den ganzen Codebase
  jetzt auf `Option`-Zugriffe umstellen.
- ⚠️ **Keine Secrets in `SettingsContent`.** `hosts.entries` hält nur eine
  `credential_ref` auf den OS-Keychain — Passwörter/Keys werden nie
  serialisiert. Der heutige Host-`Credential`-Typ aus `labonair-backend` darf
  nicht 1:1 in den Baum wandern; nur die nicht-geheimen Felder + die Referenz.
- ⚠️ camelCase-serde-Keys **exakt** wie heute (`editorVimMode`→`vimMode` war
  laut Handshake schon eine Korrektur) — sonst laden bestehende Dateien nicht.
- ⚠️ `schemars`-Derive kann bei komplexen Enums zicken — früh `cargo check`
  mit der Schema-Ableitung, nicht erst in T19-006.

## Weiterführende Tasks
- [T19-002: `SettingsStore` + Layer-Merge](./T19-002-settings-store-layered-merge.md)
- [T19-006: JSON-Schema-Generierung](./T19-006-json-schema-generation.md)
