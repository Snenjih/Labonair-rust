# T02-001: Design-Tokens aus globals.css extrahieren

## Status
⏳ Pending

## Phase
1 — Theme-System & Design-Tokens

## Abhängigkeiten
T01-001 (Cargo Workspace)

## Ziel
Die kompletten Design-Tokens aus `../Labonair/src/styles/globals.css` lesen und in eine datenorientierte Rust-Struktur übersetzen. Das Ergebnis ist eine vollständige, programmatisch nutzbare Repräsentation aller Farb-, Radius-, Schatten- und Animationswerte der App. Die Struktur liegt dort, wo sie in `crates/theme/` hingehört, und wird von der gesamten UI als einzige Design-Quelle genutzt.

## Kontext
Labonair definiert sein gesamtes visuelles Erscheinungsbild über CSS Custom Properties in `src/styles/globals.css`. Diese Datei ist die Autorität für alle Farben, Radien, Schatten, Animationen und Typografien — rund 80 bis 100 Tokens, aufgeteilt in zwei Theme-Varianten: Light (`:root`) und Dark (`.dark`).

Für die native Rust-App gibt es kein `globals.css` mehr. All diese Werte müssen daher in eine Rust-Datenstruktur übertragen werden, die GPUI-verdauliche Werte (Hue/Saturation/Lightness-Farben, Pixel-Angaben, Zeitangaben) enthält. Dieser Task ist die Grundlage für das gesamte Theme-System und damit auch für jedes spätere UI-Element.

## Zu extrahierende Token-Kategorien

Der Aufgaben-Bearbeiter soll die `globals.css`-Datei vollständig lesen und anschließend die dort definierten Werte kategorisiert übernehmen. Erwartete Kategorien:

1. **Core-Farben** (Hintergrund, Vordergrund, Card, Popover, Primary, Secondary, Muted, Accent, Destructive, Border, Input, Ring inkl. jeweiliger Foreground-Varianten)
2. **Sidebar-Farben** (Sidebar-Hintergrund, -Vordergrund, -Primary, -Accent und deren Foreground-Varianten, -Border)
3. **Surface-Tokens** (Toolbar, Title-Bar, Status-Bar)
4. **Border-Varianten** (Variant, Focused, Selected, Transparent, Disabled)
5. **Semantische Status-Farben** (Modified, Error, Warning, Info, Hint, Success)
6. **Interaktionsfarben** (Cursor, Selection)
7. **Terminal-ANSI-Palette** (16 Standard, 16 Bright, 16 Dim-Farben sowie Terminal-Hintergrund/-Vordergrund)
8. **Radius-Skala** (sm bis 4xl sowie der fixe Window-Radius)
9. **Schatten-Spezifikationen** (Row, Popover, Modal — jeweils mit Blur, Spread, Offset und Farbe)
10. **Animationswerte** (Dauer: fast/base/slow; Easing-Kurven)
11. **Typografie-Tokens** (Sans-Font, Heading-Font, auf Laufzeit änderbare App-Font-Variablen)

Da Labonair ausschließlich im Oklch-Farbraum arbeitet, sind die meisten Farbwerte als Oklch-Ausdrücke notiert. Diese müssen in die von GPUI verwendete Farbdarstellung umgerechnet werden können.

## Anweisungen zur Umsetzung

1. **globals.css vollständig lesen.** Beginne bei der Datei und lies sie vollständig durch. Extrahiere jede Custom-Property sowohl aus dem `:root`- (Light) als auch aus dem `.dark`-Block. Notiere dir bei jedem Token den genauen wertverständlichen Oklch-Wert — nichts raten oder schätzen.

2. **Farbkonvertierungsfunktion einrichten.** Da GPUI-Farben nicht Oklch sind, benötigst du eine zuverlässige Umrechnung Oklch → sRGB → GPUI-Farbformat. Nutze dafür eine etablierte Farb-Library (der Task-Bearbeiter soll eine passende, gut gewartete Rust-Crate auswählen), statt die Mathematik selbst zu implementieren. Die Konvertierung muss für dunkle Theme-Farben präzise sein; eine Abweichung von bis zu 1/255 pro Farbkanal ist akzeptabel.

