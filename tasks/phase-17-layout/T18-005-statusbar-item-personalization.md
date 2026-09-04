# T18-005: Statusbar-Item-Personalisierung (Rechtsklick → links/rechts/ausblenden)

## Notiz (aus T17-003)
Der literale `BarLoc`-Abbau + das Kollabieren des Titlebar-Buckets in
`labonair-settings-ui` (`view.rs`, `panes/themes.rs` — Bar-Item-Layout-Editor)
wurde in T17-003 **bewusst nicht** gemacht und ist hier eingeplant: T17-003
hält `BarItemId`/`BarLoc`/`Placements`/`BarLayoutTick` in
`labonair-workspace::bar_items` als transitionale Persistenz-/UI-Schicht am
Leben; der `BarLayoutTick`-`observe_global` in `AppShell` ist noch verdrahtet
(reines `cx.notify()`) und soll hier auf `statusBarItemPlacements` +
`StatusItemRegistry::resolve_side` umgestellt werden.

## Status
✅ Done

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T18-004 (Statusbar rechts — Info-Dropdowns)

## Ziel
Die vom Nutzer gewünschte Kernfunktion: **Rechtsklick auf ein Statusbar-Item →
Kontextmenü → „Nach links verschieben" / „Nach rechts verschieben" /
„Ausblenden"**. Die Wahl wird persistiert (`statusBarItemPlacements`) und beim
Start wiederhergestellt. Ersetzt den alten titlebar+statusbar-übergreifenden
`barItemPlacements`-Mechanismus.

## Kontext
- Referenz-Funktion (die zurückkommen soll):
  `reference-src/src/modules/settings/components/BarItemContextMenu.tsx` +
  `barItems.ts` (`{ bar, side, hidden }`) + `barItemLayout.tsx`. Im Port
  bislang teilweise als `crates/shell/src/bar_items.rs` (`Placements`,
  `BarLoc`, `BarSide`, `placement_patch`, `BarLayoutTick`) — mit Titlebar-Scope.
- Neu (Layout-Vertrag): **kein Titlebar-Scope**. Items leben nur in der
  Statusbar; wählbar ist nur `side ∈ {Left, Right}` + `hidden`.
