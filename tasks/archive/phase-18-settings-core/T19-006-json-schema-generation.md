# T19-006: JSON-Schema-Generierung für Settings

## Status
✅ Done

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-001 (`labonair-settings-content`), T19-005 (Rohe `settings.json` editierbar)

## Ziel
Aus dem typisierten `SettingsContent` ein JSON-Schema generieren und darüber
(a) beim Laden validieren (klare Fehlermeldungen mit Pfad statt „ungültiges
Feld"), (b) die Grundlage für spätere Editor-Autocomplete in der
`settings.json` legen.

## Kontext
- T19-001: `SettingsContent` mit `#[derive(JsonSchema)]` (schon vorgesehen).
- T19-005: `settings.json` ist jetzt manuell editierbar → Validierung wird
  nützlich.
- `crates/editor` — TreeSitter-Editor; ein echtes JSON-LSP gibt es nicht.
  „Autocomplete" hier = schema-getriebene, einfache Vervollständigung / Hover
  im Editor (klein), nicht ein voller Language-Server.
- Zed-Vorbild:
  `zed-refrence/zed/crates/settings/src/settings_store.rs` —
  `SettingsJsonSchemaParams`, `json_schema` (schemars), Registrierung als
  `zed://schemas/settings/...`.
  `zed-refrence/zed/crates/json_schema_store/`.

## Anweisungen zur Umsetzung
1. **Schema-Erzeugung**: `labonair_settings::json_schema() -> serde_json::Value`
   via `schemars::schema_for!(SettingsContent)`. Enums als `enum`-Listen mit
   `description` (aus Doc-Kommentaren via `schemars`-Feature oder Metadata).
   Bereichs-`description`s aus `assets/settings/default.json`-Kommentaren oder
   `SettingsFieldMetadata` (T19-004) ziehen, damit Hover-Texte konsistent zur
   GUI sind.
2. **Validierung beim Laden** (`SettingsStore`): nach dem Parsen eines Layers
   das JSON gegen das Schema prüfen (`jsonschema`-Crate). Ergebnis:
   `Vec<SettingsValidationError { json_path, message, line?, col? }>`.
   - Unbekannte Keys → Warnung (nicht fatal; Vorwärtskompatibilität).
   - Falscher Typ / ungültiger Enum-Wert → das Feld fällt auf Default
     (`FallibleOption`), Fehler wird gesammelt.
3. **Anzeige**: die Fehler landen im Settings-UI-Banner (aus T19-005) mit
   Pfad + Zeile („`terminal.cursorStyle`: `"blinky"` ist kein gültiger Wert —
   erlaubt: block, underline, bar (Zeile 42)"). Zusätzlich `tracing::warn!`.
4. **Schema-Datei ausgeben**: `labonair-settings` schreibt beim Start
   `~/<config_dir>/labonair/settings.schema.json` (oder embeddet es) — nützlich
   für externe Editoren und für T-später-Autocomplete.
5. **Editor-Hilfe (leichtgewichtig, optional aber angestrebt)**: wenn ein
   Editor-Tab die `labonair-settings.json` zeigt, das Schema laden und
   bieten:
   - Hover über einem Key → `description` aus dem Schema.
   - Ctrl-Space an einer Key-Position → Liste der erlaubten Keys/Enum-Werte
     der aktuellen Ebene.
   Kein voller LSP — ein schmaler, schema-getriebener Helfer im
   `labonair-editor`. Wenn der Aufwand zu groß wird: auf Hover beschränken,
   Autocomplete als Folge-Ticket.
6. **Tests**: Schema enthält alle Bereiche + Enum-Werte; ein falscher
   Enum-Wert / falscher Typ / unbekannter Key erzeugt je den erwarteten
   `SettingsValidationError` mit korrektem `json_path`; gültige Datei → keine
   Fehler.
7. `cargo run`: `settings.json` mit `terminal.fontSize: "gross"` → Banner
   „`terminal.fontSize` muss eine Zahl sein (Zeile X)", Feld nutzt Default,
   Rest lädt; Hover über `terminal.fontSize` im Editor zeigt die Beschreibung.

## Akzeptanzkriterien
- [x] `labonair_settings::json_schema()` liefert ein Schema, das alle
      `SettingsContent`-Bereiche + Enum-Werte + Beschreibungen abdeckt.
- [x] Beim Laden wird jeder Layer validiert; Typ-/Enum-Fehler ⇒ Feld-Default +
      gesammelter Fehler mit `json_path` (+ Zeile, wenn ermittelbar).
      (`User`- und `Project`-Layer, den einzigen beiden Layern mit einem
      echten Text-Loader — `Os`/`Profile` sind laut `store.rs` weiterhin nur
      strukturelle Platzhalter ohne Loader.)
- [x] Unbekannte Keys sind nur Warnungen, nicht fatal.
- [x] Die Fehler erscheinen im Settings-UI-Banner mit lesbarem Pfad/Wert.
- [x] `settings.schema.json` wird bereitgestellt (Datei oder Embed).
- [x] Editor: mind. Hover-Beschreibung über Settings-Keys (Autocomplete
      optional / Folge-Ticket, falls Aufwand zu groß — dann im PR begründen).
      (Nur Hover implementiert — Autocomplete als Folgeticket, siehe Notizen
      unten.)
- [x] Tests decken Schema-Vollständigkeit + die drei Fehlerklassen + Gutfall.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- `schemars` + `jsonschema` sind die üblichen Crates; `schemars` ist in
  T19-001 schon als Dep vorgesehen.
- Der Editor-Helfer ist der einzige Teil mit Unsicherheit — Hover ist Pflicht,
  Autocomplete ist „wenn machbar".
- **Umsetzung (2026-09-04):** Hover implementiert
  (`EditorView::update_hover`/`render_hover_tooltip`,
  `crates/workspace/src/views/editor.rs`) — Maus-Move über einer
  `labonair-settings.json`/`.labonair/settings.json`-Tab löst per
  `labonair_settings_json::json_path_at_offset` den Key-Pfad an der
  Cursorposition auf und zeigt `labonair_settings::description_for_path`s
  Schema-Beschreibung in einer schwebenden Karte. Autocomplete (Ctrl-Space)
  wurde bewusst ausgelassen — GPUI hat keine Autocomplete-Popup-Grundlage
  hier (`crates/workspace/src/views/editor.rs` hat keinen bestehenden
  Overlay/Popup-Mechanismus außerhalb der bereits vorhandenen KI-`@`-Datei-
  Mention-Popover in `panel-ai`, die pro-Kontext gebaut ist und nicht generisch
  wiederverwendbar ist) und der Aufwand (echtes Popup + Tastatur-Routing +
  Einfüge-Logik) hätte die für diesen Task veranschlagte Zeit gesprengt —
  als Folgeticket vorgemerkt, nicht Teil von T19-006/T19-007.

## Warnungen
- ⚠️ `schemars`-Derive-Ausgabe für getaggte Enums / `Option`-verschachtelte
  Structs prüfen — kann von dem abweichen, was `jsonschema` erwartet. Früh
  einen Round-Trip-Test (Schema validiert `default.json` ohne Fehler).
- ⚠️ Zeilen-/Spalten-Angaben brauchen einen Positions-fähigen JSON-Parser
  (der aus T19-005). Ohne Position trotzdem den `json_path` liefern.

## Weiterführende Tasks
- [T19-007: Globale Settings-Suche](./T19-007-global-settings-search.md)
