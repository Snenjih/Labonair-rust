# T18-001: Titlebar-Redesign — nur Tabs + ein Icon-Button

## Status
📋 Geplant

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T17-006 (`AppShell` → reine Komposition)

## Ziel
Die Titlebar (Header) nach dem Layout-Vertrag neu bauen: **links/mitte nur die
Tab-Leiste**, **rechts genau ein Icon-Button**, der ein Dropdown öffnet
(`Settings…`, `Profile` als Platzhalter, erweiterbar für geplante Features).
Alles andere aus dem heutigen Header verschwindet.

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
- Zed-Vorbild: `zed-refrence/zed/crates/title_bar/` — dünne Titlebar,
  `platform_title_bar` für Drag-Region + Traffic-Lights.

## Anweisungen zur Umsetzung
1. **`crates/shell/src/titlebar.rs`** — `struct Titlebar` (Entity):
   - Höhe `HEADER_H` (40), transparente Overlay-Titlebar, Drag-Region
     (`window.on_move`/`-drag` für Fenster-Move außerhalb der interaktiven
     Elemente).
   - **Links**: `TRAFFIC_LIGHT_INSET` freilassen (macOS-Ampeln), dann die
     **Tab-Leiste** (`workspace.tab_bar()` / das migrierte `tabs.rs`-Widget).
     Tabs übernehmen die volle verbleibende Breite mit Überlauf-Scroll.
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
6. `cargo run` (macOS): Ampeln korrekt positioniert; Tab-Leiste füllt die
   Titlebar; ein Button rechts; Dropdown mit `Settings…` (öffnet Fenster) +
   `Profile` (Platzhalter); Fenster lässt sich an leeren Titlebar-Stellen
   ziehen; Doppelklick auf Titlebar = zoom (macOS-Konvention).

## Akzeptanzkriterien
- [ ] `crates/shell/src/titlebar.rs` existiert; `render_header`/`render_app_menu`/
      `render_search`/`render_agent_badge` sind aus `app_shell.rs` entfernt.
- [ ] Die Titlebar zeigt **ausschließlich** Tab-Leiste + einen Icon-Button.
- [ ] Der Button öffnet ein Dropdown mit `Settings…` (funktional) und `Profile`
      (Platzhalter, klar als solcher dokumentiert), mit Platz für weitere
      Einträge.
- [ ] Ampel-Inset stimmt; Fenster-Drag an leeren Stellen; Doppelklick-Zoom.
- [ ] Inline-Suche existiert nicht mehr in der Titlebar (Umzug in T18-002).
- [ ] Native macOS-Menüleiste unverändert funktionsfähig.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Der Button-Glyph + genaue Menüreihenfolge sind ein kleiner Design-Punkt —
  im PR ein Screenshot für Nutzer-Sichtprüfung; Default: `Settings…`, Trenner,
  `Profile`.
- „Profile" bleibt bewusst leer bis zu einem echten Feature — kein
  Account-System in dieser Task.

## Warnungen
- ⚠️ Drag-Region: Interaktive Kinder (Tabs, Button) dürfen den Fenster-Drag
  nicht auslösen — nur der leere Titlebar-Hintergrund. GPUI:
  `window`-Drag-Handler am Container, `cx.stop_propagation()` auf den Kindern
  bzw. GPUIs Titlebar-Drag-Mechanismus aus `platform_title_bar` abschauen.
- ⚠️ Auf Linux (später) gibt es keine macOS-Ampeln — `TRAFFIC_LIGHT_INSET`
  plattformabhängig machen (0 auf Linux), damit die Tabs nicht eingerückt
  bleiben.

## Weiterführende Tasks
- [T18-002: Suche als Overlay](./T18-002-search-overlay.md)
- [T18-003: Statusbar links — Panel-Steuerung](./T18-003-statusbar-left-panel-controls.md)
