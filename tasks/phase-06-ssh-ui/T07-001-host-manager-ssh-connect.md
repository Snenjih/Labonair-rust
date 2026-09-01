# T07-001: Host-Manager und SSH-Verbindungs-Grundlage

## Status
✅ Done

## Phase
6 — SSH-UI & Host-Manager

## Abhängigkeiten
T04-001/2 (Tab-System, Pane-Inhalte)

## Ziel
Die SSH-Funktionalität der App umsetzen: Host-Manager (Hosts-Gruppen-Classifier) als "Home"-Dashboard, Verbinden zu SSH-Hosts, Erzeugen von SSH-Terminal-Sessions als Tabs, sowie Credential- und Passwort-/Key-Management. Dies legt die Grundlage für SSH-Terminals, die in der App laufen.

## Kontext
Labonair verwaltet SSH-Hosts in Gruppen und Credentials (Passwörter und Schlüssel), gespeichert in einer SQLite-Datenbank und im OS-Keyring (Passwörter/Keys nie in der DB). Der "Home"-Tab zeigt das Host-Dashboard (Hosts, Gruppen, Verbinden-Buttons). Verbindungen werden über `russh` aufgebaut.

In der Rust-Portierung übernimmt `crates/backend` die SSH-/Hosts-/Credentials-Logik (aus T01-002), und `crates/ui` das Host-Dashboard und die Verbindungs-UI.

Wichtig: Die SSH-Sitzung verwendet die gleiche Terminal-Engine wie lokale Sessions (T03-001), nur dass die Ein-/Ausgabe über die SSH-Verbindung geht und nicht über ein lokales PTY. Die Terminal-Engine selbst bleibt identisch — nur die "Leitung" (Transport) unterscheidet sich.

## Anweisungen zur Umsetzung

1. **Hosts-/Gruppen-/Credential-Modell.** Verifiziere, dass die Host-Datenbank (aus T01-002) funktioniert: Hosts, Gruppen, Credentials CRUD, mit korrekter Verschleierung (Passwörter/Keys im OS-Keyring, nicht in SQLite).

2. **Host-Dashboard (Home-Tab).** Baue das Home-Dashboard als Tab-Inhalt ("home"-Tab-Typ aus T04-001):
   - Anzeige der Hosts, gruppiert nach Gruppenzugehörigkeit.
   - Für jeden Host: Name, Adresse, Status-Indikator (verbunden/nicht verbunden, ggf. Latenz).
   - Gruppen auf-/zuklappen und verwalten (ausblenden/erzeugen/umbenennen/löschen).
   - Aktionen: Verbinden, Bearbeiten, Duplizieren, Löschen. Neu anlegen (Formular).

3. **Host-Formular.** Implementiere das Formular zum Anlegen/Bearbeiten eines Hosts:
   - Felder: Name, Adresse (Host:Port), Benutzername, Authentifizierungsmethode (Passwort / SSH-Key / Agent / None), Startverzeichnis, Tags.
   - Credential-Bezug: Auswahl eines in der App gespeicherten Credentials oder sofortige Eingabe (Passwort/Key).
   - Gruppen-Zuordnung.

4. **Credential-Verwaltung.** Implementiere die Credential-UI:
   - Liste von gespeicherten Credentials (Passwörter, SSH-Keys).
   - Erstellen/Bearbeiten/Löschen; Keys auch als Bone-generiert (via backend `credential_generate_keypair`).
   - Anzeige, welche Hosts ein Credential verwenden.
   - Kein Anzeigen der eigentlichen Passwörter/Keys im Klartext.

5. **SSH-Verbinden.** Implementiere den Verbindungsablauf:
   - Herstellen der SSH-Verbindung (russh) mit der ausgewählten Authentifizierung.
   - Authentifizierungs-Prompts (Passwort, Passphrase für verschlüsselte Keys, 2FA) anzeigen.
   - Known-Hosts-Verhalten: bei unbekanntem Host eine Sicherheits-Warnung anzeigen (Fingerprint), die der Benutzer bestätigen/vertrauen muß.
   - Nach erfolgreichem Verbinden: eine SSH-Terminal-Session als Tab eröffnen (mit der Terminal-Engine).
   - Fehlerbehandlung (Verbindungsabbruch, falsche Credentials) mit klaren Meldungen.

6. **SSH-Terminal-Session anbinden.** Die erzeugte SSH-Session nutzt die Terminal-Engine (T03-001) und die Shell-Integration (T03-004), aber mit SSH-Transport statt lokalem PTY. Stelle sicher, dass:
   - Die wichtigste Interaktion (Ein-/Ausgabe, Resize) über SSH an die ferne Shell geht.
   - Die Shell-Integrations-OSC-Sequenzen (CWD etc.) über SSH funktionieren (fern verarbeitet).
   - Mehrere SSH-Terminals parallel möglich sind.

7. **Verbindungsstatus.** Zeige den Verbindungsstatus für jeden Host (verbunden/verbindend/Fehler), aktualisiert über das Event-System. Beim Verbindungsverlust eines Terminals eine sinnvolle Meldung anzeigen und ggf. Reconnect-Option anbieten.

8. **Tests schreiben.** Erstelle Tests für:
   - Host/Gruppe/Credential-CRUD (mit Test-DB).
   - Verbindung zu einem lokalen SSH-Server (Testumgebung) — zumindest den Verbindungsablauf- und Auth-Status.
   - Fehlerbehandlung bei falschen Credentials.
   - Known-Hosts-Sicherheitswarnung.

## Akzeptanzkriterien

- [ ] Das Home-Dashboard zeigt Hosts in Gruppen mit Status und Verbinden-Aktion.
- [ ] Host-Formular legt/bearbeitet Hosts mit allen Feldern inkl. Credential-Auswahl.
- [ ] Credential-Verwaltung funktioniert; Keys lassen sich generieren; Passwörter/Keys nicht im Klartext sichtbar.
- [ ] Verbinden zu einem SSH-Host gelingt (Test-Server) und eröffnet ein SSH-Terminal als Tab.
- [ ] Auth-Prompts (Passwort/Passphrase/2FA) und Known-Hosts-Warnung funktionieren.
- [ ] Verbindungsstatus wird live angezeigt; Fehler/Verlust werden gemeldet.
- [ ] Mehrere SSH-Terminals parallel möglich.
- [ ] Alle Tests laufen grün.

## Notizen

- SSH-Transport trennt sich sauber von der Terminal-Engine — das ist ein Vorteil gegenüber der alten xterm.js zugrundegebenden Architektur (Engine einmal bauen, sowohl lokal als auch remote nutzen).
- Die Host-Datenbank und SSH-Logik liegen im backend-Crate (aus T01-002). Das UI liest/schreibt über diese API.
- Für Tests eine lokale SSH-Umgebung (z.B. via `sshd` mit generierten Keys) einrichten.

## Warnungen

- ⚠️ Niemals Passwörter/Keys im Klartext in Logs oder DB speichern — nur im OS-Keyring (backend `secrets`).
- ⚠️ Known-Hosts-Warnung darf nie umgangen werden — Sicherheitsrelevant.
- ⚠️ SSH-Verbindungsabbruch sauber behandeln: Ressourcen freigeben, Terminal-Session korrekt beenden (nicht hängen).

## Weiterführende Tasks

- [T07-002: Jump-Hosts und Tunnel](./T07-002-jump-hosts-tunnels.md)
- [T07-003: SSH-Config-Import/Export](./T07-003-ssh-config-import-export.md)
- Phase 7: SFTP-Browser
