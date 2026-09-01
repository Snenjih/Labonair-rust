# T11-002: Chat-Store und Session-Verwaltung (AI)

## Status
✅ Done

## Phase
10 — AI-Chat-System

## Abhängigkeiten
T11-001 (AI-Provider-Integration)

## Ziel
Den Zustand und die Sitzungsverwaltung des Chat-Systems implementieren: Chat-Sessions (Konversationen) mit Nachrichtenlisten, Persistenz über Neustarts, ein aktiver Session-/Provider-Zustand, und die Grundlage für die Chat-UI. Erstellen/Wechseln/Löschen von Sessions soll möglich sein, ebenso das Speichern/Starten von Konversationen.

## Kontext
In Labonair organisiert das AI-System die Konversationen in benannte Sessions. Jede Session hat eine Nachrichtenliste (Messages mit Rollen, Text, Tool-Aufrufen, ggf. Anhängen). Die Sessions werden optimiert und persistent (über tauri-plugin-store) gehalten; die aktiv-Session sowie der aktive Provider/Key werden gespeichert. Wechselt man den aktiven Provider/Key, wird der in-memory-Chat zurückgesetzt, aber die Sessions bleiben.

In der Rust-Version: Ein Chat-Store (GPUI-Entity) verwaltet Sessions (Liste + aktive ID) und deren Nachrichten. Die Nachrichten-Logik der Provider (T11-001) wird hier orchestriert (Senden, Antworten anhängen, Tool-Ausführung integrieren — die Tools kommen in T11-004).

## Anweisungen zur Umsetzung

1. **Nachrichten-Modell.** Definiere das Nachrichten-Modell:
   - Rollen: System, User, Assistant, Tool.
   - Inhalte: Klartext und/oder strukturierte Teile (Text, Tool-Calls, Tool-Ergebnisse, ggf. Anhänge wie Bilder oder Selections aus T11-004).
   - Zeitstempel, status (laufend/final/fehlerhaft) für UI-Zustand.

2. **Session-Store.** Implementiere die Session-Verwaltung:
   - Liste von Sessions (ID, autom. Titel aus erster User-Nachricht, Zeitpunkt).
   - Aktive Session-ID.
   - Erstellen, Wechseln, Löschen einer Session.
   - Löschen: Nachfrage bei Verlust, clean.

3. **Persistenz.** Persistenz der Sessions über App-Neustarts:
   - Sessions-Liste + aktive ID persistent speichern (in einem lokalen Datenspeicher der App).
   - Pro Session die Nachrichtenliste speichern und beim Start wiederherstellen.
   - Speicherfrequenz sinnvoll wählen (beim Senden/Antworterhalten und bei Änderungen), ohne die Performance zu beeinträchtigen.
   - Beim Wechsel des aktiven Providers/Key: in-memory der laufenden Session zurücksetzen (Chat neu), aber Session-Daten persistent behalten.

4. **Sende-/Antwort-Orchestrierung.** Implementiere die Kern-Orchestrierung des Sends:
   - Benutzer-Nachricht an die Session anhängen.
   - Über T11-001 den (Streaming-)Antwort vom aktiven Provider/Modell anfordern, unter Verwendung des Konversationsverlaufs.
   - Die Streaming-Antwort Token für Token an die Session/UI weitergeben.
   - Nach Abschluss (oder Fehler) die Nachricht finalisieren.
   - Stoppen einer laufenden Antwort ermöglichen.

5. **Tool-Call-Integration (Grundgerüst).** Erkenne Tool-Calls in der Streaming-Antwort und halte sie bis zur Ausführung (die Ausführungslogik und Sicherheit kommen in T11-004). Das Grundgerüst soll den Tool-Call in der Session repräsentieren und den Zustand "wartend auf Genehmigung/Ausführung" halten.

6. **Zustands-Benachrichtigung.** Stelle sicher, dass Änderungen (neue Nachricht, Streaming-Token, Session-Wechsel) die UI benachrichtigen (GPUI-Notify).

7. **Tests schreiben.** Erstelle Tests für:
   - Session-Erstellen/Wechseln/Löschen logisch korrekt.
   - Persistenz: Sessions und Nachrichten überstehen ein (simulierten) Neustart.
   - Senden eines Befehls erzeugt eine korrekte Nachrichtenabfolge (Mock-Provider).
   - Stoppen einer Antwort funktioniert.
   - Aktiver-Provider-Wechsel setzt laufende Chat korrekt zurück, ohne Sessions zu löschen.

## Akzeptanzkriterien

- [x] Nachrichten-Modell mit Rollen/Struktur existiert. (`SessionMessage` / `MessageStatus` / `SessionToolCall` / `ToolCallStatus` in `crates/ai/src/sessions.rs`)
- [x] Sessions lassen sich erstellen, wechseln, löschen; aktive ID und Titel funktionieren. (`SessionStore` + auto-title via `derive_title`)
- [x] Sessions und Nachrichten werden persistent über Neustarts gespeichert und geladen. (atomic JSON blob `~/.config/labonair/labonair-sessions.json`; `sessions_and_messages_survive_restart` test)
- [x] Senden einer Nachricht orchestriert die Provider-Streaming-Antwort korrekt in die Session. (`begin_send` → `AiClient::stream_chat` → `apply_event` → `finish_run`, wired in `AiChatStore`)
- [x] Stoppen funktioniert; Tool-Calls werden erkannt und im Wartestatus gehalten. (`stop`; `ToolCallStatus::AwaitingApproval` + `RunStatus::AwaitingApproval`)
- [x] Änderungen benachrichtigen die UI korrekt. (`SessionStore::revision` counter; `AiChatStore` calls `cx.notify()` after every mutation — `session_ops_notify` test)
- [x] Alle Tests laufen grün. (ai 44, ui 111; clippy + fmt clean)

## Notizen

- Sessions sind das dauerhafte Gedächtnis der AI — Persistenz hier sorgfältig.
- Die Trennung "Provider-Wechsel setzt Chat zurück, Sessions bleiben" ist ein Kern-Prinzip von Labonair's AI — korrekt umsetzen.

## Warnungen

- ⚠️ Persistenz-Schreiben darf nicht die Streaming-Performance stören (nicht jede Token-Aktualisierung auf Platte schreiben).
- ⚠️ Beim Löschen einer aktiven Session sauber auf eine existierende/neue Session umschalten.

## Weiterführende Tasks

- [T11-003: Chat-UI und Streaming-Markdown](./T11-003-chat-ui-markdown.md)
- [T11-004: Agent/Tool-System und Live-Bridge](./T11-004-agent-tool-system.md)
