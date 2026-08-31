# T03-003: Tastatur- und Maus-Mapping

## Status
✅ Done

## Phase
2 — Terminal-Engine

## Abhängigkeiten
T03-002 (GPUI-Terminal-Renderer)

## Ziel
Die vollständige Übersetzung von GPUI-Tastatur- und Mausereignissen in Terminal-Eingabe-Sequenzen (wie sie die Shell/`alacritty_terminal` erwartet). Dazu gehören alle Modifier-Kombinationen, Funktions- und Steuertasten, ANSI-Keypad-Modi, sowie Maus-Interaktionen (Klick, Scrollen, Selektion, Drag) mit den entsprechenden Maus-Protokollen.

## Kontext
Eine Terminal-Eingabe ist nicht einfach "der gedrückte Buchstabe". Ein Terminal entwickelt Escape-Sequenzen, die Sonderzeichen (z.B. Pfeiltasten, Entf, Strg+Kombinationen, F1–F12, Home/End), und dabei muss der aktuelle Terminal-Modus entropy (z.B. ob Application Cursor Mode, Application Keypad, Insert-Modus, Bracketed Paste aktiv sind) berücksichtigt werden.

In der Tauri/React-Version erledigte xterm.js genau diese Übersetzung im Frontend. In der Rust-Version übernimmt die Terminal-Engine (auf `alacritty_terminal`-Basis) bereits einen Teil der Modus-Logik, aber die rohe GPUI-Tastatur-/Mausereignis-→-Escape-Übersetzung muss selbst implementiert werden. Hier greift man auf das gleiche Muster zurück, das Zed nutzt (eine eigene Zuordnungsebene zwischen GPUI-Eingabe und alacritty).

Zusätzlich muss die Browser-/Native-Kompatibilität für erweiterte Protokolle hergestellt werden: Kitty-Keyboard, Bracketed Paste, Modified Cursor Keys, etc.

## Anweisungen zur Umsetzung

1. **Tastatur-Grundabdeckung.** Implementiere die Zuordnung für:
   - Alphanumerische Tasten inkl. Shift-Funktion.
   - Steuertasten-Kombinationen (Strg+Buchstabe → Byte 1–26).
   - Funktionstasten F1–F12 (mit korrektem Modus: normale vs. Anwendungs-Cursor-Modus).
   - Navigationstasten (Pfeile, Home, End, Pos1, Ende, PageUp/PageDown, Entf, Backspace, Tab).
   - Lock- und Spezialtasten (Esc, Enter, Leertaste).
   - Alt-/Option-Kombinationen (Meta-Präfix, je nach macOS-Terminalverhalten).
   - Kombinationen wie Strg+Pfeile, Alt+Pfeile, Strg+Alt+Pfeile (für word-jump, Line-Scrolling u.a.).

2. **Modus-bewusste Ausgabe.** Die erzeugte Escape-Sequenz muss die aktuellen Terminal-Modi berücksichtigen:
   - Application Cursor Mode (DECCKM) → andere Pfeiltasten-Sequenzen.
   - Application Keypad Mode (DECKPAM) → Ziffernblock als Anwendungs-Sequenzen.
   - Insert-Modus.
   - Modified Cursor Keys und andere erweiterte Modi.
   Die Modus-Zustände werden von der `alacritty_terminal`-Engine verwaltet — hole sie dort ab und wende sie bei der Sequenzerzeugung an.

3. **Erweiterte Tastatur-Protokolle.** Unterstütze die neueren Protokolle, die moderne Shells und Programme aktivieren:
   - Bracketed Paste (Terminal hat sie aktiviert → beim Einfügen Inhalt in eckige-Klammer-Escape-Sequenzen packen).
   - Kitty-Keyboard-Protokoll (fallback zu konventionellen Sequenzen, falls nicht unterstützt).
   
4. **Sonderfälle macOS.** Behandle macOS-spezifische Tastaturereignisse für die korrekte Interaktion mit der Terminal-Shell, z.B. wie Option/Alt gesendet wird (Meta vs. Escape-Präfix), und die Behandlung der Cmd-Taste (die i.d.R. an App-Aktionen geht, nicht ans Terminal).

