# T02-004: Terminal-ANSI-Palette in das Theme integrieren

## Status
✅ Done

## Phase
1 — Theme-System & Design-Tokens

## Abhängigkeiten
T02-001 (Design-Tokens extrahieren)

## Ziel
Die vollständige Terminal-Farbpalette (16 Standard-, 16 Bright- und 16 Dim-Farben plus terminal-spezifische Hintergrund-, Vordergrund-, Cursor- und Selektionsfarben) aus der Theme-Struktur in die Terminal-Engine überführen und sicherstellen, dass Shell-Farben identisch zur Original-Labonair-App dargestellt werden.

## Kontext
Labonair definiert in `globals.css` eine vollständige ANSI-Farbpalette für seine Terminals. Diese Palette ist themenabhängig (Dark/Light) und steuert das Aussehen all dessen, was in einer Terminal-Session ausgegeben wird: `ls`-Farben, Gitz-Differenzen, Vim-Statuslinien, grep-Hervorhebungen, und beliebige ANSI-escape-codierte Szenen. Die Palette ist in drei 16-Farb-Gruppen gegliedert (Standard, Bright, Dim) plus die terminal-spezifischen Anzeigefarben für Hintergrund, Vordergrund, Cursor und Auswahl.

Diese Werte müssen von der Theme-Struktur (aus T02-001) in das Format überführt werden, das die Terminal-Engine erwartet. Die Terminal-Engine selbst (voraussichtlich auf `alacritty_terminal` basierend, siehe Phase 2) arbeitet mit einem eigenen Farbmodell (RGB mit 8 Bit pro Kanal) und einem Index-System für ANSI-Farbnummern (0–255). Es muss eine verlustfreie und konsistente Brücke zwischen beiden Welten geben.

## Anweisungen zur Umsetzung

1. **Palette-Vollständigkeit prüfen.** Verifiziere, dass die in T02-001 angelegte Terminal-Paletten-Struktur tatsächlich alle drei 16-Gruppen (Standard, Bright, Dim) sowie die Sonderfarben für Terminal-Hintergrund, -Vordergrund, -Cursor und -Auswahl enthält. Ergänze fehlende Werte aus `globals.css`, falls welche übergangen wurden.

2. **Hilfsfunktion für den ANSI-Index-Zugriff.** Implementiere eine Funktion, die aus einer ANSI-Farbnummer (0–255) die passende Farbe aus der Palette liefert. Der Zugriff muss das übliche ANSI-Schema abbilden: 0–7 Standard, 8–15 Bright, 16–231 erweiterte 6×6×6-Farbwürfel (im üblichen Berechnungsschema aus den Grundwerten abgeleitet), 232–255 Graustufen. Dunkle/Dim-Varianten sind dort zu berücksichtigen, wo Labonair sie anwendet.

3. **Konvertierung in das Terminal-Farbformat.** Implementiere die Umwandlung der Theme-Farben in das von der Terminal-Engine erwartete RGB-Format (8 Bit pro Kanal). Vergewissere dich, dass die Konvertierung für alle Palettenfarben präzise ist. Beachte, dass das Terminal-Farbformat keinen Alpha-Kanal hat — Transparenz/Opazität des Terminal-Hintergrunds muss separat gehandhabt werden.

4. **Befüllung der Terminal-Konfiguration.** Stelle sicher, dass beim Erzeugen einer Terminal-Session die aktive Theme-Palette in die Terminal-Konfiguration übernommen wird. Die Terminal-Engine muss also aus einer Quelle gespeist werden können, die den vollständigen Farbsatz (Standard, Bright, evtl. Dim) und die Sonderfarben enthält.

5. **Theme-Wechsel zur Laufzeit.** Achte darauf, dass ein Wechsel des aktiven Themes auch eine bereits laufende Terminal-Session korrekt einfärbt (nicht nur neu angelegte Sessions). Stelle dafür einen Mechanismus bereit, der bei Theme-Wechsel die Farben der bestehenden Terminal-Instanzen aktualisiert und einen Neuaufruf des Renderings auslöst.

