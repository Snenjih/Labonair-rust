# T03-001: alacritty_terminal als Terminal-Engine einbinden

## Status
✅ Done

## Phase
2 — Terminal-Engine

## Abhängigkeiten
T01-001 (Cargo Workspace)
T02-004 (Terminal-ANSI-Palette in Theme integrieren)

## Ziel
Die terminalemulationsbibliothek von Alacritty (`alacritty_terminal`) als Kern der Terminal-Engine in das Projekt einbinden. Das Ziel ist eine funktionierende, nicht interaktive Terminal-Logik, die korrekt mit PTY gestartet wird, ANSI/Escape-Sequenzen verarbeitet, Scrollback verwaltet und Zell-Daten für die spätere GPUI-Rendering-Schicht bereitstellt. Zuerst nur Backend-Logik — das visuelle GPUI-Rendering folgt in einem eigenen Task.

## Kontext
Alacritty's Terminal-Logik ist als eigenständige Crate (`alacritty_terminal`) verfügbar und bewusst so gebaut, dass sie ohne Alacritty's Renderer nutzbar ist. Sie enthält die komplette ANSI/VTE-Parsierung, das Zell-Grid, Scrollback, Auswahl, Vi-Modus und das Ereignissystem. Zed selbst nutzt exakt diese Crate und baut darüber seine GPUI-Terminalansicht.

Das ist der gleiche Ansatz, der hier verwendet werden soll:
- Die `alacritty_terminal`-Crate stellt die Terminal-Logik bereit (kein Rendering).
- Eine eigene Schicht verbindet sie mit dem PTY-I/O (Ein-/Ausgabe), nutzt das Ereignissystem, um auf Daten und Zustandsänderungen zu reagieren, und bereitet die Zell-Daten für das GPUI-Rendering auf.

Wichtig: In der Original-Labonair-App lief das Terminal über `xterm.js` im Frontend und die Tauri-Rust-Schicht nur als PTY-Brücke (Ein-/Ausgabe-Bytes via `invoke`). Der gesamte Terminal-Emulationsaufwand (Parsen von Escape-Sequenzen, Grid, Scrollback) lag in xterm.js. In der Rust-App übernimmt `alacritty_terminal` genau diese Aufgabe — ein Mutterbau der Emulation in die Rust-Seite.

## Anweisungen zur Umsetzung

1. **Bibliothek als Abhängigkeit aufnehmen.** Trage `alacritty_terminal` (eine aktuelle, stabile Version) in die Abhängigkeiten des Terminal-Crates ein. Verifiziere, dass sie zusammen mit den bestehenden Abhängigkeiten (Tokio, PTY, serde) kompiliert, ohne Alacritty oder eine grafische Umgebung zu benötigen.

2. **Terminal-Sitzungsmodell entwerfen.** Lege das Datenmodell für eine Terminal-Session im Rust-Code fest. Eine Session repräsentiert eine laufende, soldatische Terminal-Instanz mit:
   - Terminal-Zustand (Grid, Scrollback, Cursor, Auswahl).
   - Zugeordnetem PTY.
   - Dimensionen (Anzahl Zellen Breite × Höhe).
   - Event-Sender/-Empfänger für Daten-Streams.
   - Metadaten wie Titel, Prozessname und aktuelles Arbeitsverzeichnis.

3. **PTY-Start und -Einbindung.** Implementiere das Starten einer echten Shell (Standard-Shell des Systems, zumeist zsh oder bash) über ein PTY. Das PTY muss mit den gewünschten anfänglichen Terminal-Dimensionen geöffnet werden. Der Ausgabe-Stream des PTY muss in die Terminal-Logik eingespeist werden (Byte-Stream → VTE-Parser), und Eingabe-Bytes des Benutzers müssen an das PTY geschrieben werden. Dafür einen Hintergrund-Lese-Mechanismus (asynchron zur UI) einrichten, der PTY-Ausgabe kontinuierlich einsammelt und der Terminal-Logik zuführt.

4. **Ereignisbehandlung einrichten.** Implementiere die EventListener-Schnittstelle der `alacritty_terminal`-Crate. Die wichtigsten Ereignisse, auf die reagiert werden muss:
   - Daten-Ausgabe an das PTY (Bytes, die die Shell erhält).
   - Anforderungen an einen Wakeup (Terminal-Inhalt hat sich geändert → Rendering anstoßen).
   - Titel-Änderung.
   - OSC-Sequenzen (z.B. Arbeitsverzeichnis-Änderung via OSC 7 und 133).
   Diese Ereignisse müssen so angebunden werden, dass ein UITeil (im nächsten Task) auf Änderungen reagieren kann.