5. **Maus-Ereignis-Mapping.** Implementiere die Übersetzung von GPUI-Mausereignissen in Terminal-Eingaben:
   - Klick: Cursor-Position setzen und Maus-Sequenzen (SGR-Modus) senden, wenn das Maus-Protokoll aktiv ist.
   - Scrollen (Mausrad): Entweder Escape-Sequenzen (im Maus-Modus) oder Scrollback-Navigation (Standard, wenn das Maus-Protokoll nicht aktiv).
   - Drag: Textauswahl erweitern; bei aktivem Maus-Selektions-Protokoll die entsprechenden Sequenzen senden.
   - Zeilen- und Spaltenzuordnung auf die angeklickte Zelle.

6. **Einfüge- und Zwischenablage.** Implementiere die Verbindung zwischen Terminal-Auswahl, Kritischer-Zwischenablage (Copy) und Einfügen (Paste), inkl. des Bracketed-Paste-Verhaltens und der Unterstützung des OSC-52-Clipboard-Protokolls, wo es die App vorsieht.

7. **Sequenz-Modularisierung.** Strukturiere die Sequenzerzeugung übersichtlich, damit sie leicht testbar und erweiterbar ist (eine Funktion pro Tasten-Kategorie). Vermeide eine unübersichtliche Riesenzuordnung.

8. **Tests schreiben.** Erstelle einen umfassenden Test-Katalog, der für eine Reihe von Eingaben die korrekte Escape-Sequenz verifiziert:
   - Einfache und Modifier-Tastenkombinationen.
   - Alle Funktionstasten in beiden Cursor-Modi.
   - Navigationstasten mit/ohne Modifier.
   - Alt-/Meta-Kombinationen.
   - Maus-Klick/Scroll-Mit/Ohne-Protokoll.
   - Bracketed Paste- und OSC-52-Verhalten.

## Akzeptanzkriterien

- [ ] Alle grundlegenden und erweiterten Tastatur-Eingaben erzeugen korrekte Escape-Sequenzen.
- [ ] Die Sequenzerzeugung berücksichtigt die aktiven Terminal-Modi (Application Cursor/Keypad, Insert, etc.).
- [ ] Bracketed Paste und Kitty-Keyboard funktionieren (bei aktiven Protokollen).
- [ ] macOS-Sonderfälle (Option/Alt-Handling) sind korrekt.
- [ ] Maus-Klick, Scrollen und Drag-Selektion funktionieren.
- [ ] Kopieren/Einfügen funktioniert inkl. Bracketed Paste und OSC-52.
- [ ] Die Sequenzerzeugung ist modular und getestet.
- [ ] Ein umfassender Test-Katalog läuft grün und deckt die oben genannten Kategorien ab.

## Notizen

- Nutze die alacritty-Terminal-Modi, aber übernimm die rohe GPUI→Sequenz-Zuordnung selbst. Zed hat diesen Ansatz — als Inspiration, nicht als Kopiervorlage.
- Vergiss nicht, dass viele Shells (zsh, bash) das VT-Keypad und die erweiterten Protokolle nur dann aktivieren, wenn der Terminal sie meldet (Terminal-Capabilities/DA1-Antworten). Bringe die Terminal-Engine in den Zustand, die passenden Antworten auf Sende-abfragen zu liefern.

## Warnungen

- ⚠️ Tastatur-Zuordnung ist ein häufiger Fehlerquelle — besonders bei Modifier-Kombinationen (Strg+Pfeil) und beim Unterschied zwischen Apps-Cursor vs. normalen Sequenzen.
- ⚠️ Vergesse nicht das Verhalten von Alt beim Drücken und Loslassen in Verbindung mit Modifier-Status auf macOS.
- ⚠️ Kitty- und Modified-Cursor-Sequenzen verlassen sich auf Terminal-Capabilities. Melde nur, was die Engine wirklich unterstützt, sonst programmiert die Shell auf falsche Annahmen.

## Weiterführende Tasks

- [T03-004: Shell-Integration und CWD-Tracking](./T03-004-shell-integration-cwd.md)
