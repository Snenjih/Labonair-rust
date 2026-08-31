# T13-003: Terminal- & Editor-Einstellungen

## Status
⏳ Pending

## Phase
12 — Settings & Preferences

## Abhängigkeiten
T13-001 (Settings-Struktur und Preferences)
Phase 2 (Terminal)
Phase 5 (Editor)

## Ziel
Die Terminal- und Editor-spezifischen Einstellungsbereiche umsetzen, sodass der Benutzer die wichtigsten Verhaltens- und Darstellungsoptionen für das Terminal und den Texteditor konfigurieren kann, und diese Änderungen sofort in den offenen Terminal-/Editor-Sessions wirken.

## Kontext
Labonair hat umfangreiche Terminal- und Editor-Einstellungen. Beispiele:

**Terminal:**
- Standard-Shell (System/zsh/bash/fish/benutzerdefiniert).
- Schrift (Font, Größe), Cursor-Style/Schlagart, Blink-Verhalten.
- Scrollback-Größe (Zeilen).
- Verhalten (z.B. Tab beim Öffnen, automatisch fokussieren, Keep-After-Exit).
- Transparenz/Opazität.

**Editor:**
- Schrift (Font, Größe), Tab-/Indent-Größe, Expandtab, Word-Wrap.
- Zeilennummern an/aus, relative Zeilennummern.
- Editor-Theme (aus T06-002).
- Vim-Modus an/aus.
- Autosave / andere Verhaltensoptionen.

Dieser Task verbindet diese Einstellungen mit dem Preferences-Store (T13-001) und den Modulen (Terminal-Engine T03, Editor T06), sodass Änderungen live wirken und gespeichert bleiben.

## Anweisungen zur Umsetzung

1. **Terminal-Einstellungsfelder.** Verifiziere/ergänze die Terminal-bezogenen Felder im Preferences-Modell und deren Definitionen. Verbinde sie mit den Terminal-Modul-Einstellungen:
   - Shell-Auswahl: wird beim Starten neuer Terminal-Sessions verwendet.
   - Schrift/Größe: wirkt auf die Font-Metrisch-Berechnung und Darstellung (Phase 2).
   - Cursor-Style/Blink: auf die Cursor-Darstellung.
   - Scrollback-Größe: auf die Engine-Scrollback.
   - Transparenz/Opazität des Terminal-Hintergrunds.

2. **Editor-Einstellungsfelder.** Verifiziere/ergänze die Editor-bezogenen Felder und deren Definitionen:
   - Schrift/Größe, Tab-/Indent, Expandtab, Word-Wrap.
   - Zeilennummern (an/aus, relativ), Editor-Theme, Vim aktivieren/deaktivieren.
   - Verbinde sie mit dem Editor-Modul (Phase 5).

3. **Live-Wirkung.** Stelle sicher, dass eine Änderung in den Settings sofort auf bereits geöffnete Terminal-/Editor-Sessions wirkt:
   - Terminal: Resize/Font wird neu berechnet, Cursor-Style aktualisiert, Scrollback für neue Sessions.
   - Editor: Die erzeugten Einstellungen werden auf die offenen Editor-Views angewendet (Font, Umbrechen, Zeilennummern, Theme-Wechsel).

4. **Persistenz und Defaults.** Stelle sicher, dass die gesetzten Werte persistent sind (über T13-001) und sinnvolle Defaults definiert sind (analog Labonair).

5. **Settings-UI-Bereiche.** Render Si in der Settings-Oberfläche die Terminal- und Editor-Kategorien mit den passenden Feldtypen (Switch, Select, Number, Custom für Font).

6. **Tests schreiben.** Erstelle Tests für:
   - Terminal-Einstellung (z.B. Shell, Scrollback, Cursor) wirkt auf eine neu gestartete Session korrekt.
   - Editor-Einstellung (z.B. Font, Word-Wrap, Vim an/aus, Theme) wirkt auf eine offene Editor-View.
   - Live-Update bei Änderung während laufender Session.
   - Persistenz und Defaults.

## Akzeptanzkriterien

- [ ] Die Terminal-Einstellungen (Shell, Font/Größe, Cursor, Scrollback, Opazität) sind in Preferences und Settings-UI vorhanden und wirken.
- [ ] Die Editor-Einstellungen (Font/Größe, Indent, Word-Wrap, Zeilennummern, Theme, Vim) sind in Preferences und Settings-UI vorhanden und wirken.
- [ ] Änderungen wirken sofort auf offene Sessions/editors (live).
- [ ] Werte sind persistent und Defaults sinnvoll.
- [ ] Die Settings-UI rendert die beiden Bereiche korrekt mit passenden Feldtypen.
- [ ] Alle Tests laufen grün.

## Notizen

- Diese Einstellungen sind von den Nutzern am häufigsten angepasst — Vollständigkeit und Korrektheit wichtig.
- Die Live-Wirkung (ohne Neustart) unterscheidet sich angenehm von veralteten App-Konfigs — hier ein Mantra sein.

## Warnungen

- ⚠️ Font-Änderungen im Terminal müssen die Zell-Metriken neu berechnen (sonst falsche Layouts/Positionierung).
- ⚠️ Editor-Thema/Widget-Wechsel sollte den Inhalt nicht reißen (keine Datenverlust/Rekonfiguration der Datei).

## Weiterführende Tasks

- [T13-004: Shortcut-Konfiguration](./T13-004-shortcut-configuration.md)
