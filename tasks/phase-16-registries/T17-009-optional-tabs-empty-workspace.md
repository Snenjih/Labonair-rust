# T17-009: Tabs optional — Empty-Workspace-State + Host-Manager-Tab entfernen

## Status
📋 Geplant

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T17-004 (`PaneGroup` mit `Option`aler Wurzel), T17-006 (`AppShell` → reine Komposition)

## Ziel
Die Regel „es muss immer mindestens ein Tab offen sein" abschaffen. Der
Workspace darf **null** Tabs halten und zeigt dann eine leere Fläche
(Empty-Surface). Damit verbunden: `TabKind::Home` (heute der unschließbare
„Home"-Tab, der die Host-Manager-View rendert — `workspace.rs:3457`) wird
entfernt; der Host-Manager bleibt vorerst als **normaler, schließbarer**
on-demand-Tab erreichbar, bis T19-010 die Verwaltung in die Settings zieht.

Grundlage: [`docs/architecture.md §8.2`](../../docs/architecture.md).
Die visuelle Ausgestaltung der Empty-Surface + das `＋▾`-Menü macht T18-001 —
diese Task liefert die **Zustands-/Logik-Seite** und einen minimalen Platzhalter.

## Kontext
- Heute erzwingen „min. 1 Tab":
  - `crates/workspace/src/tabs.rs` — `close()` bricht bei `self.tabs.len() <= 1`
    ab (`:287`); `close_others` / `close_by_kind` schützen den letzten Tab;
    `TabKind::Home` ist nie schließbar (`:291`, Tests `:501`+).
  - `crates/workspace/src/workspace.rs` — beim Start / bei leerem Store wird
    sofort `TabKind::Home` geöffnet (`:544`, `:556`); `RestoreAction::Home`
    (`:702`); `TabKind::Home => self.host_manager.clone()` als Tab-Inhalt
    (`:3457`); `t.kind != TabKind::Home`-Filter an mehreren Stellen
    (`:1642`, `:1714`, `:3145`, `:3812`).
- `TabKind` (`tabs.rs:28`) hat die Variante `Home`; Icon/Label `Home` (`:52`,
  `:68`, `:83`).
- `startup_tab`-Pref (`labonair-settings-content` / heutige `Preferences`):
  aktuell `terminal` / `restore` o.ä. — bekommt den Wert `empty`.
- Session-Restore: `crates/workspace/src/session.rs` — Snapshot der Tabs +
  Layout; `TabSnapshot::Home` (`workspace.rs:575`).
- Host-Manager-View: seit T16-008 `labonair-hosts-ui` (`HostManagerView`),
  wird über hereingereichte `on_open_ssh`/`on_open_sftp`-Callbacks bedient.
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/workspace.rs` — Workspace
  ohne aktiven Tab (leerer `Pane`), `Workspace::render` mit leerem Zentrum.

## Anweisungen zur Umsetzung
1. **`TabKind::Home` streichen.** Variante + alle `match`-Arme
   (`icon`/`label`/`title`/`TabSnapshot::Home`/`RestoreAction::Home`/
   `t.kind != TabKind::Home`-Filter). `tabs.rs::close` verliert den
   `len() <= 1`-Abbruch und den `Home`-Sonderfall; `close_others` /
   `close_by_kind` / `close_all` dürfen bis **0** Tabs gehen.
2. **`Option<ActiveTab>`-Audit über `labonair-workspace`.** Jede Stelle, die
   `tabs[active]` / `active_tab().unwrap()` / „es gibt einen aktiven Tab"
   annimmt, auf `Option` umstellen:
   - `active_tab()` / `active_tab_id()` → `Option<…>`.
   - Aktionen ohne aktiven Tab (Split, Close-Pane, Find, Rename, „Run in
     terminal", CWD-Breadcrumb, Snippet-Ausführen-Ziel, …) sind ein sauberer
     No-op **oder** öffnen zuerst einen passenden Tab — pro Aktion im
     Doc-Kommentar festhalten, welche Semantik gilt.
   - `WorkspaceLiveBridge`-Snapshot bei 0 Tabs: leerer/`None`-Zustand, kein
     Panic; MCP-Bridge meldet „kein aktiver Tab".
3. **Empty-Surface (Platzhalter).** `Workspace::render` rendert bei
   `tabs.is_empty()` einen zentrierten, dezenten Platzhalter — vorerst nur ein
   Text-Hinweis („No tabs open · ⌘T new terminal · ⌘K commands"). Die
   endgültige Optik + Doppelklick→Terminal + Datei-Drop→Editor macht T18-001;
   hier einen `on_double_click`→`new_local_terminal_tab()`-Handler schon
   verdrahten (klein, damit die Fläche nicht „tot" ist).
4. **Startverhalten.**
   - `startup_tab` bekommt den Wert `empty` (Enum + Default-JSON + Migration:
     unbekannt→bisheriger Default).
   - `Workspace::new` / Bootstrap: **kein** automatischer `Home`-Tab mehr.
     Reihenfolge:
     1. `session_restore` aktiv **und** Snapshot vorhanden → Snapshot
        wiederherstellen (auch wenn er 0 Tabs hatte → leer bleiben).
     2. sonst `startup_tab`: `terminal` → ein lokales Terminal;
        `empty` → nichts öffnen; `restore` ohne Snapshot → `empty`.
   - Beim Beenden: den tatsächlichen Tab-Zustand snapshotten (0 Tabs ist ein
     gültiger Snapshot).
5. **Host-Manager als on-demand-Tab.** Neuer `TabKind::Hosts` (schließbar, ganz
   normal). `open_host_manager(cx)` (aus `app_shell.rs` / `menu.rs` /
   `CommandId::OpenHostManager`) öffnet/fokussiert diesen Tab statt des
   entfallenen `Home`-Tabs. Inhalt = `labonair_hosts_ui::HostManagerView` mit
   den `on_open_ssh`/`on_open_sftp`-Callbacks aus dem `Workspace`.
   **Hinweis:** T19-010 entfernt `TabKind::Hosts` wieder und ersetzt den
   Einstieg durch „Open Host Settings" — hier bewusst nur die Zwischenstufe,
   damit jede Phase eine lauffähige App hat.
6. **Tests** (`labonair-workspace`):
   - `close` schließt den letzten Tab → `tabs.is_empty()`, kein Panic.
   - `close_all` → 0 Tabs; `active_tab()` → `None`; `render` (Test-Harness)
     baut ohne Panic.
   - Alle tab-abhängigen Aktionen bei 0 Tabs: No-op/definierte Öffnung, kein
     `unwrap`-Panic (ein Sweep-Test, der jede öffentliche `Workspace`-Aktion
     bei leerem Zustand einmal aufruft).
   - `startup_tab = empty` → Start mit 0 Tabs; `= terminal` → 1 Terminal;
     Snapshot mit 0 Tabs → Start bleibt leer.
   - Session-Round-Trip: 0 Tabs speichern + laden.
7. `cargo run`: alle Tabs schließen → leere Fläche, App läuft weiter
   (Titlebar/Statusbar/Panels bedienbar); `⌘T` / Doppelklick → Terminal-Tab
   erscheint; Host-Manager über Menü/Palette-`OpenHostManager` öffnet einen
   normalen, schließbaren Tab; Neustart mit `startup_tab = empty` startet leer.

## Akzeptanzkriterien
- [ ] `TabKind::Home` existiert nicht mehr; kein `len() <= 1`-Abbruch in
      `tabs.rs`; `close_all` geht bis 0 Tabs.
- [ ] `labonair-workspace` kompiliert und läuft mit `active_tab(): Option<…>`;
      Sweep-Test „jede Workspace-Aktion bei 0 Tabs" ist grün (kein Panic).
- [ ] `Workspace::render` zeigt bei 0 Tabs eine Platzhalter-Fläche mit
      Doppelklick→lokales Terminal.
- [ ] `startup_tab` kennt `empty`; Startsequenz: Snapshot > `startup_tab`;
      `empty` öffnet nichts; 0-Tab-Snapshot bleibt leer.
- [ ] Host-Manager ist als normaler, schließbarer `TabKind::Hosts`-Tab über
      Menü + `CommandId::OpenHostManager` erreichbar (keine `Home`-Referenz
      mehr).
- [ ] Session-Persistenz: 0 Tabs überleben Speichern/Laden.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Das `Option`-Audit ist der Hauptaufwand (nicht das Feature). `workspace.rs`
  ist groß — systematisch nach `active` / `unwrap` / `expect` / `[.. ]`-Index
  auf Tab-/Pane-Collections greppen und je Fund entscheiden.
- Die Empty-Surface hier ist bewusst hässlich/minimal — T18-001 macht sie
  schön und hängt das `＋▾`-Menü + Datei-Drop dran. Nicht hier ausbauen.
- `RestoreAction::Home` / `TabSnapshot::Home` in alten Snapshots: beim Laden
  überspringen (als hätte der Slot nichts enthalten), nicht als Fehler werten.

## Warnungen
- ⚠️ Viele Nicht-Workspace-Crates (shell, panels, command-palette, live-bridge,
  MCP) rufen `active_tab*()`. Nach der Signatur-Änderung deren Call-Sites
  mit-anpassen — `cargo check --workspace` als Leitplanke, nicht nur
  `-p labonair-workspace`.
- ⚠️ MCP-Bridge / `WorkspaceLiveBridge`: „kein aktiver Tab" ist ein legitimer
  Zustand — die Bridge darf nicht blockieren oder pollen, sondern einen leeren
  Snapshot melden (T14/T11-005-Verhalten beibehalten, nur um den `None`-Fall
  erweitern).
- ⚠️ `zen_mode` / Fullscreen / Session-Snapshot-Timing (Reihenfolge aus
  `AppShell::new`, siehe T17-006-Warnung) nicht durcheinanderbringen.

## Weiterführende Tasks
- [T18-001: Titlebar-Redesign — Tabs + `＋▾`-Menü + Empty-Surface](../phase-17-layout/T18-001-titlebar-redesign.md)
- [T19-010: Settings › Hosts](../phase-18-settings-core/T19-010-hosts-settings-category.md)
