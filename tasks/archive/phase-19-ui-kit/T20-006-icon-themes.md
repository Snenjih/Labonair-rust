# T20-006: Icon-Themes (JSON, umschaltbar)

## Status
✅ Done

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T20-005 (`ThemeRegistry` + JSON-Theme-Familien)

## Ziel
Die fest eingebaute Datei-Icon-Zuordnung (`IconName`-Enum + `file_icon`-Map über
~90 Endungen) durch ein austauschbares **Icon-Theme** ersetzen: eine JSON-
Definition, die Endungen/Dateinamen/Ordnerzustände auf Glyphen abbildet, mit
einem eingebauten Default und optionalen User-Icon-Themes.

## Kontext
- Heute: `labonair-ui-kit::icon` — `IconName`-Enum (~18–26 Lucide-Glyphen),
  `file_icon(ext) -> IconName` (~90 Endungen → ~18 Icons, laut Commit
  `d33fcfe`), `folder_icon(open) -> IconName`.
- Icon-Rendering: Lucide-SVGs via `gpui-component` / eingebettete Assets.
- Zed-Vorbild:
  `zed-refrence/zed/crates/theme/src/icon_theme.rs` +
  `icon_theme_schema.rs` — `IconTheme`, `IconThemeFamilyContent`,
  `DirectoryIcons`, `ChevronIcons`, `file_icons`-Mapping.
  `zed-refrence/zed/crates/file_icons/` — Laufzeit-Lookup + Default-Theme
  (`assets/icons/file_icons/*`).
  `zed-refrence/zed/assets/icon_themes/*.json`.

## Anweisungen zur Umsetzung
1. **Icon-Theme-Format** (`IconThemeContent`, JSON): `{ name, author?,
   file_stems: { "Dockerfile": "docker", ... }, file_suffixes: { "rs":
   "rust", "ts": "typescript", ... }, directory: { collapsed: "folder",
   expanded: "folder-open" }, chevron: { collapsed, expanded }, default_file:
   "file" }`. Werte = Glyph-IDs (aus einem festen Set eingebetteter SVGs).
   `#[derive(Deserialize, JsonSchema)]`.
2. **Glyph-Set**: das SVG-Set, das die Glyph-IDs bedienen kann — die heutigen
   Lucide-Glyphen + die aus `file_icon` referenzierten. Eingebettet als
   Assets; eine `GlyphId → SVG`-Auflösung.
3. **Default-Icon-Theme** `assets/icon_themes/labonair.json` — die heutige
   `file_icon`-Zuordnung 1:1 als JSON. Test: `IconTheme::default()` liefert
   für jede der heute abgedeckten Endungen dasselbe Glyph wie die alte
   `file_icon`-Funktion.
4. **`IconThemeRegistry`** (in `labonair-theme`, neben `ThemeRegistry`):
   `builtin()` + `load_user_icon_themes(dir)` +
   `list()` + `get(id)` + Live-Reload (fs-Watch User-Ordner).
5. **Lookup-API** in `labonair-ui-kit` (oder `labonair-theme`):
   `icon_for_path(path, is_dir, is_expanded) -> GlyphId` — nutzt das aktive
   Icon-Theme: erst `file_stems` (ganzer Dateiname), dann `file_suffixes`
   (längste passende Endung), dann `default_file`; Ordner über `directory`.
   `file_icon(ext)` bleibt als dünner Wrapper (Rückwärtskompatibilität für
   Call-Sites, bis sie auf `icon_for_path` umgestellt sind).
6. **Aktives Icon-Theme** aus Settings: `appearance.icon_theme` (neue ID).
   `ThemeStore` (oder ein `IconThemeStore`) hält es, `set_active_icon_theme`
   persistiert.
7. **Settings-UI**: Dropdown „Icon-Theme" (neben „Theme") mit Vorschau (ein
   paar Beispiel-Endungen als Icon-Reihe); Import / Ordner öffnen.
8. **Call-Sites**: Explorer + SFTP + Tab-Titel-Icons + Command-Palette-
   Datei-Einträge auf `icon_for_path` umstellen (die, die heute `file_icon`
   direkt rufen).
9. **Tests**: JSON→`IconTheme`; Default == alt-`file_icon` für alle
   abgedeckten Endungen; `file_stems` schlägt `file_suffixes`; längste Endung
   gewinnt (`.tar.gz`); unbekannt → `default_file`; User-Theme-Load + Live-
   Reload; unbekannte Glyph-ID in einem User-Theme → Fallback + Warnung.
