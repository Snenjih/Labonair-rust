# T17-003: `StatusItem`-Trait & `StatusItemRegistry`

## Status
✅ Done

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T16-005 (`labonair-panel` Contracts), T17-001 (`PanelRegistry`)

## Ziel
Die `render_bar_item`-`match`-Kaskade über `BarItemId` in `app_shell.rs`
abschaffen. Jedes Statusbar-Element wird ein `StatusItem` (Trait aus
`labonair-panel`), das seine Default-Seite, sein Render und sein
Ausblend-Verhalten selbst beschreibt. `labonair-shell` registriert die
konkreten Items in einer `StatusItemRegistry`; die Statusbar rendert nur noch
die Registry.

## Kontext
- Heute: `crates/shell/src/app_shell.rs` — `enum BarItemId`
  (`Updater|Notifications|JumpHosts|AgentAccess|Transfers|Bookmarks|
  <PanelToggles>|CwdBreadcrumb|AiMini|AiPanel`), `render_bar_item(id, compact,
  cx)` `match`, `render_simple_bar_button`, `render_updater_item`,
  `render_notifications_item`, `render_panel_toggle`, `render_ai_toggle`,
  `render_cwd_breadcrumb`, `bar_items::Placements` (`barItemPlacements`-Blob),
  `render_statusbar`, `render_header` (Header trägt heute auch Bar-Items).
- `crates/shell/src/bar_items.rs` (455 Z.) — `BarItemId`, `BarLoc`
  (`Titlebar`/`Statusbar`), `BarSide` (`Left`/`Right`), `Placements`,
  `default_placement`, `placement_patch`, `BAR_ITEM_ORDER`, `BarLayoutTick`.
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/status_bar.rs` —
  `trait StatusItemView: Render`, `HideStatusItem`
  (`Arc<dyn Fn(&mut SettingsContent)>` + `apply(cx)` → `update_settings_file`),
  `left_items`/`right_items: Vec<Box<dyn StatusItemViewHandle>>`,
  `add_left_item`/`add_right_item`, `set_active_pane_item`-Callback.
- Layout-Vertrag (`docs/architecture.md`): Statusbar-Items **nur** in der
  Statusbar (kein Titlebar-Scope mehr), links = Panel-Steuerung, rechts =
  Info-Dropdowns.

## Anweisungen zur Umsetzung
1. **`StatusItem`-Trait finalisieren** in `labonair-panel` (aus T16-005):
   - `fn id(&self) -> &'static str`
   - `fn default_side(&self) -> StatusSide`
   - `fn order(&self) -> i32` (Stabil-Sortierung innerhalb einer Seite)
   - `fn render(&mut self, window, cx) -> AnyElement`
   - `fn hideable(&self) -> bool` (Analogon zu „hat einen Hide-Menüpunkt";
     die Persistenz macht T18-005)
   - `fn on_active_tab_changed(&mut self, cx)` (optional — für CWD/AiMini)
   - Ein `StatusItemHandle`-Wrapper (typlos, analog `PanelHandle`).
2. **`StatusItemRegistry`** fertig bauen (in `labonair-workspace`, am
   `Workspace` oder als Global — konsistent zu T17-001s Entscheidung für die
   `PanelRegistry`):
   - `register(&mut self, StatusItemHandle)`, `iter()`,
     `resolve_side(id) -> StatusSide` (Default aus `default_side`, überschrieben
     durch das Placement-Blob — Blob-Anbindung in T18-005).
3. **Konkrete `StatusItem`s** — je ein kleiner Typ (in `labonair-shell` oder
   im jeweils sinnvollen Crate):
   - `NotificationsStatusItem` (Badge + Dropdown) — nutzt
     `labonair-notifications`.
   - `CwdStatusItem` — der CWD-Breadcrumb (aus `cwd_breadcrumb.rs`), mit
     Segment-Menü + Subdir-Dropdown.
   - `UpdaterStatusItem` — aus `updater.rs`.
   - `TransfersStatusItem` — aus `transfers.rs`.
   - `AgentAccessStatusItem` — aus `agent_access` Badge-Popover.
   - `JumpHostsStatusItem`, `BookmarksStatusItem` — die „simple bar buttons".
     Hinweis: `JumpHostsStatusItem` ruft heute `open_host_manager`
     (`app_shell.rs:1806`); dieser Aufruf folgt dem Thema-2-Umbau — bis
     T17-009/T19-010 bleibt er (Host-Tab), danach zeigt er auf
     `OpenHostSettings` bzw. die Palette-`Page::Hosts`. Hier nur 1:1
     übernehmen, nicht neu verdrahten.
   - **Panel-Toggle-Items**: entweder ein einziges
     `PanelTogglesStatusItem`, das über die `PanelRegistry` iteriert und pro
     Panel einen Toggle rendert (bevorzugt — T18-003 baut es aus), oder je
     Panel ein Item. Default: **ein** Aggregat-Item.
   - `AiMini`/`AiPanel`-Toggle → fällt mit dem generischen Panel-Toggle
     zusammen (AI ist ein Panel). Alt-Duplikate entfernen.
4. **Registrierung in `labonair-shell`**: `register_builtin_status_items(
   workspace, cx)` — die einzige Stelle mit konkreten Item-Typen.
