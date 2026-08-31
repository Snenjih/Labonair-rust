# T03-004: Shell-Integration und CWD-Tracking

## Status
✅ Done

## Phase
2 — Terminal-Engine

## Abhängigkeiten
T03-002 (GPUI-Terminal-Renderer)

## Ziel
Die Shell-Integration implementieren, die es der App ermöglicht, das aktuelle Arbeitsverzeichnis der Shell, den Prozess-Titel und Prompt-Informationen zu erfassen. Damit funktionieren CWD-Anzeige in der Statusleiste, das Vererbte-CWD für neue Tabs und die Live-Kontext-Funktion der An/Begleiter (Terminal-CWD + Pufferinhalt lesen).

## Kontext
Labonair nutzt Shell-Integration über OSC-Sequenzen, um das Arbeitsverzeichnis und Prozess-Titel zu erfahren. Konkret:
- OSC 7 (Arbeitsverzeichnis-Änderung): Die Shell meldet ihren aktuellen Pfad.
- OSC 133 (Shell-Integration, "FinalTerm"): Kennzeichnet Prompt-Beginn/-Ende, Befehlsbeginn, sowie Ausgabebeginn/-ende. Das ermöglicht z.B. "letzten Befehl ausführen"-Aktionen und sauberes CWD-Tracking.
- OSC 633 (VS Code Shell-Integration): Wird von manchen Shell-Integrationen ebenfalls genutzt.

Dazu werden PTY-Initialisierungs-Skripte verwendet, die in die Shell geladen werden, um diese Sequenzen zu erzeugen (z.B. `zshrc.zsh`, `bashrc.bash`). Diese Skripte setzen Umgebungsvariablen (z.B. `TERM_PROGRAM`), konfigurieren die Befehlszeile und installieren die OSC-Emitters.

In der React/xterm.js-Welt geschah das Parsing dieser OSC-Sequenzen im Frontend (in `osc-handlers.ts`). In der Rust-App übernimmt diese Aufgabe die Terminal-Engine: parse die OSC-Sequenzen aus dem Ausgabe-Stream heraus, aktualisiere die Session-Metadaten (CWD, Titel) und reiche die Info an die UI- und an den AI-Live-Kontext weiter.

## Anweisungen zur Umsetzung

1. **Shell-Integrations-Skripte bereitstellen.** Lege die Shell-Initialisierungsskripte (analog zu den vorhandenen `zshrc.zsh`, `bashrc.bash`, etc.) an, die beim Start in die Shell geladen werden. Diese Skripte müssen:
   - OSC 7 auf jede CWD-Änderung ausgeben.
   - OSC 133 (und ggf. OSC 633) um die Prompt-/Befehls-/Ausgabephasen markieren.
   - Die Umgebungsvariable setzen, die dem Terminal mitteilt, wer es ist (damit Programme das Verhalten anpassen).
   - Optional auch auf Historie und aktuelle Befehle reagieren (für saubere CWD-Tracking).

2. **OSC-Sequenz-Parsing.** Implementiere das Erkennen und Extrahieren aus diesen OSC-Sequenzen aus dem Ausgabe-Stream der Shell (die `alacritty_terminal`-Engine reicht bereits strukturierte Informationen bzw. die Rohdaten weiter; du entscheidest, auf welcher Ebene du präsentierst). Wichtig ist: Die OSC-Sequenzen dürfen nicht im sichtbaren Terminal-Inhalt erscheinen, sondern müssen still verarbeitet werden.

3. **CWD-Tracking.** Leite aus den OSC-7-Sequenzen das aktuelle Arbeitsverzeichnis der Session ab und halte es als Session-Metadat ergänzt. Dabei:
   - Den Pfad korrekt dekodieren (URL-Encoding, wie bei OSC 7 üblich).
   - Auch Initial-CWD beim Start übernehmen.
   - CWD-Änderungen an die UI weiterreichen (z.B. Statusleiste/Breadcrumb und neue-Tab-Ausgangsverzeichnis).

