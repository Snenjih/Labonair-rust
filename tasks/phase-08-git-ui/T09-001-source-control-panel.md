# T09-001: Source-Control-Panel (Git-Status und Staging)

## Status
⏳ Pending

## Phase
8 — Git-UI & Source-Control

## Abhängigkeiten
T04-001/2 (Tabs, Pane)
T05-001 (Explorer)
T06-004 (Diff-Ansicht)

## Ziel
Ein Source-Control-Panel im VS-Code-Stil implementieren, das den Git-Status eines Repositories anzeigt, Dateien stage/unstage kann (inkl. Hunk-basiert), Änderungen gegen den Index/HEAD vergleicht und grundlegende Git-Operationen (Commit, Pull/Push) ausführt. Dabei wird erwartet, dass die hinterlegten Git-Befehle aus dem Backend (T01-002) zur Verfügung stehen.

## Kontext
Labonair hat ein Source-Control-Panel (Sidebar-Tab) mit:
- Anzeige geänderter Dateien (getrennt nach Staged/Unstaged/Untracked).
- Staging/Unstaging von Dateien und Hunks.
- Diff-Vorschau (Utilisierung der Diff-Ansicht aus T06-004) für eine Datei.
- Commit-Formular (Message, Commit erstellen).
- Pull/Push/Fetch.
- Branch-Verwaltung (in T09-002).
- Stash-Operationen (in T09-002).

Die Git-Logik liegt im Backend (git2/libgit2 oder git-CLI) — das UI liest den Status und ruft die Operationen auf.

## Anweisungen zur Umsetzung

1. **Git-Status abrufen.** Verdrahte das Abrufen des Git-Status (via Backend) für das aktuelle Repo. Der Status soll eine strukturierte Liste geänderter Dateien liefern (Status-Codes wie 'M', 'A', 'D', 'R', '??' für untracked) getrennt nach Staged/Unstaged/Untracked.

2. **Dateiliste im Panel.** Implementiere die Anzeige der geänderten Dateien:
   - Kategorisiert (Staged / Unstaged / Untracked), auf-/zuklappbar.
   - Camera-Indicator pro Datei (z.B. farbig je Status).
   - Anzeige der Änderungsstatistik (hinzugefügte/gelöschte Zeilen), sofern Backend sie liefert.
   - Klick auf Datei öffnet die Diff-Ansicht (Seitenleiste).

3. **Staging/Unstaging.** Implementiere:
   - Stage/Unstage einer gesamten Datei (Button, Klick).
   - Stage/Unstage von Hunks (im Diff-Bereich) — selektiver/Bereich Zusamenführung.
   - stage alle / unstage alle.
   - Nach jeder Operation den Status aktualisieren (visuell korrekt neu laden).

4. **Diff-Vorschau integrieren.** Binde die Diff-Ansicht (T06-004) ein:
   - Auswahl einer Datei → Diff zwischen Arbeitsverzeichnis und Index (unstaged) bzw. Index und HEAD (staged).
   - Hunk-Navigation und ggf. Hunk-Auswahl fürs Staging direkt im Diff.
   - Farblich korrekt (Theme-Integration aus T06-004).

5. **Commit-Ablauf.** Implementiere das Committen:
   - Commit-Formular (Message-Eingabe, optional AI-Vorschlag — eigener Task/Phase 10).
   - Commit nur mit staged Dateien; Hinweis, wenn nichts staged.
   - Nach Commit: Status aktualisieren, ggf. leere-Änderungen-meldung.
   - Commit-Message-Validierung (nicht leer).

6. **Pull/Push/Fetch.** Implementiere:
   - Pull (mit Merge-Ergebnis-Meldung, Konflikt-Hinweis).
   - Push (zu upstream, mit Force-with-lease nur bei expliziter Aktion, set-upstream für neue Branches).
   - Fetch.
   - Darstellung von Erfolg/Fehler und Konflikten.

7. **Status-Polling und Aktualität.** Sorge für eine zuverlässige Synchronisierung:
   - Status bei Operationen aktualisieren.
   - Externer Dateisystem-/Git-Änderung (Datei von anderem Tool geändert) erkennen und aktualisieren (libgit2-Reflesh bzw. Watcher).
   - Polling mit Generation-Guards (ähnlich Labonair) um Race-Conditions zu vermeiden.

8. **Tests schreiben.** Erstelle Tests (gegen lokale Test-Repos):
   - Status korrekt für staged/unstaged/untracked.
   - Stage/Unstage Hunk korrekt.
   - Commit erstellt korrekt.
   - Pull/Push gegen lokales Remote (Bare-Repo) funktioniert.
   - Fehler (kein Repo, Konflikte) korrekt behandelt.

## Akzeptanzkriterien

- [ ] Das Panel zeigt geänderte Dateien kategorisiert (Staged/Unstaged/Untracked) mit Status-Indikatoren.
- [ ] Klick auf Datei zeigt die Diff-Ansicht.
- [ ] Staging/Unstaging (Datei + Hunk + alle) funktioniert und aktualisiert den Status.
- [ ] Commit-Formular erstellt Commits korrekt mit Validierung.
- [ ] Pull/Push/Fetch funktionieren mit Erfolgs-/Konflikt-Meldungen.
- [ ] Der Status aktualisiert sich zuverlässig (externe Änderungen erkannt, Race-Conditions vermieden).
- [ ] Alle Tests laufen grün.

## Notizen

- Die Git-Befehle kommen aus dem Backend (git2). Das UI muss sie nur sauber anbinden.
- Die Hunk-Erkennung (aus diffHunks in Labonair) ist für Hunk-Staging wichtig — über die Diff-Ansicht (T06-004) zugänglich machen.

## Warnungen

- ⚠️ Race-Conditions beim Status-Polling (zwei parallel ablaufende Anfragen) — Generation/Sequence-Guards implementieren.
- ⚠️ Force-Push nur bei expliziter Benutzeraktivität (Force-with-lease bevorzugen), nie automatisiert.
- ⚠️ Nach externen Dateiänderungen innerhalb von Repos die Git-Worktree aktualisieren, sonst veraltete Status.

## Weiterführende Tasks

- [T09-002: Branch-Verwaltung und Stash](./T09-002-branch-stash.md)
- Phase 9: Git-Graph
