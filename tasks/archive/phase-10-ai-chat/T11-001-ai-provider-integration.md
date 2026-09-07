# T11-001: AI-Provider-Integration (Multi-Provider BYOK)

## Status
✅ Done

## Phase
10 — AI-Chat-System

## Abhängigkeiten
T04-001 (Tab-System, Grundlage)
T01-003 (Referenz) für Original-Ai-Logik
(ggf. T01-001)

## Ziel
Die Multi-Provider-Integration für das AI-Subsystem implementieren: Verbindung zu mehreren AI-Anbietern (OpenAI, Anthropic, Google, xAI, Cerebras, Groq, DeepSeek, Mistral, OpenRouter, LM Studio, MLX, Ollama, OpenAI-kompatible) über deren HTTP-APIs, mit BYOK (bring-your-own-key) über den OS-Keyring, einem Provider-/Modell-Auswahl-System und einer gemeinsamen, einheitlichen Chat-/Streaming-API, die der Rest des Systems nutzt.

## Kontext
In Labonair verbindet sich das AI-System über den Vercel AI SDK mit vielen Anbietern (dynamisch importiert je Provider). Die User erstellen benannte Provider-Instanzen mit eigenem API-Key (im OS-Keyring gespeichert, nie auf der Platte), wählen ein Modell aus 34 vorkonfigurierten Modellen, und das System baut einen passenden Client.

In der Rust-Version gibt es kein JS-SDK. Stattdessen werden die Provider über `reqwest` direkt angesprochen (HTTP/JSON + Server-Sent-Events für Streaming). Der AI-Provider wird als einheitliche Schnittstelle modelliert: Nachrichteneingabe → gestreamte Antwort (Text + Tool-Calls). Unterschiede zwischen Anbietern (API-Endpunkt, Nachrichtenformat, Streaming-Format) werden in Provider-Adapter gekapselt.

## Anweisungen zur Umsetzung

1. **Provider-Abstraktion definieren.** Modelliere eine einheitliche Schnittstelle für alle Anbieter:
   - Eingabe: Konversationsverlauf (Nachrichten mit Rollen user/assistant/system/tool), Konfiguration (Modell, Temperatur, max Tokens), optional Tools.
   - Ausgabe: streamende Antwort (Text-Token und/oder Tool-Call-Ankündigungen), plus Metadaten (Modell-Name, Token-Verbrauch).
   - Fehlerbehandlung: verständliche Meldungen (Rate-Limit, Auth-Fehler).

2. **Provider-Adapter implementieren.** Für jeden unterstützten Anbieter einen Adapter, der die gemeinsame Schnittstelle auf die konkrete HTTP-API abbildet:
   - Endpunkt und Auth-Schema (Bearer-Token) korrekt setzen.
   - Nachrichten in das provider-spezifische Format umwandeln.
   - Streaming (SSE) zur Token-zu-Token-Ausgabe verarbeiten.
   - Gemeinsamkeiten refaktorisieren (viele teilen ähnliche Formate).
   - Lokale Anbieter (LM Studio, MLX, Ollama) mit ihren Standard-Endpunkten adressieren.

3. **Modell-Katalog und Auswahl.** Lege den Katalog der vorkonfigurierten Modelle an (orientiert an Labonair's ~34 Modellen): je Provider die verfügbaren Modell-IDs mit Anzeigename. Der Benutzer wählt Provider + Modell.

4. **Provider-Instanzen und Key-Speicher.** Implementiere die Verwaltung von Provider-Instanzen:
   - Pro Instanz: Name, Anbieter-Typ, Modell, API-Key, evtl. Basis-URL (für kompatible).
   - API-Key im OS-Keyring speichern (via Backend `secrets`, T01-002) — niemals im Anwendungszustand oder auf Platte.
   - Funktionen, um die Keys abzurufen/setzen/entfernen.
   - Ein aktiver Provider/Modell wird festgelegt und dem Rest des Systems bekannt gemacht.

5. **KI-Konfiguration/Präferenzen.** Verbinde die Provider-Auswahl mit der App-Präferenz (Phase 12): Der zuletzt gewählte Provider/Modell wird gespeichert und beim Start wiederhergestellt.

6. **Streaming-Verarbeitung im Backend.** Implementiere die Stream-Verarbeitung (SSE) robust:
   - Tokens inkrementell verarbeiten und an die UI/weitergabe (Streaming-Markdown ist in T11-003).
   - Tool-Calls und -Ergebnisse im Stream erkennen und behandeln.
   - Abbruch (user-stopped) und Fehler mittendrin sauber handhaben (Nachricht bleibt konsistent).

7. **Tests schreiben.** Erstelle Tests für:
   - Adapter-Konvertierung (Nachrichten/Konfiguration) korrekt je Provider (offline mit gemockten HTTP).
   - SSE-Parsing von Streaming-Antworten.
   - Fehlerbehandlung (400, 401, 429, Timeout).
   - Keyring-Speicherung/Abruf (mit Mock).

## Akzeptanzkriterien

- [ ] Eine einheitliche Provider-Schnittstelle existiert und wird von Adaptern für die unterstützten Anbieter implementiert.
- [ ] Provider-/Modell-Auswahl funktioniert; Benutzer können Provider-Instanzen mit Key (via Keyring) verwalten.
- [ ] Streaming-Antworten werden korrekt verarbeitet (Token + Tool-Calls) und können abgebrochen werden.
- [ ] Fehler (Rate-Limit, Auth, Timeout) werden verständlich gemeldet und die UI bleibt konsistent.
- [ ] Der Modell-Katalog finden sich die Labonair-Modelle wieder (angepasst an aktuelle API-Versionen).
- [ ] Der aktive Provider/Modell wird persistent gespeichert.
- [ ] Alle Tests laufen grün.

## Notizen

- Der AP-SDK-Ersatz ist der größte Teil der AI-Komplexität. Die Tests sind hier essenziell (Mocking über `wiremock` o.ä.).
- Lokale Anbieter sind besonders wichtig für Nutzer, die lokal laufen; ihre Adapter müssen die lokalen Standard-Pfade korrekt ansprechen.

## Warnungen

- ⚠️ Nie API-Keys in Logs oder persistente Zustände — nur Keyring.
- ⚠️ Streaming-Break-Mid-Stream: Die UI darf nicht mit einer halb-fertierten Nachricht beschädigt bleiben; sauberes Abbruch- und Fehler-Modell etablieren.
- ⚠️ Unterschiede im Streaming-Format zwischen Anbietern (manche liefern roles us.) korrekt je Adapter handhaben — nicht auf einem Format bauen.

## Weiterführende Tasks

- [T11-002: Chat-Store und Session-Verwaltung](./T11-002-chat-store-sessions.md)
- [T11-003: Chat-UI und Streaming-Markdown](./T11-003-chat-ui-markdown.md)
- [T11-004: Agent/Tool-System und Live-Bridge](./T11-004-agent-tool-system.md)
