# T08-001: SFTP-Dateibrowser

## Status
⏳ Pending

## Phase
7 — SFTP-Browser

## Abhängigkeiten
T07-001 (Host-Manager und SSH-Verbindung)
T04-001/2 (Tab-System, Pane)
T05-001 (Dateibaum-Muster aus Explorer)

## Ziel
Einen vollwertigen SFTP-Dateibrowser als Tab-Typ implementieren, der auf einen verbundenen SSH-Host zugreift und dessen Dateisystem anzeigt: Navigation, Ordner öffnen, Dateien anzeigen, bearbeiten (öffnen im Editor), Umbenennen/Löschen/Neuanlegen/Rechte-ändern (chmod), sowie die Struktur ähnlich dem lokalen Explorer aufbauen.

## Kontext
In Labonair ist der SFTP-Tab ein dual-pane Dateibrowser (lokal links, remote rechts) mit einer virtuellen Dateiliste pro Pane. Er unterstützt:
- Navigation durch lokale und remote Pfade.
- Anzeige von Dateien/Ordnern, Filtern.
- Datei-Operationen: Öffnen (in Editor), Umbenennen, Löschen, Neuanlegen, Mkdir, chmod/chown.
- Remote-Dateien im Editor öffnen (über `prepare_remote_edit`/`save_remote_edit` — temporär herunterladen, editieren, zurückspeichern).
- Integrierte Dateiübertragung (Upload/Download) — in T08-002.

Dieser Task fokussiert auf den Browser selbst (Navigation + Dateioperationen + Remote-Edit-Grundlage). Die Transfers (Upload/Download/Queue) folgt in T08-002.

## Anweisungen zur Umsetzung

1. **SFTP-Verbindung anbinden.** Nutze die bestehende SSH-Verbindung (T07-001) als Basis: Der SFTP-Tab braucht eine aktive SSH-Session zum Zielhost. Verifiziere, dass die russh-sftp-Integration im Backend (T01-002) den SFTP-Kanal bereitstellt.

2. **SFTP-Pane-Struktur.** Baue den dual-pane SFTP-Browser:
   - Linke Pane: lokales Dateisystem (analog Explorer-Logik aus T05-001, aber als eigenständige Pane).
   - Rechte Pane: remote Dateisystem des verbundenen Hosts.
   - Pro Pane: Adressleiste/Pfad-Anzeige, Navigations-Buttons (zurück, auf, neu laden), Verzeichnisliste.

3. **Remote-Verzeichnis lesen.** Implementiere das Einlesen eines remote Verzeichnisses:
   - Ordner/Dateien auflisten (sortiert, Ordner zuerst).
   - Entsprechend der App-Funktion prinzipiell auch paginiert.
   - Lade-/Fehlerzustände (Verbindung verloren, Rechte-Fehler).
   - Versteckte Dateien-Toggle.

4. **Datei-Operationen.** Implementiere die remote Dateioperationen mit klaren Fehlermeldungen:
   - Umbenennen, Löschen (Bestätigung), Neuanlegen (Datei), Ordner erstellen.
   - Rechte ändern (chmod) und Besitzer (chown) — mit Dialog zur Auswahl der numerischen/berechtigten Werte.
   - Datei-Eigenschaften (Größe, Zeitstempel, Rechte) anzeigen (Properties-Dialog).

5. **Remote-Edit.** Implementiere das Bearbeiten einer remote Datei im lokalen Editor:
   - Beim Öffnen einer remote Datei: per SFTP temporär auf lokal holen (temp-Datei), im Editor-Tab öffnen (mit Kennzeichnung, dass es eine remote Datei ist, und welchem Host/Session sie gehört).
   - Beim Speichern: die bearbeitete Datei zurückschreiben (save_remote_edit).
   - Beim Verlassen/Reconnect: den remote-Zustand sauber halten (ggf. temp aufräumen).
   - Konfliktfall (remote hat sich geändert) melden.

6. **Navigation und Kontext.** Implementiere die Kulturnavigation:
   - In Ordner navigieren (öffnen, Verzeichniswechsel), zurück/zur Ebene auf.
   - Absolute Pfad-Eingabe / Pfad-Adressbar.
   - Kontextmenü pro Datei/Ordner mit allen Aktionen.
   - Double-Click auf Datei → Editor; auf Ordner → navigieren.

7. **Integration in Tab-Panes.** Der SFTP-Tab wird als eigener Tab-Typ gehostet (aus T04-001). Er hat die dual-pane-Anordnung.

8. **Tests schreiben.** Erstelle Tests (gegen einen lokalen SFTP-Test-Server, z.B. via `sshd`/`sftp-server` oder Mock):
   - Auflisten remote Verzeichnisse korrekt.
   - Umbenennen/Löschen/Neuanlegen/Mkdir remote.
   - chmod/chown korrekt setzen.
   - Remote-Edit: Download→Edit→Upload-Pfad.
   - Fehlerbehandlung (Rechte, Verbindung).

## Akzeptanzkriterien

- [ ] Ein SFTP-Tab lässt sich zu einem verbundenen Host öffnen und zeigt dual-pane (lokal/remote).
- [ ] Remote-Verzeichnisse lassen sich navigieren und anzeigen; versteckte Dateien-Toggle.
- [ ] Remote-Umbenennen, Löschen (Bestätigung), Datei/Ordner erstellen, chmod/chown funktionieren.
- [ ] Datei-Eigenschaften (Properties) werden angezeigt.
- [ ] Remote-Dateien lassen sich im Editor öffnen und speichern (Download→Edit→Upload).
- [ ] Adressleiste und Navigation (zurück/auf/neu laden) funktionieren.
- [ ] Kontextmenü pro Datei bietet alle Aktionen.
- [ ] Alle Tests laufen grün.

## Notizen

- Die dual-pane-Struktur ähnelt einem Mini-Explorer; wiederverwende die Explorer-Liste-Logik (T05-001) wo sinnvoll.
- Remote-Edit ist ein markantes Feature — die temp-Datei-Verwaltung muss sauber sein (aufräumen, Konflikte erkennen).

## Warnungen

- ⚠️ Remote-Operationen über Live-SSH sind fehleranfällig bei Verbindungsabrissen — robuste Fehlerbehandlung und klare Meldung bei verlorener Verbindung.
- ⚠️ chmod/chown können Sicherheits-/Permission-Fehlern unterliegen — Werte sauber validieren und Fehler melden.
- ⚠️ Beim Remote-Edit temp-Dateien nie im falschen Pfad, und bei abgebrochenen Updates sauber löschen.

## Weiterführende Tasks

- [T08-002: SFTP-Transfers (Upload/Download/Queue)](./T08-002-sftp-transfers.md)
