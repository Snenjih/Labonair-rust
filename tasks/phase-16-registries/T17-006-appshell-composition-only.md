# T17-006: `AppShell` → reine Komposition

## Status
📋 Geplant

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T17-002 (`Dock`-Modell), T17-003 (`StatusItemRegistry`), T17-004 (`PaneGroup`), T17-005 (`ModalLayer` + `ToastLayer`)

## Ziel
`AppShell` von einem ~3 000-Zeilen-God-Object auf eine dünne Kompositions-Wurzel
schrumpfen: Titlebar + Workspace (mit Docks + PaneGroup) + Statusbar +
ModalLayer + ToastLayer — mehr nicht. Kein Feature-Code, keine `drain_pending_*`,
kein manuelles `observe`-Boilerplate für ein Dutzend Entities.

## Kontext
- Heute: `crates/shell/src/app_shell.rs` (2 983 Z.) — `struct AppShell` mit
  ~30 Feldern; `AppShell::new` mit ~10 `cx.observe(&x, |_,_,cx| cx.notify())
  .detach()`; `render` beginnt mit `drain_pending_commands/bookmarks/ai` +
  `sync_live_bridge` + `build_palette_data`; danach `render_header`,
  `render_sidebar` (×2), `render_statusbar`, plus ~50 `.on_action(...)`.
- Nach T17-001..005: Panels/StatusItems/Modals/Toasts/Docks/PaneGroup sind
  jeweils eigene, self-registrierende Systeme im `Workspace`.
