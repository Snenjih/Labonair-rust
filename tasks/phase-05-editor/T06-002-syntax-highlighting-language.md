# T06-002: Syntax-Highlighting und Sprach-Erkennung

## Status
⏳ Pending

## Phase
5 — Editor

## Abhängigkeiten
T06-001 (Editor-Fundament)

## Ziel
Syntax-Highlighting für die unterstützten Programmiersprachen implementieren, mit automatischer Sprach-Erkennung anhand der Dateierweiterung sowie Editor-Themes (Farbschemas), die dem App-Theme folgen. Ziel ist ein optisch ansprechendes Highlighting, das vergleichbar mit den CodeMirror-Modi in Labonair ist.

## Kontext
Labonair's Editor unterstützt viele Sprachen (C/C++, CSS, Go, HTML, Java, JavaScript, JSON, Markdown, PHP, Python, Rust, SQL, XML u.a.) über CodeMirror-Lang-Pakete, und mehrere Editor-Themes (Atomone, Aura, Copilot, Github, Nord, TokyoNight, Xcode). Die Syntax-Highlighting wird im Editor über Sprach-Regeln realisiert.

In der Rust/GPUI-Welt ist der Standard-Ansatz Tree-Sitter (Grammar-basierte Parsing), das auch Zed nutzt. Tree-Sitter-Grammatiken stehen für die meisten gängigen Sprachen zur Verfügung. Über die Editor-Engine (aus T06-001 kommen TreeSitter-Fähigkeiten) soll ein Highlighting auf Basis der Tree-Sitter-Syntax-Captures realisiert werden.

Die Editor-Themes (Farbschemas) müssen dem Benutzer-Theme folgen — d.h. auch bei Theme-Wechsel dynamisch aktualisieren.

## Anweisungen zur Umsetzung

1. **Sprach-Erkennung.** Implementiere das Zuordnen einer Datei zu einer Sprache anhand der Erweiterung (und ggf. Dateiname/Schema). Lege die Liste der unterstützten Sprachen fest (orientiere dich an Labonair's aktueller Liste). Fehlt eine Zuordnung, mindestens ein sinnvoller Fallback (kein Highlighting, oder heuristisch ermittelt).

2. **Tree-Sitter-Grammatiken einbinden.** Binde die Tree-Sitter-Grammatiken für die unterstützten Sprachen ein (als Teil der Editor-Engine oder als separate Abhängigkeit). Stelle sicher, dass die Grammatiken kompiliert und zur Laufzeit verfügbar sind — Lazy-Load-Grammatiken nach Bedarf, um die Startzeit zu schonen.

3. **Syntax-Captures auf Highlighting mappen.** Setze die Tree-Sitter-Syntax-Knoten (Captures) in Farb-Attribute um:
   - Verschiedene Token-Typen (Schlüsselwörter, Strings, Kommentare, Funktionen, Typen, Makros, Konstanten, Variablen, Operatoren usw.) erhalten verschiedene Farben.
   - Ein Mapping-Konfigurationsschema festlegen, das die Farben auf die Theme-Tokens bezieht.

4. **Editor-Themen-System.** Baue das Farbschema-System:
   - Mehrere Editor-Themes (dem Labonair-Satz nachempfunden: dark/light Varianten).
   - Die Editor-Themes müssen mit dem App-Theme kompatibel sein (auch semantisch: Kommentare, Schlüsselwörter etc. in einer Weise, die gut zum Hintergrund passt).
   - Theme-Wechsel beim Benutzer (Farbumgebung) soll die Editor-Farben entsprechend aktualisieren.

5. **Inkrementelles/Viewport-gebundenes Highlighting.** Implementiere Highlighting, das nur für sichtbare/geänderte Bereiche neu berechnet wird (nicht das ganze Dokument bei jeder Tippe), für flüssiges Tippen in großen Dateien.

6. **Weitere Sprach-Features (dezentral).** Falls die Engine Unterstützung bietet: Heuristiken für vertiefte Tokenisierung (z.B. Bracket-Matching-Hervorhebung, Error-Hervorhebung), die später erweitert werden können.

7. **Tests schreiben.** Erstelle Tests für:
   - Sprach-Erkennung anhand von Erweiterungen (Tabelle testen).
   - Korrekte Highlighting-Tokens für repräsentative Snippets pro Sprache (zumindest grundlegendes Mapping prüfbar).
   - Theme-Wechsel aktualisiert die Editor-Farben.
   - Inkrementelles Highlighting funktioniert (nur geänderte Bereiche).

## Akzeptanzkriterien

- [ ] Unterstützte Sprachen werden anhand der Dateierweiterung korrekt erkannt.
- [ ] Syntax-Highlighting auf Tree-Sitter-Basis funktioniert (Tokens farblich korrekt).
- [ ] Ein Editor-Themen-Satz (dem Labonair-Satz nachempfunden) existiert, kompatibel mit dem App-Theme.
- [ ] Editor-Farben aktualisieren sich bei Theme-Wechsel.
- [ ] Highlighting wird nur für sichtbare/geänderte Bereiche berechnet (performant bei großen Dateien).
- [ ] Alle Tests laufen grün.

## Notizen

- Die Liste der Sprachen von Labonair ist die Zielvorgabe; nicht zu viele, aber genug, um den realen Bedarf abzudecken.
- Tree-Sitter-Captures sind der flexible Weg — das Mapping auf Farben ist der eigentliche Wert.
- Editor-Themes sind separate Farbpaletten (nicht identisch mit dem gesamten App-Theme), aber harmonisch dazu.

## Warnungen

- ⚠️ Tree-Sitter-Parsing kann bei großen Dateien teuer sein — unbedingt inkrementell und viewportbasiert, sonst merkt der Nutzer Lag.
- ⚠️ Neue Sprachen später einfach ergänzbar halten (Erweiterung + Grammar-Einbindung), ohne die gesamte Editor-Logik zu ändern.

## Weiterführende Tasks

- [T06-003: Vim-Modus](./T06-003-vim-mode.md)
- [T06-004: Diff-Ansicht](./T06-004-diff-view.md)
