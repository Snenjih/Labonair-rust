# T18-002: Suche als transientes Overlay (`Cmd+F`)

## Status
✅ Done

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T17-005 (`ModalLayer` + `ToastLayer`), T18-001 (Titlebar-Redesign)

## Ziel
Die aus der Titlebar entfernte Inline-Suche als transientes Overlay
wiederbringen: `Cmd+F` öffnet ein kleines Such-Feld (oben/oben-rechts über dem
aktiven Tab-Inhalt), das gegen das aktive Terminal bzw. den aktiven Editor
sucht — ohne permanente Chrome-Fläche.

## Kontext
- Heute: `app_shell.rs` — `render_search`, `search_open`, `search_query`,
  `search_focus`, `act_find` (`Cmd+F`). Die Suche forwardet an
  `Workspace::search_active` (Terminal-Suche) bzw. Editor-Suche.
  `reference-src/src/modules/header/` — `SearchInline` / `SearchTarget`
  (adaptiv Terminal vs. Editor).
- Terminal-Suche: `crates/terminal/` (Scrollback-Suche) / `Workspace`.
  Editor-Suche: `crates/editor/src/search.rs`.
- Zed-Vorbild: `zed-refrence/zed/crates/search/` (Buffer-Search-Bar als
  Toolbar-Item über dem Editor) — bei uns leichter: ein Overlay, kein
  Toolbar-Slot.

## Anweisungen zur Umsetzung
1. **`SearchOverlay`** (Entity, in `labonair-workspace` — es kennt den aktiven
   Tab-Typ) als `ModalView`-ähnliches, aber **nicht fokus-blockierendes**
   Overlay (der Nutzer soll weiter scrollen können). Falls `ModalLayer` immer
   Fokus fängt: einen zweiten, leichten `OverlayLayer` neben `ModalLayer`
   einführen, oder `ModalView` um ein `traps_focus() -> bool` erweitern.
   Entscheidung in `docs/architecture.md`.
2. **Inhalt**: `InputState` (ui-kit `text_field`), Treffer-Zähler
   („3/17"), Prev/Next-Buttons, `Aa`-Case-Toggle, Schließen-`Esc`.
   Position: oben rechts im zentralen Workspace-Bereich, mit kleinem Abstand;
   überlappt nicht die Docks.
3. **Ziel-Routing** (`SearchTarget`): beim Öffnen den aktiven Tab-Typ lesen:
   - Terminal → `Workspace::search_active` (bestehende Terminal-Scrollback-
     Suche); Enter/Next/Prev navigieren Treffer.
   - Editor → `labonair_editor` Such-API (`search.rs`); Treffer highlighten,
     Next/Prev.
   - Andere Tab-Typen (SFTP-Liste, Git-Graph) → Overlay öffnet nicht bzw.
     zeigt „Suche hier nicht verfügbar" (kurz).
4. **`Cmd+F`**: als `Command` im `CommandRegistry` (T17-007) registrieren →
   `SearchOverlay` togglen. `act_find` aus `app_shell.rs` entfernen.
5. **Zustand**: Query pro Tab merken? Referenz merkt die letzte Query global.
   Default: **global letzte Query** vorbefüllen + selektieren beim Öffnen.
6. `cargo run`: In einem Terminal-Tab `Cmd+F` → tippen → Treffer werden
   markiert, Next/Prev springt, Zähler stimmt, `Esc` schließt, Scrollen
   während offen möglich. Dasselbe in einem Editor-Tab.

## Akzeptanzkriterien
- [x] `Cmd+F` öffnet ein Such-Overlay über dem aktiven Tab-Inhalt; die
      Titlebar hat keine Suchfläche.
- [x] Das Overlay blockiert nicht das Scrollen des Inhalts; `Esc` schließt.
- [x] Terminal-Tab: Scrollback-Suche mit Treffer-Highlight, Zähler,
      Next/Prev.
- [x] Editor-Tab: Textsuche mit Highlight, Zähler, Next/Prev.
- [x] Nicht-suchbare Tab-Typen: klare, kurze Rückmeldung statt kaputtem
      Overlay.
- [x] Letzte Query wird beim erneuten Öffnen vorbefüllt (Auswahl/Select-all
      des vorbefüllten Texts ist von der `gpui-component`-`InputState`-API
      nicht öffentlich erreichbar — siehe `docs/architecture.md` §8.14).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- „Replace" (Suchen & Ersetzen im Editor) ist **nicht** Teil dieser Task —
  nur Suche/Navigation. Falls die bestehende Editor-Suche schon Replace kann,
  darf es bleiben, muss aber nicht ins Overlay.
- Regex/Whole-Word-Toggles nur, wenn die bestehende Suche sie schon
  unterstützt — nichts Neues bauen.

## Warnungen
- ⚠️ Fokus: Das Input braucht Fokus zum Tippen, aber Next/Prev per Enter darf
  den Fokus nicht an den Inhalt verlieren. Fokus-Handling explizit testen.
- ⚠️ Bei Tab-Wechsel mit offenem Overlay: entweder Overlay schließen oder Ziel
  neu binden — nicht gegen den alten (unsichtbaren) Tab weitersuchen.

## Weiterführende Tasks
- [T18-003: Statusbar links — Panel-Steuerung](./T18-003-statusbar-left-panel-controls.md)
