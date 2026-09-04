# Terminal Concept — Verbesserungen & Feature-Porting aus Zed

> **Status:** Konzept / Entwurf
> **Autor:** Niklas + Claude Code
> **Erstellt:** 2026-09-04
> **Grundlage:** Analyse von Labonair-rust `crates/terminal/` + `crates/workspace/src/views/terminal.rs` vs. Zed `crates/terminal/` + `crates/terminal_view/`

Dieses Dokument sammelt **alle** terminalbezogenen Verbesserungen, Feature-Ports aus Zed und offene Designfragen. Es dient als Planungsgrundlage fuer zukuenftige Tasks (nach Erreichen der Feature-Paritaet).

---

## Inhaltsverzeichnis

1. [Aktueller Stand Labonair](#1-aktueller-stand-labonair)
2. [Vergleich mit Zed — Luecken-Uebersicht](#2-vergleich-mit-zed--luecken-uebersicht)
3. [A. Rendering & Performance](#a-rendering--performance)
4. [B. Selektion & Copy/Paste](#b-selektion--copypaste)
5. [C. Suche](#c-suche)
6. [D. Vi Mode](#d-vi-mode)
7. [E. Hyperlinks & Pfad-Erkennung](#e-hyperlinks--pfad-erkennung)
8. [F. Terminal-Panel & Tab-Management](#f-terminal-panel--tab-management)
9. [G. Task-Integration](#g-task-integration)
10. [H. Settings & Config](#h-settings--config)
11. [I. Render-Details & Micro-Features](#i-render-details--micro-features)
12. [J. Umgebungs-Integration](#j-umgebungs-integration)
13. [K. Persistence & Serialisierung](#k-persistence--serialisierung)
14. [L. ANSI-Verarbeitung](#l-ansi-verarbeitung)
15. [M. Allgemeine Verbesserungen](#m-allgemeine-verbesserungen)
16. [Edge Cases & Stolpersteine](#edge-cases--stolpersteine)
17. [Offene Fragen](#offene-fragen)
18. [Nicht portieren](#nicht-portieren)
19. [Umsetzungs-Reihenfolge](#umsetzungs-reihenfolge)

---

## 1. Aktueller Stand Labonair

### Dateistruktur

| Datei | Zeilen | Zweck |
|-------|--------|-------|
| `crates/terminal/src/engine.rs` | 1.224 | VTE-Emulation, OSC-Sniffer, Farbaufloesung, Render-Snapshot |
| `crates/terminal/src/session.rs` | 848 | PTY-Session (lokal + SSH), Reader-Thread |
| `crates/terminal/src/registry.rs` | 547 | Session-Registry, SessionHandle |
| `crates/terminal/src/input.rs` | 796 | Keyboard/Mouse -> Escape-Sequenzen |
| `crates/terminal/src/render.rs` | 189 | batch_runs(), grid_size() |
| `crates/terminal/src/palette.rs` | 315 | Theme Hsla -> Terminal Rgb, ANSI-256 |
| `crates/terminal/src/shell_integration.rs` | 181 | Shell-rc-Scripts (OSC 7/133) |
| `crates/workspace/src/views/terminal.rs` | 946 | GPUI-View: Rendering, Input, Cursor, Selection |
| **Gesamt** | **~5.050** | |

### Was bereits funktioniert

- alacritty_terminal 0.24 als Engine
- GPUI Cell-Rendering mit StyledRun-Batching
- Keyboard/Mouse Input (SGR Mouse, Bracketed Paste, DECCKM/DECPAM)
- Shell-Integration (OSC 7/133 via handgeschriebenem OscSniffer)
- Multi-Tab Sessions mit Registry
- Terminal Bell (audio)
- Settings: Shell, Font, Cursor, Opacity, Scrollback, Copy-on-Select, Right-Click
- Scrollback-Persistenz (gzip auf Disk)
- Theme-Integration live (ANSI-Palette, Transparenz, Hintergrundbilder)
- SSH-Remote-Terminal (integriert via russh)
- Drag and Drop: Dateipfade ins Terminal einfuegen
- Kontextmenue: Copy, Paste, Clear, Ask AI

---

## 2. Vergleich mit Zed — Luecken-Uebersicht

| Kategorie | Labonair | Zed | Differenz |
|-----------|----------|-----|-----------|
| **Rendering** | `div()`-Komposition | `TerminalElement` paint() | Performance-Luecke |
| **Viewport-Clipping** | Fehlt | Content-Mask + Intersection | Unnoetige Elemente |
| **Block-Elemente** | Nicht unterstuetzt | Quarter/Half/Full-Block | Unicode kaputt |
| **Cursor-Blink** | Statisch | Animation Timer | Kein Feedback |
| **Min. Kontrast** | Fehlt | `ensure_minimum_contrast()` | Schlecht lesbar |
| **Selektion** | Nur Simple | Simple + Semantic + Lines | Doppelklick fehlt |
| **Scrollback-Suche** | Nur Screen-find | Regex, Next/Prev, Highlight | Suche unmoeglich |
| **Hyperlinks** | Fehlt | Ctrl+Click, URL+Path | Komplett weg |
| **Scrollbar** | Fehlt (nur Mausrad) | Integrierte Scrollbar | Kein Feedback |
| **Split-Pane** | Fehlt | Rechts/Links/Oben/Unten | Kein Layout |
| **Vi-Mode** | Fehlt | Komplett | Power-User Feature |
| **Task-Terminal** | Fehlt | Status, Rerun, Auto-Hide | Kein Feedback |
| **Tab-Rename** | Fehlt | Doppelklick Inline | Kein Name |
| **Focus-In/Out** | Fehlt | ESC Sequenzen | TUI kann nicht tracken |
| **IME** | Fehlt | Marked Text | Kein CJK |
| **Foreground-Prozess** | Fehlt | Skriptname im Titel | Immer "bash" |

---

## A. Rendering & Performance

### A1. TerminalElement mit eigenem paint()

**Problem:** Jeder `StyledRun` erzeugt ein GPUI `div()`. Bei 200x50 = hunderte Divs pro Frame.

**Loesung:** `TerminalElement` mit `impl Element` + `paint()` der direkt GPUI-Shapes zeichnet.

**Vorteile:** GC-freundlich, Content-Mask-Clipping, Viewport-Intersection.

**Edge Cases:**
- `Element::request_layout()` muss korrekt funktionieren
- `paint()` bekommt `SceneBuilder` fuer Text ueber `push_text()`
- Clip-Region korrekt setzen
- Font-Metrics konsistent zwischen Layout und Paint

**Aufwand:** Hoch

### A2. Content-Mask-Clipping

**Problem:** Alle Runs werden positioniert, auch ausserhalb des Viewports.

**Loesung:** `ContentMask` setzen — clippt ausserhalb Viewport-Rechteck.

**Edge Cases:**
- Halb sichtbare Zeilen muessen trotzdem gerendert werden
- Cursor am Rand darf nicht geclippt werden
- Selektions-Highlights duerfen nicht geclippt werden

### A3. Block-Element-Rendering

**Problem:** Unicode Block-Elemente (Sextants, Quadrants) werden als Text gerendert statt als Sub-Cell-Rechtecke.

**Loesung:** 8x24 Subcell-Grid pro Zelle. Block-Elemente als farbige Rechtecke.

**Beispiele:**
- `▀` (Upper Half Block) -> oberes Halft in Vordergrundfarbe
- `░` (Light Shade) -> 25% Subcells in Vordergrundfarbe
- Sextants -> 3x2 Grid fuer Konsolen-Fonts

**Edge Cases:**
- Mix aus Block-Elementen und Text in derselben Zelle
- Block-Elemente ueber Zeilengrenzen
- Powerline-Symbole NICHT als Block-Elemente behandeln
- Font-basierte Darstellung vs. Rectangle-Fallback

**Aufwand:** Hoch

### A4. Background-Region-Merging

**Problem:** Identische Hintergrundfarben erzeugen Separate Rechtecke.

**Loesung:** Benachbarte gleichfarbige Zellen zu einem grossen Rechteck zusammenfassen.

**Edge Cases:**
- Selektions-Highlights muessen ueber dem Merged-Background liegen
- Transluzenz: Merging nur bei exakt gleicher Farbe + Alpha

### A5. Zero-Width-Character-Handling

**Problem:** Variation Selectors, Combining Marks, Emoji Modifier werden als eigene Zellen gerendert.

**Loesung:** An vorherige Zelle anhaengen (Text-Batching erweitern).

**Edge Cases:**
- Mehrere Combining Marks hintereinander
- Emoji-Sequenzen (Flaggen: Regional Indicators)
- ZWJ Sequenzen
- Combining Marks auf Wide Characters (CJK)

### A6. Cursor-Blink

**Problem:** Kein visuelles Feedback ob Cursor aktiv ist.

**Settings:**
- `blinking: Off` — nie blinken
- `blinking: On` — immer blinken
- `blinking: TerminalControlled` — Shell kann BLINK ein/ausschalten

**Edge Cases:**
- Blink reset bei Tastendruck
- Kein Blink bei fehlendem Fokus
- Kein Blink wenn `display_offset != 0`
- Timer nur starten wenn Terminal aktiv (nicht Background-Tabs)
- Smooth Animation (kein harter Toggle)

### A7. Minimum-Contrast-Adjustment

**Problem:** Farbiger Text auf farbigem Hintergrund kann unlesbar sein.

**Loesung:** Vordergrundfarbe anpassen bis Mindestratio (4.5:1 oder konfigurierbar).

**Ausnahmen:**
- TrueColor (24-bit) und 256-Farben (Index >= 16)
- Powerline-Symbole und Box-Drawing
- Dekorative Zeichen

**Edge Cases:**
- Inverse-Zellen: fg/bg getauscht — Kontrast zwischen fg und bg pruefen
- Hintergrundtransparenz: Kontrast gegen sichtbaren Hintergrund
- Performance: Kontrastberechnung pro Frame pro Zelle -> Caching noetig

### A8. Lazy Scroll Updates

**Problem:** `display_offset` wird bei jedem Scroll-Event aktualisiert.

**Loesung:** `future_display_offset` als Deferred-Wert, erst beim naechsten Render-Pass.

---

## B. Selektion & Copy/Paste

### B1. Doppelklick = Wort-Selektion (Semantic)

**Ziel:** Doppelklick waehlt das Wort unter dem Cursor.

**Edge Cases:**
- Wort-Trennung: Was ist ein "Wort"? Klammern/Operatoren als Trenner?
- CJK-Text: Jedes Zeichen ist ein "Wort"?
- Pfad-Selektion: `/Users/me/dev/file.rs` — ganzer Pfad?
- URL-Selektion: `https://example.com/path?q=1` — ganzer Link?

### B2. Dreifach-Klick = Zeilen-Selektion

**Ziel:** Dreifack-Klick waehlt die ganze Zeile.

**Edge Cases:**
- Wrap: Logische oder visuelle Zeile?
- Prompt-Zeile: Ganzen Prompt mitnehmen?

### B3. Shift+Click = Selektion erweitern

**Ziel:** Shift+Click setzt Head der Selektion.

**Edge Cases:**
- Keine aktive Selektion: Startet neue Selektion
- Mouse-Reporting aktiv: Shift+Click als Override (Escape Hatch)

### B4. Drag-Threshold (2px)

**Problem:** Click ohne Miusbewegung erzeugt Selektion (0 Zeichen).

**Loesung:** Erst nach 2px beginnt Selektion.

**Edge Cases:**
- Trackpad-Druck: Kein versehentliches Drag
- Langsames Ziehen: Threshold nicht zu gross

### B5. keep_selection_on_copy

**Ziel:** Nach `Cmd+C` bleibt Selektion sichtbar.

**Edge Cases:**
- Klick irgendwohin: Selektion verschwindet
- Neues Output: Selektion verschwindet bei Geaenderter Zelle

### B6. Image-Paste Erkennung

**Problem:** Bild in Clipboard + Cmd+V -> kein Binaer-Klartext einfuegen.

**Loesung:** Clipboard auf Bild-Typ pruefen. Bild -> Ctrl+V an TUI weiterleiten.

**Edge Cases:**
- Clipboard mit Text UND Bild: Prioritaet?
- Plattform-Unterschiede: macOS vs. Linux PRIMARY/CLIPBOARD

### B7. Path-Paste

**Ziel:** Externe Dateipfade shell-quotiert einfuegen.

**Edge Cases:**
- Pfade mit Leerzeichen, Sonderzeichen, Unicode
- Relative vs. absolute Pfade
- Nicht-existierende Pfade (trotzdem einfuegen)

### B8. Linux Primary Clipboard

**Ziel:** Selektierten Text automatisch in PRIMARY schreiben.

**Edge Cases:**
- Nur Linux/FreeBSD (macOS hat kein PRIMARY)
- Konflikt mit Copy-on-Select
- Langer Selektions-Text

---

## C. Suche

### C1. Scrollback-Suche mit Regex

**Problem:** `search()` sucht nur auf aktuellem Screen. Kein Regex, kein Next/Prev.

**Ziel:** Volle Suchleiste wie im Editor.

**Features:**
- Regex-Suche (case-insensitive als Default)
- Suche ueber gesamten Scrollback
- Next/Prev Navigation
- Alle Matches farbig markieren
- Match-Zaehler: "3 of 47"
- Aktiver Match hervorgehoben
- Selektionsbasierte Query-Vorlage

**Architektur:**
```
TerminalView
  +-- SearchBar (GPUI Entity, inline am unteren Rand)
        |-- Input-Field (Query)
        |-- Match-Counter ("3 of 47")
        |-- Prev/Next Buttons
        +-- Regex-Toggle
```

**Edge Cases:**
- Suchtext mit Sonderzeichen: Regex-Escaping wenn Regex-Modus aus
- Leerer Suchtext: Selektion loeschen
- Tab-Wechsel waehrend Suche: Beibehalten oder loeschen?
- Output waehrend Suche: Matches obsolete — neu suchen?
- Grosse Scrollbacks (100k+): Performance — Index oder linear scannen?
- Unicode-Suche: Combining Marks, Wide Characters
- PCRE: Lookahead/Lookbehind — zu komplex?

### C2. Selektionsbasierte Query-Vorlage

**Ziel:** Selektierten Text als Default-Search-Term.

**Edge Cases:**
- Selektion ueber mehrere Zeilen: Newlines entfernen?
- Selektion laenger als ein Wort: Voller Text?

---

## D. Vi Mode

### D1. Komplette Vi-Navigation

**Motions:**
- `h/j/k/l` — Pfeiltasten
- `w` — Wort rechts, `b` — Wort links, `e` — Wort-Ende
- `%` — Klammern-Match
- `$` — Zeilenende, `0` — Zeilenanfang, `^` — erstes belegtes Zeichen
- `H` — High, `M` — Middle, `L` — Low
- `{` — Absatz hoch, `}` — Absatz runter

**Scroll:**
- `g` — Top, `G` — Bottom
- `Ctrl+B` — Page Up, `Ctrl+F` — Page Down
- `Ctrl+D` — Half Page Down, `Ctrl+U` — Half Page Up

**Visual Selection:**
- `v` — Character Selection
- `V` — Line Selection
- `Escape` — Selection beenden
- `y` — Yank (Copy)

**Ausstieg:** `i` — Scroll to Bottom + Vi-Mode aus

**Edge Cases:**
- Vi-Mode vs. Vim-eigenes Vi-Mode (Mouse-Reporting Apps)
- `i` deaktiviert Vi-Mode komplett
- Selection: Labonair-Selection oder Vim-Selection?
- Performance: Vi-Cursor-Overlay schnell aktualisieren

---

## E. Hyperlinks & Pfad-Erkennung

### E1. URL-Erkennung (Ctrl+Click)

**Regex:**
- `https?://[^\s]+` — HTTP/HTTPS
- `ftp://[^\s]+` — FTP
- `ssh://[^\s]+` — SSH

**Verhalten:**
- Ctrl/Cmd + Hover -> unterstrichen + Tooltip
- Ctrl/Cmd + Click -> Browser oeffnen
- Mouse-Reporting aktiv: Link-Oeffnen erlaubbar (Setting)

**Edge Cases:**
- URL am Zeilenende mit Punkt/Komma: Punkt nicht mitnehmen
- URL in Anfuehrungszeichen: Escape-Sequenzen vorher entfernen
- URL mit Sonderzeichen: `~`, `%20`, `#`, `?` — alles gueltig
- OSC-8 Hyperlinks: Sollten unterstuetzt werden?

### E2. Pfad-Erkennung

**Regex (benutzer-konfigurierbar):**
- `/Users/[^/]+/[^ ]+` — Absolute Pfade
- `~/[^ ]+` — Home-relative
- `\./[^ ]+` — Relative Pfade

**Verhalten:**
- Ctrl/Cmd + Click -> Datei im Editor oeffnen
- Relative Pfade gegen CWD der Zeile aufloesen

**Edge Cases:**
- CWD-historische Aufloesung: Welcher CWD fuer welche Zeile?
- Pfad existiert nicht: Tooltip oder gar nicht anzeigen?
- Pfad mit Leerzeichen in Anfuehrungszeichen
- Verzeichnis: Im Explorer oeffnen?

### E3. CWD-historische Aufloesung

**Loesung:** CWD-Historie mit Scrollback-Position. Bei `\r` Input: aktueller CWD + Zeilennummer speichern.

**Edge Cases:**
- Reset bei Terminal-Clear oder Resize
- Scrollback-Cap evicted alte Eintraege: Fallback auf aktuellen CWD
- SSH: Kein lokales CWD-Tracking

---

## F. Terminal-Panel & Tab-Management

### F1. Split-Pane Layout

**Ziel:** Terminal in 2/4 Fenster aufteilen.

**Varianten:**
- Rechts/Links/Oben/Unten aufteilen
- Pane-Gruppe als Baumstruktur
- Tab-Drag zwischen Panes

**Edge Cases:**
- Resize-Ausgleich: Wenn ein Pane groesser wird, werden andere kleiner
- Mindestgroesse pro Pane (nicht kleiner als 10x5 Zellen)
- SSH-Terminal in Split: Selber Remote-Host oder unterschiedlich?
- Split ueber Workspacegrenzen hinweg

### F2. Tab-Rename

**Ziel:** Doppelklick auf Tab -> Inline-Editor fuer Titel.

**Edge Cases:**
- Tab-Titel aus Shell-Integration ueberschreiben oder ergaenzen?
- Manuell gesetzter Titel soll Shell-Updates nicht ueberschreiben
- Maximal-Laenge des Titels (UI-Begrenzung)

### F3. Tab-Drag and Drop

**Ziel:** Tabs zwischen Panes verschieben.

**Edge Cases:**
- Tab aus Pane ziehen: Pane bleibt leer oder wird geschlossen?
- Tab zwischen Split-Panes verschieben
- Tab ausserhalb jedes Pane gezogen: Neuer Split?

### F4. Pane-Cloning

**Ziel:** Duplizieren mit gleichem CWD.

**Edge Cases:**
- SSH-Terminal klonen: Gleicher Remote-Host?
- Scrollback mitklonen oder nur CWD?
- Task-Terminal klonen: Status uebernehmen?

### F5. Zoom Toggle

**Ziel:** Panel auf volle Groesse maximieren.

**Edge Cases:**
- Zoom-zurueck: Vorherige Pane-Position wiederherstellen
- Zoom-Toggle mit Hotkey (Cmd+Shift+F oder similar)
- Zoom schliesst sich wenn Terminal-Tab geschlossen wird

---

## G. Task-Integration

### G1. Task-Terminal mit Status

**Features:**
- Status-Icon im Tab: Play (Running), Check (Success), XCircle (Failed), Warning (Unknown)
- Exit-Code-Anzeige bei Completion
- Hide-Strategy: Never, Always, OnSuccess
- Rerun-Button im Tab bei Hover

**Edge Cases:**
- Task laeuft und User schliesst Tab: Prozess killen oder weiterlaufen lassen?
- Task mit langem Output: Scrollback behalten?
- Mehrere Tasks gleichzeitig: Tab-Benennung ("test", "build", etc.)

### G2. Task-Rerun

**Ziel:** Button zum Neustarten des Tasks.

**Edge Cases:**
- Rerun mit gleichen oder neuen Argumenten?
- Alten Output loeschen oder behalten?
- Rerun-Terminal: Soll es ein neues Tab sein oder das gleiche?

---

## H. Settings & Config

### H1. working_directory Policy

**Optionen:**
- `CurrentFileDirectory` — CWD des aktuellen File-Explorers
- `CurrentProjectDirectory` — Projekt-Root
- `FirstProjectDirectory` — Erster bekannter Projekt-Pfad
- `AlwaysHome` — Immer `~`
- `Always` — Benutzer-definiert

### H2. detect_venv

**Ziel:** Python Virtualenv automatisch erkennen und aktivieren.

**Edge Cases:**
- Mehrere `.venv`-Verzeichnisse: Naehestes waehlen?
- conda/anaconda vs. venv: Unterschiedlich behandeln?
- Performance: Nur bei Shell-Start pruefen, nicht waehrend Laufzeit

### H3. scroll_multiplier

**Ziel:** Scroll-Geschwindigkeit konfigurierbar.

**Edge Cases:**
- Extrem hohe Werte: Scroll-Debounce einfuehren
- Trackpad vs. Maus: Unterschiedliche Multiplier?

### H4. open_links_in_mouse_mode

**Ziel:** Ctrl+Click fuer Links auch bei aktivem Mouse-Mode.

### H5. minimum_contrast

**Ziel:** Mindestratio einstellbar (Standard: 4.5:1).

### H6. path_hyperlink_regexes

**Ziel:** Eigene Regex-Patterns fuer Pfad-Erkennung.

**Edge Cases:**
- Ungueltige Regex: Fehlermeldung oder ignorieren?
- Leere Liste: Standard-Patterns verwenden

### H7. option_as_meta

**Ziel:** macOS Option-Key Verhalten konfigurierbar (Meta vs. spezielle Zeichen).

---

## I. Render-Details & Micro-Features

### I1. Powerline-Symbole von Kontrast-Anpassung ausnehmen

**Warum:** Powerline-Symbole muessen exakt die Farbe haben die das Programm setzt.

### I2. Wavy Underline

**Ziel:** Gedeckter als eigenes Style (wie in modernen Editoren).

### I3. IME-Composition

**Ziel:** Marked Text unter Cursor rendern fuer CJK/Eingabe.

**Edge Cases:**
- UTF-16 Range-Tracking
- Composition ueber mehrere Cells
- Composition abbrechen (Escape)

### I4. Focus-In/Out Escape Sequences

**Ziel:** `\x1b[I`/`\x1b[O` senden wenn Fokus auf Terminal wechselt.

**Edge Cases:**
- Nur senden wenn Terminal FOCUS_IN_OUT Modus gemeldet hat
- Tab-Wechsel: Focus-Out auf altem, Focus-In auf neuem Terminal

### I5. Cursor-Shape bei Unfocus

**Ziel:** Automatisch zu HollowBlock wechseln wenn Terminal Fokus verliert.

### I6. Bell-Visual im Tab

**Ziel:** Emoji/Badge wenn Terminal-Bell ausgeloest.

**Edge Cases:**
- Mehrere Bells: Zaehler oder nur Badge?
- Bell loescht sich wenn Tab fokussiert wird

### I7. Breadcrumb-Trail

**Ziel:** Aktueller Titel/CWD in Toolbar anzeigen.

---

## J. Umgebungs-Integration

### J1. SHLVL-Entfernung

**Problem:** SHLVL exponentiell bei巢-spawned Terminals.

**Loesung:** SHLVL aus Environment entfernen oder auf 1 setzen.

### J2. LANG-Fallback

**Loesung:** `en_US.UTF-8` setzen wenn kein Locale in Parent-Environment.

### J3. Foreground-Prozess-Erkennung

**Problem:** Tab-Titel zeigt immer "bash"/"zsh" statt des laufenden Skripts.

**Loesung:** Foreground-Prozess aus PTY-Foreground-Group ermitteln, Name normalisieren.

**Edge Cases:**
- `node script.js` -> "script.js" (nicht "node")
- `python -m pytest` -> "pytest" (nicht "python")
- Shell-Builtins: "cd", "export" — nicht als Prozess-Titel verwenden
- Prozess beendet: Zurueck zum Shell-Namen

### J4. Shell-spezifische Init-Marker

**Ziel:** Erkennen wann Shell fertig initialisiert (nach rc-Dateien).

**Edge Cases:**
- Verschiedene Shell-Typen: bash, zsh, fish, nushell — unterschiedliche Marker
- Fallback wenn Marker nicht empfangen werden

### J5. Activation Scripts

**Ziel:** Befehle vor Shell-Start senden (z.B. `nvm use`, `conda activate`).

**Edge Cases:**
- Shell muss Activation Scripts verstehen
- Fehler in Activation Scripts: Anzeigen oder ignorieren?

---

## K. Persistence & Serialisierung

### K1. SQLite-Session-Serialisierung

**Ziel:** Tab-Hierarchie, CWD, Custom Title persistieren.

**Edge Cases:**
- Host-Wechsel: Tabs die auf Host X zeigen — beim naechsten Start?
- Leere Workspace: Kein Tab speichern
- Invalide Sessions (Shell exited): Beim Restore loeschen

### K2. Tab-Title Persistenz

**Ziel:** Custom Name ueberlebt Restart.

**Edge Cases:**
- Shell-Integration setzt neuen Titel: Custom-Name ueberschreiben oder behalten?

### K3. Orphan-Cleanup

**Ziel:** Nicht mehr existierende Sessions aus DB loeschen.

---

## L. ANSI-Verarbeitung

### L1. parse_ansi_text()

**Ziel:** Text mit Farb-Spans extrahieren (fuer AI-Kontext).

### L2. strip_ansi_text()

**Ziel:** Escape-Sequenzen entfernen mit CR-Handling.

### L3. OSC 10/11 Color Request/Response

**Ziel:** Farb-Anfragen vom Programm inline beantworten.

**Edge Cases:**
- Farb-Anfrage waehrend Theme-Wechsel: Welche Farbe zurueckgeben?

---

## M. Allgemeine Verbesserungen

### M1. Vollstaendige Scrollback-Suche (Prioritaet: Hoch)

Der groesste offene Posten. Regex, Next/Prev, Multi-Highlight, ueber gesamten Scrollback.

### M2. Hyperlinks im Terminal (Prioritaet: Hoch)

Ctrl+Click oeffnet URL/Pfad. Fehlt komplett.

### M3. Integrierte Scrollbar (Prioritaet: Hoch)

`TerminalScrollbar` mit `ScrollableHandle`. Aktuell nur Mausrad.

### M4. Block-Element Rendering (Prioritaet: Mittel)

Unicode Block-Charts korrekt darstellen (Sextants, Quadrants, Shade).

### M5. Cursor-Blink mit Terminal-Controlled Modus (Prioritaet: Hoch)

Shell kann BLINK ein/ausschalten. Standard-Terminal-Feature.

### M6. Double-Click Wort + Triple-Click Zeilen-Selektion (Prioritaet: Hoch)

Erwartetes Standard-Verhalten.

### M7. Split-Pane Terminal (Prioritaet: Hoch)

2/4 Fenster aufteilen. Production-Feature.

### M8. OSC 52 Clipboard (Prioritaet: Mittel)

Terminal-Apps koennen Clipboard lesen/schreiben. Wichtig fuer tmux/screen.

### M9. Display-Only Terminal (Prioritaet: Mittel)

Ohne PTY, fuer Inline-Output (Agent).

### M10. Foreground-Prozess-Erkennung (Prioritaet: Mittel)

Tab-Titel intelligent setzen.

### M11. Bell mit Optionen (Prioritaet: Niedrig)

System/Visual/Aus.

### M12. Tab-Icon fuer Status (Prioritaet: Niedrig)

Play/Check/XCircle je nach Task-State.

---

## Edge Cases & Stolpersteine

### Rendering

1. **Wide Characters (CJK):** Zwei Zellen pro Zeichen. Wenn `cell_w` fuer ASCII berechnet wird, overflowen CJK-Zeichen. Loesung: `ch_advance()` fuer jedes Zeichen einzeln aufrufen oder Font-Metrics fuer Wide Characters kennen.

2. **Combining Marks:** Zeichen wie `e` + `´` (U+0301) muessen auf EINER Zelle gerendert werden. Aktuell werden das zwei Zellen — das zweite Zeichen uebermalen das erste.

3. **Bidirektionaler Text:** Arabisch/Hebraeisch mixes mit LTR Text. alacritty_terminal hat kein Bidi — Text kann falsch dargestellt werden.

4. **OSC 8 Hyperlinks:** Programme koennen Text als Link markieren. Wird aktuell ignoriert.

5. **Term-Mode Flags:** alacritty_terminal 0.24 hat einige Modes die in neueren Versionen geaendert wurden. API-Stabilitaet beachten.

### Input

6. **Kitty Keyboard Protocol:** Wird von alacritty 0.24 gemeldet aber nicht vollstaendig unterstuetzt. Apps die Kitty-Keyboards nutzen bekommen falsche Sequenzen.

7. **Shift+Click in Mouse-Reporting Apps:** Vim/Emacs nutzen Mouse-Reporting. Shift+Click muss als "Escape Hatch" dienen koennen.

8. **Right-Click in TUIs:** TUIs wie htop haben eigene Right-Click-Handler. Right-Click muss an TUI weitergegeben werden wenn Mouse-Reporting aktiv.

### Search

9. **Suche in alternatem Screen:** vim/less nutzen alternatem Screen — Scrollback-Suche ergibt dort keinen Sinn.

10. **Suche waehrend Output:** Matches werden sofort obsolete wenn neue Zeilen kommen. Soll die Suche "live" bleiben oder pausieren?

### Persistence

11. **Scrollback-Loeschung bei Resize:** Wenn Spaltenanzahl sich aendert, werden Zeilen im Grid neu brot — alter Scrollback-Text kann "kaputt" aussehen.

12. **Alt-Screen Persistenz:** TUIs (vim, less) nutzen alternatem Screen — dessen Inhalt soll NICHT persistiert werden.

---

## Offene Fragen

1. **TerminalElement vs. div()-Komposition:** Soll das alte Rendering komplett ersetzt werden oder als Fallback bleiben? GPUI Element-API ist schlecht dokumentiert.

2. **Split-Pane Architektur:** Eigene `TerminalPaneGroup` Struktur oder GPUI-internes Splitting nutzen?

3. **Vi-Mode Integration:** Soll es als Toggle-Setting sein oder nur ueber Command Palette aktivierbar?

4. **Hyperlinks:** Sollen auch OSC-8 Hyperlinks unterstuetzt werden? Das waere ein weiterer OscSniffer-Einbau.

5. **Scrollback-Search Performance:** Lineares Scannen oder Index-Aufbau? Bei 100k+ Zeilen kann Linearscan 100ms+ dauern.

6. **Task-Terminal Lifecycle:** Soll ein Task-Terminal beim Beenden automatisch geschlossen werden oder sichtbar bleiben (mit Exit-Code)?

7. **Clipboard-Integration:** Soll OSC 52 (Programm-Clipboard-Zugriff) aus Sicherheitsgruenden deaktivierbar sein?

8. **IME-Unterstuetzung:** GPUI hat begrenzte IME-Unterstuetzung. Soll das als Prio behandelt werden oder warten bis GPUI das besser kann?

9. **Cursor-Blink Performance:** Animation Timer pro Terminal — oder ein globaler Timer fuer alle sichtbaren Terminals?

10. **Minimum-Contrast:** Soll der User das Verhalten konfigurieren koennen (an/aus/threshold) oder immer aktiv sein?

---

## Nicht portieren (Zed-spezifisch)

| Feature | Grund |
|---------|-------|
| Display-Only Terminal (kein PTY) | Nur fuer Zed Agent noetig |
| Embedded-Mode mit MAX_EMBEDDED_LINES | Agent-spezifisch |
| Block-Below-Cursor custom element | Agent-Panel-spezifisch |
| Pane-Serialisierung via SQLite KVP | Zed-haengiges Persistence-System |
| `terminal_path_like_target.rs` | Zed-spezifisches Tab/Pane-Dragging |
| Breadcrumb-Trail im Toolbar | Zed-spezifisches UI-Konzept |

---

## Umsetzungs-Reihenfolge

### Phase 1 — Quick Wins (1-2 Tage)
- Cursor-Blink (A6)
- Double/Triple-Click Selektion (B1, B2)
- SHLVL-Entfernung (J1)
- Drag-Threshold (B4)
- Focus-In/Out Sequences (I4)
- Cursor-Shape bei Unfocus (I5)

### Phase 2 — Core UX (3-5 Tage)
- Scrollback-Suche mit Regex + Next/Prev (C1)
- Hyperlinks + Ctrl+Click (E1, E2)
- Integrierte Scrollbar (M3)
- Path-Paste (B7)

### Phase 3 — Rendering-Upgrade (5-7 Tage)
- TerminalElement mit eigenem paint() (A1)
- Content-Mask-Clipping (A2)
- Block-Element-Rendering (A3)
- Background-Region-Merging (A4)
- Minimum-Contrast (A7)

### Phase 4 — Power-Features (7-10 Tage)
- Split-Pane Layout (F1)
- Vi-Mode (D1)
- Task-Integration (G1, G2)
- Tab-Rename + Drag (F2, F3)

### Phase 5 — Polish (3-5 Tage)
- Settings-Erweiterungen (H1-H7)
- Foreground-Prozess-Erkennung (J3)
- Bell-Visual im Tab (I6)
- ANSI-Utility-Funktionen (L1-L3)

---

**Gesamt: 80 Items**, davon ~30 Quick Wins (Niedrig/Mittel), ~25 Core Features (Mittel), ~25 Advanced (Hoch).
