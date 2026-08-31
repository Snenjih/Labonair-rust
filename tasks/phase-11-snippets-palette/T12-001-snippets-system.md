# T12-001: Befehl-Snippets-System

## Status
⏳ Pending

## Phase
11 — Snippets & Command-Palette

## Abhängigkeiten
T04-001 (Tab-System, Pane)
T07-001 (SSH — für SSH-Ausführung)
T03-001 (Terminal — für lokale Ausführung)

## Ziel
Das Befehl-Snippets-System implementieren: wiederverwendbare Kommando-Snippets verwalten (CRUD), optional mit Variablen-Prompts, und sie entweder **lokal** (in einem Terminal) oder **über SSH** (auf einem Host) ausführen, mit Ausführungslog.

## Kontext
Labonair hat Snippets:
- Wiederverwendbare Kommando-Vorlagen mit Titel, Beschreibung, Inhalt (das Kommando), Gruppen.
- Optional mit **Variablen-Prompts**: Wenn das Kommando Prompts enthält, wird vor dem Ausführen ein Dialog angezeigt, der nach Werten fragt.
- Ausführung **lokal** (in einer lokalen Terminal-Session, ggf. in einem neuen Terminal) oder **über SSH** (auf gewähltem Host/verbundener Session), wobei der Benutzer den Ziel-Host wählt.
- Ein **Ausführungslog** (Drawer), das abgeschlossene Snippets-Ausführungen und deren Output zeigt.

Das Backend (T01-002) stellt die Snippet-CRUD- und Ausführungs-Logik bereit (lokal/SSH).

## Anweisungen zur Umsetzung

1. **Snippet-Modell und Verwaltung.** Übernimm das Snippet-Datenmodell aus dem Backend: Titel, Inhalt, Beschreibung, Gruppe, Reihenfolge. Implementiere CRUD sowie Gruppen-Verwaltung und Reihenfolge (reorder) im UI.

2. **Snippets-Panel.** Baue die Snippets-Oberfläche (Sidebar-Bereich oder eigenes Panel):
   - Liste der Snippets, gruppiert nach Gruppen (auf-/zuklappbar).
   - Suche/Filter.
   - Snippet erstellen/bearbeiten/löschen (Formular: Titel, Inhalt, Beschreibung, Gruppe).
   - Ausführen-Buttons (lokal / SSH).

3. **Variablen-Prompts.** Implementiere die Variablen-Erkennung und -Prompts:
   - Im Snippet-Inhalt Platzhalter extrahieren (z.B. Syntax, die Labonair nutzt).
   - Vor der Ausführung einen Dialog anzeigen, der nach Werten für jeden Platzhalter fragt.
   - Die eingegebenen Werte in das Kommando einsetzen (defensiv escapen).

4. **Lokale Ausführung.** Implementiere das Ausführen eines Snippets lokal:
   - Ein neues Terminal öffnen (oder eine bestehende Session verwenden, je nach Konfiguration) im passenden Arbeitsverzeichnis.
   - Das aufgelöste Kommando dort absetzen.
   - Den Output im Terminal sichtbar machen; optional in das Ausführungslog aufnehmen.

5. **SSH-Ausführung.** Implementiere das Ausführen über SSH:
   - Ziel-Host wählen (Dialog mit Host-Auswahl / verbundene Sessions).
   - Verbinden (falls nötig) bzw. eine bestehende SSH-Session nutzen.
   - Das Kommando auf dem entfernten Host ausführen, Output anzeigen (Terminal oder Log).
   - Fehlerbehandlung (Verbindung, Auth).

6. **Ausführungslog (Drawer).** Implementiere ein Log, das Snippet-Ausführungen dokumentiert:
   - Einträge: Snippet, Ausführungsart (lokal/SSH), Ziel, Zeit, Output (gekürzt bzw. aufklappbar), Status (Erfolg/Fehler).
   - Öffnen/Schließen des Log-Drawers, Inhalte durchblättern.
   - Clear-Log-Funktion.

7. **Tests schreiben.** Erstelle Tests für:
   - Snippet-CRUD und Gruppen/Reorder.
   - Variablen-Extraktion und -Ersetzung (inkl. Escaping).
   - Lokale Ausführung (Mock-Terminal-Session).
   - SSH-Ausführung (Mock / lokaler Test-Server).
   - Log-Aufnahme korrekt.

## Akzeptanzkriterien

- [ ] Snippets lassen sich erstellen, bearbeiten, löschen, gruppieren und sortieren.
- [ ] Das Snippets-Panel zeigt die Liste nach Gruppen, mit Suche und Ausführen-Buttons.
- [ ] Variablen-Prompts werden vor der Ausführung angezeigt und korrekt ersetzt.
- [ ] Lokale Ausführung öffnet ein Terminal und führt das Kommando aus.
- [ ] SSH-Ausführung (Host wählen) führt das Kommando auf dem entfernten Host aus.
- [ ] Das Ausführungslog dokumentiert Abläufe mit Status und Output.
- [ ] Alle Tests laufen grün.

## Notizen

- Snippets sind für wiederkehrende Admin-/Ops-Befehle gedacht; die SSH-Ausführung ist entscheidend für den Ops-Workflow.
- Die Variablen-Ersetzung muss defensiv sein (Eingaben nicht als Shell-Feature interpretieren, korrekt quoten/escapen).

## Warnungen

- ⚠️ SSH-Ausführung mit unkontrollierten Snippet-Inhalten kann riskant sein — dem Benutzer klar machen, welche Befehle laufen; kein Klartext von Passwörtern.
- ⚠️ Variablen-Eingaben sauber escapen, um fehlerhafte/ungewollte Shell-Expansion zu vermeiden.

## Weiterführende Tasks

- [T12-002: Command-Palette](./T12-002-command-palette.md)
