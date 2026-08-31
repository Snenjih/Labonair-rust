# T15-002: FAQ d. Kommentarseite nutzen - Fehlerbehandlung & Robustheit (App-weit)

## Status
⏳ Pending

## Phase
14 — Testing & Polish

## Abhängigkeiten
Alle hinteren Systeme (Backend, Terminal, SSH, SFTP, Git, AI, Settings)

## Ziel
Die app-weite Fehlerbehandlung und Robustheit überarbeiten: unerwartete Zustände, fehlgeschlagene Operationen, Netzwerkausfälle und Grenzfälle sollen überall konsistent, verständlich und ohne Absturz behandelt werden — mit klaren Meldungen, Wiederherstellungs-Optionen und sauberem Logging.

## Kontext
Die Portierung hat viele asynchrone Teilsysteme (SSH, SFTP, Terminal, AI). Fehler können an vielen Stellen auftreten. Ein einheitliches Fehler-Framework stellt sicher, dass:
- Jede Operation einen klaren Fehlerpfad hat (keine panics/abstürze bei erwarteten Fehlern).
- Fehler dem Benutzer verständlich gemeldet werden (nicht abstrakte Fehlercodes).
- Wiederherstellungs-Optionen angeboten werden, wo sinnvoll (Reconnect, Retry, Zurück).
- Logging auf jeder Ebene (Debug/Warn/Error) vorhanden ist, um Diagnose zu erleichtern.

Das `AppError`-System (T01-002) ist das Fundament; dieser Task verankert es app-weit konsistent.

## Anweisungen zur Umsetzung

1. **Fehler-Katalog.** Definiere einen konsistenten Katalog von Fehlerkategorien (SSH, SFTP, FS, Git, AI, Terminal, Einstellungen, Netzwerk) mit klaren, benutzerfreundlichen Standardmeldungen.

2. **Fehlerpfade prüfen.** Überprüfe sämtliche Nutzer-Aktionen und Hintergrund-Operationen auf konsequente Fehlerbehandlung:
   - CRUD-Operationen (Hosts, Credentials, Snippets, Themes) — Fehler wie "existiert nicht", "keine Rechte", "ungültig" sauber abfangen.
   - Terminal/Schhell-Aktionen — Start-Fehler, Exit-Codes.
   - SSH-Verbindungsabläufe — Netzwerkfehler, Auth-Fehler, Timeout, Known-Host.
   - SFTP-Operationen/-Transfers — Fehler, Verbindungsverlust.
   - Git-Operationen — nicht-repo, Konflikt, remote-Fehler, keine Rechte.
   - AI-Aufrufe — Provider-Fehler, Rate-Limit, Timeout, abgebrochen.

3. **Verständliche Meldungen.** Stelle sicher, dass alle Fehler als klare Meldungen in der UI erscheinen (Toast/Dialog/Inline) statt als technische Fehlercodes. Jede Meldung erklärt kurz Ursache und (falls möglich) nächste Schritte.

4. **Wiederherstellung/Retry.** Wo sinnvoll, dem Benutzer eine Aktion anbieten:
   - SSH-Verbindung verloren → Reconnect-Option.
   - SFTP-Transfer fehlgeschlagen → Retry.
   - Git-Ausführung → Option zur Diagnose.
   - AI-Antwort abgebrochen → erneut senden.

5. **Keine Panics/Abstürze.** Durchsuche die Codebasis nach riskanten Stellen (`unwrap()`, `expect()`, panicking-path) bei erwartbaren Fehlern und ersetze sie durch robuste Fehlerbehandlung (zurückgeben von `AppError`/entsprechende UI-Meldung). Nur wirklich invariante Fälle dürfen panicken.

6. **Logging verbessern.** Sorge für sinnvolles Logging an Fehlerstellen (mit Kontext: Operation, Parameter, Fehlercode), sodass Support/Diagnose über Logs möglich ist. Trenne Debug/Info/Warn/Error sauber.

7. **Grenzfälle definieren.** Behandle typische Grenzfälle konsistent:
   - Leere/ungültige Eingaben in Formularen.
   - Sehr große Dateien/Ordner.
   - Fehlende Ressourcen (Datei gelöscht, Remote nicht erreichbar).
   - Konfigurationsfehler (kaputte Preferences, fehlende DB).

8. **Tests schreiben.** Erstelle Fehlerpfad-Tests für die wichtigsten Kategorien:
   - SSH-Auth-/Verbindungsfehler.
   - SFTP-Operation fehlgeschlagen.
   - Git-Konflikt/kein-Repo/keine Rechte.
   - AI-Provider-Fehler/Rate-Limit.
   - FS-Fehler (keine Rechte, Datei gelöscht).
   - Formular-Validierung.

## Akzeptanzkriterien

- [ ] Ein konsistenter Fehler-Katalog mit benutzerfreundlichen Meldungen existiert je Kategorie.
- [ ] Alle Kern-Operationen haben saubere, geprüfte Fehlerpfade (keine Panics bei erwartbaren Fehlern).
- [ ] Fehler erscheinen verständlich in der UI (Toast/Dialog/Inline), mit Ursache und ggf. nächsten Schritten.
- [ ] Reconnect/Retry-Optionen werden angeboten, wo sinnvoll.
- [ ] Keine riskanten `unwrap()`/`expect()` an Stellen mit erwartbaren Fehlern; Ersetzung sauber.
- [ ] Logging ist aussagekräftig (Kontext + Status) auf allen Fehlerwegen.
- [ ] Grenzfälle werden konsistent behandelt.
- [ ] Fehlerpfad-Tests für die Hauptkategorien laufen grün.

## Notizen

- Diese Phase ist wichtig für die subjektive Qualität: Eine App, die bei Fehlern abstürzt oder kryptisch reagiert, wirkt unfertig.
- Wiederverwende die bestehenden Error- und Logging-Mechanismen (T01-004) konsequent.

## Warnungen

- ⚠️ Keine erwarteten Fehler als Panics in der Produktion — das wäre ein Stolperstein der Portierung.
- ⚠️ Fehlermeldungen sollen dem Benutzer nützen, nicht nur dem Entwickler (keine rohen Fehlercodes/Stacktraces im UI).

## Weiterführende Tasks

- [T15-003: Cross-Platform- und Performance-Optimierung](./T15-003-cross-platform-performance.md)
