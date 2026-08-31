# T04-001: Tab-Leiste und Tab-Verwaltung

## Status
⏳ Pending

## Phase
3 — Tab-System & Workspace-Layout

## Abhängigkeiten
T01-001 (Cargo Workspace)
T02-001/2 (Theme-Grundlage)

## Ziel
Eine funktionierende Tab-Leiste mit allen gängigen Tab-Operationen implementieren: Tab öffnen, schließen, wechseln, neu anordnen (Drag-and-Drop), Trennen/Neuzuordnen, und die Anzeige des aktiven Tabs. Dabei sollen verschiedene Tab-Typen unterstützt werden (Terminal, Editor, Home, SFTP, Git-Graph usw.), auch wenn die Inhalte erst in späteren Phasen entstehen.

## Kontext
Labonair's Tab-System ist das zentrale Navigationselement. Tabs sind ein verknüpfter Datentyp über eine `kind`-Eigenschaft (workspace, editor, preview, home, sftp, git-diff, commit-diff, git-graph). Der aktive Tab und die Tab-Liste sind die Wahrheit des UI-Zustands.

In der React-Version gab es dafür `useTabs` (die Quelle der Wahrheit) und eine Tab-Leisten-Komponente. In GPUI wird das durch ein Tab-Store/-Entity + eine Tab-Leisten-UI abgebildet.

Ein wichtiges Detail: "Tab" und "Session" sind nicht dasselbe. Der sichtbare Tab (z.B. ein Editor-Tab) wird durch den Inhalt der zugehörigen Session gefüllt; die Engine (Terminal, Editor) lebt unabhängig und überlebt evtl. Tab-Zuordnungsänderungen.

## Anweisungen zur Umsetzung

1. **Tab-Datentyp definieren.** Lege einen Datentyp fest, der die verschiedenen Tab-Kategorien (künftig: Home, Workspace-/Terminal, Editor, SFTP, Git-Graph, Git-Diff, Commit-Diff, Preview, AI-Diff) abbildet. Jeder Tab trägt:
   - Eine eindeutige ID.
   - Einen Titel (dynamisch — z.B. aus Session-Titel oder Dateiname).
   - Typ-spezifische Daten (z.B. Pfad für Editor, Session-ID für Terminal).
   - Flags wie "dirty" (ungespeichert) oder "peek" (Vorschau-Tab).

2. **Tab-Store erstellen.** Ein GPUI-Entity, das die Tab-Liste und die aktive Tab-ID hält und Operationen bereitstellt:
   - Neuen Tab hinzufügen (mit optionalem gewünschten Typ).
   - Tab schließen.
   - Aktive Tab ändern.
   - Tab-Reihenfolge ändern (via Drag).
   - Tab per Typ-Liste filtern (z.B. alle Terminal-Tabs).
   - Reft auf Schließen einer Gruppe (alle schließen).
   - Änderungen an die UI über die GPUI-Benachrichtigung weitergeben.

3. **Tab-Leisten-UI bauen.** Implementiere die visuelle Tab-Leiste (die Zeile mit den Tabs am oberen Rand des Workspace-Bereichs):
   - Lese- und Anzeige des Titels, eines Icon/Type-Indikators, evtl. eines Dirty-Indikators.
   - Hervorhebung des aktiven Tabs (via Theme-Akzent).
   - Schließen-Button pro Tab.
   - Horizontal-Scrollen und Überlauf-Verhalten, wenn viele Tabs offen sind.
   - Optional: Kontextmenü pro Tab (Schließen, andere schließen, rechtss schließen usw.).

4. **Tab-Wechsel/Anzeige.** Der Inhalt, der unter der Tab-Leiste angezeigt wird, muss von der aktiven Tab-ID abhängen: Abhängig vom Tab-Typ wird die jeweils passende Inhalts-Ansicht gezeigt (Terminal-View, Editor-View, Home-View usw.). Für jetzt noch nicht implementierte Typen eine Platzhalter-Ansicht zeigen.

5. **Drag-and-Drop-Umordnung.** Implementiere das Neuanordnen von Tabs per Drag and Drop in der Tab-Leiste. Der Tab-Griff soll die Reihenfolge im Store aktualisieren, während gezogen wird, mit visuellem Live-Feedback.

6. **Tab-Schließen-Verhalten.** Lege das Verhalten beim Schließen fest:
   - Für Editor-Tabs mit ungespeicherten Änderungen: Vor Rückfrage (analog Dirty-Bestätigung) oder automatisches Verwerfen.
   - Für Terminal-Tabs: Session korrekt beenden (Stecken mit T03-005).
   - Falls der aktive Tab geschlossen wird, einen sinnvollen Nachbar-Tab aktivieren.

7. **Grundlegende Tastaturkürzel.** Binde die grundlegenden Kürzel an: Neuer Tab, Tab schließen, Nächster/Voriger Tab wechseln. (Dein Shortcut-System; die vollständige Konfigurierbarkeit kommt in Phase 12.)

8. **Tests schreiben.** Erstelle Tests für:
   - Hinzufügen/Schließen/Wechseln von Tabs.
   - Aktualisieren der Titeln.
   - Umordnen via Drag.
   - Schließen des aktiven Tabs wählt sinnvollen Nachbar.
   - Dirty-Flag für Editor-Tabs.

## Akzeptanzkriterien

- [ ] Der Tab-Datentyp deckt alle Tab-Kategorien ab.
- [ ] Der Tab-Store bietet alle grundlegenden Operationen und benachrichtigt die UI korrekt.
- [ ] Die Tab-Leiste zeigt Titel, aktiven-Tab-Hervorhebung und Schließen-Buttons und behandelt Überlauf durch Scrollen.
- [ ] Der Inhaltsbereich zeigt die passende Ansicht abhängig vom aktiven Tab-Typ (inkl. Platzhalter).
- [ ] Drag-and-Drop-Umordnung funktioniert mit Live-Feedback.
- [ ] Das Schließen verhält sich korrekt für Editor (Dirty-Frage), Terminal (Session beenden) und aktiven Tab (Nachbar aktivieren).
- [ ] Grundlegende Tastaturkürzel für Tab-Operationen funktionieren.
- [ ] Alle Tests laufen grün.

## Notizen

- Der Tab-Datentyp ist die Grundlage für viele spätere Phasen. Lege ihn sorgfältig und erweiterbar an; Typen können in späteren Phasen erweitert werden.
- Die Verbindung Tab→Session (welcher Terminal/Editor-Inhalt gehört zu welchem Tab) sollte an einem Ort klar definiert sein.
- Die heutige React-Version unterscheidet "Tab" und "Session"; diese Trennung beibehalten.

## Warnungen

- ⚠️ Beim Tab-Schließen von Terminals nicht vergessen, die Session tatsächlich zu beenden (T03-005), sonst bleiben Zombie-Prozesse zurück.
- ⚠️ Der Inhaltsbereich darf beim Tab-Wechsel seine laufenden Prozesse (Terminal) nicht beenden oder pausieren — nur die Sichtbarkeit wechselt.

## Weiterführende Tasks

- [T04-002: Split-Pane-Layout und Workspace](./T04-002-split-pane-layout-workspace.md)
