# T06-004: Diff-Ansicht

## Status
✅ Done

## Phase
5 — Editor

## Abhängigkeiten
T06-001 (Editor-Fundament)

## Ziel
Eine Diff-Ansicht im Editor implementieren, die zwei Textversionen vergleicht und die Unterschiede (eingefügt, gelöscht, geändert) farblich darstellt — sowohl als einheitliche (unified) als auch als nebeneinander (side-by-side) Darstellung, soweit realistisch. Diese Ansicht wird später von der Git-Integration (Phase 8) und der AI-Diff-Funktion (Phase 10) wiederverwendet.

## Kontext
In Labonair gibt es mehrere Diff-Funktionen:
- AI-Diff-Panes: zeigen, welche Änderungen eine AI vorgeschlagen hat (Original vs. Propose).
- Git-Diff-Panes: zeigen Arbeitsverzeichnis vs. Index/HEAD (aus Phase 8).

Eine gemeinsame Diff-Ansicht-Komponente, die zwei Textströme vergleicht und den Unterschied farblich zeigt, ist die Grundlage für all das. Sie wird hier in der Kernform gebaut und später von den Git-/AI-Features verdrahtet.

## Anweisungen zur Umsetzung

1. **Diff-Algorithmus.** Implementiere oder binde einen Diff-Algorithmus ein (z.B. Myers-Algorithmus oder eine Library), der zwei Textstrings in Hunks (Änderungsbereiche) aufteilt. Die Ausgabe ist eine strukturierte Liste von Zeilen mit Status: unverändert, eingefügt, gelöscht, geändert.

2. **Unified-Diff-Darstellung.** Implementiere die einheitliche Ansicht:
   - Zeilen beider Versionen nebeneinander mit jeweiliger Zeilennummer.
   - Ein-/Mehrfach-Hunks farblich hervorheben (Einfügungen grün/akzentuiert, Löschungen rot/negativ, geändert kombiniert).
   - Auslassungspunkte (…) für entfernte Hunks, aufklappbar.

3. **Side-by-Side-Darstellung.** Implementiere die zweispaltige Ansicht (links alt, rechts neu), mit:
   - Synchrone Zeilennummern und vertikales Scrollen beider Seiten.
   - Farblich gekennzeichnete Einfügungs-/Löschungszellen.
   - Verbindungs aneinanderliegende Zeilen (für changed).

4. **Interaktion.** Implementiere:
   - Navigation zwischen Hunks (Tastatur, Buttons).
   - Hunk-Hover/Highlight.
   - (Sofern relevant für Git-Phase) Staging von Hunks — das kann als Hook/Schnittstelle vorbereitet werden, aber die eigentliche Staging-Logik kommt in Phase 8.

5. **Themen-Integration.** Verwende die Theme-Farben für Diff-Markierungen (aus der semantischen Farbpalette, z.B. `color_modified`, `color_error`, `color_success` passend). Die Diff-Farben sollen sich bei Theme-Wechsel aktualisieren.

6. **Wiederverwendbare Komponente.** Strukturiere die Diff-Ansicht so, dass sie von mehreren Aufrufern genutzt werden kann:
   - Eingabe: zwei Textinhalte (Original, Neu) + optionale Kontext-Information.
   - Ausgabe: eine renderbare Diff-View.
   - API soll später von Git (Phase 8) und AI (Phase 10) einfach aufgerufen werden.

7. **Tests schreiben.** Erstelle Tests für:
   - Diff-Algorithmus liefert korrekte Hunks für repräsentative Beispiele.
   - Einfügungen/Löschungen/Änderungen werden korrekt erkannt.
   - Das Rendering erzeugt die richtigen Zeilennummern und Farbzuordnungen.
   - Hunk-Navigation funktioniert.

## Akzeptanzkriterien

- [ ] Der Diff-Algorithmus teilt zwei Texte in korrekte Hunks auf.
- [ ] Die Unified-Ansicht zeigt beide Versionen mit farblichen Einfügungs-/Löschungs-Markierungen und Auslassungspunkten.
- [ ] Die Side-by-Side-Ansicht synchron scrollt und beide Spalten korrekt einfärbt.
- [ ] Hunk-Navigation (Tastatur/Buttons) funktioniert.
- [ ] Diff-Farben stammen aus dem Theme und aktualisieren sich bei Theme-Wechsel.
- [ ] Die Komponente ist wiederverwendbar via klarer API (Text in → View out).
- [ ] Alle Tests laufen grün.

## Notizen

- Diese Komponente ist die konzeptionelle Grundlage für Git- und AI-Diffs. Baue sie sauber, da sie mehrfach wiederverwendet wird.
- Für sehr große Diffs ist die Perf wichtig — rendition-basiertes Rendering (nur sichtbare Hunks) berücksichtigen.

## Warnungen

- ⚠️ Side-by-Side bei sehr unterschiedlichen Dateien (starke Verschiebungen) kann verwirrend sein — der Diff-Algorithmus sollte gute Zuordnung (Minimierung der Änderungszeilen) leisten.
- ⚠️ Nicht das Web-GIT-Diff-Format als interne Darstellung zwingen; die interne Struktur soll manipulationsfreundlich sein (Zeilenlists mit Status).

## Weiterführende Tasks

- Phase 8: Git-UI & Source-Control (nutzt diese Diff-Ansicht für Arbeitsverzeichnis-Änderungen)
- Phase 10: AI-Diff (nutzt diese Ansicht für AI-Vorschläge)
