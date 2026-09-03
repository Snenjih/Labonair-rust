# T20-007: `theme_settings`-Layer (Dichte, Font-Skalen, Radius)

## Status
📋 Geplant

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T20-005 (`ThemeRegistry`), T19-002 (`SettingsStore`), T20-001 (`ui-kit`)

## Ziel
UI-Dichte, Font-Skalen und Eck-Radius nicht mehr als lose Einzel-Preferences
verstreut, sondern als kohärenten **Theme-Settings-Layer**, der das aktive
`Theme` zur Laufzeit skaliert — analog Zeds `theme_settings`. Ergebnis: ein
`ActiveTheme`, das Farbe (aus der `ThemeRegistry`) **und** Metrik (aus den
Theme-Settings) kombiniert; die ganze App liest daraus.

## Kontext
- Heute verstreut in `Preferences`/`SettingsContent`:
  `appearance.app_font_size`, `app_line_height`, `app_font_family`,
  `app_corner_radius`, `reduce_motion`; im Theme selbst: `radius: RadiusScale`,
  `typography`. `ThemeStore` hält `FontOverrides`.
- `labonair-ui-kit`-Primitives lesen Größen teils aus dem `Theme`, teils
  hartkodiert.
- Zed-Vorbild:
  `zed-refrence/zed/crates/theme/src/ui_density.rs` — `UiDensity`
  (`Comfortable`/`Default`/`Compact`), skaliert Spacing.
  `zed-refrence/zed/crates/theme/` — `ThemeSettings` (buffer/ui font size,
  line height, `theme_settings_provider.rs`), `scale.rs`,
  `buffer_line_height.rs`.

## Anweisungen zur Umsetzung
1. **`ThemeSettings`** (ein `Settings`-Struct, registriert in T19-002):
   - `ui_font_family`, `ui_font_size`, `ui_line_height`
   - `buffer_font_family`, `buffer_font_size`, `buffer_line_height` (Editor/
     Terminal-Text — falls heute getrennt geführt, hier konsolidieren)
   - `ui_density: UiDensity` (`Comfortable`/`Default`/`Compact`)
   - `corner_radius_scale: f32` (multipliziert die `RadiusScale` des Themes)
   - `reduce_motion: bool` (aus `appearance` hierher)
   - Defaults = die heutigen `appearance.*`-Defaults.
2. **`UiDensity`** — ein Skalar-Set für Spacing/Höhen:
   `Compact` ≈ ×0.85, `Default` ×1.0, `Comfortable` ×1.15 (Werte an
   `reference-src` + Augenmaß; im Doc begründen). Betrifft: Listen-Zeilenhöhe,
   Button-Höhe/Padding, Statusbar-/Titlebar-Höhe (behutsam — die 40/32 aus dem
   Layout-Vertrag bleiben Basis, Dichte skaliert um sie herum).
3. **`ActiveTheme`** — die kombinierte Sicht, die die App liest:
   `struct ActiveTheme { colors: Arc<Theme> /* aus ThemeRegistry */,
   metrics: ThemeMetrics /* aus ThemeSettings: font sizes, line heights,
   density scale, radius scale */ }`. Als Global (`GlobalActiveTheme`) +
   `cx.active_theme()`-Helfer in `labonair-ui-kit`/`labonair-gpui-ext`.
   `ThemeStore` berechnet `ActiveTheme` neu bei Theme-Wechsel **oder**
   `ThemeSettings`-Änderung.
4. **`ui-kit`-Primitives** auf `ActiveTheme` umstellen: Größen/Paddings/Radien
   kommen aus `metrics` (density-skaliert), Farben aus `colors`. Keine
   hartkodierten `px(...)` für Spacing mehr in den Primitives (Ausnahmen im
   Doc gelistet).
5. **Migration der Einzel-Prefs**: `appearance.app_font_size` etc. werden
   `theme.ui_font_size` etc. (der Settings-Migrator T19-009 bekommt die
   Mapping-Zeilen; falls Phase 18 schon durch ist, hier ein kleiner
   Nachtrag-Migrator). Alte Keys als `_legacy`.
6. **Settings-UI**: eine „Darstellung"-Sektion mit Dichte-`SegmentedControl`,
   Font-Family/Size/Line-Height-Feldern (UI + Buffer getrennt), Radius-Slider,
   „Bewegung reduzieren"-Switch — alle live.
7. **`reduce_motion`** verdrahten: `labonair-theme::Animation` /
   `ui-kit`-Transitions respektieren das Flag (Dauer 0).
8. **Tests**: `UiDensity` skaliert die Metriken wie erwartet; `ActiveTheme`
   wird bei Theme- **und** bei ThemeSettings-Änderung neu berechnet + notifiziert;
   `reduce_motion` setzt Animationsdauern auf 0; Migrator mappt die alten Keys.
9. `cargo run`: Dichte auf „Compact" → Listen/Buttons/Statusbar spürbar enger,
   ohne Layout-Bruch; UI-Font-Size ändern → gesamte Chrome skaliert;
   Buffer-Font-Size ändern → nur Terminal/Editor-Text; Radius-Slider → Ecken
   überall; „Bewegung reduzieren" → keine Transitions; alles live + persistent.

## Akzeptanzkriterien
- [ ] `ThemeSettings` (registriertes `Settings`-Struct) bündelt Font-Skalen,
      Dichte, Radius-Skala, `reduce_motion`.
- [ ] `UiDensity` (3 Stufen) skaliert Spacing/Höhen konsistent; Titlebar/
      Statusbar-Basis aus dem Layout-Vertrag bleibt Referenz.
- [ ] `ActiveTheme` = Farbe (Registry) + Metrik (ThemeSettings); als Global,
      neu berechnet bei beiden Änderungsarten.
- [ ] `ui-kit`-Primitives lesen Spacing/Radius aus `ActiveTheme.metrics`
      (keine hartkodierten Spacing-`px` mehr, Ausnahmen dokumentiert).
- [ ] Alte `appearance.*`-Metrik-Keys migriert (Mapping in T19-009 bzw.
      Nachtrag-Migrator), `_legacy` erhalten.
- [ ] Settings-„Darstellung"-Sektion steuert alles live.
- [ ] `reduce_motion` schaltet Transitions wirklich ab.
- [ ] Tests decken Density-Skalierung, ActiveTheme-Recompute, reduce_motion,
      Migration.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Abschluss des Theme-Teils der P2-Empfehlung. Danach ist „Personalisierung"
  auf drei Achsen möglich: Layout (Statusbar/Panels), Farbe (ThemeRegistry),
  Metrik (ThemeSettings/Density).
- Density-Faktoren konservativ wählen — lieber später nachjustieren als ein
  gebrochenes Compact-Layout ausliefern.

## Warnungen
- ⚠️ Density darf den Layout-Vertrag nicht verletzen: Titlebar zeigt weiter nur
  Tabs + 1 Button, Statusbar weiter links/rechts — nur die Maße ändern sich.
- ⚠️ Font-Family-Wechsel triggert GPUI-Font-Neuladen — prüfen, dass Terminal
  (alacritty-Metrik) und Editor korrekt neu vermessen (Zeilenhöhe!), nicht nur
  die Chrome.
- ⚠️ `ActiveTheme` als Global + häufige Recomputes: nur bei echter Änderung neu
  bauen (kein Recompute pro Frame) — wird in T21-001 gemessen.

## Weiterführende Tasks
- [T21-001: Render-Pfad-Profiling](../phase-20-perf-signoff/T21-001-render-path-profiling.md)
