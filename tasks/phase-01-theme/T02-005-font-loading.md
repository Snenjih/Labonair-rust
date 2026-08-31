# T02-005: Font-Handling & Font-Bundling (GPUI)

## Status
⏳ Pending

## Phase
1 — Theme-System & Design-Tokens

## Abhängigkeiten
T01-001 (Cargo Workspace)

## Ziel
Alle von Labonair genutzten Schriftarten (UI-Font + Terminal-/Mono-Font) nativ über GPUI laden
— gebündelt im Binary, ohne Abhängigkeit von System-Webfonts oder CSS `@font-face`. Fonts sind
über das Theme/Settings auswählbar (Vorbereitung für T13-003).

## Kontext
Im Original kommen Fonts über CSS (`reference-src/src/styles/globals.css` `--font-*`,
`@font-face`) und ggf. das Rust-Modul `reference-src/src-tauri/src/modules/fonts/`. GPUI hat ein
eigenes Font-System (`cx.text_system()`, `TextStyle.font_family`, `font_features`); Web-Font-Ladung
entfällt komplett.

## Anweisungen
1. `reference-src/src/styles/globals.css` + `reference-src/src-tauri/src/modules/fonts/` lesen:
   welche Font-Familien, Gewichte, Fallback-Ketten, Font-Features (Ligaturen für Terminal!) sind
   definiert. Auch `reference-src/src/modules/terminal/` auf Font-Konfiguration prüfen.
2. Font-Dateien (`.ttf`/`.otf`) unter `crates/theme/assets/fonts/` (oder `crates/app/assets/`)
   ablegen und via GPUI Asset-/`include_bytes!`-Mechanismus registrieren
   (`cx.text_system().add_fonts(...)` — genaue API in gpui-Source verifizieren).
3. Im Theme-Objekt (T02-001/002) Felder ergänzen: `ui_font_family`, `ui_font_size`,
   `buffer_font_family` (Editor), `terminal_font_family`, `terminal_font_size`,
   `terminal_line_height`, `font_features` (Ligaturen an/aus).
4. Fallback-Kette definieren, wenn ein Font fehlt (System-Sans / System-Mono).
5. Lizenz der Font-Dateien prüfen und in `crates/theme/assets/fonts/LICENSE` dokumentieren
   (Weiterverteilung im Binary!).

## Akzeptanzkriterien
- [ ] UI rendert mit dem korrekten UI-Font (visueller Abgleich mit reference-src)
- [ ] Terminal/Editor rendern mit dem korrekten Mono-Font, Ligaturen wie im Original
- [ ] Fonts sind im Binary gebündelt — App läuft ohne installierte System-Fonts korrekt
- [ ] Theme-Objekt hat Font-Felder, die T13-003 (Settings) später beschreiben kann
- [ ] Font-Lizenzen dokumentiert
- [ ] `cargo check` + `cargo clippy -- -D warnings` grün

## Notizen
- GPUI `add_fonts` erwartet Font-Bytes zur Laufzeit; Pfad/Signatur im gpui-Source nachschauen
  (Critical Rule 4 — keine erfundene API).
- Terminal-Font-Metriken (cell width/height) hängen am geladenen Font — relevant für T03-002.

## Warnungen
- ⚠️ Font-Lizenzen (z.B. proprietäre Programmier-Fonts) können Bündelung verbieten — vor Release
  klären, ggf. auf frei lizenzierten Ersatz mit gleichem Look wechseln.
