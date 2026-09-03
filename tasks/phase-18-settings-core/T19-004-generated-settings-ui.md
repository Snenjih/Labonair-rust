# T19-004: Settings-UI aus dem Modell generieren

## Status
📋 Geplant

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-002 (`SettingsStore`), T16-007 (`labonair-settings-ui` extrahiert)

## Ziel
Die parallele, handgepflegte `FIELDS`-Tabelle (131 Einträge, driftet gegen ~170
Struct-Felder) abschaffen. Die Settings-UI wird aus dem typisierten
`SettingsContent` generiert: pro Feld ein `SettingField` mit einem `pick`, und
eine Renderer-Registry pro Rust-Typ. Ein neues `bool`-Feld ⇒ **null** UI-Code.

## Kontext
- Heute: `labonair-settings-ui/src/fields.rs` — `FIELDS: &[FieldDef]`
  (`{ key, title, desc, category, FieldKind }`), `FieldKind { Switch,
  Int{min,max,step}, Float{…centi}, Select(&[&str]), FontFamily, Text }`,
  `SECTION_GROUPS`, `CATEGORIES` (10), `SettingsView` rendert daraus + Sonder-
  Panes (Theme-Grid, Shortcuts, AI, MCP, Personalisierung aus T18-007).
- T19-001/002: `SettingsContent`-Baum, `SettingsStore`,
  `SettingsStore::source_of`.
- Zed-Vorbild (fast 1:1 die Blaupause):
  `zed-refrence/zed/crates/settings_ui/src/settings_ui.rs` —
  `struct SettingField<T> { pick: fn(&SettingsContent) -> Option<&T>, ... }`,
  `SettingFieldRenderer` (Registry pro Typ: `bool → Switch`,
  Enum → `EnumVariantDropdown`, Zahl → `NumberField`,
  `String → SettingsInputField`, Sondertypen → Custom),
  `struct SettingItem { field, metadata, files }`,
  `enum SettingsPageItem { SectionHeader | SettingItem | SubPageLink |
  DynamicItem | ActionLink }`, `struct SettingsPage`.
  `zed-refrence/zed/crates/settings_ui/src/page_data.rs` — wie die Seiten aus
  Feld-Listen deklarativ zusammengesetzt werden.