10. `cargo run`: Explorer zeigt Datei-Icons wie bisher; ein User-Icon-Theme in
    den Ordner legen + im Settings-Dropdown wählen → Icons wechseln live;
    kaputte Icon-Theme-Datei wird ignoriert.

## Akzeptanzkriterien
- [x] Icon-Theme-JSON-Format (`IconThemeContent`) + eingebettetes Default
      (`assets/icon_themes/labonair.json`, aus `DEFAULT_FILE_STEMS` /
      `DEFAULT_FILE_SUFFIXES` generiert); ui-kit-Test
      `builtin_icon_theme_matches_legacy_file_icon` prüft „Default ==
      bisherige `file_icon`-Zuordnung" für jede einsegmentige Endung + jeden
      Sonder-Dateinamen.
- [x] `IconThemeRegistry`: builtin + User-Ordner (`load_user_icon_themes`),
      `list`/`get`/`contains`, Live-Reload via `ThemeStore::reload_user_icon_themes`
      + `labonair-shell` `watch_dir`, kaputte Dateien übersprungen mit Warnung.
- [x] `IconThemeContent::file_glyph` mit Reihenfolge file_stem → längste
      Endung (`archive.tar.gz` → `tar.gz` vor `gz`, `types.d.ts` → `d.ts` vor
      `ts`) → `default_file`; Ordner- **und** Chevron-Icons aus dem Theme
      (Explorer-Zeile nutzt `chevron_icon`).
- [x] `appearance.icon_theme` (neues `Preferences`/`AppearanceContent`-Feld,
      Default `"default"`) persistiert die Wahl; Themes-Pane hat einen
      Icon-Theme-Picker (SegmentedControl) mit Live-Glyph-Vorschau + Import +
      „Open icon themes folder".
- [x] Explorer + SFTP nutzen `icon_for_path` mit dem aktiven Icon-Theme.
      Tab-Titel/Command-Palette rufen `file_icon` heute **nicht** direkt (kein
      Call-Site vorhanden) — nichts umzustellen.
- [x] Tests decken Mapping-Reihenfolge, längste Endung, Default-Gleichheit,
      User-Load + Skip-broken, Live-Reload (theme-crate + store gpui-Test),
      unbekannte Glyph-ID → Fallback.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace` (898 passed, 0 failed), `check-crate-deps.sh`.

## Notizen
- Prio niedriger als T20-005 (Theme) — aber billig, weil Zeds `file_icons`/
  `icon_theme` fast direkt übertragbar ist.
- Kein „Icon-Pack aus dem Internet laden" — nur lokale JSON + eingebettetes
  Glyph-Set. Externe Glyph-Dateien (eigene SVGs mitliefern) sind ein
  Folge-Ticket.
- **Umsetzungsnotizen (2026-09-05):**
  - Kein `JsonSchema`-Derive auf `IconThemeContent` — gleiche Begründung wie
    T20-005 §8.18 (`labonair-theme` = Zero-Dep-Leaf-Crate). Nur `serde`.
    Dokumentiert in `docs/architecture.md §8.19`.
  - `.env*` war im alten `file_icon` eine Prefix-Regel; im Icon-Theme sind
    die gängigen Member als explizite `file_stems` gelistet. Ein neuartiges
    `.env.<x>` fällt jetzt auf `default_file` — per User-Icon-Theme
    ergänzbar.
  - Neue Mehrsegment-Suffixe: `d.ts` → `file-code` (echter Longest-Suffix-
    Test gegen `ts` → `braces`), `tar.gz`/`tar.bz2`/`tar.xz`/`tar.zst` →
    `archive` (gleicher Glyph wie der Tail, kein Verhaltensbruch).
  - Nicht `cargo run`-verifiziert (headless) — visueller User-Check von
    Icon-Wechsel/Live-Reload/kaputte-Datei offen (gleiche akzeptierte Lücke
    wie T20-002..005).

## Warnungen
- ⚠️ `file_suffixes`-Lookup: „längste passende Endung" (`archive.tar.gz` →
  `tar.gz` vor `gz`) explizit implementieren + testen — naives `rsplit('.')`
  reicht nicht.
- ⚠️ Das Glyph-Set ist endlich; ein User-Theme, das ein unbekanntes Glyph
  referenziert, darf nicht crashen — `default_file`-Fallback + einmalige
  Warnung.

## Weiterführende Tasks
- [T20-007: `theme_settings`-Layer](./T20-007-theme-settings-layer.md)