- Nach T17-003: `StatusItem { id, default_side, order, hideable }` +
  `StatusItemRegistry`.
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/status_bar.rs`
  `HideStatusItem` (rechtsklick → „Hide Button", persistiert via
  `update_settings_file`).

## Anweisungen zur Umsetzung
1. **Persistenz-Schema** `statusBarItemPlacements` (im gemeinsamen
   `labonair-settings.json`, Key auf oberster Ebene — wie
   `barItemPlacements` heute): `{ itemId: { side: "left"|"right", hidden:
   bool } }`. Fehlt ein Item → `default_side` + sichtbar. Lese-/Schreib-
   Funktionen in `labonair-backend::modules::settings` (analog
   `bar_item_placements_load` / `set_bar_item_placement_in`, mit
   `BarItemPlacementLock` für Atomarität über Fenster).
2. **`StatusItemRegistry::resolve_side(id)`** + `is_hidden(id)` lesen das Blob
   (gemerged über `default_side`). Die `StatusBar`-Komponente (T17-003)
   nutzt das für links/rechts-Zuordnung + Auslassen versteckter Items.
3. **Kontextmenü**: jedes Item bekommt einen `on_mouse_down(Right)` →
   `ContextMenu` (ui-kit) mit:
   - `Nach links verschieben` (deaktiviert, wenn schon links)
   - `Nach rechts verschieben` (deaktiviert, wenn schon rechts)
   - Trenner
   - `Ausblenden` (nur wenn `hideable()`)
   Auswahl → `settings_set_status_bar_placement(id, patch)` (async, über den
   Lock) → Blob schreiben.
4. **Reaktivität**: statt des alten `BarLayoutTick`-`observe_global` ein
   sauberer Weg — die Schreib-Funktion setzt ein `StatusBarLayoutTick`-Global
   / emittet ein Event, die `StatusBar` re-liest. Oder: die Settings-Datei-
   Watch aus T19-002 (falls diese Task nach Phase 18 läuft) — aber Phase 17
   ist vor Phase 18, also den leichten Tick-Mechanismus nutzen.
5. **Ausgeblendete Items zurückholen**: in der Personalisierungs-Seite
   (T18-007) und/oder per Command-Palette („Statusbar: <Item> einblenden").
   Hier mindestens die Command-Palette-Einträge registrieren, damit ein
   versteckt gesetztes Item nicht unerreichbar ist.
6. **Panel-Toggles sind ausgenommen**: `PanelTogglesStatusItem` (T18-003) ist
   fix links, nicht verschiebbar/ausblendbar über dieses Menü (einzelne Panels
   ausblenden macht das RMB-Menü *auf dem Panel-Toggle*, T18-003).
7. **Migrator**: siehe T18-006 (eigene Task) — hier nur sicherstellen, dass
   das neue Schema sauber ohne Altdaten funktioniert.
8. `cargo run`: Rechtsklick auf CWD-Breadcrumb → „Nach links verschieben" →
   Item springt auf die linke Seite (rechts neben den Panel-Toggles), bleibt
   nach Neustart links. „Ausblenden" auf Bookmarks → weg; über Palette wieder
   einblenden. Zwei Fenster gleichzeitig: Änderung im einen wird im anderen
   übernommen (Lock + Tick).

## Akzeptanzkriterien
- [x] Rechtsklick auf jedes verschiebbare Statusbar-Item öffnet ein
      Kontextmenü mit links/rechts/ausblenden (kontextabhängig deaktiviert).
- [x] Die Wahl persistiert in `statusBarItemPlacements` und überlebt Neustart.
- [x] `default_side` greift, solange nichts gesetzt ist; verstecken lässt sich
      nur, was `hideable()` meldet.
- [x] Ausgeblendete Items sind über die Command-Palette wieder einblendbar.
- [x] Panel-Toggles sind von diesem Menü ausgenommen (bleiben fix links).
- [x] Zwei Fenster: Placement-Änderung wird atomar geschrieben und im anderen
      Fenster sichtbar (kein Verlust bei Interleaving).
- [x] `crates/workspace/src/bar_items.rs` `BarLoc`/Titlebar-Scope ist entfernt
      (Datei zu `crates/workspace/src/status_placements.rs` umgebaut — reines
      Statusbar-JSON<->Struct-Modul, kein Titlebar-Konzept mehr).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace` (inkl. Round-Trip-/Merge-Test des neuen Blobs,
      analog dem bestehenden `bar_item_placement_round_trips_and_merges`).

## Notizen
- Das ist die explizit vom Nutzer geforderte Funktion — hier nicht sparen an
  Test-Abdeckung (Round-Trip, Merge, Default-Fallback, Lock).
- Reihenfolge innerhalb einer Seite bleibt `StatusItem::order` (nicht
  nutzer-sortierbar in dieser Task — nur die Seite ist wählbar). Falls
  Drag-Sortierung gewünscht: eigenes Folge-Ticket.

## Warnungen
- ⚠️ Atomare Schreibvorgänge: der bestehende `BarItemPlacementLock` +
  read-merge-write + `rename`-tmp-Muster aus
  `crates/backend/src/modules/settings/mod.rs` **übernehmen**, nicht neu
  erfinden.
- ⚠️ Ein Item nach links schieben darf die Panel-Toggles nicht verdrängen —
  linke Seite = `[Panel-Toggles fix][dann verschobene Info-Items]`.

## Weiterführende Tasks
- [T18-006: Migrator `barItemPlacements` → `statusBarItemPlacements`](./T18-006-bar-item-placements-migrator.md)
- [T18-007: Philosophie + Personalisierungs-Seite](./T18-007-philosophy-and-personalization-page.md)
