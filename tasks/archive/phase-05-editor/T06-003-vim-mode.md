# T06-003: Vim-Modus

## Status
✅ Done

## Phase
5 — Editor

## Abhängigkeiten
T06-001 (Editor-Fundament)

## Ziel
Einen funktionsfähigen Vim-Modus im Editor implementieren (Normal-, Insert-, Visual-Modus und Grundkommandos), der über eine Einstellung umschaltbar ist und das vim-typische Tastaturlayout sowie eine Auswahl an Kommandos und Motion unterstützt.

## Kontext
Labonair bietet Vim-Modus über den CodeMirror-Vim-Addon. Viele Nutzer wollen vim-ähnliche Bearbeitung (Modal-Editieren, Motion, Kommandos). In der Rust/GPUI-Welt gibt es Vim-Modus-Implementierungen in verschiedenen Editor-Engines; eine gängige Wahl (wie von Zed) ist der `vim`-Modus über die `vim`-Crate (zx- Kollektiv), die eine Vim-Emulation bietet, die mit einem Textpuffer-Wrapper interagiert.

Ziel ist ein solider, umschaltbarer Vim-Modus mit den meistgenutzten Funktionen:
- Modale (Normal/Insert/Visual) mit Status-Anzeige.
- Motion (h/j/k/l, w/W/b/B, gg/G, $/^, %, etc.).
- Editier-Kommandos (x, dd, yy, p, r, cw, etc.).
- Visual-Operationen (v, V, Strg+v).
- Grundlegende Kommandozeile (:w, :q, :e).
- Undo/Redo-Integration und Registries (minimal).

## Anweisungen zur Umsetzung

1. **Vim-Engine einbinden.** Verwende eine geeignete Vim-Emulation (z.B. die `vim`-Crate) und verbinde sie mit dem Editor-Textpuffer. Falls die Crate nicht all WHAT bieten, implementiere die Kernel-Emulation als eigene Ebene über den Editor. Entscheide den Ansatz bewusst und dokumentiere ihn.

2. **Modi und Umschaltung.** Implementiere:
   - Umschaltung Normal↔Insert (i, I, a, A, o, O, s, S, c etc.) und Rückkehr (Esc, Strg-C).
   - Visual-Modus (v, V, Strg-v) mit Erweiterung der Auswahl via Motion.
   - Anzeige des aktuellen Modus (in Statusleiste / Modus-Indikator).

3. **Motion-Implementierung.** Implementiere die üblichen Motion-Befehle korrekt (Cursor-Bewegung zeichen-, wort-, zeilen-, abschnitts-, dateibasiert; mit Counts wie `3w`, `5j`). Motion müssen mit Modifier (z.B. in Operatoren `d3w`) kombiniert werden können.

4. **Editier-Operationen.** Implementiere die gängigen operativen Kommandos (Löschen, Ändern, Yanken, Einfügen), die auf Motion-Auswahlbereichen wirken. Sicherstellen, dass sie korrekt in den Undo-Stack integriert werden.

5. **Kommandzeile (Cmdline).** Implementiere grundlegende Ex-Kommandos (`:w`, `:q`, `:e`, `:wq`, `:set` für die wichtigsten Optionen, `:noh`). Das Eingabefeld für die Cmdline in das UI integrieren (siehe Cmdline-UI).

6. **Optionen.** Unterstütze die wichtigsten Vim-Optionen, die der Editor anbieten soll: `number` (Zeilennummern), `relativenumber`, `hlsearch`, `incsearch`, `smartcase`, `expandtab`, `tabstop`, `shiftwidth`. Die Einstellungen sollen aus der App-Präferenz (Phase 12) gespeist werden.

7. **Such-Integration.** Verbinde `/`- und `?`-Suche (Vim-Stil) mit der Editor-Suche (aus T06-001) — Cursor zu Treffern usw. Üblicherweise werden Suchbegriff und Cursor-Navigation verknüpft.

8. **UI-Integration.** Zeige den Vim-Modus-Anteil in der Statusleiste an (wenn Vim aktiv). Zeige ggf. den Aufzeichnungs-/Makro-Status.

9. **Tests schreiben.** Erstelle Tests für:
   - Modus-Wechsel und Cursor-Positionen.
   - Grundlegende Bewegungen (Motion) inkl. Counts.
   - Editier-Operationen (dd, yy, dw, ciw, etc.) und Auswirkungen auf den Puffer.
   - Visual-Operationen (Auswahl löschen/ändern).
   - Cmdline-Kommandos (:w, :q-Leitplanken).
   - Optionen wirken (number togglen, tabstop).

## Akzeptanzkriterien

- [ ] Vim-Modus ist per Einstellung umschaltbar.
- [ ] Modi (Normal/Insert/Visual, ggf. Operator-Pending) funktionieren mit Status-Anzeige.
- [ ] Die wichtigsten Motion- und Editier-Kommandos funktionieren inkl. Counts und Kombination.
- [ ] Grundlegende Cmdline-Kommandos funktionieren.
- [ ] Vim-Suche (/) integriert sich mit der Editor-Editor-Suche.
- [ ] Optionen (number, tabstop, etc.) wirken und kommen aus der Präferenz.
- [ ] Alle Tests laufen grün.

## Notizen

- Der Vim-Modus ist ein zentrales Feature für viele Nutzer — Qualität hier wertschätzen. Die Core-Motion und Operatoren müssen zuverlässig sein, bevor erweiterte Features (Makros, Marks) hinzukommen.
- Die Vim-Engine (falls die Crate genutzt wird) muss sauber an den GPUI-Puffer angebunden werden (Eingabe-Übersetzung über das Tastatur-Mapping aus T03-003-Stil).

## Warnungen

- ⚠️ Undo/Redo-Interaktion mit Vim-Operationen: Jede Vim-Änderung soll vernünftige Undo-Einheiten bilden (nicht jede Tasteneingabe einzeln).
- ⚠️ Kompatibilität zwischen Vim-Modus und regulären Editor-Shortcuts klären — im Vim-Modus verhält sich die Tastatur vim-typisch, nicht App-typisch.

## Weiterführende Tasks

- [T06-004: Diff-Ansicht](./T06-004-diff-view.md)
