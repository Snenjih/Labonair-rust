# T05-002: Drag-and-Drop und erweiterte Dateiaktionen

## Status
✅ Done

## Phase
4 — File-Explorer

## Abhängigkeiten
T05-001 (Dateibaum und Explorer-Grundlagen)

## Ziel
Drag-and-Drop-Funktionalität im Datei-Explorer implementieren (Dateien/Ordner zwischen Verzeichnissen verschieben, in Terminal hineinziehen, interne Neuanordnung), sowie erweiterte Aktionen wie Kopieren/Ausschneiden/Einfügen mit Zwischenablage-Puffer. Dabei soll auch das Ablegen externer Dateien (vom Betriebssystem) in die App unterstützt werden, soweit machbar.

## Kontext
In Labonair unterstützt der Explorer:
- Drag-and-Drop zum Verschieben von Dateien/Ordnern innerhalb des Baums (mit visuellen Drop-Zielen).
- Ziehen einer Datei in ein Terminal, um deren Pfad dort einzufügen (nützlich für Shell-Operationen).
- Button-/Tastatur-gestützte Kopieren/Ausschneiden/Einfügen-Operationen mit mehrstufigem Puffer (ein Verzeichnis-Puffer, der mehrere Dateien hält).

In GPUI gibt es ein Drag-and-Drop-System auf Element-Ebene. Es wird genutzt, um den Datei-Transfer zu realisieren.

## Anweisungen zur Umsetzung

1. **Drag-Erkennung.** Implementiere das Starten eines Drag von einem Explorer-Knoten:
   - Die ausgewählten Dateien/Ordner werden beim Ziehen gekennzeichnet (visuelles Drag-Preview, das dem Zeiger folgt).
   - Unterstützung für Einzel- und Mehrfachauswahl (Bereich-/Strg-Klick).

2. **Drop-Ziele visualisieren.** Beim Ziehen sollen mögliche Drop-Ziele (Ordner im Baum, Terminal-Bereich) visuell hervorgehoben werden:
   - Hover über einen Ordner → Ziel-Verzeichnis markieren.
   - Hover über Terminal → "Pfad einfügen"-Zustand.
   - Abbrechen (Esc) oder außerhalb loslassen → kein Effekt.

3. **Verschieben im Baum.** Beim Loslassen über einem Ziel-Ordner:
   - Die gezogenen Elemente in das Zielverzeichnis verschieben (FS-Move/rename).
   - Bei Namenskonflikt einen Klärungsdialog zeigen (überschreiben/umbenennen/abbrechen).
   - Den Baum korrekt aktualisieren.

4. **Ziehen ins Terminal.** Beim Loslassen einer Datei über einer Terminal-Session:
   - Den Pfad der gezogenen Datei(en) als (ggf. gequoteten, escape-ten) Text in das Terminal einfügen (analog zum Einfügen eines Pfads in die Shell).
   - Mehrere Dateien mit Leerzeichen trennen.

5. **Kopieren/Ausschneiden/Einfügen.** Implementiere einen Zwischenablage-Puffer im Explorer:
   - "Kopieren" und "Ausschneiden" setzen den Puffer auf die aktuelle Auswahl (ein Stapel, der mehrere Elemente hält) und markieren visuell, dass Elemente im Puffer sind.
   - "Einfügen" führt die Operation aus (kopiert bzw. verschiebt) am Ziel-Ordner.
   - Der Puffer bleibt bestehen, bis er neu gesetzt oder geleert wird; der Status ist in der UI sichtbar (z.B. kleines Banner).
   - Beim Ausschneiden: Rote Markierung der ursprünglichen Elemente, bis eingefügt oder verworfen.

6. **Externe OS-Drops.** Unterstütze (sofern GPUI es bietet) das Ablegen externer Dateien/Ordner aus dem Betriebssystem in den Explorer (z.B. um Dateien in ein Verzeichnis zu kopieren). Wo nicht unterstützt, eine sinnvolle Alternative (z.B. "Hier ablegen nicht unterstützt"-Hinweis) bereitstellen.

7. **Fehlerbehandlung.** Alle Transfer-Operationen müssen Fehler sauber melden (Zugriffsfehler, Namenskonflikte, gleichzeitiges Ändern) — mit klaren Meldungen im UI, Abbruch des Teiltransfers, und konsistentem Baum-Zustand.

8. **Tests schreiben.** Erstelle Tests für:
   - Verschieben von Dateien/Ordnern in ein Zielverzeichnis (echte FS mit Temp-Verzeichnis).
   - Einfügen eines Pfads in eine Terminal-Session (Mock-Session).
   - Kopieren/Ausschneiden/Einfügen-Puffer-Logik (setzen, einfügen, verwerten).
   - Namenskonflikt-Handling.
   - Fehlerverhalten (Ziel existiert nicht, keine Rechte).

## Akzeptanzkriterien

- [ ] Dateien/Ordner lassen sich per Drag-and-Drop zwischen Verzeichnissen im Baum verschieben, mit klar sichtbaren Drop-Zielen.
- [ ] Das Ziehen einer Datei ins Terminal fügt deren Pfad korrekt (gequotet) ein.
- [ ] Kopieren/Ausschneiden/Einfügen funktioniert über einen mehrstufigen Puffer mit sichtbarem Zustand und Verwerfen-Möglichkeit.
- [ ] Externe OS-Dateien können (falls unterstützt) in den Explorer abgelegt werden.
- [ ] Fehler werden sauber gemeldet und hinterlassen keinen inkonsistenten Baum-zustand.
- [ ] Alle Tests laufen grün.

## Notizen

- Die Terminal-Einfüge-Funktion ist ein wichtiges Labonair-Detail — unterstützt den Workflow "Datei in Shell-Operation verwenden".
- Der Puffer ist app-intern (nicht die OS-Zwischenablage); er lebt im Explorer-Store.
- Das visuelle Feedback (Drag-Preview, Drop-Ziel-Markierung) ist für die Benutzerfreundlichkeit entscheidend.

## Warnungen

- ⚠️ Beim Verschieben von Ordnern mit eigenem Inhalt sicherstellen, dass fälschliches Verschieben in sich selbst (Ziel = Unterordner) verhindert wird.
- ⚠️ Race-Conditions beim gleichzeitigen Verschieben/Löschen vermeiden — Operationen sperren oder sequenziell ausführen, Baum nach Fehler konsistent neu laden.

## Weiterführende Tasks

- Phase 5: Editor (öffnet Dateien aus dem Explorer nebst Drag-and-Drop-Integration)
