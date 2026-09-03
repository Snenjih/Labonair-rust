# T19-009: Settings-Migrator (`preferences`/`editor`/`mcp` → `SettingsContent`, Keybinds → `keymap.json`)

## Status
📋 Geplant

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-004 (Settings-UI aus Modell), T19-008 (Keymap als Datei)

## Ziel
Ein einmaliger, idempotenter Migrator, der die alte `labonair-settings.json`
(getrennte Keys `preferences`, `editor`, `mcp`, `keybinds`-Blob) in das neue
flache `SettingsContent`-Layout überführt und die Keybind-Overrides in eine
`keymap.json` schreibt — ohne Nutzerdaten zu verlieren.

## Kontext
- Alt: `crates/backend/src/modules/settings/` — `preferences.rs`
  (`"preferences"`-Objekt), `editor.rs` (`"editor"`), `mcp.rs` (`"mcp"`),
  `preferences.keybinds` (`BTreeMap<slug,String>`), `barItemPlacements`
  (schon von T18-006 zu `statusBarItemPlacements` migriert).
- Neu: T19-001 `SettingsContent` (flacher Baum mit Bereichen `general`,
  `appearance`, `terminal`, `editor`, `ai`, `mcp`, `personalization`, …),
  T19-008 `keymap.json` (kontext-basiert).
- Vorbild-Muster: T18-006 (der Bar-Item-Migrator) — dieselbe Vorsicht
  (`.bak`, idempotent, Legacy behalten).
- Zed hat `MigrationStatus` / `migrator`-Crate
  (`zed-refrence/zed/crates/migrator/`) — Konzept ansehen, aber unser Fall ist
  einfacher (einmalige Struktur-Umformung, kein fortlaufendes Migrations-
  Framework).

## Anweisungen zur Umsetzung
1. **`migrate_settings_v1_to_v2(dir: &Path) -> Result<MigrationOutcome, String>`**
   in `crates/backend/src/modules/settings/migrations.rs` (bzw. neu):
   - **No-op**, wenn die Datei bereits das neue Layout hat (Erkennung: ein
     `schemaVersion: 2`-Feld **oder** das Fehlen der alten Top-Level-Keys
     `preferences`/`editor` bei Vorhandensein neuer Bereichs-Keys).
   - Sonst: `.bak` der ganzen Datei, dann transformieren.
2. **Feld-Mapping** `preferences.* → SettingsContent.*` — eine explizite
   Tabelle (Code + Doc-Kommentar), Kategorie für Kategorie. Beispiele:
   - `preferences.theme` → `general.theme`
   - `preferences.restoreWindowState` → `general.restoreWindowState`
   - `preferences.appFontSize` → `appearance.appFontSize`
   - `preferences.backgroundOpacity` → `appearance.backgroundOpacity`
   - `"editor".*` (der separate Key) → `editor.*`
   - `"mcp".*` → `mcp.*`
   - Unbekannte/entfernte alte Keys → in einen `_migratedUnknown`-Block
     schreiben (nicht verlieren, aber nicht ins Schema zwingen) + loggen.
   - Die Tabelle muss **alle** ~170 alten Felder abdecken — ein Test zählt
     ab (kein altes Feld fällt still weg).
3. **Keybinds**: `preferences.keybinds` (`{ slug: "cmd-x" }`) → `keymap.json`:
   - Pro Eintrag: `slug` → `CommandId` (über eine `slug→CommandId`-Tabelle,
     die aus dem alten `ShortcutId`-Enum + T17-007 stammt), `context` =
     der Default-Kontext dieses Kommandos (meist `"Workspace"`).
   - `""` (leerer String = „unbinden" im alten Modell) → `"cmd-x": null` im
     entsprechenden Kontext-Block.
   - Ergebnis in `~/<config_dir>/labonair/keymap.json` schreiben (nur wenn
     Overrides existieren; sonst keine Datei anlegen).
4. **Ergebnis-Datei**: neue `labonair-settings.json` mit `schemaVersion: 2` +
   den Bereichs-Keys; alte `preferences`/`editor`-Keys entfernt **oder** nach
   `preferences_legacy`/`editor_legacy` umbenannt (Default: umbenennen —
   Sicherheitsnetz, wie T18-006).
5. **Aufruf**: in `bootstrap` (T17-006), **vor** `labonair_settings::init(cx)`
   (T19-002) und vor dem Keymap-Load (T19-008). Reihenfolge:
   `migrate_bar_item_placements` (T18-006) → `migrate_settings_v1_to_v2` →
   `settings::init` → `keymap::load`.
6. **Tests**:
   - Voll ausgefüllte alte Datei → jedes Feld landet am neuen Ort; Zählung
     alt==neu (minus bewusst entfernte, gelistet).
   - Keybind-Overrides inkl. Unbind (`""`) → korrektes `keymap.json` mit
     `context` + `null`.
   - Zweiter Aufruf = no-op (`schemaVersion: 2` erkannt).
   - Datei bereits v2 → unangetastet.
   - Teilweise alte Datei (nur `preferences`, kein `editor`) → funktioniert.
7. `cargo run` mit einer echten alten `labonair-settings.json` (aus einem
   Pre-Rework-Build): App startet, alle bisherigen Einstellungen sind in der
   neuen GUI sichtbar, Custom-Keybinds funktionieren, `.bak` + `_legacy` sind
   da.

## Akzeptanzkriterien
- [ ] `migrate_settings_v1_to_v2` ist idempotent, macht `.bak`, erkennt
      `schemaVersion: 2`.
- [ ] Feld-Mapping deckt **alle** alten `preferences`-Felder ab (Test zählt
      ab; bewusst entfernte sind explizit gelistet).
- [ ] `"editor"`- und `"mcp"`-Keys wandern in die neuen Bereiche.
- [ ] Keybind-Overrides → `keymap.json` mit `context` + `null`-Unbind; keine
      Datei, wenn es keine Overrides gab.
- [ ] Alte Keys als `*_legacy` erhalten; neue Datei hat `schemaVersion: 2`.
- [ ] Aufreihung beim Start korrekt (bar-items → settings-v2 → settings::init
      → keymap::load).
- [ ] Tests: Vollmigration + Zählabgleich, Keybind-Migration, Idempotenz,
      bereits-v2, Teildatei.
- [ ] `cargo run` mit echter Alt-Datei: alle Einstellungen + Keybinds intakt.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Diese Task ist der „Nutzer verliert nichts"-Garant für Phase 18. Test-
  Abdeckung großzügig.
- Die `slug→CommandId`-Tabelle für die Keybind-Migration am besten in
  T17-007 schon mit anlegen und hier nur nutzen.

## Warnungen
- ⚠️ Nur die betroffenen Top-Level-Keys anfassen (`serde_json::Map`-Merge),
  nicht die ganze Datei blind neu schreiben — es könnten fremde Keys drinstehen
  (`statusBarItemPlacements` von T18-006!). Die dürfen nicht verloren gehen.
- ⚠️ `schemaVersion` einführen, damit künftige Migrationen (v2→v3) einen
  sauberen Ankerpunkt haben.
- ⚠️ Wenn der Zählabgleich in den Tests „Feld X nicht gemappt" meldet: Mapping
  ergänzen, **nicht** den Test aufweichen.

## Weiterführende Tasks
- [T20-001: `ui-kit` Primitive-Set vervollständigen](../phase-19-ui-kit/T20-001-ui-kit-primitive-set.md)