5. **`render_statusbar`** in eine `StatusBar`-Komponente (in
   `labonair-workspace`) verschieben: liest `StatusItemRegistry`, sortiert je
   Seite nach `order`, rendert links/rechts. `render_bar_item` +
   `render_simple_bar_button` + `render_*_item` + `bar_items.rs` `BarLoc` —
   löschen. `BarItemId`/`Placements` bleiben übergangsweise nur als
   Persistenz-Blob-Parser (wird in T18-005 durch `statusBarItemPlacements`
   ersetzt) oder gleich hier durch `id: &str` ersetzen.
6. **Header entkoppeln**: alle Bar-Items aus `render_header` entfernen (die
   Titlebar trägt ab Phase 17 nur Tabs + einen Button). Übergangsweise dürfen
   die Items schon jetzt alle in die Statusbar wandern.
7. `cargo run`: Statusbar zeigt links den Panel-Toggle-Bereich, rechts die
   Info-Items; Notifications-Badge zählt; CWD-Breadcrumb klickbar; Updater/
   Transfers/Agent-Access-Popover funktionieren.

## Akzeptanzkriterien
- [x] `render_bar_item`-`match`, `render_simple_bar_button`,
      `render_*_item`, `build_bar_bucket` existieren nicht mehr.
      > **Deviation:** `BarLoc`/`BarSide`/`Placements`/`BAR_ITEM_ORDER`/
      > `BarLayoutTick` (in `labonair-workspace::bar_items`) **bleiben**. Grund:
      > sie werden weiterhin von `labonair-settings-ui` (`view.rs`,
      > `panes/themes.rs` — der Bar-Item-Layout-Editor) konsumiert, und dieser
      > Editor + der `barItemPlacements → statusBarItemPlacements`-Migrator sind
      > explizit T18-005 / T18-006 zugeordnet. Der `BarLayoutTick`-
      > `observe_global` in `AppShell` bleibt verdrahtet (jetzt reines
      > `cx.notify()`), damit die Reaktivität bis T18-005 nicht verloren geht
      > (siehe `## Warnungen`). Der `render_bar_item`-`match` selbst,
      > `render_simple_bar_button`, alle `render_*_item`, `render_bar_menu`,
      > `build_bar_bucket`, `move_bar_item`, `persist_placement`,
      > `panel_for_item`/`item_for_panel` **sind entfernt**. Sanktioniert via
      > `docs/architecture.md §8` + Koordinator-Entscheidung.
- [x] Jedes Statusbar-Element ist ein `StatusItem` mit `id`/`default_side`/
      `order`/`render_status` (+ `hideable`/`on_active_tab_changed`);
      `labonair-shell` hat genau eine `register_builtin_status_items`-Stelle
      (`crates/shell/src/status_items.rs`).
- [x] Die `StatusBar`-Komponente in `labonair-workspace`
      (`status_bar.rs`) rendert ausschließlich aus der `StatusItemRegistry`
      (Feld am `Workspace`, konsistent mit `PanelRegistry` aus T17-001),
      sortiert je Seite nach `order`.
- [x] Header/Titlebar trägt keine Bar-Items mehr (`render_header` hat keine
      `build_bar_bucket`-Aufrufe).
- [~] `cargo run`: nicht auf diesem headless VPS testbar (kein X11). Alle
      `render_*_item`-Rümpfe wurden 1:1 in `status_items.rs`-Entities portiert
      (Notifications-Badge + Dropdown, CWD-Breadcrumb als eigene Entity mit
      `expanded`/`crumb_menu`/`subdir_menu` + Async-Subdir-Listing, Updater,
      Transfers, Agent-Access-Badge + Popover, Jump-Hosts, Bookmarks,
      Panel-Toggles-Aggregat über die `PanelRegistry`; zusätzlich
      `cursor-position` + `preview-url` als kleine Items, damit keine
      Feature-Regression). Dropdown-Anker von `top` auf `bottom` gedreht, da
      die Statusbar jetzt am unteren Fensterrand sitzt.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `scripts/check-crate-deps.sh`. `cargo test --workspace` kann auf diesem
      VPS nicht **linken** (fehlende X11-Dev-Libs `-lxcb`/`-lxkbcommon`/
      `-lxkbcommon-x11`); `check --all-targets` + `clippy --all-targets`
      kompilieren (typ-prüfen) den `#[cfg(test)]`-Code — akzeptierter Ersatz.

## Notizen
- Die **Personalisierung** (Rechtsklick → links/rechts/ausblenden + Persistenz)
  ist T18-005. Hier zählt nur: Items sind self-describing + registry-gerendert,
  Default-Seiten stimmen mit dem Layout-Vertrag überein.
- `HideStatusItem` bei Zed schreibt in `SettingsContent` — bei uns wird das in
  T18-005 an `statusBarItemPlacements` gebunden; hier reicht `hideable() -> bool`.

## Warnungen
- ⚠️ Der CWD-Breadcrumb hat State (expandiert? offenes Segment-Menü? Subdir-
  Listing in flight?) — als eigene Entity (`CwdStatusItem`) modellieren, nicht
  als reine Render-Funktion.
- ⚠️ `BarLayoutTick`-`observe_global` in `AppShell` (Re-Read bei Settings-Edit)
  muss auf den neuen Persistenz-Mechanismus zeigen oder in T18-005 sauber
  ersetzt werden — nicht einfach löschen und die Reaktivität verlieren.

## Weiterführende Tasks
- [T17-006: `AppShell` → reine Komposition](./T17-006-appshell-composition-only.md)
- [T18-004: Statusbar rechts — Info-Dropdowns](../phase-17-layout/T18-004-statusbar-right-info-dropdowns.md)
- [T18-005: Statusbar-Item-Personalisierung](../phase-17-layout/T18-005-statusbar-item-personalization.md)
