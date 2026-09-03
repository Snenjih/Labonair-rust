# T16-006: `labonair-workspace` extrahieren

## Status
✅ Done

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-002 (`labonair-ui-kit`), T16-005 (`labonair-panel` Contracts)

## Ziel
Den Workspace-Kern (`workspace.rs` + `pane.rs` + die Tab-Content-Views) aus
`crates/ui` in `labonair-workspace` lösen. `pane_group` (rekursiver Split-Baum)
wird dabei als eigenes Modul angelegt. Reiner Move; das Verhalten von Tabs,
Splits und Tab-Inhalten bleibt identisch. Die Dock-/Layer-Umbauten kommen erst
in Phase 16.

## Kontext
- Heute: `crates/ui/src/workspace.rs` (4 076 Z.) — `Workspace`-Entity: Tabs,
  aktiver Tab, Split-Panes, `run_in_active_terminal`,
  `inject_into_active_terminal`, `active_terminal_lines`, `active_cwd`,
  `set_git_graph`, `select_tab_by_index`, `active_is_terminal`,
  `active_has_split`, Session-Snapshot-Replay.
  `crates/ui/src/pane.rs` (377 Z.) — `SplitAxis`, Split-Container.
- Tab-Content-Views heute in `crates/ui/src/`: `terminal.rs` (Terminal-View),
  `editor.rs` (Editor-View), `sftp.rs`, `preview.rs` (Markdown),
  `ssh_connection.rs`, `git_graph.rs` (als Tab), `diff.rs`.
- Konsumenten: `app_shell.rs` (hält `Entity<Workspace>`, ruft ~15 Methoden),
  `live_bridge.rs` (`WorkspaceLiveBridge` liest Workspace-Snapshot),
  `session.rs` (Snapshot load/save).
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/{workspace.rs,pane.rs,
  pane_group.rs,item.rs,persistence.rs}`.

## Anweisungen zur Umsetzung
1. **`crates/workspace/` anlegen** (`labonair-workspace`,
   `src/workspace.rs` Lib-Root).
2. **Verschieben** (`git mv`, Modulpfade anpassen):
   - `workspace.rs` → `src/workspace.rs`
   - `pane.rs` → `src/pane.rs`
   - `src/pane_group.rs` **neu**: den Split-Container-Teil aus `pane.rs`
     herauslösen (rekursiver Baum), `pane.rs` behält die einzelne Pane.
     Wenn `pane.rs` heute schon nur den Baum enthält, Datei umbenennen und
     eine schlanke `pane.rs` (Einzel-Pane) einführen — nur so weit nötig, um
     T17-004 vorzubereiten; kein Voll-Refactor.
   - Tab-Content-Views: `terminal.rs`, `editor.rs`, `sftp.rs`, `preview.rs`,
     `ssh_connection.rs`, `git_graph.rs`, `diff.rs` → `src/views/*.rs`.
     (`git_graph` bleibt vorerst hier als Tab-View; die *Panel*-Variante ist
     Sache von T16-008.)
3. **`src/session.rs`**: `crates/ui/src/session.rs` mitnehmen (Snapshot
   load/save gehört zum Workspace-Zustand).
4. **`src/live_bridge.rs`**: `WorkspaceLiveBridge` mitnehmen — er liest den
   Workspace-Snapshot; die AI-seitige Nutzung bleibt in `crates/ai` bzw.
   `panel-ai` und importiert von hier.
5. Dependencies: `gpui`, `labonair-ui-kit`, `labonair-gpui-ext`,
   `labonair-theme`, `labonair-panel`, `labonair-terminal`, `labonair-editor`,
   `labonair-backend`, `labonair-notifications`. **Kein** `labonair-ui`,
   **kein** `labonair-shell`.
6. Workspace-`Cargo.toml`: Member + Dep-Eintrag.
7. `crates/ui`: `mod workspace; mod pane; mod session; mod live_bridge;` +
   die Tab-View-Module raus; `crate::workspace::` / `crate::pane::` /
   `crate::session::` / `crate::live_bridge::` → `labonair_workspace::…` in
   `app_shell.rs`.
8. `cargo run`: Tabs öffnen/schließen/wechseln/splitten; Terminal-, Editor-,
   SFTP-, Preview-Tabs rendern; Session-Restore beim Neustart; Git-Graph-Tab.

## Akzeptanzkriterien
- [ ] `crates/workspace/` ist Member; `crates/ui` enthält weder `workspace.rs`
      noch `pane.rs` noch die Tab-View-Dateien.
- [ ] `src/pane_group.rs` existiert und enthält den rekursiven Split-Baum;
      `src/pane.rs` die Einzel-Pane (Aufteilung minimal, T17-004 baut darauf).
- [ ] `cargo tree -p labonair-workspace` zeigt keine Kante zu `labonair-ui`.
- [ ] `cargo run`: alle Tab-Typen + Split-Layout + Session-Restore verhalten
      sich identisch zu vor der Task (manuelle Prüfung).
- [ ] `WorkspaceLiveBridge` funktioniert weiter (AI-Agent-Terminal-Tools,
      relative Pfade) — Smoke-Test laut `handshake.md`-Beschreibung.
- [ ] Bestehende Workspace-/Pane-/Session-/LiveBridge-Tests laufen im neuen
      Crate.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Größter Einzel-Move der Phase. Wenn er zu groß wird: erst `workspace.rs` +
  `pane.rs` + `pane_group.rs` verschieben und grün machen, in einem zweiten
  Commit die Tab-Views — beides innerhalb dieser Task.
- `Dock`/`ModalLayer`/`ToastLayer` entstehen **nicht** hier — nur die
  Datei-Heimat wird geschaffen (Phase 16 füllt sie).

## Warnungen
- ⚠️ `Workspace::new` bekommt heute viele Argumente (`registry`, `theme`,
  `background`, `backend`, `tokio`, `agent_access`, `session_snapshot`,
  `window`, `cx`). Signatur **nicht** ändern — nur den Crate wechseln.
- ⚠️ Zyklus-Falle: `git_graph.rs` importiert evtl. `crate::git::` (Panel-Code).
  Falls ja: den gemeinsamen Git-Backend-Zugriff über `labonair-backend`
  führen, nicht über den künftigen `panel-scm`-Crate.

## Weiterführende Tasks
- [T16-007: `labonair-settings-ui` extrahieren](./T16-007-extract-settings-ui-crate.md)
- [T16-008: Panel-Crates ausgliedern](./T16-008-split-panel-crates.md)
