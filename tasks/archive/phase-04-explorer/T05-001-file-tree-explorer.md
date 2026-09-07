# T05-001: Dateibaum und Datei-Explorer-Grundlagen

## Status
✅ Done

## Phase
4 — File-Explorer

## Abhängigkeiten
T04-002 (Split-Pane-Workspace, Sidebar)

## Ziel
Einen lokalen Datei-Explorer in der Sidebar implementieren, der den Dateibaum des aktuell geöffneten Arbeitsverzeichnisses (oder einer gewählten Route) anzeigt: Dateien/Ordner auf-/zuklappen, klicken um Dateien zu öffnen (Editor) oder Ordner zu betreten, Inline-Umbenennen, Neu-/Löschen-Kontextaktionen, sowie Ladezustände beim Navigieren. Dazu gehört die Erkennung des "Arbeits-CWD" (Wurzel), die auch von Terminal-Aktivitäten abhängt.

## Kontext
In Labonair ist der Datei-Explorer eine Seitenleiste mit einem Dateibaum. Die Wurzel (Root) hängt vom "Workspace-CWD" ab — z.B. das Arbeitsverzeichnis, aus dem ein Terminal gestartet wurde. Der Explorer zeigt Ordner aufklappbar, Dateien klickbar (öffnen Editor), und bietet Kontextaktionen (neue Datei/Ordner, umbenennen, löschen, etc.). Er nutzt `@tanstack/react-virtual` für große Verzeichnislisten und einen asynchronen, generationbasierten Lade-Mechanismus mit Deduplication und Lade-Indikatoren.

In der Rust-App wird der Dateibaum in der Sidebar (aus T04-002) gerendert. Die Dateisystem-Logik läuft direkt über Rust-FS-Aufrufe (kein Tauri-IPC).

## Anweisungen zur Umsetzung

1. **Wurzel (Root) bestimmen.** Implementiere die Bestimmung des aktuellen Arbeitsverzeichnisses:
   - Beim Start ein sinnvolles Initialverzeichnis wählen (z.B. Home oder zuletzt genutzt).
   - Das Arbeitsverzeichnis kann sich durch Aktivität ändern (z.B. neues Terminal im Ordner X → Explorer zeigt X). Die Abhängigkeit vom Terminal-CWD (T03-004) herstellen.

2. **Verzeichnis-Lesen und Baumaufbau.** Implementiere das Einlesen eines Verzeichnisses:
   - Unterordner und Dateien auflisten (sortiert, Verzeichnisse zuerst).
   - Versteckte Dateien standardmäßig ausblenden (mit Toggle).
   - Ladezustände (lädt/langsam/geladen) pro Knoten anzeigen.
   - Große Verzeichnisse effizient behandeln (Lazy-Load: nur sichtbare Knoten laden, nicht das ganze Baum im Voraus).

3. **Baum-UI rendern.** Baue die Baum-Ansicht:
   - Auf-/Zuklappen von Ordnern (mit Chevron-Indikator und Animation).
   - Auswahl eines Knotens hervorheben.
   - Einrückung basierend auf Baumtiefe.
   - Virtualisiertes Scrollen für große Bäume (nur sichtbare Zeilen rendern).
   - Datei- und Ordner-Icons anhand von Dateitypen (Material-Icons-Stil, analog Labonair).

4. **Klick-Interaktionen.** Implementiere:
   - Klick auf Ordner → Auf-/Zuklappen (bzw. in neuen Editor unter bestimmten Bedingungen).
   - Klick auf Datei → Editor öffnen (Editor kommt in Phase 5; bis dahin Platzhalter oder einfache Textvorschau).
   - Doppelklick auf Ordner → in Verzeichnis navigieren (Root wechseln) oder aufklappen je nach Konvention.

