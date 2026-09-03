# T19-001: `labonair-settings-content` — typisierter Settings-Baum + `MergeFrom`

## Status
📋 Geplant

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T16-001 (ADR & Ziel-Crate-Graph), T16-007 (`labonair-settings-ui` extrahiert)

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
   - `connections: ConnectionsContent` (SSH/Explorer/Host-Availability +
     die im `subagent-2.md` als fehlend markierten Felder — jetzt modellieren)
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
7. **Tests**: `defaults()` == geparste `default.json`; `MergeFrom` (user über
   default, projekt über user); `parse` mit einem kaputten Feld ⇒ Default +
   gemeldeter Fehler, restliche Felder intakt; `SettingsContent → Preferences`
   Round-Trip der Default-Werte.

## Akzeptanzkriterien
- [ ] `crates/settings-content/` + `crates/settings-macros/` existieren,
      Workspace-Members, ohne GPUI-/UI-/backend-Deps.
- [ ] `SettingsContent` deckt alle heutigen `Preferences`-Felder ab **plus**
      die in `subagent-2.md` als fehlend markierten (Connections, Bookmarks,
      Statusbar-Toggles) — Feldnamen camelCase-serde wie bisher.
- [ ] `MergeFrom` + `#[derive(MergeFrom)]` funktionieren; Schicht-Merge
      getestet.
- [ ] `SettingsContent::defaults()` und `assets/settings/default.json` sind
      inhaltsgleich (Test erzwingt das); `default.json` ist kommentiert.
- [ ] `parse` ist feld-fehlertolerant (Test).
- [ ] `impl From<&SettingsContent> for Preferences` erlaubt dem bestehenden
      Code, unverändert weiterzulaufen.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Diese Task ändert **noch nicht** das Laufzeitverhalten — sie baut nur das
  Modell + die `default.json`. T19-002 schaltet den Store darauf um.
- `zed-refrence/zed/crates/settings_content/` ist fast 1:1 die Vorlage — dort
  die Struktur der Submodule + `merge_from.rs` genau lesen und übernehmen,
  Zed-spezifische Bereiche (collab, dap, vim-tiefe) weglassen.

## Warnungen
- ⚠️ `Option<T>` überall macht den Zugriff im restlichen Code umständlich —
  deshalb die `Preferences`-Ableitung als Brücke. Nicht den ganzen Codebase
  jetzt auf `Option`-Zugriffe umstellen.
- ⚠️ camelCase-serde-Keys **exakt** wie heute (`editorVimMode`→`vimMode` war
  laut Handshake schon eine Korrektur) — sonst laden bestehende Dateien nicht.
- ⚠️ `schemars`-Derive kann bei komplexen Enums zicken — früh `cargo check`
  mit der Schema-Ableitung, nicht erst in T19-006.

## Weiterführende Tasks
- [T19-002: `SettingsStore` + Layer-Merge](./T19-002-settings-store-layered-merge.md)
- [T19-006: JSON-Schema-Generierung](./T19-006-json-schema-generation.md)
