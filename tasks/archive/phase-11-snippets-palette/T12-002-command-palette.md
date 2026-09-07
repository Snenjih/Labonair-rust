# T12-002: Command-Palette und Shortcut-System

## Status
✅ Done

## Phase
11 — Snippets & Command-Palette

## Abhängigkeiten
T04-001 (Tab-System)
(plus alle in späteren Phasen definierten Kommandos)

## Ziel
Ein globales Command-Palette-System (öffnen per Cmd+K / Strg+K) und ein konfigurierbares Tastaturkürzel-System implementieren, das App-Kommandos central registriert, durchsuchbar auflistet und per Kürzel/palette auslöst. Die Kommandos sollen aus verschiedenen Domänen (Tab, Terminal, Editor, Explorer, AI, Git, etc.) stammen.

## Kontext
Labonair hat eine Command-Palette (cmdk-basiert) mit vielen Kommandos je Domäne und ein kurzbefehle-System mit einer Registry, aus der auch das native Menü synchronisiert wird. Kommandos haben IDs, Titel, Kategorien, optional Argumente, und sind per Tastatur erreichbar.

In der Rust-Version: Eine Command-Registry + ein Command-Picker (gpui-component bietet eine Palette/Picker-Implementierung). Shortcuts werden über das GPUI-Action-System registriert und können (später in Settings) umgeordnet werden.

## Anweisungen zur Umsetzung

1. **Kommandoregistry.** Implementiere eine zentrale Registry von App-Kommandos:
   - Jedes Kommando: eindeutige ID, Titel, Kategorie, optional Eingabe-/Argument-Schema, Handler (ausführbar-Callback), optional Shortcut-Binding.
   - Kommandos aus allen Domänen registrieren (Tab, Terminal, Editor, Explorer, AI, Git, Snippets, Settings, System).

2. **Command-Picker-UI.** Implementiere die Command-Palette:
   - Öffnen/Schließen per Shortcut (Cmd+K/Strg+K) und Menu.
   - Suchfeld: filtert Kommandos nach Titel/Kategorie, layoutet Ergebnisse als Liste mit Kategorien.
   - Navigation per Pfeiltasten, Auswahl per Enter — führt das Kommando aus.
   - Tastatureingaben über das aktive Feld.

3. **Argument-Eingabe.** Für Kommandos, die Eingaben erfordern (z.B. "Neues Terminal in Pfad", "Gehe zu Datei"), unterstütze eine Fortsetzungsebene (nach Auswahl des Kommandos ein Folgefrage-/Eingabefeld anzeigen oder das Kommando mit aktueller Auswahl ausführen).

4. **Kommandotitel/Kontext-Abhängigkeit.** Unterstütze Optionen, dass Kommandos kontextabhängig erscheinen (z.B. nur wenn ein Editor offen, nur wenn SSH verbunden). Die Palette soll verfügbare Kommandos basierend auf dem aktuellen Zustand anzeigen.

5. **Shortcut-System.** Implementiere das Tastaturkürzel-System:
   - Registrierung von Bindings (Kombination → Kommando-ID) über das GPUI-Action-Mechanismus.
   - Ein zentrales Modul, das Kommando-ID ↔ Shortcut abbildet.
   - Standard-Bindings analog Labonair (New Tab, Close Tab, Command Palette, AI Toggle, etc.).
   - Konfliktauflösung (keine doppelten Bindings).

6. **Native-Menü-Sync.** Synchronisiere die wichtigsten Kommandos in das native macOS-Menü (sofern GPUI native Menüs unterstützt) bzw. ein eigenes App-Menü-System, sodass Kürzel auch über das Menü funktionieren und die Anzeige der Kürzel korrekt ist.

7. **Erweiterbarkeit.** Strukturiere die Registry so, dass spätere Phasen/Features einfach Kommandos hinzufügen können (jestene Domänen registrieren ihre eigenen Kommandos).

8. **Tests schreiben.** Erstelle Tests für:
   - Registrierung und Auflistung von Kommandos.
   - Filterung/Suche nach Titel/Kategorie.
   - Ausführung eines Kommandos via ID.
   - Kontext-Abhängigkeit (verfügbar/ nicht verfügbar).
   - Shortcut-Trigger löst das Kommando aus.
   - Konflikt-Erkennung bei doppelten Bindings.

## Akzeptanzkriterien

- [ ] Die Command-Palette ist öffenbar (Cmd+K/Strg+K), listet und filtert Kommandos, führt per Enter aus.
- [ ] Kommandos aus allen Hauptdomänen sind registriert.
- [ ] Kontext-abhängige Kommandos erscheinen nur, wenn relevant.
- [ ] Eingabe-pflichtige Kommandos zeigen eine Folgefrage/Eingabe.
- [ ] Das Shortcut-System löst Kommandos aus; Standard-Bindings funktionieren; keine Konflikte.
- [ ] Native Menü und/oder App-Menü zeigt die Kürzel korrekt (soweit GPUI es unterstützt).
- [ ] Alle Tests laufen grün.

## Notizen

- Die Kontext-Abhängigkeit (Verfügbarkeit) ist ein wichtiges UX-Feature — Palette nicht mit nutzlosen Kommandos überfrachten.
- Die Registry dient auch als zentrale Dokumentation der Features der App.

## Warnungen

- ⚠️ Doppelte Shortcut-Bindings sauber erkennen/auflösen, sonst feuern versehentlich zwei Kommandos.
- ⚠️ Die Palette und die Texteingabe dürfen sich nicht in den Fokus/Fokus-Verwaltung stören.

## Weiterführende Tasks

- Phase 12: Settings & Preferences (Kürzel-Konfiguration)
