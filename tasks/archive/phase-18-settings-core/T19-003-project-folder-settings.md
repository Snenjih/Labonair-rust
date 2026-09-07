# T19-003: Projekt-/Ordner-Settings (`.labonair/settings.json`)

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-002 (`SettingsStore` + Layer-Merge)

## Ziel
Pro geöffnetem Arbeitsverzeichnis eine optionale `.labonair/settings.json`, die
als eigene Schicht **über** den User-Settings gemerged wird. Use-Case: „für
dieses Repo standardmäßig SSH-Host X, Startlayout Y, dieses Snippet-Set,
diese AI-Directives".

## Kontext
- T19-002: `SettingsStore` mit `SettingsLayer::Project(WorktreeId)` (bislang
  leer). `recompute` merged Default → User → … → Project → Language.
- „Arbeitsverzeichnis" im Port: die aktive CWD / der Explorer-Root
  (`Workspace::active_cwd`, `ExplorerView`-Root). Es gibt kein formales
  „Worktree"-Konzept wie in Zed — hier reicht „das Verzeichnis, das der
  Explorer/der aktive Terminal-Tab gerade als Root hat".
- Zed-Vorbild:
  `zed-refrence/zed/crates/settings/src/settings_store.rs` — `LocalSettingsKind`,
  `SettingsLocation`, `set_local_settings(worktree_id, path, kind, content)`,
  `local_settings_file_relative_path` (`.zed/settings.json`).
  `zed-refrence/zed/crates/project/` — wie Projekt-Settings-Dateien beim
  Öffnen/Schließen von Worktrees registriert werden.

## Anweisungen zur Umsetzung
1. **Projekt-Wurzel-Begriff**: `SettingsStore` bekommt
   `set_active_project_root(Option<PathBuf>)`. Quelle: `Workspace` ruft es bei
   Änderung des Explorer-Roots / der aktiven CWD (ein `cx.observe` auf den
   Explorer bzw. ein Event). Mehrere gleichzeitig offene Roots (Splits mit
   verschiedenen CWDs): **v1 = genau eine aktive Projekt-Wurzel** (die des
   aktiven Panes); Multi-Root als späteres Ticket notieren.
2. **Datei laden**: bei gesetzter Wurzel `<root>/.labonair/settings.json`
   lesen (falls vorhanden), `parse` (feld-fehlertolerant), als
   `SettingsLayer::Project` setzen, `recompute`. Wurzel weg / Datei weg →
   Layer leeren.
3. **Live-Watch**: die Projekt-Datei ebenfalls beobachten (Debounce wie
   User-Datei). Wurzelwechsel → alten Watch abmelden, neuen anmelden.
4. **Sicherheits-/Scope-Grenzen** (wichtig — Projekt-Dateien sind
   fremd-beeinflussbar, z.B. geklontes Repo):
   - **Whitelist** erlaubter Keys für die Projekt-Schicht. Erlaubt: Layout/
     Startverhalten, Default-SSH-Host (nur Referenz auf einen bereits
     gespeicherten Host, **kein** Credential), Snippet-Set-Auswahl,
     AI-Directives-Datei-Verweis, Editor-Format-Optionen.
   - **Verboten** (werden beim Merge ignoriert + einmal geloggt/getoastet):
     alles Sicherheitsrelevante — `credential_encryption`, MCP-Bridge-Port/
     -Enable, Auto-Revoke, Auto-Update-Quelle, beliebige Pfade zu ausführbaren
     Hooks, Keymap. Die Whitelist in `labonair-settings` als Konstante +
     Doc-Kommentar mit Begründung.
   - Zed hat für Ähnliches `GrantedWritePath` — Konzept ansehen
     (`zed-refrence/zed/crates/settings/src/granted_write_path.rs`).
