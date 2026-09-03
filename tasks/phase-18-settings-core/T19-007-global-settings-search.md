# T19-007: Globale Settings-Suche über alle Seiten

## Status
📋 Geplant

## Phase
18 — Settings-System Zed-Style

## Abhängigkeiten
T19-004 (Settings-UI aus Modell)

## Ziel
Die Settings-Suche von „pro Kategorie filtern" auf eine **globale** Suche
umstellen: ein Suchfeld oben im Settings-Fenster durchsucht Titel,
Beschreibungen und JSON-Pfade **aller** Felder aller Kategorien; Treffer werden
gruppiert nach Kategorie angezeigt und springen bei Auswahl direkt zum Feld
(mit kurzem Highlight).

## Kontext
- Heute: `labonair-settings-ui` — Suche greift nur innerhalb der aktiven
  Kategorie (`settings.rs:4200` Bereich, `CATEGORIES[self.active_cat]`).
- T19-004: Felder sind jetzt `SettingField` mit
  `metadata { title, description }` + `json_path`. Custom-Panes
  (Theme/Shortcuts/AI/MCP/Personalisierung) haben eigene, nicht-Feld-Inhalte.
- Fuzzy-Matcher: `labonair-command-palette::fuzzy` (aus T16-004).
- Zed-Vorbild:
  `zed-refrence/zed/crates/settings_ui/src/settings_ui.rs` — Suchleiste +
  `StringMatchCandidate`-Index über alle `SettingsPageItem`s aller Seiten,
  `fuzzy::match_strings`, Ergebnis-Navigation.

## Anweisungen zur Umsetzung
1. **Such-Index** aufbauen: beim Öffnen des Fensters (und bei
   Schema-/Feld-Änderung) eine `Vec<SearchEntry { page, item_ix, haystack:
   String }>` über **alle** Seiten. `haystack` = `title + " " + description +
   " " + json_path` (letzteres, damit `terminal.cursorStyle` auffindbar ist).
   Custom-Panes: je Pane ein Grob-Eintrag + optional handgepflegte
   Stichworte (z.B. „Keymap, Shortcut, Tastenkürzel" für die Shortcuts-Pane).
2. **Suchleiste** oben im Fenster (immer sichtbar, ersetzt die per-Kategorie-
   Suche). Tippen → `fuzzy::match` über den Index (Score-sortiert, Limit ~50).
3. **Ergebnis-Darstellung**: bei nicht-leerer Query wird die Kategorie-
   Sidebar-Navigation durch eine flache, nach Kategorie gruppierte
   Trefferliste ersetzt (Kategorie-Header + Feld-Zeilen mit Titel +
   gematchtem Beschreibungs-Snippet + `json_path` klein). Leere Query →
   normale Kategorie-Ansicht.
4. **Sprung zum Feld**: Klick/Enter auf einen Treffer → Kategorie öffnen, zum
   Feld scrollen, ~1 s Highlight (Hintergrund-Puls). Bei Custom-Pane-Treffer:
   Pane öffnen.
5. **Tastatur**: `↑/↓` durch Treffer, `Enter` = Sprung, `Esc` = Query leeren.
   Fokus bleibt beim Suchfeld.
6. **Keine Ergebnisse**: klare Leer-Anzeige („Keine Einstellung für
   »…« gefunden").
7. **Tests**:
   - Query `cursor` findet `terminal.cursorStyle` **und** `editor.cursorBlink`
     (o.ä.), gruppiert unter Terminal / Editor.
   - Query nach exaktem `json_path` findet genau das Feld.
   - Query, die eine Custom-Pane-Stichwortliste trifft (`shortcut`), zeigt die
     Shortcuts-Pane als Treffer.
   - Leere Query → Kategorie-Ansicht unverändert.
8. `cargo run`: `cursor` tippen → Treffer aus mehreren Kategorien; Enter
   springt + highlightet; `Esc` zurück zur Kategorie-Ansicht.

## Akzeptanzkriterien
- [ ] Eine globale Suchleiste durchsucht Titel + Beschreibung + `json_path`
      aller Felder aller Kategorien.
- [ ] Treffer sind nach Kategorie gruppiert; Auswahl springt zum Feld mit
      kurzem Highlight.
- [ ] Custom-Panes sind über Stichworte auffindbar.
- [ ] Tastatur-Navigation (`↑/↓/Enter/Esc`); Fokus bleibt im Suchfeld.
- [ ] Die alte per-Kategorie-Suche ist entfernt.
- [ ] Tests decken Mehrkategorie-Treffer, `json_path`-Suche, Custom-Pane,
      Leerfall.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Der Index ist klein (~200 Einträge) — kein Performance-Thema, kein
  Hintergrund-Job nötig.
- Snippet-Highlighting der gematchten Zeichen ist nice-to-have; Score-Sort +
  Sprung sind Pflicht.

## Warnungen
- ⚠️ Bei offener Suche + Feldänderung: den Index nicht bei jedem Tastendruck
  neu bauen — nur beim Öffnen / bei Schema-Änderung.
- ⚠️ Der Sprung muss auch funktionieren, wenn das Zielfeld in einem
  eingeklappten Abschnitt liegt (Abschnitt vorher aufklappen).

## Weiterführende Tasks
- [T19-008: Keymap als Datei mit Kontexten](./T19-008-keymap-file-with-contexts.md)
