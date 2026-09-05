# T20-003: View-Migration Welle 2 (Hosts, Snippets, AI-Chat, SFTP, Git-Graph, Settings-UI)

## Status
✅ Done

## Phase
19 — UI-Kit & Theme-System

## Abhängigkeiten
T20-002 (View-Migration Welle 1)

## Ziel
Die restlichen Views auf `ui-kit` umstellen, sodass **die gesamte App** aus
einem UI-Vokabular besteht. Danach gibt es keine hand-gerollten Buttons/
Listen/Menüs/Felder mehr außerhalb von `labonair-ui-kit`.

## Kontext
- Betroffen: `labonair-hosts-ui` (kein Panel — Host-Liste + Formular, wird von
  `labonair-settings-ui` eingebettet, T19-010), `labonair-panel-snippets`,
  `labonair-panel-ai` (Chat + Composer + ModelPicker + Plan-Mode +
  Agent-Access), `labonair-workspace::views::sftp`,
  `labonair-panel-git-graph`, `labonair-settings-ui` (die generischen Teile;
  Custom-Panes soweit sinnvoll).
- `labonair-panel-ai` ist der größte Brocken: `ai_chat.rs`, `ai_composer.rs`
  (Slash/`@`-Popups), ModelPicker (380px-Panel, Tabs, Provider-Rail,
  Star-Toggle), Plan-Mode-Strip + PlanDiffReview, Agent-Access-Popover.
- `labonair-settings-ui` nutzt nach T19-004 schon die Renderer-Registry —
  hier die verbleibenden Layout-`div()`s (Sidebar, Seiten-Header,
  Sonder-Panes) auf `ui-kit` heben.
- `reference-src/src/modules/{ssh,sftp,snippets,ai,git-graph}/` +
  `reference-src/src/settings/` — Layout-Referenz.

## Anweisungen zur Umsetzung
1. Pro View dasselbe Vorgehen wie T20-002: Buttons → `Button`/`IconButton`,
   Zeilen → `ListItem`/`ListHeader`, Menüs → `context_menu`/`PopoverMenu`,
   Eingaben → `TextInput`/`NumberField`/`Select`, Umschalter →
   `SegmentedControl`/`ToggleButton`, Info → `Badge`/`Banner`/`Indicator`,
   Tasten → `Kbd`/`KeybindingHint`.
2. **AI-Panel-Besonderheiten**:
   - Slash-/`@`-Popover (`ai_composer`) → `PopoverMenu` mit `ListItem`s;
     Fuzzy-Filter-Verhalten unverändert.
   - ModelPicker → `List` + `Tab`/`SegmentedControl` (All/Favorites/Recent) +
     `ListItem` mit Star-`IconToggleButton`.
   - PlanDiffReview → `Section`/`Disclosure` je Datei + `Button` (Apply/Reject).
   - AI/Shell-Toggle im Composer → `SegmentedControl`.
3. **SFTP-View**: Datei-Liste → `Table` **oder** `List` (je nachdem, was
   T20-001 gebaut hat); Transfer-Zeilen → `ListItem` mit `Indicator`/
   Fortschritt.
4. **Git-Graph**: die Graph-Zeichnung (Commit-Knoten/Kanten) ist **kein**
   `ui-kit`-Fall; nur die Umgebung (Toolbar, Commit-Detail-Panel,
   Kontextmenü) migrieren.
5. **Settings-UI**: Kategorie-Sidebar → `List`/`ListItem`; Seiten-Header →
   `SectionHeader`; „Zurücksetzen"/„Öffnen"-Buttons → `Button`; Banner
   (Fehler aus T19-005/006) → `Banner`.
6. **Abschluss-Audit**: `grep` über alle Nicht-`ui-kit`-Crates nach
   `on_click`-tragenden `div()`, „Zeile mit Icon+Label"-Mustern, `anchored`+
   `deferred`-Handmenüs — Ergebnis ≈ 0 (dokumentierte Ausnahmen: Render-Kerne,
   Graph-Zeichnung).
7. Vorher/Nachher-Screenshots je View im PR.

## Akzeptanzkriterien
- [x] Hosts, Snippets, AI (alle Teil-Views), SFTP, Git-Graph-Umgebung,
      Settings-UI nutzen ausschließlich `ui-kit`-Primitives für Buttons/
      Listen/Menüs/Felder/Umschalter. 16 Commits (`a4fda76` Hosts,
      `b620b4e` Snippets, `3fbf8bd`/`7e1122e`/`85c2b0c`/`baeb812`/`d96095a`
      panel-ai sub-areas, `b031120` SFTP, `489b366` transfer queue,
      `25a6ca1` git-graph, `e754773`/`a3a4f01`/`4187a7f`/`4719c62`
      settings-ui, `7b5513a` shell updater, `3c4bcc1` close-out audit).
- [x] App-weiter `grep`: keine hand-gerollten interaktiven `div()`-Bausteine
      außerhalb `labonair-ui-kit` ohne dokumentierte Ausnahme. `3c4bcc1`
      migrierte die verbleibenden Fundstellen (bookmarks, status-items,
      notifications, preview, panel-scm, workspace tab-strip/loading-screen/
      close-confirm, hosts-ui import/export/quick-connect, panel-ai session
      toggle) und dokumentierte den Rest inline (titlebar account trigger,
      search-overlay glyph buttons, command-palette result row/breadcrumb/
      mode chip, SFTP path bar, git commit-message box, AI code-block
      Run/Copy links) — Ausnahmen: alacritty-Zellen, TreeSitter-Text,
      Git-Graph-Zeichnung, Terminal-Hintergrundbild, plus die oben
      genannten dokumentierten UI-Sonderfälle.
- [x] Kein Verhaltensbruch in irgendeiner migrierten View (Sichtprüfung +
      bestehende Tests, `cargo test --workspace` grün nach jedem Commit).
- [x] AI-Composer Slash/`@`-Popups, ModelPicker, Plan-Mode funktionieren
      unverändert (mechanische Primitive-Swaps ohne Logikänderung, siehe
      `3fbf8bd`/`7e1122e`/`85c2b0c`/`baeb812`).
- [x] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Ein Commit pro View, Gate grün. `labonair-panel-ai` ggf. in mehrere Commits
  (Chat / Composer / ModelPicker / Plan).
- Nach dieser Task ist der P2-Punkt „Komponenten überall integrieren,
  einheitliches System" erfüllt.

## Warnungen
- ⚠️ Der AI-Composer hat empfindliches Fokus-/Popover-Verhalten (Popup über
  dem Input, Enter-Completion, `InputEvent::Change`-Subscription). Beim
  Wechsel auf `PopoverMenu` das Fokus-Handling explizit durchtesten.
- ⚠️ „Surgical Changes" — kein Mitschleppen von Logik-Refactors in den großen
  AI-Dateien.

## Weiterführende Tasks
- [T20-004: Component-Gallery](./T20-004-component-gallery.md)
- [T21-001: Render-Pfad-Profiling](../phase-20-perf-signoff/T21-001-render-path-profiling.md)
