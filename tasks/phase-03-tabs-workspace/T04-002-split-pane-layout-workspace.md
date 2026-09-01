# T04-002: Split-Pane-Layout und Workspace

## Status
✅ Done

## Phase
3 — Tab-System & Workspace-Layout

## Abhängigkeiten
T04-001 (Tab-Leiste)

## Ziel
Das flexible Split-Pane-Layout implementieren, das mehrere Panele (Terminals, Editoren) gleichzeitig sichtbar anordnet (horizontal/vertikal splitten, Panelsgrößen per Drag anpassen, zusammenführen), inklusive eines übergeordneten Workspace-Konzepts, das die zentralen Arbeitsbereiche (Sidebar, Terminal-Bereich, zentraler Arbeitsbereich) zusammenfügt.

## Kontext
Labonair arbeitet mit einem Pane-Baum (Split-Pane-Modell): Ein Workspace kann in Panes unterteilt werden, die entweder geleget (ein einzelner Inhalt) oder Splits (zwei Kinder, horizontal/vertikal getrennt mit einstellbaren Größenverhältnissen) sind. Darüber gibt es zusätzliche Sektionen (z.B. Sidebar links, Terminal-Bereich unten, Statusleiste unten), vergleichbar mit Zed's Workspace-Layout.

In der React-Version wurde das über `react-resizable-panels` abgebildet, mit einem Pane-Baum-Typ (`PaneNode = PaneSplit | PaneLeaf`) und einem Layout-Algorithmus. In GPUI bietet `gpui-component` eine Dock-API für Panels mit Splits, aber für die exakte Labonair-Parallelität ist möglicherweise eine eigene Pane-Baum-Struktur sinnvoller.

Dieser Task legt das Layout-Fundament, das Terminal (Phase 2) wie auch spätere Phasen (Explorer, Editor, SFTP, Git) als Inhalte in den Panes hostet.

## Anweisungen zur Umsetzung

1. **Pane-Baum-Datentyp definieren.** Lege die Struktur fest, die das Split-Layout repräsentiert:
   - Ein Blatt (Pane): enthält genau einen Inhalt (z.B. eine Terminal-Session oder ein Editor).
   - Ein Split: enthält zwei Kinder in horizontaler oder vertikaler Anordnung, mit Größenverhältnissen (Anteile).
   - Rekursive Verschachtelung (ein Split kann in einem Split stecken).
   
2. **Layout-Operationen.** Implementiere die grundlegenden Operationen auf dem Pane-Baum:
   - Ein Leaf in horizontale/vertikale Split aufteilen (mit zwei neuen Leaves, inkl. dem ursprünglichen Inhalt und einem neuen).
   - Ein Leaf schließen (einen Split in die verbleibende Hälfte kollabieren).
   - Split-Richtung beim Aufteilen wählen.
   - Größenverhältnisse der Split-Kinder ändern.

3. **Pane-Content-Registry.** Halte eine Zuordnung von Pane (Leaf) → Inhalt. Der Inhalt kann verschiedene Typen haben (Terminal, Editor, ...). Der Inhalt wird als eigenständige View-Entity erstellt und kann einem Leaf zugeordnet sein.

4. **Split-Pane-UI rendern.** Implementiere das Rendering des gesamten Pane-Baums:
   - Für jedes Split-Kind die aufgeteilte Fläche mit einem Trennlinien-Handle (draggable) zwischen den Kindern.
   - Für jedes Leaf die zugehörige Inhalts-View anzeigen.
   - Die Größenerhältnisse via Drag an den Trennlinien anpassbar machen (mit Live-Feedback und Persistenz der letzten Größen).

5. **Tab-Bezug.** Kläre, wie Panes und Tabs zusammenspielen: Meist repräsentiert ein Workspace-Tab die Pane-Baum-Struktur (d.h. ein Workspace-Tab enthält mehrere Panes). Das Tab-System (T04-001) verwaltet die Auswahl; hier geht es um den Inhalt innerhalb eines Workspace.
   - Ein Leaf zeigt den Inhalt des aktiven Unter-Inhalts.
   - Beim Splitten wird der aktuelle Inhalt dupliziert/ein neu erzeugter Inhalt daneben gelegt (z.B. neues Terminal im gleichen Verzeichnis).

6. **Resize-Handles und Mindestgrößen.** Implementiere robuste Resize-Handles:
   - Mindestgröße pro Pane, damit nichts ungültig klein wird.
   - Doppelklick auf eine Trennlinie → zurücksetzen/ausgleichen.
   - Bei sehr kleinen Fenstern sinnvolle Standardgrößen.

7. **Workspace-Hülle.** Baue die Gesamtanordnung der App-Fenster:
   - Eine Grundstruktur: Kopfzeile (Header), Seitenleiste (für Explorer/Home, links, ein-/ausblendbar), zentraler Arbeitsbereich (der Pane-Baum), untere Statusleiste.
   - Laden/Erzeugen initialer Panes (z.B. beim Start einen Workspace mit einer Home- oder einem Terminal-Pane).
   - Seitenleiste ein-/ausblenden und ihre Breite anpassen (analog Labonair).

8. **Persistenz der Layouts.** Bereite die Grundlage vor, um das aktuelle Layout (Pane-Baum + Größen + Inhalt-Zuordnungen) zu serialisieren und wiederherzustellen (die eigentliche Persistenz/Phase-13).

9. **Tests schreiben.** Erstelle Tests für:
   - Pane-Baum-Operationen (Splitten, Schließen, Richtungen).
   - Größenverhältnis-Änderungen.
   - Korrekte Serialisierung des Layouts.
   - Regeln: Schließen eines Leaves → Baum bleibt konsistent und nicht leer.

## Akzeptanzkriterien

- [ ] Der Pane-Baum unterstützt beliebige horizontale/vertikale Verschachtelung.
- [ ] Splitten (mit Richtung), Schließen und Größenanpassung funktionieren.
- [ ] Split-Handles sind draggable mit Mindestgrößen und Doppelklick-Resets.
- [ ] Panes hosten Inhalte (vorerst Terminal- und Platzhalter-Views für andere Typen).
- [ ] Die Workspace-Hülle (Header, Sidebar links, zentraler Arbeitsbereich, Statusleiste unten) ist aufgebaut; Sidebar ist ein-/ausblendbar.
- [ ] Das Layout lässt sich serialisieren (für Phase 13).
- [ ] Alle Tests laufen grün.

## Notizen

- Das Pane-Baum-Konzept von Labonair (in `src/modules/tabs/types.ts`) ist die Referenz für die Struktur. Übernimmt die Logik nach Rust.
- gpui-component bietet eine Dock-Implementierung; entscheide, ob sie ausreicht oder ob eine eigene Pane-Baum-Struktur präziser ist (eigene ist oft einfacher zu behalten und extensions-freundlicher).
- Die Größenverhältnisse (Anteile der Kinder) persistent speichern — über Tabs hinweg soll das Layout stabil bleiben.

## Warnungen

- ⚠️ Tiefe Verschachtelungen (viele Splits) müssen performant und korrekt gezeichnet werden — prüfe das Rendering bei 4–8 Panes.
- ⚠️ Beim Website-/Tab-Wechsel darf der Pane-Baum nicht verlorengehen — Inhalt ist pro Workspace-Tab gespeichert, nicht global.
- ⚠️ Mindestgrößen konsequent durchsetzen, sonst kann das Layout beim kleinen Fenster umkippen.

## Weiterführende Tasks

- Phase 4: File-Explorer (wird in der Sidebar angezeigt)
- Phase 5: Editor (als Pane-Inhalt)
