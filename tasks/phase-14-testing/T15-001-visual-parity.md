# T15-001: Visuelle Paritäts-Verifikation (Design-Feinschliff)

## Status
⏳ Pending

## Phase
14 — Testing & Polish

## Abhängigkeiten
Alle vorigen Phasen (insb. Theme T02, Terminal T03, Editor T06)

## Ziel
Systematisch die optische Übereinstimmung der Rust-App mit dem Original Labonair verifizieren und Feinschliff betreiben, sodass jede Ansicht (Terminal, Explorer, Editor, Settings, AI-Chat, Git, etc.) dem Referenz-Design 1:1 entspricht — Farben, Abstände, Radien, Schatten, Icons, Typografie und Verhalten.

## Kontext
Das Ziel ist ein Design, das identisch zur Tauri/React-Version wirkt, aber mit Rust-Performance. Die Theme-Tokens aus Phase 1 sind die Grundlage, aber die tatsächliche SVG-/Zeichenqualität (Padding, Icon-Größen, Hover-States, Fokus-Ring, Scrollbar-Optik) muss visuell abgeglichen werden.

Da kein WebView/CSS dem Isolierung existiert, müssen diese Details manuell in GPUI nachgebaut werden. Der Vergleich geschieht gegen die Referenz-App (die originale Labonair kann parallel laufen) bzw. gegen Screenshots/Dokumentation.

## Anweisungen zur Umsetzung

1. **Referenz-Screenshots sammeln.** Erstelle (oder nutze vorhandene) Screenshots der original Labonair-App bei verschiedenen Ansichten/Themen (Light/Dark), um einen visuellen Abgleich durchzuführen. Dazu gehört: Hauptfenster mit Terminal, Explorer-Sidebar, Editor mit Syntax-Highlighting, Settings-Kategorien, AI-Chat, Git-Panel.

2. **Side-by-Side-Vergleich.** Öffne beide Versionen (Referenz und Rust-App) und vergleiche systematisch pro Bereich:
   - Farben (Hintergrund, Text, Akzent, Border, Hover, Selection, Scrollbar).
   - Spacing/Padding (Dichten, Abstände in Eingaben, Buttons, Panels).
   - Radien (Karten, Button, Popover).
   - Schatten (Popover/Modal-Höhen).
   - Typografie (Font-Wahl, Größen, Zeilenhöhen, in UI und Terminal).
   - Icons (Größen, Abstand zu Text, Farben je Zustand).
   - Interaktions-/Fokus-Zustände (Hover-Highlights, Cursor).

3. **Feinschliff-Katalog.** Führe eine Liste (Checkliste) der gefundenen Abweichungen und behebe sie im Theme-/Komponenten-Code. Gehe Kategorie für Kategorie durch und korrigiere die GPUI-Stile, bis die Ansichten übereinstimmen.

4. **Interaktions-Parität.** Verifiziere auch die (nicht-statischen) Verhaltensaspekte des Designs:
   - Hover-/Fokus-/Aktiv-States der Buttons, Tabs, Listen-Einträge.
   - Drag-And-Drop-Feedback, Resize-Handles.
   - Scrollbar-Erscheinung und -Behaviour.
   - Übergänge/Animationsdauer (aus Theme-Tokens).

5. **Terminal-spezifische Parität.** Besonders sorgfältig prüfen:
   - Zell-Größen/Proportionen und Cursor-Darstellung.
   - ANSI-Farben und Font-Treue (aus T02-004).
   - Scrollback/Scrollbar-Optik.

6. **Themen-Wechsel prüfen.** Verifiziere die optische Konsistenz bei Light- UND Dark-Theme (und bei Benutzer-Theme-Import).

7. **Automatische Helfer.** Sofern sinnvoll, Grundprüfungen (z.B. logische Farb-Kontrastprüfungen, minimale Größen) in Tests integrieren, um Regressionen zu vermeiden. Der finale Feinschliff bleibt aber manuell-visuell.

## Akzeptanzkriterien

- [ ] Jede Hauptansicht (Terminal, Explorer, Editor, Settings, AI-Chat, Git) wurde gegen die Referenz verglichen und entspricht dieser im Design.
- [ ] Ein Feinschliff-Katalog der gefundenen Abweichungen wurde gepflegt und abgearbeitet.
- [ ] Farben, Spacing, Radien, Schatten, Icons, Typografie und Interaktions-States stimmen überein.
- [ ] Terminal-Zell-/Cursor-/ANSI-Darstellung entspricht der Referenz.
- [ ] The Light- und Dark-Theme (und Benutzer-Themes) sind optisch konsistent.
- [ ] Keine offensichtlichen visuellen Regressionen bei Theme-Wechsel.

## Notizen

- Diese Phase ist iterativ und führt zusammen mit T15-003 (Feinabstimmung) zu einem "Polished"-Zustand.
- Nutze die Theme-Tokens aus Phase 1 als einzige Autorität für Farb-/Radius-/Schattenwerte — vermeide Hardcodes, die die Parität brechen.

## Warnungen

- ⚠️ Nicht durch willkürliche "schönere" Werte von den Token abweichen — das Ziel ist 1:1-Parität, keine Neugestaltung.
- ⚠️ Der manuelle visuelle Abgleich kann subjektiv sein — bei Unklarheit die Original-App als maßgeblich nehmen.

## Weiterführende Tasks

- [T15-003: Cross-Platform- und Performance-Optimierung](./T15-003-cross-platform-performance.md)