5. **Dimensionen/Resize.** Implementiere das Ändern der Terminal-Dimensionen (Columns × Rows) über eine Reize-Methode, die sowohl dem Terminal-Zustand als auch dem zugrunde liegenden PTY mitgeteilt wird.

6. **Scrollback- und Auswahl-Grundlagen.** Stelle sicher, dass Scrollback korrekt konfiguriert wird und terminaltypische Operationen (Alternate Screen für Full-Screen-Applikationen wie Vim) funktionieren. Grundlegende Auswahl-Operation (Textmarkierung) muss im Terminal-Zustand möglich sein, auch wenn die visuelle Interaktion erst im Rendering-Task umgesetzt wird.

7. **Zell-Daten für das Rendering aufbereiten.** Implementiere eine Methode, die den "renderbaren Inhalt" des Terminals liefert: eine strukturierte Aufzählung der zu zeichnenden Zellen (Zeichen, Vordergrund-Farbe, Hintergrund-Farbe, Attribute wie Fett/Kursiv/Unterstrichen). Diese Daten werden im nächsten Task vom GPUI-Rendering konsumiert. Die Farben müssen dabei aus der Theme-Palette (T02-004) stammen.

8. **Test-Terminal erzeugen.** Schreibe einen kleinen Smoke-Test (oder ein Debug-Binär), das eine Shell startet, einen Befehl ausführt und die ersten Zeilen Terminal-Inhalt auf Konsole ausgibt — ohne GUI. Damit lässt sich verifizieren, dass die gesamte PTY→Parser→Zell-Kette funktioniert, bevor die grafische Ebene gebaut wird.

9. **Tests schreiben.** Erstelle Tests, die:
   - Das Starten und Beenden einer PTY-Session verifizieren.
   - Das Verarbeiten einfacher ANSI-/Escape-Sequenzen verifizieren (z.B. Cursor-Bewegung, Farbwechsel, Klartext-Eingabe).
   - Das Resize-Verhalten verifizieren (Breite/Höhe wirken sich auf Grid und PTY aus).
   - Scrollback-Funktionalität verifizieren (mehr Zeilen produzieren als sichtbar sind).

## Akzeptanzkriterien

- [ ] Ein Terminal-Crate existiert und nutzt `alacritty_terminal` als Kern.
- [ ] Eine Shell lässt sich über PTY starten und ein Befehl wird korrekt im Terminal-Zustand verarbeitet.
- [ ] ANSI-/Escape-Sequenzen werden korrekt in Zellen übersetzt (verifiziert anhand eines Text-Benchmarks ohne GUI).
- [ ] Resize wirkt auf Terminal und PTY.
- [ ] Scrollback und Alternate Screen funktionieren.
- [ ] Eine Methode liefert strukturierte Zell-Daten (Zeichen + Farben aus der Theme-Palette).
- [ ] Das Ereignissystem reicht Änderungen (Titel, CWD, Wakeup) korrekt weiter.
- [ ] Die Debug-Ausgabe zeigt korrekte Terminal-Inhalte und bestätigt die Kette PTY→Parser→Zellen ohne GUI.
- [ ] Alle Tests laufen grün.

## Notizen

- Der Fokus dieses Tasks liegt ausschließlich auf der Terminal-Logik — kein GPUI-Rendering. Trennung von Logik und Darstellung ist Absicht.
- Der Ansatz "alacritty_terminal als Kern" ist exakt der, den auch Zed verwendet. Du kannst Zed's Vorgehen inspiriert aber nicht kopiert nutzen — eigene, schlanke Struktur aufbauen.
- Das PTY-Handling (portable-pty) stellt die Verbindung zum System-Shell her.

## Warnungen

- ⚠️ `alacritty_terminal` verfügt über einen begleitenden Konfigurationstyp — übernimm nur, was benötigt wird, und übernimm die Theme-Farben aus der eigenen Palette (T02-004) statt Alacritty-Defaults.
- ⚠️ Vermeide es, das Terminal-Rendering direkt mit Alacritty's OpenGL-Renderer zu koppeln — das würde den Vorteil der sauberen Trennung zunichte machen.
- ⚠️ Das Ereignissystem muss thread-sicher sein: PTY-I/O läuft asynchron zur UI. Stelle sicher, dass Zustandsänderungen sicher über Kanäle an die UI-Schicht gelangen (keine direkten Schreibzugriffe aus dem I/O-Thread).

## Weiterführende Tasks

- [T03-002: GPUI-Terminal-Renderer für Zellen bauen](./T03-002-gpui-terminal-renderer.md)
- [T03-003: Tastatur- und Maus-Mapping](./T03-003-keyboard-mouse-mapping.md)
- [T03-004: Shell-Integration und CWD-Tracking](./T03-004-shell-integration-cwd.md)
