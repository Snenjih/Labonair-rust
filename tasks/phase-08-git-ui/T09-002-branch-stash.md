# T09-002: Branch-Verwaltung und Stash

## Status
✅ Done

## Phase
8 — Git-UI & Source-Control

## Abhängigkeiten
T09-001 (Source-Control-Panel)

## Ziel
Branch-Verwaltung (Liste, wechseln, erstellen, löschen, umbenennen) und Stash-Operationen (stash push/pop/apply/drop, Liste) in das Source-Control-System integrieren, jeweils mit passender UI im Panel und in einer Branch-Leiste.

## Kontext
Labonair's Source-Control bietet:
- Eine **Branch-Leiste** (unten/relevant im Panel): zeigt den aktuellen Branch an und erlaubt das Wechseln/Erstellen.
- **Branch-Operationen**: Liste aller Branches, checkout, create (neuer Branch), delete, rename. Auch Tags (anlegen/löschen).
- **Stash-Operationen**: stash push (mit Message), stash list, stash pop, stash apply, stash drop.

Das Backend (T01-002) stellt die Branches/Tags/Stash-Kommandos bereit. Der Task bindet sie ein und erstellt die UI.

## Anweisungen zur Umsetzung

1. **Branch-Leiste.** Implementiere eine Branch-Leiste (in oder unter dem Source-Control-Panel):
   - Anzeige des aktuell ausgecheckten Branch (mit Icon).
   - Klick → öffnet den Branch-Picker.
   - Anzeige des Sync-Status (ahead/behind vs. upstream), falls Backend ihn liefert.

2. **Branch-Picker und -Verwaltung.** Implementiere eine Branch-Liste:
   - Alle lokalen Branches anzeigen (mit Markierung des aktuellen).
   - Wechseln (checkout) eines Branches.
   - Einen neuen Branch erstellen (von aktuellem Stand, mit optionaler Benennung und basis-Branch-Auswahl) und optional sofort wechseln.
   - Löschen eines Branches (mit Schutz für current/merged, Bestätigung).
   - Umbenennen eines Branchs.
   - Suche/Filter in der Branch-Liste.

3. **Tags.** Implementiere grundlegende Tag-Verwaltung:
   - Tags auflisten (im Branch-Picker oder separater Bereich).
   - Neues Tag anlegen (Name, optional Description/Messge).
   - Tag löschen.

4. **Stash-Funktionen.** Implementiere:
   - **Stash push**: aktuelle ungespeicherte Änderungen stashen (mit optionaler Message; inclusive/only-untracked wählbar je Backend).
   - **Stash list**: alle Stashes anzeigen (Message, Branch, Datum).
   - **Stash pop** (anwenden und entfernen) und **stash apply** (anwenden ohne entfernen).
   - **Stash drop** (löschen eines Eintrags).
   - Nach Operation den Status aktualisieren.

5. **Stash-Panel-UI.** Implementiere eine minimale Stash-Liste im Panel (auf-/zuklappbar), mit Aktionen pro Stash-Eintrag (apply/pop/drop) und dem Erstellen (push)-Button.

6. **Konflikte und Fehler.** Behandle Fehler sauber:
   - Wechseln bei ungespeicherten Änderungen (Konflikt zwischen Working-Tree und Branch) → Hinweis und Optionen (committen, stashen, verwerfen, abbrechen).
   - Stash pop/apply mit Konflikten → klare Meldung, wobei die Konflikte im Working-Tree verbleiben.

7. **Tests schreiben.** Erstelle Tests (lokale Test-Repos):
   - Branch-Liste und Wechseln korrekt.
   - Erstellen/Löschen/Umbenennen von Branches.
   - Tag-Anlegen/Löschen.
   - Stash push/list/pop/apply/drop mit korrekten Zuständen des Working-Trees.
   - Fehlerbehandlung (unbehandelte Änderungen beim Wechseln).

## Akzeptanzkriterien

- [ ] Die Branch-Leiste zeigt den aktuellen Branch und Sync-Status und öffnet den Picker.
- [ ] Branch-Wechseln, -Erstellen, -Löschen, -Umbenennen funktionieren.
- [ ] Tags lassen sich auflisten, anlegen, löschen.
- [ ] Stash push/list/pop/apply/drop funktionieren vollständig.
- [ ] Unbehandelte Konflikte beim Wechseln/Stash werden klar gemeldet und gehandhabt.
- [ ] Nach jeder Operation wird der Status korrekt aktualisiert.
- [ ] Alle Tests laufen grün.

## Notizen

- Die Branch-Picker- und Stash-Operationen greifen auf Backend-Kommandos zurück. Halte die UI-Operationen dünn und sauber über eine Git-Aufruf-Schicht.
- Der Schutz des aktuellen Branches vor Löschung muss in der UI und ggf. im Backend-Defensiv sein.

## Warnungen

- ⚠️ Branch-Wechsel mit nicht-committeten Änderungen: entweder das Standard-Verhalten (git wird verweigern/mergen) verständlich machen oder dem Nutzer Optionen anbieten; niemals heimlich stashen/verwerfen.
- ⚠️ Stash pop mit Konflikten darf den Stash-Eintrag NICHT automatisch entfernen (git behält ihn) — dies verständlich machen.

## Weiterführende Tasks

- Phase 9: Git-Graph
