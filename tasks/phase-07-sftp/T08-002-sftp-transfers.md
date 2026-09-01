# T08-002: SFTP-Transfers (Upload/Download/Queue)

## Status
✅ Done

## Phase
7 — SFTP-Browser

## Abhängigkeiten
T08-001 (SFTP-Dateibrowser)

## Ziel
Die Dateiübertragung zwischen lokalem und remote Dateisystem implementieren (Upload/Download), einschließlich einer Fortschrittsanzeige, einer Transfer-Queue mit mehreren parallelen/sequentiellen Jobs, Dateikonflikt-Behandlung (überschreiben/skip/umbenennen) und Fehlerbehandlung. Dies bildet Labonair's Transfer-System nach.

## Kontext
Labonair verfügt über eine Transfer-Queue (`enqueue_transfer`, `cancel_transfer`, `resolve_conflict`) mit:
- Upload (lokal → remote) und Download (remote → lokal).
- Fortschritts-Events (`transfer_progress`) und Fehler-Events (`file_error`).
- Konflikt-Dialog (`file_conflict`), wenn eine Zieldatei existiert — der Benutzer wählt überschreiben, überspringen oder umbenennen.
- Einem Hintergrund-Worker (tokio mpsc) im Backend, der die Transfers verarbeitet.

Das Backend (T01-002) enthält bereits den SFTP-Worker mit der Transfer-Logik (russh-sftp). Dieser Task bindet ihn ans UI und implementiert die Transfer-Benutzeroberfläche (Progress, Queue, Konflikte, Abbrechen).

## Anweisungen zur Umsetzung

1. **Transfer-Queue im Backend verifizieren.** Verifiziere, dass der SFTP-Worker (aus T01-002) die Transfers verarbeitet: enqueue, cancel, konflikt-Resolver, progress- und error-Events über das Event-Bus. Stelle sicher, dass er lokale und remote Dateien korrekt liest/schreibt.

2. **Transfer-Auslösung.** Implementiere das Anstoßen von Upload/Download:
   - Aus der SFTP-Pane: Drag-and-Drop einer lokalen Datei auf die remote Pane (Upload) und umgekehrt (Download).
   - Über Kontextmenü/Aktionen ("Hochladen"/"Herunterladen" auf ausgewählte Dateien/Ordner).
   - Rekursiv bei Ordnern (komplette Ordnerstruktur übertragen).

3. **Transfer-Queue-UI.** Baue die Transfer-Anzeige (z.B. ein Dropdown/Bereich im Header oder eine Seitenleiste):
   - Liste der aktiven Jobs: Dateiname, Richtung (Upload/Download), Quelle → Ziel, Fortschritt (Balken + Prozent), Größe.
   - Status je Job (läuft, wartend, abgeschlossen, fehlgeschlagen, fehlerhaft).
   - Abbrechen-Button pro laufenden Job (der verbleibende Teil wird verworfen).
   - Zusammenfassung (gesamt aktiv, fertig).

4. **Fortschritts-Updates.** Verdrahte die `transfer_progress`-Events mit der UI (via Event-Bus aus T01-004): Der Fortschritt aktualisiert sich live für aktive Jobs. Nur sichtbare/geänderte Jobs neu rendern (Performance).

5. **Konflikt-Behandlung.** Implementiere den Konflikt-Dialog:
   - Wenn eine Zieldatei existiert (pro Datei): Dialog mit Optionen "Überschreiben", "Überspringen", "Umbenennen" (automatisch einen neuen Namen wählen), "Abbrechen".
   - Option "Für alle weiteren anwenden" (Behandlung aller Konflikte im los).
   - Bei Umbenennen: neuen Dateinamen (Datei_1.ext, etc.) generieren.
   - Das Ergebnis an den Worker zurückgeben (`resolve_conflict`).

6. **Fehlerbehandlung.** Definiere Verhalten bei fehlgeschlagenen Transfers:
   - Fehler anzeigen (Datei nicht lesbar/schreibbar, Verbindung verloren).
   - Job als fehlgeschlagen markieren; restliche Jobs der Queue fortführen oder unterbrechen je nach Schwere.
   - Verbindungsverlust: sinnvolle Wiederverbindungs-Option, wenn möglich (SFTP-Session reconnecten).

7. **Tests schreiben.** Erstelle Tests (gegen lokalen SFTP-Test-Server):
   - Upload einer Datei auf den remote (Inhalt korrekt).
   - Download einer Datei lokal.
   - Rekursiver Upload/Download eines Ordners.
   - Fortschritt-Events werden gefeuert.
   - Konflikt-Behandlung (überschreiben/skip/umbenennen) korrekt.
   - Abbrechen eines Jobs wirkt.
   - Fehlerfall (Quelle fehlt) → korrekter Fehlerstatus.

## Akzeptanzkriterien

- [ ] Transfers lassen sich per Drag/Drop und Kontextmenü auslösen, auch rekursiv für Ordner.
- [ ] Die Transfer-Queue-UI zeigt Jobs mit Richtung, Fortschritt (live), Status und Abbrechen.
- [ ] Fortschritt-Updates aktualisieren die UI lives und effizient.
- [ ] Konflikt-Dialog mit Überschreiben/Skip/Umbenennen + "Für alle" funktioniert.
- [ ] Fehler werden korrekt angezeigt und behandelt; Verbindungsverlust-Kontext berücksichtigt.
- [ ] Alle Tests laufen grün.

## Notizen

- Die Queue-Logik liegt bereits im Backend-Worker (russh-sftp) — fokussiere hier auf die UI-Anbindung und die Event-Verdrahtung.
- Für gute UX ist die Live-Fortschrittsanzeige wichtig; aber durch effizientes Rendering nur geänderter Jobs performant halten.

## Warnungen

- ⚠️ Konkurrierende Schreibzugriffe bei mehreren parallelen Transfers in dasselbe Zielverzeichnis — Konflikt-Erkennung robust halten.
- ⚠️ Beim Abbrechen eines Transfers die (teilweise) geschriebene Zieldatei behandeln (nicht unbedingt löschen, aber korrekt als unvollständig markieren / nicht fälschlich als abgeschlossen ansehen).
- ⚠️ Verbindungsverlust während Transfers: Datei-Zuordnung in der Queue nicht verlieren; bei Reconnect-orientiertem Wiederanlauf sinnvolle Semantik.

## Weiterführende Tasks

- Phase 8: Git-UI & Source-Control
