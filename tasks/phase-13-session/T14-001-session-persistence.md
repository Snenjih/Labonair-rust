# T14-001: Session-Persistenz (Tabs/Layout wiederherstellen)

## Status
⏳ Pending

## Phase
13 — Session-Persistence & Scrollback

## Abhängigkeiten
Phase 3 (Tab-System & Workspace-Layout)
T04-002 (Split-Pane-Layout)
T03-005 (Lokale PTY-Sessions)

## Ziel
Die Wiederherstellung der App-Sitzung beim Neustart implementieren: den Zustand der geöffneten Tabs, des Workspace-Layouts (Pane-Baum), und die wichtigsten Session-Metadaten so speichern und beim nächsten Start wiederherstellen, dass der Benutzer dort weiterarbeitet, wo er aufgehört hat. Dies entspricht Labonair's Session-Capture/Restore.

## Kontext
Labonair hat ein Session-System (`capture.ts`/`restore.ts`), das den aktuellen Tab-Zustand erfasst (offene Tabs mit ihren Typen und Meta-Daten) und beim Start wiederherstellt. Es ist standardmäßig aktiviert (Einstellung).

Der Pane-Baum (Splits) und alle offenen Tabs (Terminal-Sessions lokal & SSH, Editor-Tabs, SFTP, Git-Graph, Home, etc.) werden als Snapshot gespeichert. Beim Neustart werden die Tabs neu angelegt: lokale Terminals neu gestartet (die PTY-Prozesse können nicht überleben), Editor-Tabs neu geladen, und SSH-Sessions ggf. neu verbunden (lazy).

## Anweisungen zur Umsetzung

1. **Snapshot-Modell.** Definiere ein serialisierbares Snapshot-Modell der Sitzung:
   - Liste der Tabs (mit Typ und typ-spezifischen Metadaten: Pfad für Editor, Host/Session für SSH, URL für Preview, Repo für Git-Graph, etc.).
   - Aktiver Tab und ggf. aktive Pane.
   - Workspace-Layout (Pane-Baum + Größenverhältnisse).
   - Das aktive CWD (für die Wiederherstellung der Explorer-Wurzel und neuer Terminals).

2. **Capture (Speichern).** Implementiere die Erfassung des aktuellen Zustands:
   - Bei App-Beenden (und/oder periodisch) einen Snapshot erzeugen.
   - Serialisierung des Tab-/Layout-Zustands (spezifische Felder je Tab-Typ).
   - Speichern in einem persistenten Speicherort.

3. **Restore (Wiederherstellen).** Implementiere das Wiederherstellen beim Start (falls aktiviert):
   - Snapshot laden.
   - Tabs neu anlegen (mit den relevanten Metadaten/Typen).
   - Lokale Terminal-Sessions neu starten (PTY) mit passenden Metadaten (CWD, ein evtl. Startbefehl).
   - SSH-Sessions als "kalt" markieren und lazy wieder verbinden (die Verbindung selbst läuft beim Start nicht gleich, sondern wenn der Benutzer den Tab aktiviert).
   - Editor-Tabs neu laden (Datei lesen). Bei fehlender Datei einen sinnvollen Hinweis.
   - Das Workspace-Layout (Pane-Baum, Größen) wiederherstellen.
   - Die aktive Tab wieder aktivieren.

4. **Nicht-wiederherstellbare Zustände.** Definiere das Verhalten für Zustände, die nicht (sinnvoll) wiederhergestellt werden können:
   - SSH-Sessions: verbinden lazy oder nur auf Benutzer-Input.
   - Laufende Prozesse/Shell-States: können nicht überleben; ein Hinweis im re-wiederhergestellten Terminal ist sinnvoll.
   - Ungespeicherte Editor-Änderungen: Diese sollten vorher auffordern (siehe Dirty-Handling) oder als Verlust riskieren nur bei expliziter Einstellung.

5. **Einstellung.** Füge eine Einstellung "Session wiederherstellen" hinzu (im General-Bereich), standardmäßig aktiv.

6. **Tests schreiben.** Erstelle Tests für:
   - Capture erzeugt einen vollständigen Snapshot des aktuellen Tab-/Layout-Zustands.
   - Restore stellt Tabs, Layout und aktive Markierung korrekt wieder her.
   - Lokale Terminals werden neu gestartet (mit korrektem CWD).
   - Editor-Tabs laden den Dateiinhalt.
   - SSH-Tabs werden als kalt markiert und lazy verbunden.
   - Nicht-wiederherstellbare Zustände werden sinnvoll behandelt.
   - Bei deaktivierter Einstellung wird kein Restore durchgeführt.

## Akzeptanzkriterien

- [ ] Ein serialisierbares Snapshot-Modell deckt alle Tab-Typen und das Layout ab.
- [ ] Capture speichert den Zustand bei Beenden.
- [ ] Restore stellt Tabs, Pane-Baum/Layout, aktive Tab und das CWD wieder her.
- [ ] Lokale PTY-Sessions werden neu gestartet; Editor-Tabs neu geladen; SSH-Tabs lazy/kalt.
- [ ] Nicht-wiederherstellbare Zustände werden sauber behandelt, ohne Absturz.
- [ ] Die Einstellung "Session wiederherstellen" existiert (Standard an) und wirkt.
- [ ] Alle Tests laufen grün.

## Notizen

- Da PTY-Prozesse den Neustart nicht überleben, ist der Fokus auf "den Zustand wiederherstellen, weiterzuarbeiten" (Tabs, Pfade, Layout), nicht auf die Shell-Prozess-Kontinuität.
- Die Wiederherstellung von SSH-Tabs sollte non-blocking und lazy sein, um den Start zu beschleunigen.

## Warnungen

- ⚠️ Beim Restore keine Netzwerk-/Verbindungsverzögerung den Start blockieren — alles lazy/nicht-blockierend.
- ⚠️ Ungespeicherte Editor-Änderungen beim Beenden: geeignete Nachfrage oder klare Policy definieren (nicht einfach still verlieren).

## Weiterführende Tasks

- [T14-002: Scrollback-Persistenz](./T14-002-scrollback-persistenz.md)
