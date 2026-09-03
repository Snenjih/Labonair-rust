# T19-003: Projekt-/Ordner-Settings (`.labonair/settings.json`)

## Status
📋 Geplant

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
- [ ] `.labonair/settings.json` unter der aktiven Projekt-Wurzel wird als
      eigene Schicht über User gemerged; Wurzelwechsel lädt/entlädt korrekt.
- [ ] Live-Watch der Projekt-Datei; Debounce; keine Crashes bei kaputter Datei.
- [ ] Key-Whitelist: nur erlaubte Bereiche aus der Projekt-Schicht; verbotene
      Keys werden ignoriert und **einmal** sichtbar gemeldet.
- [ ] `SettingsStore::source_of(json_path)` liefert die Herkunfts-Schicht.
- [ ] Command „Projekt-Settings öffnen/erstellen" legt ein kommentiertes
      Gerüst an und öffnet es.
- [ ] Tests decken Merge-Vorrang, Whitelist-Ablehnung, Wurzelwechsel, Leerfall.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- v1 bewusst „eine aktive Projekt-Wurzel". Multi-Root (jeder Split sein
  eigenes Projekt) ist ein sinnvolles, aber separates Feature.
- Die Whitelist ist eher zu eng als zu weit anzusetzen — erweitern ist
  billig, ein Sicherheitsloch teuer.

## Warnungen
- ⚠️ Projekt-Settings sind **nicht vertrauenswürdig** (kommen mit dem Repo).
  Niemals daraus etwas Ausführbares, Netzwerk- oder Credential-relevantes
  ableiten. Die Whitelist ist Pflicht, nicht optional.
- ⚠️ `.labonair/` sollte in `.gitignore`-Vorschlägen NICHT stehen — der Sinn
  ist, dass es eingecheckt wird. Aber der Command darf kein `git add` machen.

## Weiterführende Tasks
- [T19-004: Settings-UI aus Modell generieren](./T19-004-generated-settings-ui.md)
- [T19-009: Settings-Migrator](./T19-009-settings-migrator.md)
