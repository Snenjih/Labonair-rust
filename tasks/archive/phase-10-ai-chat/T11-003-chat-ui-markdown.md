# T11-003: Chat-UI und Streaming-Markdown

## Status
✅ Done

## Phase
10 — AI-Chat-System

## Abhängigkeiten
T11-002 (Chat-Store und Session-Verwaltung)

## Ziel
Die Benutzeroberfläche des AI-Chats implementieren: eine Chat-Ansicht, in der die Konversation angezeigt wird (Nachrichten, Rollen-Styling, gestreamte Markdown-Antworten mit Codeblöcken), ein Eingabebereich (Composer) mit Senden/Stoppen und Anhängen, sowie die Zustandsanzeigen (Provider/Modell, Busy-Indicator). Markdown (Streaming) muss gerendert werden.

## Kontext
Labonair hat einen AI-Begleiter mit:
- Einem Chat-Fenster/Animierung (ständige oder dockbar) mit Nachrichtenliste.
- Streaming-Markdown-Rendering (via `streamdown`), das Output laufend aktualisiert — inkl. Codeblöcke mit Syntax-Highlighting, Inline-Formatierung.
- Einem Composer (Eingabefeld) mit Senden/Stoppen, Anhängen (Bilder, Textdateien, Selections aus Terminal/Editor).
- Provider-/Modell-Auswahl, Sessions-Switch (aus T11-002), Busy/Agent-Status.

In der Rust-Welt ersetzt ein GPUI-Markdown-Renderer das Web-`streamdown`. Dieser Task baut die komplette Chat-UI auf Basis der Session/Provider-Logik.

## Anweisungen zur Umsetzung

1. **Nachrichtenliste rendern.** Implementiere die Darstellung der Konversation:
   - Nachrichten nach Rolle gestylt (User nach rechts/andersfarbig, Assistant links, System/Tool kompakter).
   - Scrollen bei neuen Nachrichten (auto-scroll ans Ende, wenn der Benutzer oben ist nicht springen).
   - Anhänge in Nachrichten darstellen (Bild-Thumbnails, Datei-Chips, Selection-Blöcke).

2. **Streaming-Markdown-Rendering.** Implementiere das Markdown-Rendering inkl. Streaming:
   - Ein Markdown-Parser/-Renderer (z.B. ein Rust-Markdown-Engine der GPUI-Welt oder eigene Token-Logik) einsetzen.
   - Während des Streamings die gerenderte Ansicht inkrementell aktualisieren (partielle Blöcke, laufender Text) — flüssig ohne Komplett-Neu-Rendering.
   - Codeblöcke mit Syntax-Highlighting (TreeSitter aus T06-002 oder Shiki-Ersatz) und Kopieren-Button.
   - Inline-Formatierungen (Bold, Italic, Links, Inline-Code), Listen, Tabellen.

3. **Composer (Eingabebereich).** Implementiere den Eingabebereich:
   - Mehrzeiliges Textfeld (Enter sendet, Shift+Enter neue Zeile üblich).
   - Senden-Button (deaktiviert bei leer/laufend).
   - Stopp-Button während einer laufenden Antwort.
   - Anhang-Auswahl: Bilddateien, Textdateien, sowie Selections (aus Terminal/Editor) — werden als Markierung/chips vor dem Absenden angezeigt und beim Absenden als Teil der Nachricht eingebettet.
   - Keyboard-Shortcuts (Enter senden, etc.).

4. **Zustandsanzeige.** Zeige den Systemzustand:
   - Gewählte Provider/Modell (aus T11-001).
   - Busy-/Agent-Status (läuft/denkt/wartet auf Tool-Genehmigung).
   - Session-Auswahl (Auswahl-/Neue-Sitzung) im Kopfbereich (aus T11-002).

5. **Tool-Genehmigungs-Karten.** Für Tool-Calls, die Genehmigung erfordern (aus T11-002/T11-004), eine eingebettete Karte in der Nachricht anzeigen:
   - Angezeigte Operation (z.B. "Schreibe Datei X" oder "Führe Kommando aus").
   - Genehmigen/Ablehnen-Buttons.
   - Status (wartend, genehmigt, abgelehnt, läuft).

6. **Layout-Integration.** Integriere die Chat-Ansicht als eigenen Tab-Typ oder eingebetteten Bereich, damit sie mit dem restlichen Workspace koexistiert (dockbar/unandockbar ähnlich Labonair). Kleines (Minimized) und maximiertes (Full) Layout berücksichtigen.

7. **Tests schreiben.** Erstelle Tests für:
   - Streaming-Markdown-Rendering erzeugt korrekte Struktur (Stichprobentests).
   - Composer-Logik (Text, Anhänge, Senden/Stoppen-Zustände).
   - Auto-Scroll-Verhalten.
   - Tool-Genehmigungs-Karten-Zustände.
   - Nachrichtenlisten-Rendering nach Rollen.

## Akzeptanzkriterien

- [x] Die Chat-Ansicht zeigt die Konversation nach Rolle gestylt und scrollbar.
- [x] Streaming-Markdown wird inkrementell und flüssig gerendert, inkl. Codeblöcke mit Highlighting und Kopieren.
- [x] Der Composer unterstützt Senden/Stoppen, mehrzeilige Eingabe, Anhänge (Bild/Text/Selection) mit Chip-Anzeige.
- [x] Provider/Modell- und Busy-Status werden angezeigt; Session-Verwaltung (Auswahl/Neu) funktioniert.
- [x] Tool-Genehmigungs-Karten mit Aktionen und Status funktionieren.
- [x] Die Chat-Ansicht ist in das Workspace-Layout integriert (als dockbares Sidebar-Panel `SidebarPanel::Ai`).
- [x] Alle Tests laufen grün.

## Notizen

- Die Streaming-Markdown-Darstellung ist entscheidend für das Nutzungsgefühl — achte auf fließende, inkrementelle Updates (nicht alle 100ms Komplett-Neu-Rendering des ganzen Verlaufs).
- Anhänge (Selections aus Terminal/Editor) sind ein Alleinstellungsmerkmal von Labonair — sie als `<selection>`-Blöcke in der Nachricht korrekt einbetten.

## Warnungen

- ⚠️ Streaming-Framer und große Konversationen: inkrementelles Rendering nur geänderter Teile, sonst wird die UI träge.
- ⚠️ Auto-Scroll: Nur wenn der Benutzer am Ende ist, ans Ende springen — sonst reißt man ihm die Position aus.

## Weiterführende Tasks

- [T11-004: Agent/Tool-System und Live-Bridge](./T11-004-agent-tool-system.md)
