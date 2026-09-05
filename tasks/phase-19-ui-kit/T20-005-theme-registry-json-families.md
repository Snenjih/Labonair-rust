# T20-005: `ThemeRegistry` + JSON-Theme-Familien

## Status
✅ Done

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T16-009 (Theme-Store-Heimat entschieden), T19-002 (`SettingsStore`)

## Ziel
Aus „ein eingebautes Light/Dark + optional ein importiertes Custom-Theme" wird
eine **Registry**: mehrere Theme-Familien (je Light-/Dark-Variante), aus JSON
geladen (eingebettete + User-Ordner), zur Laufzeit umschaltbar, mit Live-Reload.

## Kontext
- Heute: `labonair-theme` — `Theme`-Struct (oklch→Hsla aus
  `reference-src/src/styles/globals.css`), `ThemeStore` (nach T16-009 in
  `labonair-theme`): `default light/dark` + `set_custom_theme` (genau eins),
  `import_theme_file` / `ThemeFile`, `preview_theme_file`,
  `theme_variant_overrides`.
- `crates/theme/src/import.rs` — bestehender Theme-Datei-Import.
- Settings: `appearance.app_theme` (aktive Theme-ID),
  `appearance.theme_variant_overrides`.
- Zed-Vorbild:
  `zed-refrence/zed/crates/theme/src/registry.rs` — `ThemeRegistry`,
  `ThemeFamily`, `ThemeMeta`, `list_names`, `get(name)`,
  `load_user_themes`, `ThemeNotFoundError`.
  `zed-refrence/zed/crates/theme/src/schema.rs` — `ThemeFamilyContent`
  (JSON-Format), `zed-refrence/zed/assets/themes/*.json`.

## Anweisungen zur Umsetzung
1. **JSON-Theme-Format** definieren (`ThemeFamilyContent`): `{ name, author?,
   themes: [ { name, appearance: "light"|"dark", colors: { ...token map... } } ] }`.
   Die Token-Map = die Felder des heutigen `Theme`-Structs (core, sidebar,
   surface, border, status, interaction, terminal-palette, radius, shadows).
   `#[derive(Deserialize, JsonSchema)]`. Nicht gesetzte Tokens → aus dem
   Built-in-Default derselben Appearance erben (Merge).
2. **Built-ins als JSON**: das aktuelle Light/Dark aus `globals.css` als
   `assets/themes/labonair.json` (eine Familie mit zwei Varianten) ablegen.
   Ein Test: das aus JSON geladene Built-in == das bisher hartkodierte `Theme`
   (kein visueller Bruch).
3. **`ThemeRegistry`** (in `labonair-theme`):
   - `builtin()` lädt die eingebetteten `assets/themes/*.json`.
   - `load_user_themes(dir)` lädt `~/<config_dir>/labonair/themes/*.json`.
   - `list() -> Vec<ThemeMeta { family, variant_name, appearance }>`.
   - `get(id) -> Result<Theme, ThemeNotFoundError>` (id = „Familie/Variante"
     oder flacher Variantenname).
   - Fehlerhafte Theme-Datei → überspringen + Warnung, nicht crashen.
