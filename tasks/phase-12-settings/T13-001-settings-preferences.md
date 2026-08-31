# T13-001: Einstellungen-Struktur und Preferences

## Status
⏳ Pending

## Phase
12 — Settings & Preferences

## Abhängigkeiten
T01-004 (Event-System, Logging)
T01-001 (Workspace)

## Ziel
Das Einstellungs-System aufbauen: eine zentrale Preferences-Struktur (alle konfigurierbaren App-Optionen), persistent gespeichert und über App-Neustarts geladen, mit einem Einstellungs-Fenster/Modul, das die Werte kategorisiert anzeigen und bearbeiten lässt. Dies ist die Grundlage für einzel-phasen-spezifische Einstellungen (Theme, Terminal, Editor, etc.).

## Kontext
Labonair hat ein Preferences-Objekt mit ~130 Feldern, aufgeteilt in Kategorien (General, Appearance & Layout, Terminal, Editor, Command Palette, File Manager, Connections, Source Control, AI, Bookmarks) und eine Settings-UI (ein separates Fenster) mit Definitionen je Feld (Switch, Select, Input, NumberInput, Custom). Änderungen werden gespeichert und wirken sofort.

In der Rust-Version wird das Preferences-Modell als konkret typisierte Struktur angelegt (nicht generisch), persistent gespeichert (lokale Config-Datei im Anwendungsdatenpfad), und über einen zentralen Store den Modulen bereitgestellt. Die spezifischen Settings-Bereiche (Theme, Terminal, Editor-Theme) füllen sich in den Folge-Tasks dieser Phase.

## Anweisungen zur Umsetzung

1. **Preferences-Datenmodell.** Lege die Preferences als gut benannte, typisierte Struktur an. Übernimm die relevanten Felder aus Labonair's Preferences (in Kategorien gegliedert). Beispiele: Theme-Präferenz (System/Light/Dark), Editor-Einstellungen (Font, Fontsize, Tab-Size, Word-Wrap, Line-Numbers), Terminal-Einstellungen (Shell, Font, Fontsize, Scrollback, Cursor-Style), Generell (Start-Verhalten, Sprache/Autostart), AI (Standard-Provider/Modell), Source-Control-Einstellungen, File-Manager (Versteckte-Dateien-Standard), Command-Palette-Einstellungen.

2. **Persistenz.** Implementiere das Speichern/Laden der Preferences:
   - Local storage in einer Konfigurationsdatei im Anwendungsdatenpfad (analog `labonair-settings.json`, aber Rust-nativ).
   - Laden beim Start, Schreiben bei Änderung.
   - Migration/Default-Werte für neue/fehlende Felder (nicht alle Felder müssen aus früheren Versionen vorhanden sein).

3. **Preferences-Store.** Implementiere einen zentralen Store (GPUI-Entity), der die Preferences hält und Zugriff/Mutator bereitstellt:
   - Lesen der Werte aus jedem Modul.
   - Setzen eines Werts (mit Typ-Validierung) → speichern + notifizieren.

4. **Kategorisierung und Definitionen.** Definiere die Einstellungs-Kategorien und die Felddefinitionen (für die UI): je Feld: Schlüssel, Titel, Beschreibung, Kategorie, Werttyp (Switch/Select/Input/Number). Das steuert die automatische Renderisierung der Settings-UI.

5. **Settings-UI-Grundgerüst.** Baue die strukturelle Settings-Oberfläche:
   - Ein Einstellungs-Fenster/Modul mit Seitenleiste von Kategorien und einem Inhaltsbereich je Kategorie.
   - Renderisierung der Felder anhand der Definitionen (Switch, Select, Input, Number).
   - Änderungen übernehmen sofort und speichern.
   - Suche nach Einstellungs-Titel ggf. einfach bedienend.

6. **Verdrahtung mit Modulen.** Sorge dafür, dass die Preferences von den Modulen konsumiert werden: Theme (T02-002), Terminal (Phase 2), Editor (Phase 5) lesen ihre Einstellungen aus dem Preferences-Store. Ändert sich ein Wert in den Settings, reagieren die Module sofort.

7. **Tests schreiben.** Erstelle Tests für:
   - Laden/Speichern der Preferences (Round-Trip).
   - Default-Werte bei fehlendem Feld.
   - Setzen eines Werts persisted und notifziert.
   - Kategorisierung korrekt renderbar.

## Akzeptanzkriterien

- [ ] Ein typisiertes Preferences-Modell mit den relevanten Feldern existiert, in Kategorien gegliedert.
- [ ] Preferences werden persistent gespeichert und beim Start geladen; Defaults für fehlende Felder.
- [ ] Ein zentraler Preferences-Store bietet Lesen/Setzen mit Effekt (speichern + notifizieren).
- [ ] Die Settings-UI zeigt Kategorien und rendert die Feldtypen korrekt; Änderungen wirken sofort und gespeichert.
- [ ] Theme/Terminal/Editor können ihre Werte aus den Preferences lesen und reagieren auf Änderungen.
- [ ] Alle Tests laufen grün.

## Notizen

- Die Preferences sind quasi die "Systemsteuerung" der App — eine solide, saubere Datenstruktur erspart später viele Probleme.
- Nur Felder aufnehmen, die tatsächlich von der App genutzt werden; keine toten Optionen.

## Warnungen

- ⚠️ Persistenz-Datei-Korruption defensiv behandeln (falls Datei beschädigt, nicht abstürzen, sondern Defaults laden und ggf. Backups).
- ⚠️ Konkrete Typen für Werte verwenden (nicht alles als String), um Fehlkonfiguration zu vermeiden.

## Weiterführende Tasks

- [T13-002: Appearance- & Theme-Einstellungen](./T13-002-appearance-theme-settings.md)
- [T13-003: Terminal- & Editor-Einstellungen](./T13-003-terminal-editor-settings.md)
- [T13-004: Shortcut-Konfiguration](./T13-004-shortcut-configuration.md)
