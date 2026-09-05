# T20-001: `ui-kit` Primitive-Set vervollständigen

## Status
✅ Done

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T16-002 (`labonair-ui-kit` Skeleton)

## Ziel
`labonair-ui-kit` von 5 Dateien (~880 Z.: Button, ContextMenu, Icon, TextField)
zu einem vollständigen Design-System ausbauen, das alle in der App
wiederkehrenden UI-Bausteine als konsistente, token-gebundene Primitives
bereitstellt — damit jede View aus denselben Teilen gebaut wird.

## Kontext
- Heute: `crates/ui-kit/src/` — `button`, `context_menu`, `icon`, `text_field`
  + Re-Export von `gpui_component::{Badge, Switch, Tooltip}`. Alles andere
  (Listen, Dropdowns, Dialoge, Disclosure, Tabs, Tabellen, Keybinding-Hints)
  wird pro View aus `div()` von Hand gebaut → visuelle Drift.
- `gpui-component` 0.5.1 liefert einige Primitives (Input, Select, Dropdown,
  Dialog, Tooltip, Switch, Badge, ContextMenu, Lucide-Icons) — Umfang genau
  prüfen (`cargo doc -p gpui-component` / Source in `~/.cargo`).
- Token-Quelle: `labonair-theme` (`Theme`-Struct, oklch→Hsla aus
  `reference-src/src/styles/globals.css`).
- `reference-src/src/components/ui/**` — die cva-basierte Primitive-Sammlung
  der alten App (shadcn-Stil): maßgeblich für Radien, Höhen, Paddings,
  Zustände.
- Zed-Vorbild: `zed-refrence/zed/crates/ui/src/components/*` (List, ListItem,
  ListHeader, Button-Familie, PopoverMenu, DropdownMenu, ContextMenu, Table,
  Disclosure, KeyBinding, KeybindingHint, Indicator, Banner, Divider,
  Tab, TabBar, ToggleButton, Checkbox, …), `zed-refrence/zed/crates/ui/src/
  styles/`, `prelude.rs`.

