# T13-004: Shortcut-Konfiguration

## Status
⏳ Pending

## Phase
12 — Settings & Preferences

## Abhängigkeiten
T13-001 (Settings-Struktur und Preferences)
T12-002 (Command-Palette und Shortcut-System)

## Ziel
Die benutzerdefinierte Konfiguration von Tastaturkürzeln in den Einstellungen umsetzen: Der Benutzer kann die standardmäßig zugewiesenen Kürzel für App-Kommandos ansehen und neu belegen (und auf Standard zurückzusetzen), mit Konflikt-Erkennung. Die geänderten Belegungen werden persistent gespeichert und sofort aktiv.

## Kontext
Labonair hat ein Keyboard-Shortcut-System mit einer Registry von Shortcuts, die auch ins native Menü synchronisiert wird. Die Belegung (Kombination → Aktion) ist konfigurierbar und wird gespeichert.

In der Rust-Version: Das Shortcut-System aus T12-002 wird um eine persistente, benutzerbearbeitbare Belegung erweitert. Die Einstellungen zeigen eine Liste der Kommandos mit ihrer aktuellen Tastenkombination, lassen sich neu binden (Kombination aufnehmen), auf Standard zurücksetzen und Konflikte melden.

## Anweisungen zur Umsetzung

1. **Shortcut-Belegungs-Speicher.** Implementiere eine persistente Belegung (Kombination → Kommando-ID), die in den Preferences (T13-001) gespeichert wird. Standard-Belegungen werden beim ersten Start aufgesetzt.

2. **Shortcut-Konfigurations-UI.** Baue die Interface in den Einstellungen:
   - Liste der Kommandos (aus T12-002-Registry) mit aktueller Kürzel-Zuordnung.
   - Aufnahme einer neuen Kombination (Feld fokussieren, Tasten kombinieren, speichern).
   - Zurücksetzen eines einzelnen Kürzels auf Standard, oder aller auf Standard.
   - Suche/Filter in der Liste.

3. **Konflikt-Erkennung.** Erkenne, wenn zwei Kommandos dieselbe Kombination beanspruchen:
   - Beim Setzen eines Kürzels, das bereits von einem anderen Kommando genutzt wird: Warnung und klare Auswahl (neu überschreiben das andere, oder abbrechen).
   - Keine stillen doppelten Belegungen zulassen.

4. **Sofort-Wirkung.** Stelle sicher, dass eine geänderte Belegung sofort im Shortcut-System wirkt (kein Neustart nötig) und an das nativen Menü (falls GPUI dort Kürzel zeigt) synchronisiert wird.

5. **Standard-Wiederherstellung.** Implementiere das Zurücksetzen einzelner oder aller Belegungen auf die Standardwerte (aus T12-002).

6. **Tests schreiben.** Erstelle Tests für:
   - Setzen/Ändern einer Kombination und deren Auswirkung (Trigger).
   - Konflikt-Erkennung und -Auflösung.
   - Zurücksetzen auf Standard (einzelnes + alle).
   - Persistenz über Neustart.
   - Standard-Belegungen beim Erststart.

## Akzeptanzkriterien

- [ ] Die Shortcut-Belegung ist persistent speicherbar und wird beim Start geladen.
- [ ] Die Settings-UI listet Kommandos mit Belegung und erlaubt Neu-Bindung (Kombination aufnehmen).
- [ ] Konflikte werden erkannt und klar behandelt (kein stilles Überschreiben).
- [ ] Geänderte Belegungen wirken sofort und werden an das Menü synchronisiert.
- [ ] Einzele und alle Belegungen lassen sich auf Standard zurücksetzen.
- [ ] Alle Tests laufen grün.

## Notizen

- Diese Einstellung ist für Power-User wichtig — saubere UX (aufgreifen, zurücksetzen, Konfliktmeldung) ist der Fokus.
- Die Registry aus T12-002 ist die Quelle der standard-Zuordnungen.

## Warnungen

- ⚠️ Systemreservierte Kombinationen (z.B. Cmd+Tab auf macOS) dürfen nicht versehentlich belegt werden oder müssen mit einer Warnung erklärt werden.
- ⚠️ Konflikte nie stillschweigend zulassen — sonst starten versehentlich zwei Aktionen.

## Weiterführende Tasks

- Phase 13: Session-Persistence & Scrollback
