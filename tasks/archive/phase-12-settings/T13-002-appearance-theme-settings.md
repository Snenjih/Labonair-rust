# T13-002: Appearance- & Theme-Einstellungen

## Status
✅ Done

## Phase
12 — Settings & Preferences

## Abhängigkeiten
T13-001 (Settings-Struktur und Preferences)
T02-002 (Theme-Provider und Theme-Store)
T02-003 (Theme-Import/Export)
T02-004 (Terminal-Palette)

## Ziel
Den Erscheinungsbild-/Theme-Bereich der Einstellungen umsetzen: Wahl der Theme-Präferenz (System/Light/Dark), Verwaltung benutzerdefinierter Themes (Import/Export/Löschen, aus T02-003), Auswahl von Schriftart und ggf. Hintergrund-Bildern, sowie zusätzliche Erscheinungs-Optionen (z.B. Akzent-Interaktionsfarben), die mit dem Theme-Store (T02-002) synchronisiert sind.

## Kontext
Labonair's Appearance-Bereich umfasst:
- Theme-Option: System/Light/Dark.
- Benutzer-Themes (Import/Export/Delete) — aus T02-003.
- Hintergrund-Bilder (Backgrounds): Liste, Import, Anwendung als Fensterhintergrund mit Effekten.
- Schriftwahl (System- und eigene Fonts) für UI und Terminal.
- Diverse Layout-/Erscheinungs-Optionen.

Dieser Task verdrahtet diese mit dem Settings-System und dem Theme-Store, sodass Änderungen in den Einstellungen das Erscheinungsbild sofort und über App-Neustarts aktivieren.

## Anweisungen zur Umsetzung

1. **Theme-Präferenz in Settings.** Binde die Theme-Präferenz (System/Light/Dark) an die Settings-Oberfläche und an den Theme-Store (T02-002). Änderungen speichern (in Preferences) und beim Start wiederherstellen.

2. **Benutzer-Theme-Verwaltung in Settings.** Integriere die Theme-Import/Export/Delete-UI (aus T02-003) in den Appearance-Bereich: Liste der verfügbaren Themes (Standard + Benutzer), Import-Button (Dateiauswahl), Export-Button (aktives Theme), Löschen (nur Benutzer-Themes). Aktivieren eines Themes setzt den Theme-Store entsprechend.

3. **Hintergrund-Bilder.** Implementiere die Background-Verwaltung:
   - Liste der verfügbaren Hintergrund-Bilder (importiert).
   - Import (Dateiauswahl) und Löschen.
   - Anwenden als Fensterhintergrund (mit passender Bildbehandlung: scale/cover, Opacity, Verdunkelung für Lesbarkeit).
   - Auswahl/Abschalten des Hintergrunds.
   - Persistente Speicherung der Wahl.

4. **Schrift-Auswahl.** Implementiere die Schriftwahl:
   - UI-Font und Terminal-Font wählen (aus System-Fonts + geladenen Custom-Fonts).
   - Font-Import-Verwaltung (System + eigene Fonts) — ggf. Grundgerüst, Details in Terminal-Settings-Task.
   - Font-Größe für UI und Terminal (als Number-Feld).

5. **Weitere Erscheinungs-Optionen.** Übernimm relevante zusätzliche Anzeige-Optionen (z.B. Fensterradius, Akzent-Hervorhebung, Dichte) soweit von der App unterstützt.

6. **Sofort-Wirkung und Persistenz.** Stelle sicher, dass jede Änderung im Appearance-Bereich sofort wirkt (Theme/Bild/Font anwenden) und beim Neustart erhalten bleibt (über Preferences).

7. **Tests schreiben.** Erstelle Tests für:
   - Theme-Präferenz setzen wendet richtig an und wird gespeichert.
   - Benutzer-Theme-Import/Export/Delete aus der Settings-UI heraus funktioniert.
   - Hintergrund-Bild-Import/Anwenden/Löschen.
   - Schrift-Font-Auswahl und -größe wirken und werden gespeichert.

## Akzeptanzkriterien

- [ ] Die Theme-Präferenz ist über die Settings wählbar und wird angewendet/gespeichert.
- [ ] Benutzer-Themes lassen sich aus den Settings importieren, exportieren, aktivieren und löschen.
- [ ] Hintergrund-Bilder lassen sich importieren, anwenden (mit Bildbehandlung) und löschen; die Wahl wird gespeichert.
- [ ] UI- und Terminal-Schrift lassen sich wählen und die Größe einstellen (persistent).
- [ ] Änderungen wirken sofort und überleben App-Neustarts.
- [ ] Alle Tests laufen grün.

## Notizen

- Hintergrund-Bilder und Themes müssen konsistent zusammenspielen (Bild-Verdunkelung für Lesbarkeit, wenn Theme hell/dunkel).
- **T02-003 hat die Funktions-Ebene fertig, nur die Settings-UI-Verdrahtung fehlt (dieser Task, Schritt 2).** Vorhandene APIs: `ThemeStore::import_theme_file(ThemeFile, cx) -> Result<Vec<String>, String>` (Warnungen anzeigen), `ThemeStore::clear_custom_theme(cx)`, `ThemeStore::active_theme_file(name) -> ThemeFile` (Export, dann `ThemeFile::to_json`), `labonair_ui::ThemeFile::from_json`. Persistenz + Liste + Löschen liegen im Backend `themes`-Modul (`themes_get_all`, `theme_get_default`, `theme_import`, `theme_export`, `theme_delete` — `theme_delete` schützt bereits `id == "default"`).
- Die Font-Verwaltung ist auch für das Terminal relevant (Phase 2) — gemeinsamer Font-Store sinnvoll.

## Warnungen

- ⚠️ Beim Anwenden von Hintergrund-Bildern auf Performance achten (skalierte, gecachte Versionen statt Originale).
- ⚠️ Theme- und Background-Wahl müssen konsistent sein — kein unsichtbarer/unlesbarer Zustand (z.B. helles Bild bei hellem Theme ohne Verdunkelung).

## Weiterführende Tasks

- [T13-003: Terminal- & Editor-Einstellungen](./T13-003-terminal-editor-settings.md)
