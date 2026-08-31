# T07-003: SSH-Config-Import/Export

## Status
⏳ Pending

## Phase
6 — SSH-UI & Host-Manager

## Abhängigkeiten
T07-001 (Host-Manager und SSH-Verbindung)

## Ziel
Import und Export von SSH-Config-Einträgen zwischen der App und der `~/.ssh/config`-Datei unterstützen, sowie das Parsen und Anzeigen einer bestehenden SSH-Config.

## Kontext
Labonair bietet `parse_ssh_config_cmd`, `import_ssh_config_entries` und `export_ssh_config`. Damit kann der Benutzer:
- Seine bisherige `~/.ssh/config` einlesen und die dort definierten Hosts in die App-Datenbank übernehmen.
- Die in der App gespeicherten Hosts als `~/.ssh/config`-Einträge exportieren.
- SSH-Config-spezifische Settings (Port, User, IdentityFile, ProxyJump, etc.) korrekt interpretieren.

## Anweisungen zur Umsetzung

1. **SSH-Config-Parser.** Übernehme/verifiziere den Parser für das SSH-Config-Format aus dem Backend (T01-002). Er muss die gängigen Direktiven verstehen: `Host`, `HostName`, `User`, `Port`, `IdentityFile`, `ProxyJump`, `ProxyCommand`, `HostKeyAlgorithms`, `StrictHostKeyChecking`, und Optionen wie `Match`, `Include`.

2. **Config-Einträge auf App-Hosts mappen.** Implementiere das Abbilden der geparsten SSH-Config-Einträge auf das App-Host-Modell:
   - Host-Alias → Host-Name, HostName → Adresse, Port, User etc. übernehmen.
   - IdentityFile → ein (möglicherweise zu erstellendes) SSH-Key-Credential verweisen oder als Pfad speichern.
   - ProxyJump → Jump-Host-Zuordnung ableiten.
   - Optionen, die die App nicht abbildet, als zusätzliche Metadaten erhalten (für spätere Export/erneute Verwendung).

3. **Import-UI.** Baue die Import-Oberfläche:
   - Liste der gefundenen SSH-Config-Einträge anzeigen (Host-Alias, Hostname, Port, User).
   - Auswahl, welche Einträge importiert werden sollen.
   - Konfliktbehandlung: Einträge, die in der App bereits existieren (gleicher Alias) — überschreiben/überspringen/umbenennen.
   - Nach Import die Host-Liste aktualisieren.

4. **Export-Funktionalität.** Implementieren den Export der App-Hosts in SSH-Config:
   - Auswahl, welche Hosts exportiert werden.
   - Generierung wohlgeformter `Host`-Blöcke inkl. der relevanten Direktiven.
   - Optional: An die bestehende `~/.ssh/config` anhängen oder in eine neue Datei schreiben.

5. **Round-Trip-Konsistenz.** Verifiziere, dass ein Import→Export→Import die Hosts weitgehend verlustfrei wiederherstellt (zumindest die abgebildeten Felder).

6. **Tests schreiben.** Erstelle Tests für:
   - Parsen repräsentativer SSH-Config-Beispiele (verschiedene Direktiven, ProxyJump, Kommentare).
   - Mapping auf App-Hosts korrekt.
   - Import mit Konflikt-Handling.
   - Export erzeugt valide SSH-Config-Syntax.
   - Round-Trip.

## Akzeptanzkriterien

- [ ] SSH-Config-Dateien lassen sich parsen und die Einträge in der App anzeigen.
- [ ] Geparste Einträge mappen korrekt auf App-Hosts (Port, User, IdentityFile, ProxyJump).
- [ ] Import-UI erlaubt Auswahl + Konfliktbehandlung.
- [ ] Export erzeugt wohlgeformte SSH-Config-Blöcke.
- [ ] Round-Trip (Import→Export→Import) erhält die abgebildeten Felder.
- [ ] Alle Tests laufen grün.

## Notizen

- SSH-Config kann komplex sein (Patterns in `Host`, `Include`, `Match`). Priorisiere die üblichen Direktiven; exotische Fälle sollten gelesen aber ggf. nur informativ behandelt werden (nicht falsch abbilden).
- `IdentityFile` auf einen existerenden oder zu erstellenden Key abbilden — kein Klartext von Key-Inhalten in der App.

## Warnungen

- ⚠️ Korrektes Parsen von SSH-Config ist fehleranfällig (Patterns, Kommentare, Whitespace). Defensive Tests und keine Annahmen über Format.
- ⚠️ Beim Export nicht versehentlich die bestehende `~/.ssh/config` überschreiben ohne Benutzer-Aktivierung — nur mit klarer Aktion/Fenster.

## Weiterführende Tasks

- Phase 7: SFTP-Browser