5. **Kontextaktionen.** Implementiere das Kontextmenü (Rechtsklick) pro Knoten mit Aktionen:
   - Neue Datei, Neuer Ordner (im aktuellen Verzeichnis, mit Inline-Namenseingabe).
   - Umbenennen (Inline-Eingabe).
   - Löschen (mit Bestätigung).
   - Kopieren/Einfügen/Ausschneiden (Grundgerüst; DnD selbst in T05-002).
   - In Terminal öffnen (CWD setzen).
   - Datei/Ordner-Pfad kopieren.

6. **Dateioperationen anbinden.** Verdrahte die Dateioperationen (erstellen, umbenennen, löschen) mit den direkten FS-Rust-Aufrufen. Diese müssen robust sein: Fehler (Permission denied, Datei existiert) mit klaren Meldungen im UI, nicht als Crash.

7. **Aktualisierung bei Änderungen.** Sorge dafür, dass sich der Baum aktualisiert, wenn:
   - Das zugrunde liegende Dateisystem sich ändert (Datei erstellt/gelöscht von extern) — über einen Dateibeobachter (Watcher).
   - Daten nach einer Operation veraltet sind (nach Umbenennen/Löschen neu laden).

8. **Seitenleisten-Integration.** Die Explorer-Ansicht wird in der Sidebar angezeigt (aus T04-002) und ist über Tab/Shortcut umschaltbar (z.B. selbst bei Explorer/SFTP/Home-Tabs). Stellen, wo nur Home (Host-Liste) gezeigt wird, sind später zu klären.

9. **Tests schreiben.** Erstelle Tests für:
   - Korrektes Auflisten und Sortieren.
   - Lazy-Load-Logik (nur sichtbare laden).
   - Inline-Umbenennen/Neuanlage/Delete-Aktionen (mit Mock-FS oder Temp-Verzeichnis).
   - Verzeichniswechsel (Root-Änderung) und Baum-Erneuerung.
   - Versteckte-Dateien-Toggle.

## Akzeptanzkriterien

- [ ] Der Explorer zeigt den Dateibaum des Arbeitsverzeichnisses mit Auf-/Zuklappen.
- [ ] Wurzel hängt am Arbeits-CWD (und reagiert auf Terminal-CWD-Änderungen).
- [ ] Verzeichnis wird effizient (lazy) geladen, versteckte Dateien-Toggle funktioniert.
- [ ] Klick auf Datei öffnet einen Editor (bis Phase 5 Platzhalter/Textvorschau).
- [ ] Kontextmenü mit Neuanlage, Umbenennen (Inline), Löschen (Bestätigung), Pfad kopieren, In-Terminal-Öffnen funktioniert.
- [ ] Der Baum aktualisiert sich bei externen FS-Änderungen (Watcher) und nach eigenen Operationen.
- [ ] Der Explorer ist in der Sidebar integriert und sichtbar/ausblendbar.
- [ ] Alle Tests laufen grün.

## Notizen

- Das virtuelle Scrollen ist wichtig für große Verzeichnisse — implementiere es grundlegend, um spätere Skalierung zu ermöglichen.
- Die Icons: Reproduziere den Standard der Dateityp-Icons (Material-Icons-Stil) so nah wie möglich. Eine Icon-Quelle in GPUI (z.B. ein verfügbares Icon-Set) einbinden.
- Die Fehlerbehandlung (Permission etc.) sollte dem Stil der App folgen: klare, freundliche Meldungen statt technische Stacktraces.

## Warnungen

- ⚠️ Lazy-Load und Cache: Beim Zurückkehren zu einem bereits geladenen Ordner nicht unnötig neu laden — aber nach Operationen korrekt neu errüschen (Stale-Guard gegen Race-Conditions).
- ⚠️ Externe FS-Änderungen (Watcher) können bei manchen Dateien (temporär) viele Events auslösen — deduplizieren/throtteln, um Überlastung zu vermeiden.

## Weiterführende Tasks

- [T05-002: Drag-and-Drop und erweiterte Dateiaktionen](./T05-002-drag-drop-actions.md)
- Phase 5: Editor (öffnet Dateien aus dem Explorer)