5. **Anzeige der Herkunft**: In der Settings-UI (T19-004) pro Feld anzeigen,
   aus welcher Schicht der effektive Wert stammt („aus Projekt: .labonair/
   settings.json"). Hier mindestens die API `SettingsStore::source_of(json_path)
   -> SettingsLayer` bereitstellen.
6. **Erstellen-Helfer**: Command „Projekt-Settings öffnen/erstellen" → legt
   `<root>/.labonair/settings.json` mit einem kommentierten Gerüst an
   (`assets/settings/initial_project_settings.json`, Vorbild
   `zed/assets/settings/initial_local_settings.json`) und öffnet sie im Editor.
7. **Tests**: Projekt-Layer schlägt User-Layer für einen erlaubten Key;
   verbotener Key im Projekt-File wird ignoriert + gemeldet; Wurzelwechsel
   lädt/entlädt korrekt; keine Projekt-Datei ⇒ genau User-Verhalten.
8. `cargo run`: In einem Ordner `.labonair/settings.json` mit
   `{"general":{"defaultStartupTab":"terminal"}}` anlegen → beim Öffnen dieses
   Ordners greift es; in einem anderen Ordner nicht; Settings-UI zeigt die
   Herkunft; ein verbotener Key (`mcp.bridgePort`) wird ignoriert + getoastet.

## Akzeptanzkriterien
- [x] `.labonair/settings.json` unter der aktiven Projekt-Wurzel wird als
      eigene Schicht über User gemerged; Wurzelwechsel lädt/entlädt korrekt.
- [x] Live-Watch der Projekt-Datei; Debounce; keine Crashes bei kaputter Datei.
- [x] Key-Whitelist: nur erlaubte Bereiche aus der Projekt-Schicht; verbotene
      Keys werden ignoriert und **einmal** sichtbar gemeldet.
- [x] `SettingsStore::source_of(json_path)` liefert die Herkunfts-Schicht.
- [x] Command „Projekt-Settings öffnen/erstellen" legt ein kommentiertes
      Gerüst an und öffnet es.
- [x] Tests decken Merge-Vorrang, Whitelist-Ablehnung, Wurzelwechsel, Leerfall.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- v1 bewusst „eine aktive Projekt-Wurzel". Multi-Root (jeder Split sein
  eigenes Projekt) ist ein sinnvolles, aber separates Feature.
- Die Whitelist ist eher zu eng als zu weit anzusetzen — erweitern ist
  billig, ein Sicherheitsloch teuer.

**Umsetzung (2026-09-04):** `crates/settings/src/project.rs` (neu) —
`PROJECT_SETTINGS_WHITELIST` (nur `general`/`workspace`/`editor`-Bereiche,
je eine explizite Leaf-Liste; `hosts`/`ai`/`mcp`/`connections`/`keymap`/
`appearance`/`file_manager`/`personalization` komplett ausgeschlossen —
`hosts`/`ai` bewusst, weil noch kein sicheres skalares "Referenz auf
Host/Directives-Datei"-Feld existiert; das ist eine spätere, engere
Whitelist-Erweiterung, keine Blockade dieser Task), `filter_and_parse`
(whitelist-filtert dann parst über `labonair_settings_content::parse`),
`ensure_project_settings_file` (+ `assets/settings/initial_project_settings.json`
Gerüst). `SettingsStore` (`store.rs`) bekommt `current_project`/
`next_worktree_id`/`project_watch_generation`/`project_rejected`,
`set_active_project_root`/`reload_project_layer`/`rewatch_project`/
`source_of`. Live-Watch: `watch::spawn_project` — generation-basiert (kein
Cancel-Handle über die `cx.spawn`-Grenze nötig; ein Root-Wechsel bumpt die
Generation, der alte Poll-Loop erkennt das und beendet sich selbst). Crate-
Root-Wrapper `set_active_project_root(cx, root)` / `refresh_project_watch(cx)`
in `settings.rs`, weil `labonair-settings` laut `scripts/check_crate_deps.py`
ein reines Leaf-Crate bleiben muss (kein `labonair-workspace`-Dep) — die
Anbindung an "aktive Pane-CWD" sitzt daher in `labonair-workspace`:
`Workspace::sync_project_settings_root` (aufgerufen einmal pro `render`,
no-op wenn CWD unverändert) + `Workspace::open_or_create_project_settings`
(Command, öffnet die Datei über das bestehende `open_file`/Editor-Tab).
Befehl als `CommandId::OpenProjectSettings` in der Command-Palette
(`crates/command-palette/src/palette.rs`) + `crates/shell/src/commands.rs`
registriert (keine Tastenkombination, wie bei den meisten
Application-Commands). **Nicht mit `cargo run` verifiziert** (headless
VPS — kein GUI-Fenster; wie bei mehreren vorherigen Titlebar/Overlay-Tasks
in `docs/architecture.md` §8.13/§8.14 vermerkt). Settings-UI-Anzeige der
Herkunft (`source_of`) ist explizit T19-004's Aufgabe — hier nur die API.

## Warnungen
- ⚠️ Projekt-Settings sind **nicht vertrauenswürdig** (kommen mit dem Repo).
  Niemals daraus etwas Ausführbares, Netzwerk- oder Credential-relevantes
  ableiten. Die Whitelist ist Pflicht, nicht optional.
- ⚠️ `.labonair/` sollte in `.gitignore`-Vorschlägen NICHT stehen — der Sinn
  ist, dass es eingecheckt wird. Aber der Command darf kein `git add` machen.

## Weiterführende Tasks
- [T19-004: Settings-UI aus Modell generieren](./T19-004-generated-settings-ui.md)
- [T19-009: Settings-Migrator](./T19-009-settings-migrator.md)