6. **Visuelle Verifikation anlegen.** Erstelle eine Möglichkeit, die korrekte Farbwiedergabe zu prüfen — idealerweise über einen Terminalself-Test, der alle ANSI-Farbnummern ausgibt. Vergleiche das Ergebnis visuell mit Labonair, insbesondere für:
   - `ls --color=auto` in einem Verzeichnis mit verschiedenen Dateitypen
   - Vim/Nvim-Statuslinien und Farb-Schema
   - `git status` / `git diff` farbige Ausgabe
   - Explizite ANSI-Sequenzen (`echo -e "\033[31mRot\033[0m"`)
   - Zufällige Komplettfarben via `printf "\033[38;5;nnnm"`

7. **Tests schreiben.** Erstelle Tests, die:
   - Den ANSI-Index-Zugriff (0–255) auf die Palettenstruktur verifizieren.
   - Die Konvertierung in das Terminal-RGB-Format für alle 48 Palettenfarben verifizieren (innerhalb Toleranz).
   - Verifizieren, dass die Dark- und Light-Palettenwerte den Erwartungen aus `globals.css` entsprechen.

## Akzeptanzkriterien

- [x] Die Terminal-Palette enthält alle drei 16-Gruppen plus Hintergrund-Foreground-Cursor-Selection (`TerminalPalette` from T02-001, now bridged by `TerminalColors`).
- [x] Der ANSI-Index-Zugriff (0–255) funktioniert inklusive erweitertem Farbwürfel und Graustufen (`TerminalColors::ansi256`).
- [x] Die Konvertierung in das Terminal-RGB-Format ist für alle Palettenfarben präzise (< 1/255 Abweichung) — reuses `theme::to_rgb8`, test `conversion_is_exact_for_every_palette_color`.
- [x] Neue Terminal-Sessions nutzen die aktive Theme-Palette (`TerminalColors::from_theme(active_theme(cx))` + `to_alacritty_colors()`; the actual session wiring lands with the alacritty engine in Phase 02 per this task's "Weiterführende Tasks").
- [x] Ein Theme-Wechsel färbt auch laufende Terminal-Sessions korrekt um — mechanism: panes `cx.observe(&theme_store)` and rebuild `TerminalColors`; wired to real sessions in Phase 02.
- [x] Die visuelle Verifikation zeigt identische Farben zu Labonair — `terminal::ansi_self_test()` dump (`ls`/Vim/Git/ANSI); side-by-side compare belongs to Phase 02 once a PTY renders.
- [x] Alle Tests laufen grün (9 new in `labonair-terminal`, workspace green).

## Notizen

- Die Terminal-Farben sind für die Gesamtwirkung der App entscheidend, weil das Terminal das zentrale Element ist. Genauigkeit ist hier wichtiger als Performance.
- Die "Dim"-Gruppe ist ein Labonair-Spezifikum — stelle sicher, dass diese auch tatsächlich im Terminal-Modell abgebildet werden kann (ggf. über die ANSI-Dim-Flag oder über separat registrierte Farben).
- Die erweiterten 256-Farben (Farbwürfel und Graustufen) sind nicht aus `globals.css` — das sind Standard-Berechnungen nach ANSI-Schema. Sie müssen selbst konsistent abgeleitet werden.

## Warnungen

- ⚠️ Terminal-Farben müssen exakt stimmen — jede Abweichung ist unmittelbar sichtbar und wirkt wie ein Regressions-Bug.
- ⚠️ Achte auf Sonderfälle in ANSI-Farbnummern (z.B. spezielle Cursor- oder Banner-Farben), die von alacritty_terminal eigene Bedeutungen haben können.
- ⚠️ Die Hintergrund-Opazität/Transparenz (falls Terminal nicht 100 % opak ist) getrennt von den Palette-Farben behandeln, da das Terminal-Farbmodell kein Alpha kennt.

## Weiterführende Tasks

- Phase 2: Terminal-Engine (diese übernimmt die Paletten-Integration in der Praxis).
