# T03-002: GPUI-Terminal-Renderer für Zellen bauen

## Status
✅ Done

## Phase
2 — Terminal-Engine

## Abhängigkeiten
T03-001 (alacritty_terminal einbinden)
T01-001 (Cargo Workspace)
T02-004 (Terminal-Palette)

## Ziel
Einen GPUI-Renderer bauen, der die Zell-Daten der Terminal-Engine (aus T03-001) als sichtbare Terminal-Oberfläche darstellt. Das Ergebnis ist ein anzufassendes Terminal-Element in GPUI, das eine laufende Shell anzeigt, auf Ein-/Ausgabe reagiert, korrekt skaliert und die Farb- und Font-Eigenschaften aus dem Theme nutzt.

## Kontext
Im T03-001 wurde die Terminal-Logik aufgebaut: PTY + `alacritty_terminal` + eine Zell-Daten-Schnittstelle. Jetzt kommt die Darstellung. Es gilt, ein GPUI-UI-Element zu implementieren, das:

- für jede Zeile und Spalte die Zellen zeichnet (Zeichen + Vordergrund-/Hintergrundfarbe + Attribute),
- Font mit korrekter Breite verwendet und monospace-Glyphen samt einiger Standard-Blockelemente (▄▀█ etc.) darstellt,
- Cursor (Block, Strich oder Unterstrich, je nach Modus) und Auswahl-Hervorhebung zeichnet,
- Scrollback anzeigt und scrollbar ist,
- auf Terminal-Wakeup-Ereignisse neu zeichnet,
- Resize (und damit die Zell-Größe) an die Fenstergröße anpasst.

Das ist der Kern der gesamten Terminal-Erfahrung und der wichtigste Einzelbaustein der App. Qualität hier entscheidet über die gesamte Wahrnehmung.

## Anweisungen zur Umsetzung

1. **Zell-Rendering-Grundlagen.** Implementiere das Zeichnen einer Zelle:
   - Bestimme für die aktuelle Terminal-Konfiguration die Zell-Breite und -Höhe (basierend auf Font-Metrik).
   - Zeichne pro Zelle den Hintergrund (rechteckige Füllung, ggf. mit einer Farbe).
   - Zeichne das Zeichen im Vordergrund (mit korrekter monospace-Shaping und ggf. Ligaturen).
   - Berücksichtige Attribute: Fett, Kursiv, Unterstrichen, Durchgestrichen, blinken, versteckt, umgekehrt (reverse).
   - Bildschirm-Randleffekte (farbige Rahmen) berücksichtigen, wo relevant.

2. **Zeilen-Batching für Performance.** Statt pro Zelle einzeln zu zeichnen, gruppiere benachbarte Zellen mit identischem Stil (gleiche Vordergrund-, gleiche Hintergrundfarbe, gleiche Attribute) zu Textläufen. Das reduziert die Anzahl der Zeichenaufrufe deutlich und ist essenziell für flüssiges Scrollen bei viel Ausgabe.

3. **Cursor und Auswahl.** Zeichne:
   - Den Terminal-Cursor gemäß Cursor-Modus (Block/Strich/Unterstrich, umgekehrt bei Insert-Modus), mit der Cursor-Farbe aus dem Theme.
   - Die Textauswahl (markierter Bereich) mit der Selektionsfarbe.
   - Ggf. die Suchtreffer-Hervorhebung (falls später implementiert).

4. **Scrollback-Anzeige.** Baue Unterstützung ein, um über den Scrollback zu scrollen:
   - Mausrad-/Touchpad-Scrolling.
   - Scroll-indikatoren / Scrollbar am Rand.
   - Rücksetzen des Scrolls zum aktuellen Cursor (z.B. durch Tippen oder spezifische Tasten).
   - Anzeige korrekt Ackordreitung des sichtbaren Bereichs über Scrollback + aktuellen Bildschirm.

5. **Font-Verwaltung.** Übernimm die Monospace- und die Font-Auswahl aus dem Theme bzw. der App-Konfiguration:
   - Monospace-Font für Terminal-Zellen (mit korrekten Maßen und ggf. Ligaturen).
   - Font-Fallback bei fehlenden Glyphen (insb. um CJK/Unicode und Blockelemente korrekt darzustellen).
   - Anwendbar über das GPUI-Textsystem, das die os-seitige Schriftformung nutzt.

