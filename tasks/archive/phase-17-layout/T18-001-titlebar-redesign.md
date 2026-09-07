# T18-001: Titlebar-Redesign — nur Tabs + ein Icon-Button

## Status
✅ Done

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T17-006 (`AppShell` → reine Komposition), T17-009 (Tabs optional / Empty-State)

## Ziel
Die Titlebar (Header) nach dem Layout-Vertrag neu bauen: **links die
Tab-Leiste + ein `＋▾`-Neuer-Tab-Menü-Button**, **rechts genau ein Icon-Button**,
der ein Dropdown öffnet (`Settings…`, `Profile` als Platzhalter, erweiterbar).
Alles andere aus dem heutigen Header verschwindet. Zusätzlich: die **Empty-Surface**
(sichtbar, wenn 0 Tabs offen sind — Logik kommt aus T17-009) bekommt hier ihre
endgültige Optik inkl. **Doppelklick → lokales Terminal** und
**Datei-Drop → Editor-Tab**. Siehe `docs/architecture.md §4` + `§8.2`.

## Kontext
- Heute: `crates/shell/src/app_shell.rs` — `render_header` (Zeile ~1334):
  `HEADER_H = 40.0`, `TRAFFIC_LIGHT_INSET = 78.0`, Sidebar-Toggle, App-Titel,
  Inline-Suche (`render_search`, `search_open`/`search_query`/`search_focus`),
  `render_app_menu` (`⋯`-Dropdown, `app_menu_open`), `render_agent_badge`,
  diverse Bar-Items (nach T17-003 schon in die Statusbar gewandert).
- Tab-Leiste: heute in der Workspace-Ansicht bzw. `tabs.rs` (nach T16-009
  ggf. schon in `labonair-shell`).
- Fenster-Chrome: `main.rs` `TitlebarOptions { appears_transparent: true,
  traffic_light_position }`.
- `reference-src/src/modules/header/` — Ursprungs-Header (wird bewusst
  vereinfacht, **nicht** 1:1 nachgebaut).
