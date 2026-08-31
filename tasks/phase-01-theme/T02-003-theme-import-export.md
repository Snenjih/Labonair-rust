# T02-003: Theme-Import/Export für Benutzer-Themes

## Status
⏳ Pending

## Phase
1 — Theme-System & Design-Tokens

## Abhängigkeiten
T02-001 (Design-Tokens extrahieren)
T02-002 (Theme-Provider und Theme-Store)

## Ziel
Die vollständige Benutzer-Theme-Funktionalität umsetzen: Themes als JSON-Dateien importieren, das aktive Theme exportieren, Benutzer-Themes persistent speichern und in einer Theme-Liste verwalten. Das Ganze so, wie es Labonair in seinen Einstellungen anbietet, und kompatibel zum dortigen JSON-Format.

## Kontext
In Labonair kann der Benutzer eigene Themes als JSON-Dateien importieren (über den Menü-/Einstellungsbereich), Themes exportieren und löschen. Diese Themes definieren die komplette Farbpalette inklusive Terminal-Farben. Standard-Themes (Light/Dark) sind nicht löschbar; nur benutzerimportierte Themes können entfernt werden.

Der Import-Pfad in der Rust-App muss es erlauben, eine JSON-Datei zu laden, deren Inhalt zu validieren, in die interne Theme-Struktur zu konvertieren und als aktives Theme zu aktivieren. Der Export erzeugt umgekehrt eine wiederverwendbare JSON-Datei aus dem aktuell aktiven Theme.

## Anweisungen zur Umsetzung

1. **JSON-Schema definieren.** Lege ein festes, versioniertes JSON-Schema für Theme-Dateien fest. Das Schema muss mindestens folgende Daten aufnehmen können:
   - Metadaten (Name, Anzeigename, Autor, Version).
   - Alle Farbwerte der Kern-Tokens (Hintergrund, Vordergrund, Primär, Sekundär, Gedämpft, Akzent, Destruktiv, Rahmen, Eingabe, Ring, Card, Popover usw.).
   - Die vollständige Terminal-Farbpalette (Standard, Bright, Dim).
   - Radius- und Schattenwerte, wo Sie vom Benutzer-Thema abweichen dürfen.
   
   Das Schema soll kompatibel zu Labonairs vorhandenem Theme-Dateiformat sein, damit bestehende Benutzer-Themes weiterverwendet werden können.

2. **Farbwerte parsen.** Implementiere robustes Parsen der Farbwerte aus der JSON-Datei. Unterstütze die gängigen Formate (Oklch, Hex, RGB), wie sie auch in Labonairs Theme-Dateien vorkommen. Fehlerhafte oder unbekannte Farbwerte dürfen nicht zum Absturz führen — sie müssen entweder übersprungen (mit sinnvollem Fallback) oder mit einer klaren Fehlermeldung abgelehnt werden.

3. **Benutzer-Themes speichern.** Lege einen dauerhaften Speicher für importierte Themes an. Da Labonair Themes in einer Datenbank ablegt (analog zu Hosts und Snippets), soll dies auch hier über ein persistentes Datenschema laufen. Importierte Themes müssen über App-Neustarts hinweg erhalten bleiben.

4. **Import-Ablauf implementieren.** Erstelle den Ablauf, um eine Theme-Datei einzulesen, das Schema zu validieren, in die interne Theme-Struktur umzuwandeln, in den Speicher zu schreiben und als aktives Theme zu aktivieren (über den Theme-Store aus T02-002).

5. **Export-Ablauf implementieren.** Erstelle den Ablauf, um das aktuell aktive Theme (Standard oder Benutzer-Thema) in eine wohlgeformte JSON-Datei zu serialisieren und dem Benutzer an einem wählbaren Speicherort zu sichern.

6. **Verwaltung der Theme-Liste.** Stelle Funktionen bereit, um alle verfügbaren Themes aufzulisten, ihre Metadaten anzuzeigen, eines zu aktivieren und benutzerimportierte Themes zu löschen. Standard-Themes (Light/Dark) müssen vor dem Löschen geschützt sein.

7. **Einstellungs-UI anbinden.** Verdrahte die Funktionen mit der späteren Einstellungs-Oberfläche im Bereich "Erscheinungsbild": eine Liste der verfügbaren Themes, Schaltflächen zum Importieren (Datei-Auswahl), Exportieren (des aktiven Themes) und Löschen (nur Benutzer-Themes).

8. **Tests schreiben.** Erstelle Tests, die:
   - Ein gültiges Theme-JSON erfolgreich parsen und in die korrekte interne Struktur überführen.
   - Ungültige oder unvollständige JSON-Dateien ablehnen bzw. mit Fallback behandeln.
   - Import/Export einen Round-Trip durchlaufen (exportiertes Theme lässt sich wieder importieren mit identischem Ergebnis).
   - Das Schützen der Standard-Themes vor Löschung verifizieren.

## Akzeptanzkriterien

- [ ] Es gibt ein festes, dokumentiertes JSON-Schema, das zu Labonair's Theme-Format kompatibel ist.
- [ ] Farbwerte in gängigen Formaten werden zuverlässig geparst; fehlerhafte Werte führen nicht zum Absturz.
- [ ] Importierte Themes werden persistent gespeichert und überleben App-Neustarts.
- [ ] Import übernimmt das Theme in die Benutzer-Theme-Struktur und aktiviert es.
- [ ] Export erzeugt eine wohlgeformte, wiederverwendbare JSON-Datei des aktiven Themes.
- [ ] Die Theme-Liste zeigt verfügbare Themes mit Metadaten; Löschen ist nur für Benutzer-Themes möglich, nicht für Light/Dark.
- [ ] Die Einstellungs-UI zeigt die Theme-Verwaltung mit Import/Export/Delete.
- [ ] Alle Tests laufen grün.

## Notizen

- Der Export sollte immer ein gültiges, reimportierbares JSON liefern (Round-Trip). Nimm dir dafür ein Beispiel aus Labonair's `theme_export`-Verhalten.
- Metadaten wie Autor und Version sind auch in der UI sichtbar — sie nicht ignorieren.
- Die persistente Ablage soll demselben Muster folgen wie andere dauerhafte Daten der App (d.h. an einem sinnvollen Anwendungsdatenpfad).

## Warnungen

- ⚠️ Beim Import niemals vertrauliche oder unerwartete Daten ausführen — es handele sich um reine Datenparsing, keine Ausführung.
- ⚠️ Der Import darf nicht dazu führen, dass ein halb-fertiges oder ungültiges Theme als aktiv gesetzt wird — erst nach erfolgreicher Validierung aktivieren.
- ⚠️ Die Standard-Themes dürfen unter keinen Umständen aus dem Speicher gelöscht oder überschrieben werden können.

## Weiterführende Tasks

- [T02-004: Terminal-ANSI-Palette in das Theme integrieren](./T02-004-terminal-palette.md)
