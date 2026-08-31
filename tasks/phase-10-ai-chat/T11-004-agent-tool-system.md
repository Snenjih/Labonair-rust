# T11-004: Agent/Tool-System und Live-Bridge

## Status
⏳ Pending

## Phase
10 — AI-Chat-System

## Abhängigkeiten
T11-001 (AI-Provider-Integration)
T11-002 (Chat-Store und Session-Verwaltung)
T11-003 (Chat-UI)

## Ziel
Das Agent-/Tool-System implementieren, das dem AI-Begleiter erlaubt, mit der App und dem System zu interagieren: Dateisystem-Werkzeuge (lesen/schreiben/bearbeiten/suchen), Shell-Ausführung, Terminal-Zugriff, Sub-Agents, Todo-Verwaltung — mit einem Sicherheitslayer (Genehmigungspflicht für ändernde Werkzeuge, Deny-Liste für sensible Pfade). Zusätzlich die Live-Bridge, die dem Agenten den aktuellen Terminal-Kontext (CWD + Puffer) bereitstellt.

## Kontext
Labonair's AI nutzt Tools (read_file, list_directory, write_file, edit, grep, glob, run_command, terminal, subagent, todo) und ein Sicherheitsmodell:
- Lesende Werkzeuge führen automatisch aus.
- Ändernde Werkzeuge (write_file, edit, create_directory, run_command) erfordern eine **Benutzergenehmigung** (in-UI-Karte, siehe T11-003).
- Ein Deny-Listen-Mechanismus blockiert sensible Pfade (`.env*`, `.ssh/`, Credentials) — auf Lese- UND Schreibpfad.
- Die Live-Bridge stellt dem Agenten den aktuell aktiven Terminal-Kontext bereit (CWD + letzte Zeilen des Buffers), lazzy (nicht vorschnappschuss).

In dem Rust-Agenten wird das als Tool-Aufruf-Set umgesetzt, das der Provider (T11-001) bei Tool-Calls anspricht und dessen Ergebnisse zurückführt, nach optionaler Genehmigung.

## Anweisungen zur Umsetzung

1. **Tool-Schnittstelle modellieren.** Definiere eine gemeinsame Tool-Schnittstelle: Name, Beschreibung (für das Modell), Eingabe-Schema (JSON), Ausführungslogik, und ob es genehmigungspflichtig ist. Fasse alle Werkzeuge in einer Registry zusammen, die der Provider an das Modell weitergibt.

2. **Dateisystem-Werkzeuge.** Implementiere die Werkzeuge für:
   - Datei lesen (mit Pfad-Sicherheitsprüfung) und Verzeichnis listen.
   - Datei schreiben (genehmigungspflichtig), erstellen, Verzeichnis anlegen.
   - Bearbeiten (edit/multi-edit) mit **Read-before-Edit**-Durchsetzung.
   - Suche: grep (Inhalte) und glob (Dateinamen).
   Alle diese greifen über das Backend-FS (T01-002) auf das System zu.

3. **Shell-Werkzeug.** Implementiere das Ausführen von Shell-Kommandos (genehmigungspflichtig):
   - Einmalige Ausführung (subshell) analog Labonair's `run_command`.
   - Rückgabe von stdout/stderr und Exit-Code.
   - Timeout und Sichtbarkeit der Ausgabe.
   - Sicher: Deny-Liste beachten, keine Blockierung der UI (async).

4. **Terminal-Werkzeug.** Implementiere Werkzeuge für Terminal-Interaktion:
   - Terminal lesen (aktuellen Puffer/CWD des aktiven Terminals) — über die Live-Bridge.
   - Terminal schreiben (Befehl an die aktive Shell senden), genehmigungspflichtig.

5. **Sub-Agent- und Todo-Werkzeuge.** Implementiere:
   - Sub-Agent: ein weiteres Modell aufrufen (ggf. anderes Modell/prov), das Teilaufgaben erledigt und Ergebnisse zurückliefert.
   - Todo-Verwaltung: Todos auflisten/anlegen/markieren, damit der Agent seine Arbeit strukturieren kann.

6. **Sicherheits-/Genehmigungslayer.** Implementiere zentral:
   - **Deny-Liste**: sensible Pfade (`.env*`, `.ssh/`, SSH-Credentials, Provider-Keys) auf Lese- UND Schreibpfaden blockieren (nie umgehen).
   - **Genehmigungspflicht**: ändernde Werkzeuge setzen `needs_approval`; die Antwort pausiert, bis der Benutzer in der UI-Karte genehmigt/ablehnt (aus T11-003).
   - Automatische Weiterleitung nach Genehmigung, mit sauberer Nachrichten-Konsistenz.

7. **Live-Bridge.** Implementiere den Kontext-Zugriff:
   - Ein Modul, dass den **aktuell aktiven** Terminal (CWD + letzte N Zeilen Pufferinhalt) abfragen kann.
   - Lazy: erst beim tatsächlichen Zugriff sammeln (nicht vorschnappschuss), da die aktive Tab sich ändern kann.
   - Diese Daten dem Agenten als Kontext (bei Nachricht an das Modell) zur Verfügung stellen.

8. **Tests schreiben.** Erstelle Tests für:
   - Tool-Ausführung (read/write/grep/glob/shell) gegen Temp-Verzeichnisse.
   - Deny-Liste blockiert sensible Pfade (auf Read und Write).
   - Genehmigungspflicht: Ändernde Werkzeuge pausieren, bis Zustimmung; Ablehnen bricht sauber ab.
   - Read-before-Edit-Durchsetzung.
   - Live-Bridge liefert aktuellen Terminal-Kontext korrekt.
   - Sub-Agent- und Todo-Werkzeuge.

## Akzeptanzkriterien

- [ ] Eine gemeinsame Tool-Schnittstelle und -Registry existieren.
- [ ] FS-, Shell-, Terminal-, Sub-Agent- und Todo-Werkzeuge funktionieren auf den Backend-Pfaden.
- [ ] Deny-Liste blockiert sensible Pfade auf Lese- UND Schreiboperationen (nie umgangen).
- [ ] Ändernde Werkzeuge erfordern und respektieren die Benutzergenehmigung (Ausführung nur nach Zustimmung).
- [ ] Die Live-Bridge stellt dem Agenten den aktuell aktiven Terminal-Kontext bereit (lazy).
- [ ] Nach Genehmigung läuft die Agenten-Ausführung automatisch weiter (konsistent).
- [ ] Alle Tests laufen grün.

## Notizen

- Der Sicherheitslayer ist Kern des Vertrauens in die AI-Automatisierung — sorgfältig und defensiv implementieren.
- Die read-before-edit-Durchsetzung verhindert Datenverlust bei AI-gestützten Dateiänderungen — wichtig.

## Warnungen

- ⚠️ Deny-Liste niemals durch relative/absolute-Pfad-Tricks umgehbar sein lassen (auch `..`-Auflösung beachten).
- ⚠️ Shell-Kommandos nie im UI-Thread ausführen — immer async, mit Timeout und Abbrechbarkeit.
- ⚠️ Genehmigungs-Pause muss konsistent zur Nachrichten-Logik sein — kein halb-ausgeführter Zustand nach Ablehnen.

## Weiterführende Tasks

- Phase 11: Snippets & Command-Palette
