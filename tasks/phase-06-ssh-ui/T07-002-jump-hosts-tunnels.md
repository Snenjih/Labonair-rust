# T07-002: Jump-Hosts und Tunnel

## Status
✅ Done

## Phase
6 — SSH-UI & Host-Manager

## Abhängigkeiten
T07-001 (Host-Manager und SSH-Verbindung)

## Ziel
Jump-Host-Routing und Tunnel-Funktionalität implementieren: Verbinden durch einen oder mehrere Jump-Hosts (Bastion), sowie das Aufsetzen lokaler/ferner/dynamischer Port-Forwarding-Tunnel zu Zielhosts, teilweise über den Jump-Host. Zusätzlich die UI-Elemente für Tunnel-Verwaltung und den Jump-Host-Status.

## Kontext
In Labonair gibt es:
- **Jump-Hosts**: Ein Host kann hinter einem anderen Host liegen (Bastion/Router), d.h. die Verbindung läuft Host → Jump-Host → Ziel. Dies ermöglicht es, auf Systeme in einem privaten Netz zuzugreifen.
- **Tunnel**: `ssh_start_tunnels` / `ssh_stop_tunnels` setzen Port-Forwarding auf (Local → Remote:Port, Remote → Local, dynamisch/SOCKS). Diese werden über die SSH-Verbindung realisiert.

In der Rust-Version wird die russh-basierte Tunnel-/Jump-Logik aus dem Backend (T01-002) übernommen, und das UI stellt die Verwaltung bereit.

## Anweisungen zur Umsetzung

1. **Jump-Host-Attribut.** Erweitere das Host-Modell um die Jump-Host-Zuordnung: Ein Host kann einen (oder mehrere, je nach Backend-Unterstützung) Jump-Host bzw. eine Jump-Host-Hierarchie referenzieren. Das Konfigurieren im Host-Formular bereitstellen.

2. **Verbindung durch Jump-Hosts.** Implementiere das Verbindungsrouting:
   - Beim Verbinden zu einem Ziel-Host zuerst die Verbindung zum Jump-Host herstellen und authenitifizieren (ggf. mit eigenem Credential).
   - Von dort mittels port-forward/direct-tcpip zur Ziel-Host-Verbindung. Wenn vorhanden, verschachtelte Jump-Hosts unterstützen.
   - Fehlerbehandlung pro Hop (Auth-Fehler am Jump-Host mit eigener Meldung).

3. **Tunnel-Verwaltung im Backend.** Verifiziere/übernehme die Tunnel-Implementierung aus dem Backend:
   - Lokale Weiterleitung (bind auf localhost:Port → Ziel:ZielPort).
   - Ferne Weiterleitung (Zielseitige bind).
   - Dynamische Weiterleitung (SOCKS-Proxy).
   - Starten und Stoppen von Tunneln; Zuordnung eines Tunnels zur einer SSH-Session.

4. **Tunnel-UI.** Baue eine Anzeige/Verwaltung der aktiven Tunnel:
   - Liste laufender Tunnel (Quelle → Ziel, Typ, Status).
   - Erstellen eines Tunnels (Form: Typ, lokale/remote-Ports, Zielhost/Port).
   - Stoppen eines Tunnels.
   - Fehleranzeige.

5. **Jump-Host-Status.** Zeige an, wenn eine Verbindung über Jump-Hosts läuft (z.B. ein Badge in der Statusleiste / Header für die aktive Verbindung). Analyst oder getrennt deutlich machen, ob man über einen Bastion verbunden ist.

6. **Reconnect-Verhalten.** Beim Verbindungsverlust eines Tunnels oder einer Jump-Verbindung:
   - Sinnvolle Meldung.
   - Optionaler Reconnect (mit Erneut-Auth wenn nötig).

7. **Tests schreiben.** Erstelle Tests für:
   - Jump-Host-Auflösung und Routing-Logik (gegen lokale Test-Server als Bash/Jump).
   - Tunnel-Aufbau/-Abbau (lokal/remote/dynamisch), gegen Test-Server auf localhost.
   - Fehlerbehandlung (Ziel nicht erreichbar, Auth-Fehler).
   - Jump-Host-Status korrekt in der UI.

## Akzeptanzkriterien

- [ ] Hosts lassen sich mit Jump-Host-Zuordnung konfigurieren (Formular).
- [ ] Verbinden durch Jump-Hosts funktioniert (Test-Server als Jump/Bastion).
- [ ] Tunnels (lokal/remote/dynamisch) lassen sich starten, anzeigen und stoppen.
- [ ] Ein Tunnel-UI (Liste, Erstellen, Stoppen) ist vorhanden.
- [ ] Der Jump-Host-Status ist in der UI sichtbar (Badge/Status).
- [ ] Fehler- und Reconnect-Verhalten sind sauber.
- [ ] Alle Tests laufen grün.

## Notizen

- Jump-Hosts und Tunnels sind für den Einsatz hinter Firewalls/privaten Netzen zentral. Qualität der Verbindungsstabilität wichtig.
- Die russh-Bibliothek unterstützt Tunnel und Jump-a-Way via direkte TCP-Weiterleitung; nutze dies im Backend.

## Warnungen

- ⚠️ Mehrstufige Jump-Hosts erhöhen die Fehleranfälligkeit — jede Stufe braucht saubere Fehler-/Auth-Meldungen.
- ⚠️ Tunnel-Bindungen (lokal) nicht blind auf alle Interfaces, sondern standardmäßig nur auf localhost binden (Sicherheit).

## Weiterführende Tasks

- [T07-003: SSH-Config-Import/Export](./T07-003-ssh-config-import-export.md)