4. **`ThemeStore` umbauen**:
   - hält die `ThemeRegistry` + `active_theme_id` + `ThemePreference`
     (System/Light/Dark bleibt) + `system_appearance`.
   - `resolve()` → wählt aus der aktiven Familie die Variante nach
     Preference/System-Appearance; fällt auf Built-in zurück, wenn die ID
     fehlt.
   - `set_active_theme(id)` → schreibt `appearance.app_theme`
     (via `SettingsStore::update_user_settings`), `cx.notify()`.
   - `theme_variant_overrides` bleiben: pro Familie kann der Nutzer die
     Light-/Dark-Zuordnung überschreiben (bestehende Semantik).
   - `import_theme_file` → kopiert die Datei in den User-Themes-Ordner und
     lädt die Registry neu (statt „das eine Custom-Theme" zu setzen).
   - `preview_theme_file` / `cancel_preview` bleiben (temporär aktives Theme
     ohne Persistenz).
5. **Live-Reload**: User-Themes-Ordner per fs-Watch → Registry neu laden; wenn
   das aktive Theme betroffen ist, neu `resolve()` + `notify`.
6. **Settings-UI** (Theme-Pane, Custom-Item): Dropdown/Grid aller
   `ThemeRegistry::list()`-Einträge mit Vorschau-Swatches; Auswahl →
   `set_active_theme`; „Theme-Datei importieren" / „Themes-Ordner öffnen";
   die Light/Dark-Override-Steuerung.
7. **Tests**: JSON→`Theme` Round-Trip; Built-in-JSON == alt-hartkodiert;
   fehlende Tokens erben Default; unbekannte aktive ID → Fallback + Warnung;
   User-Theme-Ordner-Load; Live-Reload wechselt live.
8. `cargo run`: zwei User-Themes in den Ordner legen → erscheinen im
   Settings-Dropdown; umschalten wirkt live (inkl. Terminal-Palette + Editor-
   Syntax-Basis); System-Preference-Umschaltung wählt die richtige Variante;
   eine kaputte Theme-Datei wird ignoriert (Warnung), App läuft weiter.

## Akzeptanzkriterien
- [x] JSON-Theme-Format (`ThemeFamilyContent`) + eingebettete Built-in-Datei
      (`crates/theme/assets/themes/labonair.json`, full-color, regenerierbar
      via `REGEN_BUILTIN_THEME=1`); Test „Built-in-JSON == hartkodiertes Theme"
      (`registry::tests::builtin_json_round_trips_to_the_hardcoded_theme`).
- [x] `ThemeRegistry` lädt eingebettete + User-Themes, listet `ThemeMeta`,
      liefert `Theme` per ID (`get`/`resolve`/`ThemeNotFoundError`), überspringt
      kaputte Dateien mit Warnung.
- [x] `ThemeStore` nutzt die Registry (`set_active_theme`/`registry()`/
      `reload_user_themes`); Preference/System-Appearance wählt die Variante.
      **Deviation:** die `appearance.app_theme`-Persistenz bleibt in
      `labonair-settings-ui` (`labonair-theme` darf nicht auf `SettingsStore`
      zeigen) — dokumentiert in `docs/architecture.md §8.18`.
- [x] Nicht gesetzte Tokens erben den Default derselben Appearance
      (`missing_tokens_inherit_the_same_appearance_default`).
- [x] Live-Reload des User-Themes-Ordners (`labonair_settings::watch_dir` in
      `labonair-shell` → `reload_theme_registry` → `ThemeStore::reload_user_themes`;
      `store::tests::reload_user_themes_live_swaps_and_drops_a_vanished_family`).
- [x] Settings-Theme-Pane: alle Registry-Varianten als Swatch-Karten wählbar,
      Import / Export / „Open themes folder", Light/Dark-Variant-Override
      (`render_variant_picker` jetzt registry-basiert).
- [x] Tests decken Round-Trip, Built-in-Gleichheit, Erben, Fallback,
      User-Load, Live-Reload (7 `registry::tests` + 2 neue `store::tests`).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`.

## Umsetzungshinweise (T20-005)
- **`JsonSchema` derive weggelassen** — `labonair-theme` bleibt der
  Null-Workspace-Dep-Leaf-Crate; `schemars` dafür einzuziehen war nicht
  gerechtfertigt (keine Akzeptanzkriterien-Anforderung). Siehe §8.18.
- Legacy-`ThemeFile` (`variants`-Map) wird von `ThemeFamilyContent::from_json`
  weiter akzeptiert → bestehende User-Theme-Dateien laden unverändert.
- Theme-IDs: `"<Datei-Stamm>/<Variantenname>"` bzw. `"default"` (Built-in).
- **Nicht mit `cargo run` verifiziert** (headless) — visuelle Prüfung durch
  den Nutzer offen.

## Notizen
- Critical Rule 3 bleibt: die **Built-in**-Werte kommen weiter 1:1 aus
  `globals.css` — die JSON-Datei ist nur eine andere Darreichungsform
  derselben Werte, nichts wird „umdesignt".
- Zeds `theme/src/schema.rs` ist eine gute Format-Vorlage; unser Token-Set ist
  kleiner (kein Player-Cursor, keine 200 Syntax-Scopes — die Editor-Syntax
  bleibt vorerst wie in `syntax_theme.rs`).

## Warnungen
- ⚠️ `theme_variant_overrides` haben schon eine Semantik (Handshake:
  „per-theme light/dark variant overrides") — nicht brechen, in die Registry-
  Welt übersetzen.
- ⚠️ Terminal-ANSI-Palette + Editor-Syntax-Basis hängen am aktiven `Theme` —
  beim Theme-Wechsel müssen Terminal-Views und Editor-Views neu einfärben
  (`cx.observe(&theme_store)` prüfen, dass es überall greift).

## Weiterführende Tasks
- [T20-006: Icon-Themes](./T20-006-icon-themes.md)
- [T20-007: `theme_settings`-Layer](./T20-007-theme-settings-layer.md)
