# T17-002: `Dock`-Modell (Links / Rechts / Unten)

## Status
📋 Geplant

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T17-001 (`Panel`-Trait & `PanelRegistry`)

## Ziel
Das heutige Dual-Slot-Modell (`left_slot`/`right_slot`, je genau ein Panel)
durch ein echtes Dock-Modell nach Zed-Vorbild ersetzen: drei Docks (links,
rechts, **unten** — neu), jeder Dock hält **mehrere** registrierte Panels mit
einem aktiven, unterstützt Zoom und Resize, und persistiert seinen Zustand.

## Kontext
- Heute: `crates/shell/src/app_shell.rs` — `left_slot`/`right_slot:
  crate::sidebar_slot::SidebarSlot { open: bool, panel: SidebarPanel, width },
  `render_sidebar(side, cx)`, manuelle `on_drag_move` + `SidebarResize(BarSide)`
  + throttled Persistenz (`SAVE_THROTTLE`, `last_sidebar_save`).
  `crates/shell/src/sidebar_slot.rs` (204 Z.).
- Kein Bottom-Dock. Panels nicht zwischen Seiten verschiebbar. 44px
  „Activity-Rail" (in `subagent-1.md` als Erfindung markiert) — entfällt
  (Layout-Vertrag; Panel-Wechsel läuft ab Phase 17 über Statusbar-Toggles).
- Zed-Vorbild:
  `zed-refrence/zed/crates/workspace/src/dock.rs` — `struct Dock`,
  `DockPosition`, `Dock::{add_panel, activate_panel, toggle_open,
  set_panel_zoomed, resize}`, `PanelButtons`, `RESIZE_HANDLE_SIZE`,
  `persistence::model::DockData`.
  `zed-refrence/zed/crates/workspace/src/workspace.rs` — `left_dock`,
  `right_dock`, `bottom_dock` Felder + Serialisierung.

## Anweisungen zur Umsetzung
1. **`crates/workspace/src/dock.rs` anlegen** (`Dock` lebt am `Workspace`, nicht
   in `shell`). Port von Zeds `Dock`, reduziert:
   - `struct Dock { position: DockPosition, panels: Vec<PanelHandle>,
     active: Option<usize>, open: bool, size: Pixels, zoomed: bool }`.
   - Methoden: `add_panel`, `remove_panel`, `activate_panel(name)`,
     `toggle_open`, `toggle_panel(name)` (öffnet + aktiviert, oder schließt
     wenn schon aktiv), `set_zoomed`, `set_size` (mit `min_size` je aktivem
     Panel geklammert).
   - `render(&mut self, window, cx) -> impl IntoElement` — Container +
     aktives Panel + Resize-Handle (`RESIZE_HANDLE_SIZE = px(6.)`).
2. **`Workspace`** bekommt `left_dock`, `right_dock`, `bottom_dock: Entity<Dock>`
   (oder `Dock` inline). `Workspace::new` befüllt sie aus der `PanelRegistry`:
   jedes registrierte Panel wandert in den Dock seiner `default_position`.
3. **Panel zwischen Docks verschieben**: `Workspace::move_panel(name,
   DockPosition)` — entfernt aus altem Dock, fügt in neuen ein, respektiert
   `Panel::position_is_valid`. (UI dafür kommt in T18-007; die API hier.)
4. **Persistenz** `DockData`-Äquivalent: `{ position, open, size, zoomed,
   active_panel, panel_order: Vec<String>, panel_positions: {name: position} }`
   pro Dock. Serde-Struct in `labonair-workspace`; persistiert über die
   bestehende Session-/Prefs-Infrastruktur (`session.rs` oder eigener
   `dock_layout`-Key). Beim Start wiederherstellen; Resize/Toggle/Move
   schreiben (throttled, `SAVE_THROTTLE` beibehalten).
5. **`app_shell.rs` / `render`**: `left_slot`/`right_slot` +
   `render_sidebar` + `sidebar_slot.rs` löschen. Der Body wird:
   `[left_dock.render] [ workspace ] [right_dock.render]` in einer Row, darüber/
   darunter der `bottom_dock` innerhalb der Workspace-Spalte
   (`[tabs][ row(left,center,right) ][bottom_dock][statusbar-ist-außerhalb]`).
   Exakte Verschachtelung an Zeds `workspace.rs`-Layout orientieren.
6. **Resize**: der manuelle `on_drag_move`/`SidebarResize`-Code zieht in
   `Dock::render` (Handle + `DragMoveEvent`), nicht mehr im Shell.
7. `cargo run`: alle drei Docks öffnen/schließen; mehrere Panels im linken
   Dock, umschalten; Bottom-Dock mit Git-Graph; Resize an allen drei Kanten;
   Zoom (Panel füllt den Dock-Bereich); Zustand überlebt Neustart.

## Akzeptanzkriterien
- [ ] `Dock` (L/R/B) existiert in `labonair-workspace`; `sidebar_slot.rs` ist
      gelöscht; `render_sidebar` im Shell entfällt.
- [ ] Jeder Dock hält mehrere Panels mit einem aktiven; Umschalten ohne die
      anderen zu zerstören.
- [ ] Bottom-Dock funktioniert (Git-Graph als Default-Bewohner).
- [ ] Resize an allen drei Kanten, mit `min_size`-Klammerung; Zoom-Toggle.
- [ ] `Workspace::move_panel` verschiebt ein Panel zwischen Docks (per Test +
      manuell über einen temporären Debug-Shortcut verifizierbar).
- [ ] Dock-Layout (offen/Größe/aktiv/Reihenfolge/Positionen) persistiert und
      wird beim Start wiederhergestellt.
- [ ] 44px-Activity-Rail ist entfernt.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Die *Toggles* (welche UI schaltet Panels ein/aus) kommen in T18-003 in die
  Statusbar. Hier reicht ein temporärer Debug-Weg (Command-Palette-Eintrag
  „Toggle Panel: …") zum Testen.
- `PanelButtons` aus Zed (die kleinen Dock-Rand-Buttons) **nicht** übernehmen —
  Labonair-Toggles leben in der Statusbar.

## Warnungen
- ⚠️ GPUI-Flex-Layout: Bottom-Dock muss innerhalb der Workspace-Spalte liegen
  (unter den Tabs, über der Statusbar), sonst überlappt es die Seiten-Docks
  falsch. Zeds Verschachtelung in `workspace.rs` genau nachbauen.
- ⚠️ Resize-Persistenz throtteln (nicht pro Frame schreiben) — der heutige
  `SAVE_THROTTLE`/`last_sidebar_save`-Mechanismus bleibt, nur verschoben.
- ⚠️ `Entity<Dock>` vs. `Dock` inline: wenn Panels `cx.subscribe` auf den Dock
  brauchen (Zoom/Close), muss der Dock eine Entity sein.

## Weiterführende Tasks
- [T17-004: `PaneGroup` rekursiver Split-Baum](./T17-004-panegroup-split-tree.md)
- [T18-003: Statusbar links — Panel-Steuerung](../phase-17-layout/T18-003-statusbar-left-panel-controls.md)
