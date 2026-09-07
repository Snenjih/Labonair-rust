# T20-004: Component-Gallery (Debug-Fenster)

## Status
✅ Done

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T20-001 (`ui-kit` Primitive-Set)

## Ziel
Eine „Component-Gallery": ein Debug-Fenster (oder -Tab), das jedes
`ui-kit`-Primitive in allen relevanten Zuständen und Größen zeigt — als
visuelle Referenz und Regressions-Sichtfläche gegen `reference-src`.

## Kontext
- `labonair-ui-kit` (nach T20-001) — die Primitive-Sammlung.
- `labonair-theme` — Light/Dark + (nach T20-005) mehrere Theme-Familien.
- Zed-Vorbild: `zed-refrence/zed/crates/component/` (`Component`-Trait,
  Registrierung) + `zed-refrence/zed/crates/component_preview/`
  (`ComponentPreview`-View, `component_preview.rs` — listet alle registrierten
  Komponenten mit Beispielen, umschaltbar). Wir bauen eine **reduzierte**
  Variante: kein Trait-Registry-Zwang, eine handgepflegte Gallery-Seite.
- `reference-src/src/components/ui/**` — die Zielzustände (Hover/Active/Focus/
  Disabled/Selected, Größen, Varianten).

## Anweisungen zur Umsetzung
1. **`crates/ui-kit/src/gallery.rs`** (feature-gated: `#[cfg(feature = "gallery")]`
   oder nur `debug_assertions`) — `struct Gallery` (`Render`), das
   abschnittsweise jedes Primitive zeigt:
   - Pro Primitive ein `Section` mit: alle `Variant`s × alle `Size`s ×
     Zustände (default / hover-simuliert / active / focus / disabled /
     selected), plus ein paar realistische Kompositionen (z.B. „ListItem mit
     Icon + Badge + Trailing-Button").
   - `Banner` in Info/Warn/Error; `Kbd`/`KeybindingHint` mit mehreren Chords;
     `NumberField` an min/max; `Select` mit Auswahl; `ContextMenu` als
     dauerhaft offenes Beispiel; `Disclosure` auf/zu.
2. **Zugang**: ein Command „Open Component Gallery" (im `CommandRegistry`,
   nur Debug-Builds) öffnet die Gallery als eigenes `cx.open_window` (klein)
   **oder** als Workspace-Tab (`labonair-workspace::views::gallery`). Default:
   eigenes Fenster (stört den Arbeits-Workspace nicht).
3. **Theme-Umschalter** in der Gallery: Light/Dark + (nach T20-005) alle
   registrierten Theme-Familien — Klick schaltet live, um Kontrast/Zustände
   in jedem Theme zu prüfen.
4. **„Dichte"-Umschalter** (nach T20-007, sonst als TODO): kompakt/normal.
5. **Kein Prod-Ballast**: Die Gallery darf in Release-Builds nicht
   einkompiliert werden (`cfg`), und `labonair-app` zieht das `gallery`-Feature
   nur in `dev`/`debug`.
6. **Doku**: `docs/architecture.md` (Abschnitt „UI-Kit") um „Component-Gallery:
   `cargo run` + Command »Open Component Gallery« (Debug)" ergänzen.
7. `cargo run` (debug): Gallery öffnen → jedes Primitive sichtbar, Theme
   umschaltbar, Zustände erkennbar; Abgleich mit `reference-src`-Screenshots
   im PR.

## Akzeptanzkriterien
- [x] `Gallery`-View zeigt jedes `ui-kit`-Primitive in Varianten × Größen ×
      Zuständen + realistische Kompositionen. (Button/Toggle/Checkbox/ListItem/
      Disclosure/SegmentedControl/NumberField/Select/Banner/Kbd/ContextMenu/
      Divider/Indicator. `Badge`/`Switch`/`Tooltip` are gpui-component
      re-exports that style from their own `cx.theme()` global — deliberately
      not rendered, same reason they aren't wrapped.)
- [x] Über einen Debug-Command erreichbar (eigenes Fenster): command palette →
      *Debug: Open Component Gallery* → `open_gallery_window`.
- [x] Live-Theme-Umschalter (System/Light/Dark; Familien folgen mit T20-005).
- [x] Nicht in Release-Builds einkompiliert — `#[cfg(debug_assertions)]` /
      `#[cfg(any(debug_assertions, feature = "gallery"))]` on the module, the
      `pub use`, the palette row and the shell command registration. Verified
      with `cargo check --release` (compiles out) and
      `cargo check --release --features gallery` (compiles in).
- [x] `docs/architecture.md` §8.17 dokumentiert den Zugang.
- [~] PR-Screenshots — no PR workflow in play (direct commits to `master`) and
      no headless GPUI screenshot path; same accepted gap as T20-002/003.
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace` (0 failures), plus `cargo check --features
      gallery` and `scripts/check-crate-deps.sh`.

## Notizen
- Bewusst reduziert ggü. Zeds `component`-Trait-Registry — eine handgepflegte
  Seite reicht für ein Ein-Personen-Projekt und ist billiger zu warten.
- Die Gallery ist auch das schnellste Werkzeug, um bei T20-002/003 „sieht das
  noch aus wie die Referenz?" zu beantworten.

## Warnungen
- ⚠️ „hover-simuliert" — GPUI hat keinen einfachen Weg, Hover zu faken.
  Entweder einen `force_hover`-Prop an den Primitives (nur `#[cfg(debug)]`)
  oder die Zustände als statische Stil-Varianten in der Gallery nachbauen.
  Ehrlich dokumentieren, was echt und was nachgestellt ist.

## Weiterführende Tasks
- [T20-005: `ThemeRegistry` + JSON-Theme-Familien](./T20-005-theme-registry-json-families.md)
