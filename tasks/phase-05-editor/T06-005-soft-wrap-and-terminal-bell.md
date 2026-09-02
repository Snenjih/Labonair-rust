# T06-005: Editor Soft-Wrap + hörbare Terminal-Glocke

## Status
⏳ Pending

## Phase
5 — Editor (+ Terminal-Nachzügler)

## Abhängigkeiten
T06-001 (Editor-Grundlagen), T03-002 (GPUI-Terminal-Renderer), T13-003

## Ziel
Zwei beim T15-006-Audit bestätigte, in früheren Tasks explizit zurückgestellte
Restpunkte schließen:

1. **Editor Soft-Wrap.** Die Preference `editor_word_wrap` wird gespeichert und
   in den Settings angezeigt, hat aber **keine Renderer-Wirkung** — der
   Phase-5-Editor ist ein fixes Zeilenhöhen-Absolut-Grid. Referenz:
   CodeMirror `lineWrapping`. Umsetzung: Zeilen im Editor-Renderer bei aktivem
   Word-Wrap an der Viewport-Breite umbrechen (visuelle Zeilen ≠ Logische
   Zeilen; Cursor-/Selektions-Mathematik + Gutter anpassen).

2. **Hörbare Terminal-Glocke.** `terminal_bell` ist eine reine
   gespeicherte Preference; ein BEL (`\a`) löst derzeit keinen Ton aus.
   Referenz: xterm `bellStyle: "sound"`. Umsetzung: kurzer System-Beep bzw.
   gebündeltes WAV bei BEL, wenn `terminal_bell` an ist (macOS `NSBeep`
   / Audio-Ausgabe).

## Anweisungen
1. Editor-Renderer (`crates/ui/src/editor.rs` / `crates/editor/`): Soft-Wrap-
   Layout hinter `editor_word_wrap`, inklusive Cursorbewegung Home/End/Up/Down
   über visuelle Zeilen und Gutter-Nummerierung pro logischer Zeile.
2. Terminal (`crates/ui/src/terminal.rs` / `crates/terminal/`): BEL-Event des
   Emulators an die UI weiterreichen, bei `terminal_bell` einen Ton ausgeben
   (kleine, plattformgekapselte Beep-Funktion; Linux später).
3. Tests: Soft-Wrap-Zeilenberechnung unit-getestet; Bell-Gate unit-getestet.

## Akzeptanzkriterien
- [ ] Bei `editor_word_wrap` an brechen lange Zeilen sichtbar um, Navigation
      bleibt korrekt
- [ ] Bei `terminal_bell` an erzeugt ein `printf '\a'` einen hörbaren Ton, aus → still
- [ ] `cargo check` + `clippy -D warnings` + `cargo test` grün

## Notizen
- Aus T15-006 ausgegliedert. Soft-Wrap ist echte Renderer-Arbeit; die Glocke
  braucht eine Audio-Ausgabe (Crate-Wahl offen — `rodio` vs. plattform-native).
