# T19-005: Rohe `settings.json` editierbar (kommentar-erhaltend)

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-002 (`SettingsStore`), T19-004 (Settings-UI aus Modell)

## Ziel
Die `settings.json` wird ein gleichberechtigter Bearbeitungsweg neben der GUI:
ein Command „Open Settings (JSON)" öffnet die User-Datei im Editor; die GUI
schreibt **chirurgisch** (nur der betroffene JSON-Knoten wird ersetzt,
Kommentare + Formatierung des Nutzers bleiben erhalten). Beide Wege bleiben live
synchron.

## Kontext
- Heute: `PreferencesStore::set_value` serialisiert bei jeder Änderung das
  **ganze** `preferences`-Objekt neu (`serde_json::to_value` → Map → String).
  Kommentare gibt es keine (die Datei ist maschinen-geschrieben). JSON-
  Editieren ist nicht vorgesehen.
- T19-002: fs-Watch auf `labonair-settings.json`; T19-004: `SettingField.write`.
- Zed-Vorbild:
  `zed-refrence/zed/crates/settings_json/` — `update_value_in_json_text`,
  `infer_json_indent_size` (surgische Text-Edits, JSON5/Kommentare tolerant).
  `zed-refrence/zed/crates/settings/src/settings_file.rs` —
  `update_settings_file(fs, cx, |content, _| ...)` (lädt Text, wendet die
  Mutation auf `SettingsContent` an, diff't gegen den alten Text, schreibt
  minimal).

## Anweisungen zur Umsetzung
1. **`labonair-settings-json` Crate** (oder Modul in `labonair-settings`):
   Port von `settings_json`:
   - `update_value_in_json_text(text: &str, key_path: &[&str], new_value:
     &serde_json::Value) -> Edit` — findet den Knoten (JSON mit `//`-/`/* */`-
     Kommentaren + trailing commas tolerant, via `jsonc`-Parser oder
     `serde_json` mit Preprocessing) und ersetzt nur dessen Span.
   - `infer_indent(text) -> IndentStyle`.
   - Wenn der Key-Pfad nicht existiert: an der richtigen Stelle einfügen
     (verschachtelte Objekte anlegen).
   - Rückgabe: `(new_text, byte_range)` für ein optionales Editor-Highlight.
2. **`SettingsStore::update_user_settings(|content| ...)`** (Port
   `update_settings_file`):
   - User-Datei-Text laden.
   - `SettingsContent` daraus parsen, Closure anwenden, den geänderten
     Teilbaum ermitteln.
   - Pro geändertem `json_path`: `update_value_in_json_text` anwenden.
   - Atomar schreiben (`.tmp` + `rename`), `.bak` bei erstem Schreiben pro
     Session.
   - `SettingsStore` re-liest (der fs-Watch triggert `recompute`; zusätzlich
     direkt, um Latenz zu sparen).
3. **`SettingField.write`** (T19-004) leitet ab jetzt über
   `update_user_settings` — die GUI erhält Kommentare/Formatierung.
4. **Command „Open Settings (JSON)"** (im `CommandRegistry`): öffnet
   `~/<config_dir>/labonair/labonair-settings.json` in einem Editor-Tab
   (`labonair-editor`). Existiert die Datei nicht → aus
   `assets/settings/initial_user_settings.json` (kommentiertes Gerüst,
   Vorbild `zed/assets/settings/initial_user_settings.json`) anlegen.
5. **Live-Sync GUI ⇄ Datei**: Datei im Editor speichern → fs-Watch →
   `recompute` → GUI-Felder aktualisieren. GUI-Feld ändern → surgischer
   Write → der offene Editor-Tab lädt neu (Datei-extern-geändert-Erkennung,
   die der Editor eh braucht) oder zeigt einen dezenten „extern geändert,
   neu laden?"-Hinweis. Default: automatisch neu laden, wenn keine
   ungespeicherten Editor-Änderungen.
6. **Fehlertoleranz**: kaputtes JSON in der Datei → GUI zeigt oben ein Banner
   „settings.json enthält einen Fehler (Zeile X): …", behält den letzten
   guten `merged`-Wert, GUI-Schreiben ist dann **blockiert** (sonst
   überschreibt man den kaputten Zustand blind) mit Hinweis „erst JSON
   reparieren".
7. **Tests**:
   - `update_value_in_json_text`: bestehende Kommentare + Einrückung bleiben;
     nur der Zielwert ändert sich; neuer Key wird korrekt eingefügt.
   - Round-Trip: GUI-Change → Datei → `recompute` → `pick` liefert den Wert.
   - Kaputtes JSON: `merged` bleibt, Banner erscheint, GUI-Write blockiert.
8. `cargo run`: `settings.json` mit Kommentaren anlegen; in der GUI ein Feld
   ändern → Datei ansehen: Kommentare intact, nur ein Wert geändert; Datei im
   Editor ändern + speichern → GUI aktualisiert sich; kaputte Klammer
   einfügen → Banner + blockiertes GUI-Schreiben.

## Akzeptanzkriterien
- [x] `labonair-settings-json` (Crate/Modul) portiert `update_value_in_json_text`
      + Indent-Inferenz; behandelt `//`-Kommentare und trailing commas.
- [x] `SettingsStore::update_user_settings` schreibt chirurgisch + atomar;
      Kommentare/Formatierung bleiben erhalten.
- [x] `SettingField.write` nutzt diesen Pfad; GUI-Änderungen zerstören keine
      Kommentare.
- [x] Command „Open Settings (JSON)" öffnet die Datei (legt sie ggf.
      kommentiert an).
- [x] Live-Sync in beide Richtungen ohne Neustart.
- [x] Kaputtes JSON: Banner + letzter guter Wert + blockiertes GUI-Schreiben,
      kein Crash, kein Blind-Überschreiben.
- [x] Tests decken surgische Edits, Key-Insert, Round-Trip, Kaputt-Fall.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- `zed-refrence/zed/crates/settings_json/` ist klein und herauslösbar — die
  beste Vorlage. Nicht selbst einen JSON-Patcher erfinden.
- Editor-„extern geändert"-Erkennung: falls `labonair-editor` das noch nicht
  kann, hier das Minimum (mtime-Check beim Fokus-Gewinn) ergänzen — aber
  klein halten, kein Voll-Feature.

## Warnungen
- ⚠️ Niemals die ganze Datei neu serialisieren (das war der alte Weg und
  killt Kommentare). Immer nur den Ziel-Span.
- ⚠️ Zwei Fenster + Datei-Editor gleichzeitig: der atomare `rename` + fs-Watch
  muss Interleaving aushalten (kein Lost Update). Ggf. ein Datei-Lock wie beim
  `BarItemPlacementLock`.

## Weiterführende Tasks
- [T19-006: JSON-Schema-Generierung](./T19-006-json-schema-generation.md)
- [T19-008: Keymap als Datei mit Kontexten](./T19-008-keymap-file-with-contexts.md)
