# T02-002: Theme-Provider und Theme-Store

## Status
⏳ Pending

## Phase
1 — Theme-System & Design-Tokens

## Abhängigkeiten
T02-001 (Design-Tokens extrahieren)
T01-001 (Cargo Workspace)

## Ziel
Ein zentraler Theme-Store und Theme-Provider in GPUI implementieren, der das aktuell aktive Theme verwaltet, auf System-Dark/Light-Wechsel reagiert, das Setzen der Theme-Präferenz steuert und der gesamten UI-Schicht über einen wiederverwendbaren Zugriffsmechanismus (entsprechend GPUI-Entity-Konventionen) bereitgestellt wird. Damit ist gewährleistet, dass alle UI-Komponenten stets die gleiche, konsistente Design-Quelle nutzen.

## Kontext
In der Tauri/React-Version wird das Theme über `next-themes` und CSS-Klassen (`dark`/`light` auf dem Root-Element) verwaltet. Das löst sich auf, wenn klassische UI-Bausteine jeweils Farbwerte aus CSS-Variablen lesen.

In GPUI gibt es keine CSS-Klassen und keine Stylesheets. Stattdessen wird der App-Zustand über Entities verwaltet und Komponenten lesen Werte direkt aus einem Entity, oder sie werden über Kontext weitergereicht. Der Theme-Store ist daher das zentrale Zustandsobjekt, das die Theme-Präferenz (System/Light/Dark), das aktuell aufgelöste Design-Mode, das aktive Theme und ggf. ein benutzerdefiniertes Theme hält.

Zusätzlich muss der Store auf macOS-System-Dark/Light-Änderungen reagieren (wie vorher `prefers-color-scheme`), damit die App automatisch umschaltet, wenn die Präferenz auf "System" steht.

## Anweisungen zur Umsetzung

1. **Store-Struktur anlegen.** Erstelle in `crates/ui/` (oder einem passenden Theme-Modul) einen Theme-Store als GPUI-Entity. Der Store hält folgende Zustände:
   - Die Theme-Präferenz (System, Light oder Dark), wie sie der Benutzer in den Einstellungen gewählt hat.
   - Das aufgelöste Modus-Ergebnis (Light oder Dark) nach Anwendung der System-Erkennung.
   - Das daraus resultierende aktive Theme (die vollständige Theme-Struktur aus T02-001).
   - Optional ein benutzerdefiniertes, importiertes Theme (aus T02-003).

2. **System-Dark/Light-Erkennung und -Reaktion.** Implementiere die Erkennung des aktuellen System-Erscheinungsbilds über die von GPUI bereitgestellten Mechanismen zur Beobachtung von Erscheinungsbild-Änderungen (nicht über plattformspezifische APIs). Wenn die Präferenz auf "System" steht, muss der Store bei einer Änderung des System-Erscheinungsbilds automatisch auf das passende Theme wechseln und alle abhängigen Komponenten benachrichtigen.

3. **Präferenz-Setzung und Auflösung.** Implementiere eine Methode, um die Theme-Präferenz zu setzen. Bei Setzen der Präferenz muss das aufgelöste Modus und das aktive Theme entsprechend aktualisiert werden:
   - "System" → System-Erscheinungsbild als Grundlage.
   - "Light" → immer Light-Theme.
   - "Dark" → immer Dark-Theme.

4. **Benutzerdefinierte Themes.** Stelle eine Möglichkeit bereit, ein importiertes Theme zu aktivieren. Sobald ein benutzerdefiniertes Theme aktiv ist, überschreibt es das Standard-Theme unabhängig vom aufgelösten Modus. Für den Fall, dass ein benutzerdefiniertes Theme entfernt wird, fällt der Store auf das Standard-Theme des aufgelösten Modus zurück.

5. **Benachrichtigung der UI.** Stelle sicher, dass jede Änderung (Präferenz, Modus, aktives Theme oder Custom-Theme) alle UI-Komponenten benachrichtigt, sodass diese neu zeichnen. Nutze die dafür vorgesehenen GPUI-Benachrichtigungsmechanismen (`notify` ergibt im GPUI-Kontext einen Re-Render).

6. **Zugriffs-Helfer für Komponenten.** Stelle einfache, sprechende Zugriffsfunktionen bereit, mit denen jede UI-Komponente aus dem Theme-Store direkt auf die relevanten Farb-/Wert-Felder zugreifen kann (z.B. Funktionen für Hintergrund-, Vordergrund-, Card-, Muted- oder Border-Farben). Ebenso für Radius-, Schatten- und Animationswerte, wo dies für Komponenten relevant ist.

7. **Integration in die App-Startsequenz.** Binde den Theme-Store in `crates/app/` so ein, dass das Fenster beim Start bereits das korrekt aufgelöste Theme nutzt und dass System-Erscheinungsbild-Änderungen während der Laufzeit beobachtet werden.

8. **Tests schreiben.** Erstelle Unit-Tests, die:
   - Die Präferenz-Setzung auf Light/Dark/System korrekt verifizieren.
   - Die Reaktion auf System-Erscheinungsbild-Änderungen verifizieren (wenn Präferenz = System).
   - Das Überschreiben und Zurückfallen bei benutzerdefinierten Themes verifizieren.
   - Verifizieren, dass die Zugriffs-Helfer die richtigen Werte aus dem aktiven Theme liefern.

## Akzeptanzkriterien

- [ ] Der Theme-Store existiert als GPUI-Entity und hält Präferenz, aufgelöstes Modus, aktives Theme und optional ein Custom-Theme.
- [ ] Die System-Dark/Light-Erkennung funktioniert; bei Präferenz "System" wechselt die App automatisch mit dem System.
- [ ] Das Setzen der Präferenz auf Light/Dark/System aktualisiert das aktive Theme korrekt.
- [ ] Ein importiertes Theme überschreibt das Standard-Theme; Entfernen führt zum korrekten Zurückfall.
- [ ] Änderungen benachrichtigen alle UI-Komponenten (Re-Render).
- [ ] Die Zugriffs-Helfer sind für alle wichtigsten Token-Kategorien vorhanden.
- [ ] `cargo run` startet die App bereits mit korrektem Dark- oder Light-Theme (abhängig vom System) und sichtbaren Beispielfarben.
- [ ] Alle Tests laufen grün.

## Notizen

- Labonair nutzt als Standardfarbe "Primary" ein Goldgelb (im Oklch-Wertebereich ~79,7 % Helligkeit, ~0,13 Chroma, ~82° Farbton). Keine Sorge — der zusammenhängende Wert kommt aus T02-001, hier musst du nichts hartkodieren, sondern auf den Theme-Store verweisen.
- Der Theme-Store ist die einzige Stelle, die "weiß", welches Theme aktiv ist. Komponenten dürfen nicht eigene Theme-Zustände führen.
- Die System-Erkennung soll GPUI-Mechanismen nutzen und nicht direkt macOS-Bindings — so bleibt die App auf Linux kompatibel.

## Warnungen

- ⚠️ Nicht zwei separate Theme-Pfade aufbauen (z.B. eigene Struktur zusätzlich zu gpui-component's Theme). Verwende eine einzige, eigene Quelle, die auf Labonair's Token-Struktur basiert.
- ⚠️ Theme-Änderungen lösen in GPUI einen Re-Render des gesamten Fensters aus — das ist erwartet, aber vermeide ineffiziente Muster (z.B. Theme-Werte wiederholt und teuer neu zu berechnen statt zu cachen).

## Weiterführende Tasks

- [T02-003: Theme-Import/Export für Benutzer-Themes](./T02-003-theme-import-export.md)
