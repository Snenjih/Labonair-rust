# T14-002: Scrollback-Persistenz

## Status
⏳ Pending

## Phase
13 — Session-Persistence & Scrollback

## Abhängigkeiten
T14-001 (Session-Persistenz)
Phase 2 (Terminal, Scrollback-Handling)
T03-001 (Terminal-Engine)

## Ziel
Die Persistenz von Terminal-Scrollback-Inhalt bei App-Beenden und Wiederherstellung beim Neustart implementieren, sodass nach einem Neustart der Terminal-Verlauf (was in einer Session ausgegeben wurde) verfügbar ist — inklusive der Aufräumlogik für alte/verwaiste Scrollback-Daten.

## Kontext
In Labonair kann der Scrollback einer Terminal-Session gespeichert und beim Restore wiederhergestellt werden (`scrollback_save`/`load`/`cleanup`, dazu `saveAllScrollbacks`, `cleanupScrollbacks`). Das ermöglicht es, den Terminal-Verlauf auch nach dem Neustart zu sehen (obwohl der Shell-Prozess selbst neu startet).

Mechanik: Beim Beenden wird der Scrollback-Inhalt (die Zellen des Terminals) einer Session in eine Datei geschrieben. Beim Wiederherstellen wird der Scrollback in die neu gestartete Terminal-Session eingefügt (als vorheriger Verlauf), sodass der Benutzer ihn durchblättern kann.

## Anweisungen zur Umsetzung

1. **Scrollback-Speicherung.** Implementiere das Speichern des Scrollback einer Session:
   - Beim Beenden (oder periodisch bei aktiven Sessions) den Scrollback-Inhalt (die renderbaren Zellen) einer Session als Text/Struktur in eine Datei schreiben (im Anwendungsdatenpfad, verwaltete Verzeichnisstruktur).
   - Metadaten je Abbildung: Session-ID/Tab, Zeit, Dateiname.

2. **Scrollback-Wiederherstellung.** Implementiere das Laden:
   - Beim Restore einer Terminal-Session (aus T14-001) den gespeicherten Scrollback in die neu gestartete Session einfügen, damit der Verlauf durchblätterbar ist.
   - Korrektes Äquivalent der Ereignisreihenfolge: Der Scrollback erscheint VOR der aktuellen (frischen) Ausgabe der neuen Shell.

3. **Aufräum-Logik.** Implementiere das Aufräumen:
   - Verwaiste Scrollback-Dateien (die keiner Session/Tab mehr zugeordnet sind) entfernen.
   - Beim Löschen einer Session/Tab auch den zugehörigen Scrollback löschen.
   - Eine Übersicht/Obergrenze (z.B. maximale Größe) vermeiden, dass Dateisystem überquillt.
   - Alt-Screen-Sonderfälle (Vollbild-Apps) sauber behandeln (nicht sinnlos persistieren).

4. **Integration mit Session-Restore.** Sorge für eine nahtlose Zusammenarbeit mit T14-001: Wenn eine Session wiederhergestellt wird und ein Scrollback existiert, wird es mitgeladen; wird keine Session wiederhergestellt, wird der Scrollback aufgeräumt.

5. **Tests schreiben.** Erstelle Tests für:
   - Speichern eines Scrollback (Inhalt korrekt persistiert).
   - Laden in eine neu gestartete Session (Verlauf sichtbar, frischer Prompt danach).
   - Aufräumen verwaister/Alt-Screen-Dateien.
   - Löschen des Scrollback bei Session-Löschung.

## Akzeptanzkriterien

- [ ] Scrollback-Inhalt lässt sich persistieren (in eine Datei) und wieder laden.
- [ ] Nach einem simulierten Restart zeigt eine wiederhergestellte Terminal-Session den vorherigen Verlauf (dahinter die neue Ausgabe).
- [ ] Verwaiste Scrollback-Dateien werden aufgeräumt; Alt-Screen-Sonderfälle behandelt.
- [ ] Beim Löschen einer Session wird deren Scrollback entfernt.
- [ ] Eine Größen-/Anzahl-Obergrenze verhindert Dateisystem-Überlauf.
- [ ] Alle Tests laufen grün.

## Notizen

- Der Scrollback umfasst potenziell viele Zeilen — die Speicherung sollte kompakt und effizient sein (nicht Stunden an Text inkl. aller Ansi-Codes, sondern die Zellen ohne überflüssige Ansi).
- Das Wiederherstellen des Verlaufs ist ein erzählenswertes UX-Feature (vergleichbar mit dem, was andere moderne Terminals bieten).

## Warnungen

- ⚠️ Scrollback-Dateien dürfen nicht ins Unendliche wachsen — Größenobergrenze und Aufräumen zwingend.
- ⚠️ Alt-Screen (Vim, weniger etc.) sollte nicht als Dauer-Scrollback persistiert werden — sauber behandeln.

## Weiterführende Tasks

- Phase 14: Testing & Polish