- `reference-src/src/modules/tabs/lib/tabUtils.tsx` — `NewTabDropdownItems`
  (das `＋`-Dropdown der Referenz: Terminal / Editor / Preview / Git Graph ·
  Trenner · `SSH ▸` Recent-Hosts · `SFTP ▸` Recent-Hosts · „All hosts…").
- Zed-Vorbild: `zed-refrence/zed/crates/title_bar/` — dünne Titlebar,
  `platform_title_bar` für Drag-Region + Traffic-Lights.

## Anweisungen zur Umsetzung
1. **`crates/shell/src/titlebar.rs`** — `struct Titlebar` (Entity):
   - Höhe `HEADER_H` (40), transparente Overlay-Titlebar, Drag-Region
     (`window.on_move`/`-drag` für Fenster-Move außerhalb der interaktiven
     Elemente).
   - **Links**: `TRAFFIC_LIGHT_INSET` freilassen (macOS-Ampeln), dann die
     **Tab-Leiste** (`workspace.tab_bar()` / das migrierte `tabs.rs`-Widget),
     gefolgt vom **`＋▾`-Menü-Button** am Ende des Tab-Strips (kein separater
     Bereich — er gehört zur Tab-Leiste, daher zählt er *nicht* als
     „zweiter Button" gegen den Layout-Vertrag). Tabs übernehmen die
     verbleibende Breite mit Überlauf-Scroll; der `＋▾`-Button bleibt sichtbar.
   - **`＋▾`-Dropdown** (`PopoverMenu` / `context_menu`-Primitive), Port von
     `NewTabDropdownItems`:
     - `Terminal` (`⌘T`), `Editor` (`⌘E`), `Preview` (`⌘P`), `Git Graph`
     - Trenner
     - `SSH ▸` — Submenü: die zuletzt verbundenen Hosts (Limit ~5) →
       `workspace.open_ssh_tab(host)`; darunter „Alle Hosts…" →
       `open_settings_window(Some("hosts"))` (bis T19-010 da ist:
       `open_host_manager`).
     - `SFTP ▸` — dasselbe für `workspace.open_sftp_tab(host)`.
     - Host-Liste kommt als injizierte Daten vom `Workspace` (`known_hosts`),
       **keine** Titlebar→`hosts-ui`-Kante.
   - Klick auf das leere `＋` (ohne Dropdown-Pfeil) = „neues lokales Terminal"
     als Schnellaktion; der `▾`-Teil öffnet das Menü. (Falls das visuell
     fummelig wird: ganzer Button = Menü, `⌘T` bleibt die Schnellaktion.)
   - **Rechts**: **ein** `IconButton` (`IconName::…` — z.B. `Settings2` oder
     ein Avatar/`CircleUser`-Glyph). Klick → `PopoverMenu` (ui-kit-Primitive,
     bis T20-001 fertig ist: `labonair_command_palette`/`context_menu`
     wiederverwenden) mit:
     - `Settings…` → `open_settings_window(None)`
     - `Profile` → Platzhalter (Menüpunkt vorhanden, öffnet vorerst einen
       kleinen „Coming soon"-Toast oder ein leeres Modal). In Doc-Kommentar:
       „Platzhalter — künftige Account-/Profil-Features hängen sich hier ein."
     - Trenner + Platzhalter-Kommentar für weitere Einträge.
   - **Kein** Titel-Text, **keine** Suche, **kein** `⋯`-Menü, **keine**
     Bar-Items, **kein** Sidebar-Toggle (Panel-Toggles sind jetzt Statusbar,
     T18-003).
2. **Entfernen** aus `app_shell.rs`: `render_header`, `render_app_menu`,
   `render_search`, `render_agent_badge` (Agent-Access ist ein Statusbar-Item
   seit T17-003), `search_open`/`search_query`/`search_focus`/`app_menu_open`/
   `agent_badge_open`-Felder, `HEADER_H` bleibt (nach `titlebar.rs`).
   `act_find` bindet ab jetzt an das Such-Overlay (T18-002) — hier als TODO
   markieren, wenn T18-002 noch nicht da ist, provisorisch die alte Suche als
   Overlay behalten.
3. **`AppShell::render`**: erstes Kind ist `self.titlebar.clone()`.
4. **`zen_mode_show_header`**-Pref: bleibt — steuert, ob die Titlebar
   gerendert wird (dann übernimmt der OS-Fensterrahmen; auf macOS mit
   `appears_transparent` ggf. Sonderfall — Verhalten dokumentieren).
5. **Native macOS-Menüleiste** (`menu.rs`): unverändert erhalten — der
   Titlebar-Dropdown ist der plattformübergreifende Zweitweg, kein Ersatz.
6. **Empty-Surface** (`Workspace::render` bei `tabs.is_empty()` — die Logik +
   der Platzhalter kommen aus T17-009, hier die endgültige Optik):
   - Zentriert, dezent: App-Glyph/Wortmarke klein, darunter eine kurze
     Shortcut-Liste als `KeybindingHint`-Zeilen — `⌘T` Neues Terminal,
     `⌘E` Editor, `⌘K` Befehle, `⌘,` Einstellungen, `⌘⇧N` Hosts.
     Farben/Spacing aus `globals.css` (muted-foreground, gap-Tokens).
   - **Doppelklick** irgendwo auf die Fläche → `workspace.new_local_terminal_tab()`.
   - **Datei-Drop** auf die Fläche → Editor-Tab für die Datei
     (`workspace.open_path_in_editor(path)`); mehrere Dateien → mehrere Tabs.
   - Reagiert auf Theme-Wechsel; kein Eigen-State, rein aus `Workspace`.
7. `cargo run` (macOS): Ampeln korrekt positioniert; Tab-Leiste + `＋▾`-Button
   füllen die Titlebar; ein Button rechts; `＋▾`-Menü mit Terminal/Editor/
   Preview/Git-Graph + SSH/SFTP-Submenüs (Recent-Hosts); Dropdown rechts mit
   `Settings…` + `Profile`; Fenster-Drag an leeren Stellen; Doppelklick auf
   Titlebar = zoom. Alle Tabs schließen → Empty-Surface mit Shortcut-Hints;
   Doppelklick darauf → Terminal-Tab; Datei drauf ziehen → Editor-Tab.

## Akzeptanzkriterien
- [x] `crates/shell/src/titlebar.rs` existiert; `render_app_menu` (und der `⋯`
      App-Menu-Zweig) entfernt. `render_header`/`render_search`/`render_agent_badge`
      waren nach T17-006 nie in `app_shell.rs` — sie lagen bereits in
      `titlebar.rs`; `render_agent_badge` existierte gar nicht mehr. `render_search`
      bleibt als **provisorisches Float-Overlay** bis T18-002 (Anweisung 2).
- [x] Die Titlebar zeigt **ausschließlich** Tab-Leiste + `＋`-Menü-Button
      (links, Teil des Tab-Strips) + einen Icon-Button rechts.
- [x] `＋`-Menü: Terminal/Editor/Preview/Git-Graph, Trenner, `SSH` / `SFTP`
      mit Recent-Hosts + „Alle Hosts…" (→ `open_host_manager`). Host-Liste als
      injizierte Daten (`Workspace::recent_hosts`), keine `hosts-ui`-Kante.
      (Bereits in `render_tab_bar`/`render_new_tab_menu` seit T17-009-Vorarbeit.)
- [x] Der rechte Button (`IconName::Ellipsis`) öffnet ein Dropdown mit
      `Settings…` (funktional) und `Profile` (Platzhalter → „Coming soon"-Toast),
      Trenner + Doc-Kommentar für weitere Einträge.
- [~] Ampel-Inset (`#[cfg]`-Split 78/8 px); Fenster-Drag an leeren Stellen
      (`WindowControlArea::Drag` + `start_window_move`); Doppelklick-Zoom
      (`titlebar_double_click`). **Code umgesetzt — visuell nicht prüfbar
      (headless VPS).**
- [~] Empty-Surface: erscheint bei 0 Tabs (T17-009-Logik), zeigt Shortcut-Hints,
      Doppelklick → `new_terminal_tab`, Datei-Drop (`ExternalPaths`) → ein
      `open_file` je Datei. **Visuelle Optik nicht prüfbar (headless).**
- [x] Inline-Suche ist kein Inline-Kind der Titlebar mehr (schwebendes
      Provisorium bis T18-002).
- [x] Native macOS-Menüleiste unverändert (`menu.rs` nicht angefasst).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `scripts/check-crate-deps.sh`. `cargo test --workspace` **nicht ausführbar**
      (Test-Binaries linken auf diesem headless VPS nicht — `cargo check/clippy
      --all-targets` als projektakzeptierter Ersatz).

## Abweichungen (T18-001)
- Entfernungsziele lagen nach T17-006 in `titlebar.rs`, nicht `app_shell.rs`.
- Rechter Button: `IconName::Ellipsis` statt `Settings2`/`CircleUser` (kein
  solches Glyph im Bundle; Task erlaubt Fallback).
- Rechtes Dropdown: handgebaut (`absolute` unter dem Button) statt `context_menu`
  — dessen Vollbild-Overlay rendert nicht aus dem 40 px-Titlebar-Container.
- Kein separater `▾`-Split-Button: ganzer `＋` = Menü, `⌘T` = Schnellaktion
  (Task erlaubt das explizit).
- `render_search` als Provisorium behalten statt gelöscht (T18-002 offen).
- Details in `docs/architecture.md` §8.13.

## Notizen
- Der Button-Glyph + genaue Menüreihenfolge sind ein kleiner Design-Punkt —
  im PR ein Screenshot für Nutzer-Sichtprüfung; Default: `Settings…`, Trenner,
  `Profile`.
- „Profile" bleibt bewusst leer bis zu einem echten Feature — kein
  Account-System in dieser Task.
- `Cmd+Shift+N` ist seit T16-007 die Command-Palette-`Page::Hosts` (nicht mehr
  „Open Host Manager") — im `＋▾`-Menü und in der Empty-Surface entsprechend
  als Hosts-Shortcut zeigen.
- Empty-Surface-Optik ist ein Design-Punkt für Nutzer-Sichtprüfung (Screenshot
  im PR). Referenz-Vibe: Zed-Welcome / leerer Pane, aber schlichter.

## Warnungen
- ⚠️ Drag-Region: Interaktive Kinder (Tabs, `＋▾`-Button, rechter Button)
  dürfen den Fenster-Drag nicht auslösen — nur der leere Titlebar-Hintergrund.
  GPUI: `window`-Drag-Handler am Container, `cx.stop_propagation()` auf den
  Kindern bzw. GPUIs Titlebar-Drag-Mechanismus aus `platform_title_bar`
  abschauen.
- ⚠️ Der `＋▾`-Button muss auch bei voll überlaufender Tab-Leiste sichtbar
  bleiben (fixe Position am Strip-Ende, nicht mitscrollen).
- ⚠️ Empty-Surface-Doppelklick vs. Titlebar-Doppelklick-Zoom nicht verwechseln
  — die Fläche liegt im Workspace-Bereich, nicht in der Titlebar.
- ⚠️ Auf Linux (später) gibt es keine macOS-Ampeln — `TRAFFIC_LIGHT_INSET`
  plattformabhängig machen (0 auf Linux), damit die Tabs nicht eingerückt
  bleiben.

## Weiterführende Tasks
- [T18-002: Suche als Overlay](./T18-002-search-overlay.md)
- [T18-003: Statusbar links — Panel-Steuerung](./T18-003-statusbar-left-panel-controls.md)
