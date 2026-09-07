# T18-006: Migrator `barItemPlacements` → `statusBarItemPlacements`

## Notiz (aus T17-003)
Literaler `BarLoc`-Abbau + Settings-UI-Titlebar-Bucket-Kollaps ist hier (bzw.
T18-005) eingeplant — T17-003 hat `render_bar_item`/`render_*_item`/
`build_bar_bucket` entfernt, aber `BarItemId`/`BarLoc`/`Placements`/
`BarLayoutTick` (`labonair-workspace::bar_items`) als transitionalen
Blob-Parser stehen lassen, weil `labonair-settings-ui` sie noch nutzt und der
Migrator hier lebt.

## Status
✅ Done

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T18-005 (Statusbar-Item-Personalisierung)

## Ziel
Ein einmaliger, idempotenter Migrator: bestehende `barItemPlacements`-Daten
(altes Schema mit `{ bar: titlebar|statusbar, side, hidden }`) in das neue
`statusBarItemPlacements`-Schema (`{ side: left|right, hidden }`, nur Statusbar)
überführen, damit Nutzer ihre Anpassungen nach dem Update nicht verlieren.

## Kontext
- Alt: `crates/backend/src/modules/settings/mod.rs` —
  `KEY_BAR_ITEM_PLACEMENTS = "barItemPlacements"`, Blob-Form
  `{ itemId: { itemId, bar, side, hidden } }`, `bar ∈ {titlebar, statusbar}`.
- Neu (T18-005): `statusBarItemPlacements`, `{ itemId: { side, hidden } }`.
- Item-ID-Änderungen prüfen: einige alte `BarItemId`s entfallen/fusionieren
  (`AiMini`/`AiPanel` → Panel-Toggle; Panel-Toggles sind nicht mehr
  verschiebbar). Eine Mapping-Tabelle alt→neu ist Teil dieser Task.
- Datei: `~/<data_dir>/labonair/labonair-settings.json` (bzw. `config_dir()`).

## Anweisungen zur Umsetzung
1. **Mapping-Tabelle** alt-`itemId` → neu-`itemId` (in Code + Doc-Kommentar):
   - `updater`, `notifications`, `jump-hosts`, `agent-access`, `transfers`,
     `bookmarks`, `cwd-breadcrumb` → gleiche ID, übernehmen.
   - `ai-mini`, `ai-panel` → **entfallen** (AI = Panel-Toggle, nicht
     platzierbar) → beim Migrieren verwerfen.
   - Panel-Toggle-IDs (`explorer`, `snippets`, `source-control`, `hosts`,
     `ai`) → **entfallen** (Panel-Toggles fix links) → verwerfen.
   - Unbekannte IDs → verwerfen (mit `tracing::debug!`).
2. **Regeln der Umsetzung** je altem Eintrag:
   - `bar == "titlebar"` → das Item wandert in die Statusbar; `side`
     übernehmen (`left`/`right`); `hidden` übernehmen.
   - `bar == "statusbar"` → `side` + `hidden` direkt übernehmen.
   - `side` fehlt → `default_side` des Items (nicht setzen, Eintrag weglassen).
   - `hidden == true` bleibt `hidden == true`.
3. **Migrator-Funktion** `migrate_bar_item_placements(dir: &Path) ->
   Result<MigrationOutcome, String>` in
   `crates/backend/src/modules/settings/` (bzw. neuem `migrations`-Modul):
   - Wenn `statusBarItemPlacements` bereits existiert → **no-op**
     (`Outcome::AlreadyMigrated`).
   - Wenn nur `barItemPlacements` existiert → transformieren, `statusBarItemPlacements`
     schreiben, `barItemPlacements` als `barItemPlacements_legacy` behalten
     (nicht löschen — Sicherheitsnetz) **oder** entfernen; Default: umbenennen
     zu `_legacy`.
   - `.bak` der gesamten Settings-Datei vor dem Schreiben.
   - Idempotent: zweiter Aufruf = no-op.
4. **Aufruf**: einmal beim App-Start, vor dem ersten `StatusItemRegistry`-
   Aufbau (in `bootstrap(...)` aus T17-006). Ergebnis loggen.
5. **Tests**:
   - Alt-Blob mit `titlebar`/`statusbar`/`hidden`/fehlendem `side` → korrektes
     Neu-Blob.
   - `ai-mini` / Panel-Toggle-IDs werden verworfen.
   - Zweiter Aufruf = no-op; `statusBarItemPlacements` unverändert.
   - Kein `barItemPlacements` und kein neues → no-op, keine Datei angefasst.
6. `cargo run` mit einer präparierten alten `labonair-settings.json`: die
   früher (im alten Build) nach rechts geschobene CWD-Breadcrumb ist nach dem
   Start rechts; ein früher ausgeblendetes Item ist ausgeblendet.

## Akzeptanzkriterien
- [x] `migrate_bar_item_placements` existiert, ist idempotent, macht `.bak`.
- [x] Titlebar-platzierte Alt-Items landen mit übernommener `side`/`hidden` in
      der Statusbar.
- [x] Entfallene IDs (`aiMini`, `aiPanel`, Panel-Toggles) werden sauber
      verworfen.
- [x] `barItemPlacements` bleibt als `_legacy` erhalten (Sicherheitsnetz).
- [x] Aufruf beim Start vor Registry-Aufbau; Ergebnis wird geloggt.
- [x] Tests decken: Transformation, Verwerfen, Idempotenz, Leerfall.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Klein und defensiv. Der Migrator läuft genau einmal pro Nutzer und darf im
  Zweifel lieber Defaults setzen als falsch raten.
- Die Mapping-Tabelle in `docs/architecture.md` (Abschnitt „Layout-Vertrag")
  spiegeln, damit klar ist, welche Bar-Items es überhaupt noch gibt.

## Warnungen
- ⚠️ Nicht die ganze `labonair-settings.json` neu schreiben — nur die zwei
  Keys anfassen (`serde_json::Map` mergen, wie
  `set_bar_item_placement_in` es macht).
- ⚠️ Reihenfolge beim Start: Migrator **vor** dem ersten Lesen von
  `statusBarItemPlacements` durch die `StatusBar`.

## Weiterführende Tasks
- [T18-007: Philosophie + Personalisierungs-Seite](./T18-007-philosophy-and-personalization-page.md)
- [T19-009: Settings-Migrator](../phase-18-settings-core/T19-009-settings-migrator.md)
