# T10-001: Git-Graph-Rendering (Commit-Graph)

## Status
⏳ Pending

## Phase
9 — Git-Graph (Canvas)

## Abhängigkeiten
T04-001/2 (Tab-System, Pane)

## Ziel
Eine Commit-Graph-Ansicht als Tab-Typ implementieren, die den Git-Verlauf eines Repositories als Graph darstellt: Commit-Knoten, Branch-Linen (Lanes), Merge-Verzweigungen, Commit-Titel/Details, aktuelle Branch-Hervorhebung. Dazu gehören Layout, Rendering (GPUI-eigenes/Custom-Painting oder Canvas) und Interaktion (über Commit-Knoten hängen, Details zeigen, zu Commit-Navigieren/Diff ansehen).

## Kontext
Labonair's Git-Graph zeigt den Commit-Verlauf mit:
- Commit-Knoten mit Branch-Lanes (unterschiedlich gefärbt je Lane).
- Git-Log-Einträgen (Hashes, Autor, Datum, Message, Branch-Anzeigebadge).
- Klick auf einen Commit → Commit-Detail (Metadaten + zugehörige Diff) in einem Commit-Diff-Panel.
- Typischerweise erstellen*-Graph-Layout (Lane-basiert).

Das Layout wird durch einen Algorithmus (Graph-Layout) für Lane-Zuteilung pro Commit berechnet. Dafür werden Git-Log-Daten (topologisch sortiert, mit Parents, Branch-Refs) von Git geliefert (Backend).

Der angebundene Editor (Phase 5) oder eine eigene Canvas-/Custom-Rendering-Ansicht wird für das Graph-Rendering genutzt. `gpui` erlaubt Custom-Painting (quads, path, text) — für einen Graph eignet sich ein gemischtes Rendering aus Linien (paths), Knoten (quads/circles) und Text.

## Anweisungen zur Umsetzung

1. **Git-Log-Daten abrufen.** Verwende die Git-Log-Funktion des Backends (T01-002) und liefere die topologischen Graph-Daten: jeder Commit mit Hash, Eltern (für Merge-Vorversions), Autor (Name/E-Mail), Zeitstempel, Message (erste Zeile + ggf. Body), Branch-Refs (welche Branches/Tags zeigen hierhin), und ggf. relative Zeit.

2. **Graph-Layout-Algorithmus.** Implementiere einen Lane-Zuweiser:
   - Bestimme für jeden Commit die Lane (Spalte) im Graph, sodass Branch-Verläufe stabil und nicht kreuzend dargestellt werden.
   - Verwalte aktive Lanes (Branch-Linien, die durch den Verlauf laufen), neue Branches (neue Lane), Merge-Kollaps (Lane endet/verzweigt).
   - Ein konsistentes, verständliches Layout erzeugen (ähnlich git-graph-Tools).

3. **Canvas-/Custom-Rendering des Graphs.** Implementiere das Zeichnen:
   - Commit-Knoten (Kreise/Quadrate) auf der jeweiligen Lane.
   - Branch-Linen vertikal durch die Lanes (verschiedene Farben je Lane), mit Merge/Verzweigungs-Kanten.
   - Commit-Text (Hash, Message, Autor, relative Zeit) neben dem Knoten.
   - Aktuelle-Branch-/HEAD-Markierung, ggf. HEAD-Referenz-Pfeil.
   - Bei vielen Commits Viewport-basiertes Rendering (nur sichtbare Zeilen zeichnen) + Virtualisierung.

4. **Interaktion.** Implementiere:
   - Scrollen (vertikal) und ggf. horizontales (falls nötig) über den Verlauf.
   - Hover über Commit-Knoten → Tooltip (Message, Details).
   - Klick auf Commit → Commit-Detail anzeigen (Metadaten + Diff), z.B. in einem nebenstehenden Commit-Diff-Panel.
   - Klick-Auswahl eines Commits hervorheben.

5. **Commit-Detail/Diff-Panel.** Implementiere das Detail-Panel für einen gewählten Commit:
   - Metadaten (Hash, Autor, Datum, Parent(s), Message, Branch-/Tag-Refs).
   - Diff des Commits (gegen Parent; für Merge gg. einen gewählten Parent) — nutze die Diff-Ansicht (T06-004).
   - Navigation zwischen Commits (vorher/nächster).

6. **Branch/Tag-Abzeichen.** Zeige auf Commits sichtbare Branch-/Tag-Namen (Badges) an, wenn sie darauf zeigen, mit Markierung des HEAD.

7. **Ansicht-Optionen.** Biete sinnvolle Optionen, wie: welche Branches anzeigen (alle / aktuelle), Verzweigung zeigen/noch, Commit-Stil (Graph/Liste umschalten), sofern machbar.

8. **Tests schreiben.** Erstelle Tests für:
   - Git-Log-Datenstruktur korrekt geladen (Detail eines Test-Repos).
   - Lane-Zuweisung bei einfachen und Merge-Verhältnissen stabil (keine überschneidende/Kreuzende Lanes nach end).
   - Graph-Layout für gegebene Topologie korrekt (Referenzdaten).
   - Viewport-Rendering logisch korrekt (nur sichtbare).

## Akzeptanzkriterien

- [ ] Der Git-Graph zeigt den Commit-Verlauf als lane-basierten Graphen mit Knoten, Branch-Linen und Commit-Text.
- [ ] Merge-Verzweigungen und -Zusammenführungen sind korrekt dargestellt.
- [ ] Klick auf einen Commit zeigt Commit-Detail mit Diff (via Diff-Ansicht).
- [ ] Branch-/Tag-Badges und HEAD-Markierung sichtbar.
- [ ] Viewport-basiertes Rendering funktioniert performant bei großen Verläufen.
- [ ] Suche/Hover über Commits zeigt Details.
- [ ] Alle Tests laufen grün.

## Notizen

- Der Lane-Layout-Algorithmus ist der Kern des Graph-Renderings — investiere in dessen Korrektheit und Stabilität.
- Nutze die GPUI-Custom-Painting-Fähigkeiten (quads, paths, text) für den Graph. Das ist genau das, wofür custom-painting gedacht ist.

## Warnungen

- ⚠️ Große Repos (zehntausende Commits) dürfen das Rendering nicht verlangsamen — Viewport-basiert + Zeilen-Sharing vorsehen.
- ⚠️ Lane-Zuweisung bei komplexen Merges kann zu instabilen Layouts führen; das Feature gut mit Merge-lastigen Test-Repos abdecken.

## Weiterführende Tasks

- Phase 10: AI-Chat-System