6. **Resize-Verhalten.** Die Terminal-Zell-Anzahl (Columns × Rows) muss sich aus der Größe des UI-Elements ableiten:
   - Beim Ändern der Fenster-Größe die maximale ganzzahlige Spalten-/Zeilen-Anzahl berechnen, die in die verfügbare Fläche passt.
   - Diese Dimensionen an die Terminal-Engine (T03-001) übergeben, damit der PTY die Größe erfährt.
   - Das Rendering entsprechend neu erfolgt.

7. **Ereignis-Integration.** Stelle sicher, dass das GPUI-Element auf Terminal-Wakeup- und Datenänderungen (aus dem Ereignissystem von T03-001) reagiert und sich nur neu zeichnet, wenn sich tatsächlich etwas geändert hat (nicht jeden Frames-Trivial-Re-Render). Ideal für eingebautes Dmg-Tracking der Terminal-Engine.

8. **Einfache Interaktion.** Stelle eine minimale Ein-/Ausgabe-Interaktion sicher: Das Element nimmt Tastatureingaben an (füttert sie an die Engine → PTY) und leitet Mausereignisse (Klick auf Zelle setzt Cursor/Selektion) an die Terminal-Logik. Die vollständige Tastatur-/Maus-Zuordnung (inkl. Modifier, Scroll, Drag-Auswahl) folgt im nächsten Task — hier reicht die grundlegende Datenbahn.

9. **Beispiel-Terminal öffnen.** Ist das Element fertig, soll ein einfacher Test es ermöglichen, ein Terminal als erstes interaktives Fenster in der App zu öffnen: eine echte Shell, in die man tippen kann und die Ausgabe anzeigt.

10. **Tests schreiben.** Verifiziere das Rendering-Verhalten:
   - Korrekte Zell-Größen-Berechnung aus Font-Metriken.
   - Korrektes Zeilen-Batching (identischer Stil → ein Lauf).
   - Resize-Berechnung (Columns/Rows aus Pixelgröße).
   - Farbe-Übernahme aus der Theme-Palette korrekt auf Zellen.

## Akzeptanzkriterien

- [ ] Ein GPUI-Element zeigt eine laufende Shell korrekt an (Zeichen + Farben + Attribute).
- [ ] Benachbarte Zellen gleichen Stils werden als ein Textlauf gebatcht (Performance-Grundlage).
- [ ] Cursor (Block/Strich/Unterstrich) und Auswahl werden korrekt gezeichnet.
- [ ] Über den Scrollback kann gescrollt werden (Mausrad/Touchpad), mit Rücksetz-Funktion.
- [ ] Fonts inkl. Fallback und Ligaturen funktionieren korrekt.
- [ ] Resize passt Spalten-/Zeilenanzahl an die Fenster-Größe an und informiert die Terminal-Engine.
- [ ] Das Element reagiert nur bei tatsächlichen Änderungen neu (keine verschwenderischen Re-Renders).
- [ ] Eine echte Shell startet im Terminals-Element und ist interaktiv bedienbar.
- [ ] Die Zell-Farben entsprechen exakt der Theme-Palette.
- [ ] Alle Tests laufen grün.

## Notizen

- Das Zeilen-Batching ist der wichtigste Performance-Treiber. Investiere darin.
- Die Font-Metrik-Aufgabe (Zell-Breite als Pixel-Wert aus Font-Shaping) ist kritisch für die exakte Ausrichtung — versuche es möglichst genau, ggf. durch die GPUI-Textsystem-Berechnungen.
- GPU-typische Effekte wie Subpixel-Antialiasing und Ligaturen überlässt du dem GPUI-Textsystem.

## Warnungen

- ⚠️ Vernachlässige nicht das Scrollback-Rendering — viele Terminal-Texte sind länger als die sichtbaren Zeilen und ohne Scrollback wirkt die App eingeschränkt.
- ⚠️ Vermeide, das gesamte Terminal bei kleinen Änderungen neu zu zeichnen — nutze das vorhandene Muss-Tracking und zeichne nur geänderte Bereiche.
- ⚠️ CJK- und Breitzeichen (doppelte Spaltenbreite) korrekt behandeln — das GPUI-Textsystem übernimmt die Breitenberechnung, aber du musst sie bei der Zell-Positionierung berücksichtigen.

## Weiterführende Tasks

- [T03-003: Tastatur- und Maus-Mapping](./T03-003-keyboard-mouse-mapping.md)
- [T03-004: Shell-Integration und CWD-Tracking](./T03-004-shell-integration-cwd.md)