- `WorkspaceLiveBridge` (`sync_live_bridge`) — der Snapshot wird heute pro
  Frame aus Shell + Explorer neu geschrieben.
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/workspace.rs`
  (`Workspace::render` — komponiert `titlebar`, `Dock`s, `pane_group`,
  `status_bar`, `modal_layer`, `toast_layer`) + `zed-refrence/zed/crates/
  title_bar/`.

## Anweisungen zur Umsetzung
1. **Zielzustand `AppShell`** (Felder):
   `workspace: Entity<Workspace>`, `titlebar: Entity<Titlebar>`
   (T18-001 baut den Inhalt), `theme`, `background`, `focus_handle`,
   `window_geometry_saver`. **Nicht mehr**: `explorer`, `bookmarks`,
   `git_panel`, `snippets`, `ai_chat`, `command_palette`, `updater`,
   `agent_access`, `notifications`, `placements`, `bar_menu`, `crumb_menu`,
   `subdir_menu`, `left_slot`, `right_slot`, `pending_*`, `live_bridge`,
   `search_*` — die leben ab jetzt in `Workspace` / Registries / Panels.
2. **`AppShell::new`**: erzeugt `Workspace` (das intern
   `PanelRegistry`/`StatusItemRegistry`/`Dock`s/`ModalLayer`/`ToastLayer`
   aufbaut und `register_builtin_panels` + `register_builtin_status_items`
   aufruft). Ein einziges `cx.observe(&workspace, |_,_,cx| cx.notify())`.
   Startup-Sequenz (MCP-Prefs-Hydrate, Session-Snapshot, Theme-Pref anwenden,
   `apply_prefs_to_theme`, `apply_keybinds`, `set_settings_deps`, Updater-
   Check) in eine `bootstrap(workspace, backend, cx)`-Funktion auslagern —
   entweder in `labonair-shell` oder in `Workspace::new`.
3. **`render`**:
   ```
   div().flex().flex_col().size_full()
     .child(titlebar)                    // Tabs + 1 Button (T18-001)
     .child(workspace.clone())           // Docks + PaneGroup + Bottom-Dock
     .child(statusbar via workspace)     // aus StatusItemRegistry (T17-003)
     .child(workspace.modal_layer())
     .child(workspace.toast_layer())
   ```
   Keine `drain_*`, kein `build_palette_data` hier (die Palette baut ihre
   Daten selbst / über den `CommandRegistry`, T17-007).
4. **`drain_pending_ai` + `sync_live_bridge` entfernen**:
   - AI-Run-in-Terminal-Events: `Workspace` subskribiert den AI-Panel direkt
     (`cx.subscribe_in`) und ruft `run_in_active_terminal`.
   - `WorkspaceLiveBridge`-Snapshot: nicht mehr pro Frame, sondern
     event-getrieben — bei Tab-Wechsel / CWD-Änderung / Explorer-Root-Änderung
     via `cx.observe`/`cx.subscribe`. Die `WorkspaceLiveBridge`-API bleibt,
     nur der Aktualisierungs-Trigger ändert sich.
5. **Actions**: die `.on_action`-Kette wandert in T17-007 in den
   `CommandRegistry`. In dieser Task nur die, die reine Fenster-Aktionen sind
   (minimize/zoom/fullscreen), am Shell-`div` lassen; der Rest wird in T17-007
   registriert. Wenn T17-007 vorgezogen wird, hier direkt komplett.
6. **Zeilenbudget**: `app_shell.rs` < ~400 Z. nach der Task (Titlebar-Inhalt
   ist eigene Datei, T18-001).
7. **Leerer Workspace**: `render` muss auch mit 0 Tabs / `PaneGroup::root ==
   None` sauber komponieren (Titlebar + leerer Workspace-Bereich + Statusbar +
   Layer). Kein `unwrap` auf einen aktiven Tab/Pane im Shell-`render`. Die
   eigentliche Empty-Surface (Hinweis, Doppelklick→Terminal) baut T18-001; das
   `Option`-Audit über `Workspace` macht T17-009. Hier nur: Shell rendert
   nicht kaputt, wenn nichts offen ist.
8. `cargo run`: vollständige App, End-to-End identisch — nur der Shell-Code ist
   klein; zusätzlich einmal alle Tabs schließen → App bleibt stehen (leerer
   Mittelbereich, Titlebar/Statusbar da).

## Akzeptanzkriterien
- [ ] `struct AppShell` hat ≤ 8 Felder; `AppShell::new` hat genau ein
      `cx.observe`.
- [ ] `app_shell.rs` < ~400 Zeilen.
- [ ] `drain_pending_commands`, `drain_pending_bookmarks`, `drain_pending_ai`,
      `sync_live_bridge` (als Per-Frame-Aufruf) existieren nicht mehr;
      `pending_*`-Felder sind entfernt.
- [ ] `render` komponiert exakt: Titlebar, Workspace, Statusbar, ModalLayer,
      ToastLayer — keine weiteren Kinder.
- [ ] `WorkspaceLiveBridge`-Snapshot wird event-getrieben aktualisiert (Test:
      Tab-Wechsel/CWD-Änderung aktualisiert den Snapshot; kein Update ohne
      Änderung).
- [ ] `cargo run`: End-to-End-Sichtprüfung — Tabs, Docks, alle Panels,
      Statusbar-Items, Command-Palette, Toasts, Settings-Fenster, native
      Menüs, Session-Restore — identisch zu vor der Phase.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Das ist die „Zahltag"-Task der Phase 16: hier wird der God-Object-Schmerz
  konkret aufgelöst. Wenn einzelne Zuständigkeiten unklar sind → in
  `docs/architecture.md` (T16-001) nachsehen, nicht neu erfinden.
- `Titlebar` als eigene Entity anlegen (leerer Platzhalter, der Tabs +
  einen Dummy-Button zeigt) — T18-001 füllt ihn.

## Warnungen
- ⚠️ Reihenfolge der Startup-Effekte hat Bedeutung (MCP-Port vor
  MCP-enable; Theme-Pref vor erstem Render; Session-Snapshot vor
  Default-Tabs). Die bestehende Reihenfolge aus `AppShell::new`
  (`app_shell.rs:211`+) beim Auslagern **beibehalten**.
- ⚠️ `maybe_persist_geometry(window)` (Fenster-Größe/Position throtteln) muss
  weiter pro Render laufen — das ist legitim, nicht mit den `drain_*`
  verwechseln.
- ⚠️ Nicht mehr Verhalten ändern als nötig — diese Task ist Struktur, kein
  Feature.

## Weiterführende Tasks
- [T17-007: `CommandRegistry`](./T17-007-command-registry.md)
- [T18-001: Titlebar-Redesign](../phase-17-layout/T18-001-titlebar-redesign.md)