## Anweisungen zur Umsetzung
1. **`SettingField<T>`** in `labonair-settings-ui`:
   - `pick: fn(&SettingsContent) -> Option<&T>` (liest den effektiven,
     gemergten Wert; `None` ⇒ Default anzeigen).
   - `write: fn(&mut SettingsContent, T)` (setzt den User-Layer-Wert;
     Persistenz via T19-005 surgical write).
   - `json_path: &'static str` (für „Herkunft anzeigen" + Schema + JSON-Sprung).
   - `metadata: SettingsFieldMetadata { title, description, unit?, range?,
     placeholder?, requires_restart? }`.
2. **`SettingFieldRenderer`-Registry** pro Rust-Typ:
   - `bool` → `Switch` (ui-kit).
   - `enum` (mit `strum`/`VariantArray` oder `JsonSchema`-Enum) →
     `Dropdown` mit den Varianten + Labels.
   - `u32`/`i64`/`NonZeroU32` → `NumberField` mit `range`/`step` aus Metadata.
   - `f32` → `NumberField` (Float-Modus).
   - `String` → `TextInput`; Spezialfälle über Metadata-Marker
     (`FontFamily` → Font-Picker, `Color` → Color-Input, `Path` → Datei-Dialog).
   - Unbekannter Typ → `Custom`-Renderer (Fallback: JSON-Schnipsel-Editor für
     das Feld).
   - Registrierung analog `SettingFieldRenderer::add::<T>(render_fn)`.
3. **Seiten deklarativ** (`page_data.rs`-Äquivalent): pro Kategorie eine
   `SettingsPage` mit `Vec<SettingsPageItem>` — `SectionHeader` +
   `SettingItem(SettingField)` in der gewünschten Reihenfolge. Diese Liste ist
   die **einzige** handgepflegte Stelle, aber sie enthält nur
   `SettingField`-Referenzen (kein Kontroll-Typ, kein min/max — das steckt im
   Feld/Metadata). Nicht gelistete Felder: automatisch am Ende der zum
   `json_path`-Präfix passenden Kategorie unter „Weitere".
4. **Sonder-Panes** bleiben als `SettingsPageItem::Custom`: Theme-Grid
   (bis T20-005), Shortcuts (bis T19-008), AI-Provider/Agents/Directives,
   MCP-Bridge, Personalisierung (T18-007).
5. **`FIELDS` / `FieldDef` / `FieldKind` / `SECTION_GROUPS` löschen.**
   `CATEGORIES` bleibt (Sidebar-Reihenfolge) — oder wird zu einer Liste von
   `SettingsPage`s.
6. **Herkunfts-Badge**: jedes gerenderte Feld zeigt (dezent) die Schicht des
   effektiven Werts (`SettingsStore::source_of`) — „Standard" / „Benutzer" /
   „Projekt". Bei nicht-Standard: „auf Standard zurücksetzen"-Button.
7. **Suche**: bleibt vorerst wie heute (pro Kategorie/Query); die globale
   Suche über alle Seiten ist T19-007.
8. **Tests**:
   - Für jedes Feld im `SettingsContent`, das einen `SettingField` haben soll,
     ein Test, dass es in genau einer Seite auftaucht (kein verwaistes Feld) —
     das ersetzt die alte „FIELDS deckt Preferences ab"-Prüfung.
   - Renderer-Registry: `bool`/`enum`/`u32`/`f32`/`String` rendern das
     erwartete Control.
   - `write` → `SettingsStore` → `pick` Round-Trip.
9. `cargo run`: Settings-Fenster, alle Kategorien; ein `bool`-Feld ohne
   `SettingsPageItem`-Eintrag erscheint automatisch unter „Weitere";
   Herkunfts-Badge + Reset; Änderungen persistieren + wirken live.

## Akzeptanzkriterien
- [ ] `FIELDS`, `FieldDef`, `FieldKind`, `SECTION_GROUPS` existieren nicht mehr.
- [ ] Die UI wird aus `SettingField` + Renderer-Registry generiert; ein neues
      `bool`-Feld erfordert 0 Zeilen UI-Code (nur optional einen
      `SettingsPageItem`-Eintrag für die Platzierung).
- [ ] Renderer für `bool`/`enum`/`u32`/`f32`/`String` + Custom-Fallback.
- [ ] Jedes Feld zeigt seine Herkunfts-Schicht + „auf Standard zurücksetzen".
- [ ] Kein `SettingsContent`-Feld ist im UI unerreichbar (Test erzwingt das).
- [ ] Änderungen persistieren (User-Layer) und wirken live (fs-Watch/Observer).
- [ ] Sonder-Panes (Theme/Shortcuts/AI/MCP/Personalisierung) unverändert
      funktionsfähig als Custom-Items.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Das ist die P0-3-Kern-Task. `zed-refrence/zed/crates/settings_ui/src/
  settings_ui.rs` + `page_data.rs` sind die direkte Vorlage — Struktur
  übernehmen, Umfang auf Labonairs Bereiche reduzieren.
- „DynamicItem" / „SubPageLink" aus Zed sind nice-to-have (z.B. für die
  AI-Provider-Liste) — optional, wenn es die AI-Pane vereinfacht.

## Warnungen
- ⚠️ `pick`/`write` als `fn`-Pointer (nicht Closures) halten, damit
  `SettingField` `Copy`/`'static` bleibt (wie bei Zed) — sonst Lebenszeit-
  Schmerz in der Registry.
- ⚠️ Enum-Varianten-Labels: nicht die Rust-Namen zeigen. Über `strum`
  `EnumMessage` oder eine Metadata-Map menschenlesbare Labels.
- ⚠️ „Live wirken" heißt: `SettingsStore` benachrichtigt, die Module lesen neu.
  Für noch nicht auf `Settings`-Trait migrierte Module läuft es über die
  `GlobalPreferences`-Brücke — sicherstellen, dass die Brücke nach jedem
  `write` aktualisiert wird.

## Weiterführende Tasks
- [T19-005: Rohe `settings.json` editierbar](./T19-005-raw-json-settings-editor.md)
- [T19-007: Globale Settings-Suche](./T19-007-global-settings-search.md)
