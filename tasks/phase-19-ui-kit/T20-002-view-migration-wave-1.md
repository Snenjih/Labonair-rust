# T20-002: View-Migration Welle 1 (Terminal, Editor, Explorer, SCM)

## Status
📋 Geplant

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T20-001 (`ui-kit` Primitive-Set)

## Ziel
Die vier meistgenutzten Views auf die `ui-kit`-Primitives umstellen: alle
hand-gerollten `div()`-Buttons, -Listen, -Menüs, -Felder durch die
Design-System-Bausteine ersetzen. Sichtbares Ergebnis: identisches (oder besser
konsistentes) Aussehen, aber ein UI-Vokabular.

## Kontext
- Betroffen: `labonair-workspace::views::{terminal, editor}` (Terminal-View-
  Chrome, Editor-Gutter/Statuszeile/Kontextmenüs — **nicht** die Zell-/
  Text-Render-Kerne), `labonair-panel-explorer`, `labonair-panel-scm`.
- Diese Views enthalten laut Bestandsaufnahme viel `div().flex()...on_click`
  statt `button()`, eigene „Zeile mit Icon+Name" statt `ListItem`, eigene
  Kontextmenüs statt `context_menu`.
- `reference-src/src/modules/{terminal,editor,explorer,source-control}/` —
  Referenz für Layout/Spacing/Zustände.
- Frühere Teil-Arbeit: Commit `946023f` „unify buttons + breadcrumb menus"
  (hosts.rs `btn()` → `components::button`) — dasselbe Muster jetzt für diese
  vier.

## Anweisungen zur Umsetzung
1. **Pro View** systematisch:
   - Alle klickbaren `div()` → `ui_kit::Button` / `IconButton` /
     `ToggleButton` (Größe/Variante passend).
   - „Baum-/Listen-Zeile" (Explorer-Datei, SCM-Change) → `ListItem`
     (Icon, Label, Trailing-Badge, `selected`, `disabled`).
   - Kontextmenüs (Explorer-Rechtsklick, SCM-Change-Rechtsklick,
     Editor-Rechtsklick) → `ui_kit::context_menu` (mit Trennern, Icons,
     deaktivierten Items, Keybind-Hints).
   - Eingaben (Explorer-Umbenennen-Inline-Feld, SCM-Commit-Message) →
     `TextInput` / Multiline-Input.
   - Abschnitts-Header (SCM „Staged"/„Changes") → `ListHeader` / `Section`.
   - Toolbar-Icons (Explorer-Refresh/New-File, SCM-Stage-All) → `IconButton`
     in einer `h_stack`.
2. **Keine Verhaltensänderung**: Klick-Ziele, Tastatur, DnD (Explorer),
   Diff-Sprünge (SCM) bleiben identisch. Nur die Bausteine wechseln.
3. **Zell-/Text-Render-Kerne unangetastet**: der alacritty-Zellen-Renderer
   und der TreeSitter-Text-Renderer sind kein `ui-kit`-Fall.
4. **Token-Audit**: bei der Gelegenheit hartkodierte Farb-/Spacing-Literale
   in diesen vier Views durch Theme-Token-Zugriffe ersetzen (Critical Rule 3)
   — nur die, die man beim Umbau ohnehin anfasst, kein Flächen-Refactor
   fremder Zeilen (Global Rule „Surgical Changes").
5. **Vorher/Nachher-Screenshots** je View im PR (Referenz-App daneben) für
   die Sichtprüfung.
6. **Tests**: bestehende View-Tests müssen grün bleiben; wo ein Primitive
   Verhalten kapselt (z.B. `context_menu`-Auswahl), ggf. Test anpassen statt
   löschen.

## Akzeptanzkriterien
- [ ] Terminal-View-Chrome, Editor-Chrome, Explorer, SCM enthalten keine
      hand-gerollten Button-/ListItem-/ContextMenu-/Field-`div()`s mehr
      (`grep` im PR-Text dokumentieren: Anzahl entfernter Stellen).
- [ ] Aussehen 1:1 zu vorher bzw. konsistenter (Screenshot-Vergleich im PR).
- [ ] Kein Verhaltensbruch: Explorer-DnD/Umbenennen/Kontextmenü,
      SCM-Stage/Unstage/Diff, Editor-Kontextmenü — alle unverändert.
- [ ] Angefasste hartkodierte Farb-/Spacing-Literale in diesen Views sind
      auf Theme-Token umgestellt.
- [ ] Bestehende Tests grün (angepasst, nicht ausgehöhlt).
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Reihenfolge: Explorer zuerst (klarster Listen-/Menü-Fall), dann SCM, dann
  Editor-Chrome, dann Terminal-Chrome. Ein Commit pro View, Gate grün.
- Wenn beim Umbau ein Primitive fehlt/nicht passt → zurück zu T20-001
  (Primitive ergänzen), nicht in der View einen Sonderweg bauen.

## Warnungen
- ⚠️ „Surgical Changes": nur Zeilen anfassen, die zum Primitive-Wechsel
  gehören. Nicht die Explorer-Logik oder den Diff-Algorithmus „mitverbessern".
- ⚠️ Explorer-DnD hängt an konkreten GPUI-Drag-Handlern — beim Wechsel auf
  `ListItem` sicherstellen, dass das Primitive Drag-Props durchreicht oder
  der Drag-Wrapper außen bleibt.

## Weiterführende Tasks
- [T20-003: View-Migration Welle 2](./T20-003-view-migration-wave-2.md)