3. **Rust-Struktur anlegen.** Erstelle die Datenstruktur(en) in `crates/theme/`. Die Hauptstruktur soll alle oben genannten Token-Kategorien als benannte Felder enthalten (also nicht als generische Key-Value-Map, sondern als fest typisierte, sprechend benannte Felder). Für die Terminal-Farben soll eine eigene verschachtelte Struktur entstehen, die die 16er-Gruppen (Standard, Bright, Dim) sauber trennt.

4. **Light- und Dark-Instanzen konstruieren.** Implementiere Fabrikmethoden, die die aus `globals.css` abgelesenen Werte für die beiden Standard-Theme-Varianten zurückgeben. Jedes Feld muss dabei den exakten Wert aus der CSS-Datei wiedergeben.

5. **Parse- und Validierungshilfen.** Einfache Hilfsfunktionen sollen es ermöglichen, einzelne Farbwerte aus unterschiedlichen Textformaten (Oklch, Hex, RGB) zu parsen. Diese werden später für den Benutzer-Theme-Import gebraucht.

6. **Tests schreiben.** Erstelle Tests, die die Konvertierungsgenauigkeit validieren (z.B. bekannte Referenzfarbwerte aus der CSS-Datei durchlaufen die Konvertierung und müssen innerhalb der Toleranz landen). Teste außerdem, dass beide Theme-Instanzen alle Felder gefüllt haben (keine Default-Werte versehentlich).

## Akzeptanzkriterien

- [ ] Alle Token-Kategorien aus der Aufgabenbeschreibung sind als benannte, typisierte Felder in den Rust-Strukturen vorhanden.
- [ ] Die Light- und Dark-Fabrikmethoden liefern exakt die Werte aus `globals.css` (verifiziert anhand gezielter Stichproben).
- [ ] Die Oklch-Konvertierung funktioniert zuverlässig und ist getestet; Abweichungen pro Farbkanal liegen unter 1/255.
- [ ] Die Terminal-ANSI-Palette ist vollständig (16 + 16 + 16 Farben plus Terminal-spezifische Hintergrund-/Vordergrund-/Cursor-/Selektionsfarben).
- [ ] Radius-, Schatten- und Animationswerte sind als strukturierte Daten abgebildet (nicht als Strings).
- [ ] Das Crate kompiliert und alle Tests laufen grün.

## Notizen

- Alle Werte sind bereits definiert — es gibt nichts zu erfinden. Dieser Task ist reine Übersetzungsarbeit.
- Der Farbraum ist durchgängig Oklch; ein eigenständiger RGB/Hex-Farbraum taucht nur als Konvertierungsziel auf.
- Schatten-Spezifikationen enthalten mehrere Parameter (Blur, Spread, Offset, Farbe) — lege sie als zusammengesetzte Struktur ab, nicht als Einzelwerte.
- Schau dir ggf. an, wie Labonair dieselben Schatten im `shadow-*`-Bereich realisiert, um die Parameter korrekt zu interpretieren.

## Warnungen

- ⚠️ Farbwerte nicht erfinden oder "schöner" machen — exakt die Oklch-Werte aus der CSS-Datei übernehmen.
- ⚠️ Die Oklch-Konvertierung ist nicht linear: Bei dunklen Farben des `.dark`-Themes können kleine Rundungsfehler entstehen. Nur innerhalb der Toleranz akzeptieren.
- ⚠️ Manche Regeln in `globals.css` nutzen `color-mix(...)` — diese Mischrechnungen müssen manuell anhand der zugrunde liegenden Werte aufgelöst werden, da GPUI kein CSS-Äquivalent hat.

## Weiterführende Tasks

- [T02-002: Theme-Provider und Theme-Store](./T02-002-theme-provider-store.md)
- [T02-003: Theme-Import/Export für Benutzer-Themes](./T02-003-theme-import-export.md)
- [T02-004: Terminal-ANSI-Palette in das Theme integrieren](./T02-004-terminal-palette.md)