4. **Prompt-/Befehlserkennung.** Nutze die OSC-133-Markierungen, um Prompt-Beginn/-Ende und Ausgabeabschnitte zu erkennen. Das ist die Basis für:
   - "Nur den aktuellen/letzten Befehl selektieren" (Block-Auswahl, analog Labonairs "Block"-Konzept).
   - Saubere Kontext-Ausgabe (Prompt + Befehl + Ausgabe) für den AI-Versuch (Live-Kontakt).
   - Später u.a. für Befehls-Voorrschlag und die Terminal-Threads-Funktion.

5. **Prozess-/Titel-Erkennung.** Erfasse den Titel der Terminal-Session aus OSC-0/OSC-2 (Fenster-/Prozess-Titel), um die Tab-Beschriftung und Status-Anzeigen korrekt zu setzen.

6. **Integration in die Session-Metadaten.** Alle erfassten Metadaten (CWD, Titel, Prompt-State) müssen an einer zentralen Stelle pro Session gespeichert und der UI zugänglich gemacht werden — vergleichbar mit dem, was in der React-Version als Session-Daten und OSC-Handler existierte.

7. **Integration in den AI-Live-Kontakt.** Sorge dafür, dass das CWD- und Puffer-Erfassen-Mechanismen angebunden werden, die später vom AI-Modul genutzt werden: Der AI-Begleiter soll "das aktuelle Terminal (CWD + letzte Zeilen)" lesen können. Baue die Grunddatenhaltung dafür auf (auch wenn das AI-Panel selbst erst in Phase 10 kommt).

8. **Tests schreiben.** Erstelle Tests, die:
   - OSC-7-Sequenzen korrekt parsen und CWD aktualisieren (inkl. URL-Dekodierung).
   - OSC-133-Markierungen korrekt erkennen und die Prompt-Phasen modellieren.
   - OSC-0/2-Titel korrekt erfassen.
   - Verifizieren, dass OSC-Sequenzen nicht im sichtbaren Terminal-Inhalt auftauchen.

## Akzeptanzkriterien

- [ ] Die Shell-Initialisierungsskripte werden beim Terminal-Start geladen und erzeugen die nötigen OSC-Sequenzen.
- [ ] OSC-7-CWD-Änderungen werden korrekt geparst und als Session-Metadaten gespeichert.
- [ ] Die CWD-Anzeige (z.B. Statusleiste) zeigt das korrekte, aktuelle Verzeichnis an.
- [ ] OSC-133-Markierungen werden für die prompt-/ausgabe-basierte Block-Erkennung genutzt.
- [ ] Der Session-Titel wird aus OSC-0/2 korrekt erfasst.
- [ ] OSC-Sequenzen erscheinen nicht im sichtbaren Terminal-Inhalt.
- [ ] Die Datenhaltung ermöglicht es später, das aktive Terminal-CWD und den Pufferinhalt für den AI-Begleiter zu lesen.
- [ ] Alle Tests laufen grün.

## Notizen

- Die bestehenden Shell-Skripte in `reference-src/src-tauri/src/modules/pty/scripts/` (zshrc.zsh, bashrc.bash usw.) sind eine ideale Vorlage — übernimm deren Logik.
- Einige Programme deaktivieren Shell-Integration, wenn keine Builder-Umgebungsvariable (die die Terminal-Identität meldet) gesetzt ist. Stelle diese korrekt ein (analog `TERM_PROGRAM`).
- Die OSC-Sequenzen sind die zentrale Schnittstelle zwischen Shell und Terminal — konsistent und fehlerfrei umsetzen.

## Warnungen

- ⚠️ OSC-7-Pfade sind URL-kodiert (%20 für Leerzeichen, etc.) — unbedingt korrekt dekodieren, sonst zeigen Breadcrumb und AI-Kontext falsche Pfade.
- ⚠️ Nicht jede Shell meldet alles zuverlässig — das CWD-Tracking muss auch ohne vollständige Integration einen sinnvollen Fallback haben (z.B. durch Initial-CWD + nicht verlässliche Updates).
- ⚠️ Multiple OSC-133-Markierungen innerhalb eines Ausgabestroms (prompt/cmdoutput) korrekt voneinander getrennt verarbeiten, um den Puffer-Kontext nicht zu verfälschen.

## Weiterführende Tasks

- Phase 3: Tab-System & Workspace-Layout (nutzt das CWD-Tracking für neue Tabs)
- Phase 10: AI-Chat-System (nutzt den Live-Terminal-Kontakt)
