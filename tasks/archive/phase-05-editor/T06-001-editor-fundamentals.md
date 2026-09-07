# T06-001: Editor-Fundament und Datei-Öffnen/Speichern

## Status
✅ Done

## Phase
5 — Editor

## Abhängigkeiten
T04-001/2 (Tab-System, Pane-Inhalte)
T05-001 (Explorer öffnet Dateien)

## Ziel
Einen funktionsfähigen Texteditor als Inhaltstyp für Pane- und Editor-Tabs implementieren. Der Editor soll Textdateien öffnen, anzeigen, bearbeiten und speichern können, mit einem Dirty-Zustand (ungespeicherte Änderungen), Undo/Redo, Grundfunktionen (Find/Replace, Zeilennummern, Umbruch), und korrekter Integration in Tab- und Workspace-System.

## Kontext
In Labonair ist der Editor CodeMirror 6-basiert und unterstützt viele Sprachmodi, Themen, Vim-Modus, AI-Autovervollständigung und Diff-Ansichten. Für die Rust-Portierung ist die realistische Basis ein Tree-Sitter-basierter Editor (die `gpui`-Welt bzw. `gpui-component` bietet einen Code-Editor mit TreeSitter + LSP-Ansatz). Ziel ist ein Editor, der zunächst die Kernfunktionen abdeckt und in späteren Tasks (Syntax-Highlighting, Vim, Diff, AI-Autocomplete) erweitert wird.

Dieser Task legt das Editor-Fundament: Datei-Öffnen, Anzeigen, Bearbeiten, Speichern, Dirty-State, Undo/Redo, Tab-Integration.

## Anweisungen zur Umsetzung

1. **Editor-Engine auswählen/einbinden.** Verwende eine geeignete, in GPUI verfügbare Editor-Engine (TreeSitter-basiert) oder eigene simplere Editor-Logik als Basis. Binde sie als Abhängigkeit/Modul ein und stelle die Kern-Kapazitäten bereit: Textpuffer, Cursor, Auswahl, Einfügen/Löschen, Undo/Redo.

2. **Datei öffnen.** Implementiere das Öffnen einer Datei:
   - Dateipfad als Eingabe (aus Explorer, Kommandozeile, Drop).
   - Datei lesen (Text mit korrekter Encoding-Erkennung; UTF-8 primär).
   - Große Dateien: überlegt laden/rendern (nicht das ganze Dokument in ein einzelnes langsames Rendering).
   - Editor-View als Pane-Inhalt bzw. Editor-Tab anlegen.

3. **Editor-View rendern.** Baue die Editor-Anzeige:
   - Zeilennummern (links), aktuelle Zeile hervorgehoben.
   - Zeilenumbruch-Anzeige mit horizontalem Scrollen.
   - Cursor und Auswahl darstellen.
   - Ampelfarbige Syntax-Highlighting ist ein Folge-Task; hier zunächst einfache Text-Darstellung (aber Architektur für Highlighting vorbereiten).
   - Font/Theme aus der App-Theme nutzen (vgl. T02-xxx).

4. **Bearbeitung und Undo/Redo.** Implementiere:
   - Einfügen/Löschen von Text an Cursor/Auswahl.
   - Cursor-Navigation über Tastatur/Maus.
   - Undo/Redo (mit Verknüpfung, auch über File-Operationen hinweg).
   - Tastaturkürzel: Strg+S (Speichern), etc.

5. **Dirty-Zustand.** Verfolge ungespeicherte Änderungen:
   - "Dirty"-Flag im Tab markieren (* im Titel, Punkt-Indikator in der Tab-Leiste).
   - Beim Schließen eines Dirty-Tabs: Rückfrage (Speichern/Nicht speichern/Abbrechen), siehe T04-001.
   - Beim Bearbeiten die Saved-Version tracken für Diff-Zwecke (später).

6. **Speichern.** Implementiere:
   - Speichern (Strg+S) schreibt den Puffer zurück auf die Datei.
   - "Speichern unter" (Zieldatei wählen) optional.
   - Fehlerbehandlung (Datei gelöscht, Schreibrechte) mit klarer Meldung.
   - Nach dem Speichern Dirty-Flag zurücksetzen und ggf. externe Watcher benachrichtigen.

7. **Find/Replace.** Implementiere eine Grundversion von Suchen/Ersetzen:
   - Suchleiste (öffnen per Shortcut Cmd+F / Strg+F), Treffer hervorheben, Markieren/Navigieren.
   - Ersetzen (einzeln/alle).
   - Groß-/Kleinschreibung und ganze-Wort-Optionen.

8. **Tab-/Pane-Integration.** Der Editor-Abschnitt wird als Inhalt eines Pane-Leafs bzw. eines Editor-Tabs gehostet:
   - Öffnen mehrerer Dateien → mehrere Editor-Tabs (mit Dirty-Status pro Tab).
   - Vorschau-Tab (peek) für temporär geöffnete Dateien (analog VS Code / Labonair): beim Klick auf andere Datei ersetzt der Vorschau-Tab den Inhalt; Doppelklick/Edit macht den Tab dauerhaft.
   - Editor-Tab lädt den Inhalt neu, wenn er aktiv wird (falls Datei geändert wurde von außen), mit ggf. Konflikt-Hinweis.

9. **Tests schreiben.** Erstelle Tests für:
   - Öffnen/Speichern von Dateien (mit Temp-Verzeichnis).
   - Dirty-Zustand korrekt setzen/zurücksetzen.
   - Undo/Redo-Logik.
   - Find/Replace-Verhalten.
   - Vorschau-Tab-Semantik.

## Akzeptanzkriterien

- [ ] Editor-Engine ist eingebunden und modal.
- [ ] Dateien lassen sich öffnen, anzeigen und bearbeiten.
- [ ] Dirty-Zustand wird in der Tab-Leiste angezeigt; Schließen fragt bei ungespeicherten Änderungen nach.
- [ ] Speichern/Unter funktioniert, mit korrekter Fehlerbehandlung.
- [ ] Undo/Redo funktionieren.
- [ ] Einfaches Find/Replace funktioniert.
- [ ] Daten lassen über Explorer geöffnete Dateien in Editor-Tabs; Vorschau-Tab-Semantik funktioniert.
- [ ] Zeilennummern, Cursor, Auswahl und Scroll-verhalten sind korrekt.
- [ ] Alle Tests laufen grün.

## Notizen

- Syntax-Highlighting ist bewusst in einem separaten Task (T06-002), aber die Architektur sollte es von Anfang an ermöglichen (Sprach-Erkennung vorbereiten).
- Die Vorschau-Tab-Semantik (peek) ist ein markantes Labonair/VS-Code-Feature und sollte korrekt umgesetzt werden.
- Encoding: Primär UTF-8; andere Encodings (Latin-1 u.a.) bei Bedarf später.

## Warnungen

- ⚠️ Große Dateien ohne caching können das Rendering einfrieren — nur sichtbare Bereiche rendern (Viewport-basiert).
- ⚠️ Dirty-State und extern geänderte Dateien können zu Datenverlust-Konflikten führen — sauberes Handling einplanen (Reload-Hinweis vor Überschreiben).

## Weiterführende Tasks

- [T06-002: Syntax-Highlighting und Sprach-Erkennung](./T06-002-syntax-highlighting-language.md)
- [T06-003: Vim-Modus](./T06-003-vim-mode.md)
- [T06-004: Diff-Ansicht](./T06-004-diff-view.md)