## Anweisungen zur Umsetzung
1. **Inventur**: aus allen Panel-/Workspace-/Settings-Crates die
   hand-gerollten UI-Muster sammeln (`grep` nach wiederkehrenden `div()`-
   Konstrukten: „Zeile mit Icon+Label+Chevron", „Sektion mit Header +
   Kindern", „Popover-Liste", „Segmentierter Umschalter", „Key-Chip"). Liste
   in `docs/architecture.md` (Abschnitt „UI-Kit").
2. **Primitives bauen/kapseln** — je entweder Wrapper um `gpui-component` oder
   Eigenbau, immer token-gebunden über `labonair-theme`:
   - **Layout**: `v_stack`/`h_stack`-Helfer (falls nicht in `gpui-ext`),
     `Divider`, `Section` (Header + Body + optional Disclosure),
     `Disclosure` (aufklappbar).
   - **Listen**: `List`, `ListItem` (Icon/Label/Trailing/selected/disabled),
     `ListHeader`, `ListSeparator`.
   - **Menüs/Popover**: `PopoverMenu` (Anker + Items), `DropdownMenu`
     (Button, der ein PopoverMenu öffnet), `ContextMenu` (bestehendes
     erweitern: Sub-Menüs, Trenner, Icons, deaktivierte Items, Keybind-Hint).
   - **Eingaben**: `TextInput` (bestehend), `NumberField` (mit min/max/step,
     +/− Stepper — wird von T19-004 gebraucht), `Select`/`EnumDropdown`,
     `Checkbox`, `Switch` (bestehend), `SegmentedControl` (der „AI/Shell"-
     Toggle etc.).
   - **Navigation**: `Tab`, `TabBar` (die Titlebar-Tab-Leiste baut darauf,
     T18-001/T16-009), `ToggleButton`/`IconToggleButton` (Statusbar-Panel-
     Toggles).
   - **Feedback/Info**: `Badge` (bestehend), `Indicator` (Punkt/Puls),
     `Banner` (Info/Warn/Error-Streifen — Settings-Fehler-Banner T19-005),
     `Tooltip` (bestehend), `KeyBinding`/`Kbd` (Tasten-Chip),
     `KeybindingHint` (Label + Kbd).
   - **Tabelle** (nur wenn real gebraucht — Host-Liste?, Transfer-Queue?):
     `Table` mit Spalten + Zeilen; sonst weglassen.
3. **API-Konsistenz**: alle Primitives folgen demselben Muster wie das
   bestehende `button()` (Builder-Fn oder `RenderOnce` + `#[derive(IntoElement)]`),
   `Size`-Enum (`Xs/Sm/Md`), `Variant`-Enum wo sinnvoll, `disabled`,
   `on_click`. Ein `ui_kit::prelude` re-exportiert alles.
4. **Zustände** aus `reference-src`: Hover/Active/Focus/Disabled/Selected 1:1
   an den Theme-Tokens (`hover_fill`, `selected_fill`, `border.focused`,
   `DISABLED_OPACITY`).
5. **Doc-Kommentare** je Primitive: Zweck + Referenz (`reference-src`-Pfad
   oder Zed-Datei) + ein Mini-Beispiel.
6. **Snapshot-/Render-Tests** wo möglich (GPUI-Test-Kontext): Primitive
   rendert ohne Panik in allen Zuständen; `NumberField` klemmt an min/max;
   `Select` gibt die Auswahl.
7. **Noch keine flächendeckende View-Migration** — die kommt in T20-002/003.
   Hier nur: Primitives existieren, sind dokumentiert, in der Gallery (T20-004)
   sichtbar, und mind. **eine** echte Call-Site je neuem Primitive umgestellt
   (als Referenz + Beweis, dass die API trägt).

## Akzeptanzkriterien
- [x] `labonair-ui-kit` stellt mind. die unter Anweisung 2 gelisteten
      Primitives bereit, alle token-gebunden über `labonair-theme`.
      (`v_stack`/`h_stack`, `Divider`, `Disclosure`, `List`/`ListItem`/
      `ListHeader`/`ListSeparator`, `PopoverMenu`, erweitertes `ContextMenu`
      (Sub-Menüs/Trenner/Icons/disabled/**Keybind-Hint**), `NumberField`,
      `Select`, `Checkbox`, `SegmentedControl`, `ToggleButton`/
      `IconToggleButton`, `Indicator`, `Banner`, `Kbd`/`KeybindingHint` —
      alle über die neue `Palette`-Token-Momentaufnahme aus `labonair-theme`.)
- [x] Einheitliche API (Size/Variant/disabled/on_click), `ui_kit::prelude`.
      (`button`/`context_menu`/`popover` wurden dafür von `&impl UiTheme` auf
      `Palette` umgestellt, damit die Crate *eine* Konvention hat.)
- [x] Zustände (Hover/Active/Focus/Disabled/Selected) entsprechen
      `reference-src` (je Primitive im Doc-Kommentar mit der cva-Klasse
      belegt: `button.tsx`, `checkbox.tsx`, `toggle.tsx`, `toggle-group.tsx`,
      `tabs.tsx`, `alert.tsx`, `kbd.tsx`, `separator.tsx`, `select.tsx`,
      `item.tsx`/`command.tsx`).
- [x] Jedes neue Primitive hat Doc-Kommentar + ≥1 echte Call-Site umgestellt
      (Liste in `docs/architecture.md` §8.16, Spalte „Call sites", ✓-Einträge).
- [x] `NumberField` erfüllt die Anforderungen aus T19-004 (min/max/step,
      Stepper) — `FieldControl::Int` **und** `Float` laufen darüber, die
      privaten `step_btn`/`slider_track`/`bump_int`/`bump_float`-Helfer in
      `settings-ui` sind entfallen.
- [x] Render-Tests für die zustandsbehafteten Primitives (29 Tests in
      `crates/ui-kit`: `NumberField` klemmt an min/max + rundet Float-Drift
      weg, `SegmentedControl::selection` meldet die Auswahl, `selected_label`
      löst das Select-Label auf, jedes Primitive baut in allen
      Variant/Size/disabled/selected-Kombinationen).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace` (878 Tests).

**Bewusst nicht gebaut** (Notizen-Regel „≥2 reale Call-Sites"):
`Table` (weder Host-Liste noch Transfer-Queue ist ein Spalten-Grid — beides
Card/Row-Layouts) und `Tab`/`TabBar` (die einzige echte Tab-Leiste ist
`Workspace::render_tab_bar` mit Drag-Reorder/Close/Indicator/Kontextmenü; die
übrigen „Tab"-Streifen sind segmentierte Umschalter und laufen über
`SegmentedControl`). Begründung + Verweis auf T20-002 in
`docs/architecture.md` §8.16.

## Notizen
- Nicht spekulativ bauen: nur Primitives, für die es in der Inventur ≥2
  reale Call-Sites gibt. `Table` nur, wenn wirklich gebraucht.
- Wo `gpui-component` gut passt: dünn wrappen (nur Token-Anbindung +
  konsistente API), nicht nachbauen.

## Warnungen
- ⚠️ `gpui-component` 0.5.1 ist an `gpui` 0.2.2 gepinnt — beim Wrappen keine
  API annehmen, die es nicht gibt (Source prüfen, nicht raten — Critical Rule
  4).
- ⚠️ Kein Primitive darf `labonair-theme` umgehen und Hex-/Hsla-Literale
  hartkodieren (Critical Rule 3).
- ⚠️ `RenderOnce` + `#[derive(IntoElement)]` vs. Builder-Fn — eine Konvention
  wählen und durchhalten (Zed nutzt beides je nach Fall; für uns: `RenderOnce`
  für alles mit >2 Feldern, sonst Fn).

## Weiterführende Tasks
- [T20-002: View-Migration Welle 1](./T20-002-view-migration-wave-1.md)
- [T20-004: Component-Gallery](./T20-004-component-gallery.md)
