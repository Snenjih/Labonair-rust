# T18-003: Statusbar links — Panel-Steuerung

## Status
📋 Geplant

## Phase
17 — Neues Layout & Statusbar-Personalisierung

## Abhängigkeiten
T17-002 (`Dock`-Modell), T17-003 (`StatusItemRegistry`)

## Ziel
Die linke Seite der Statusbar wird zur **Panel-Steuerzentrale**: pro
registriertem Panel ein Toggle-Button (Icon + optional Label), der das Panel in
seinem Dock ein-/ausschaltet und das aktive Panel hervorhebt. Das ersetzt die
alte 44px-Activity-Rail und die Sidebar-Umschaltleiste.

## Kontext
- Nach T17-001/002: `PanelRegistry` (im `Workspace`), `Dock` (L/R/B) mit
  `toggle_panel(name)` / `activate_panel(name)` / `open`-Zustand.
- Nach T17-003: `StatusItem`-Trait + `StatusItemRegistry`; die Statusbar
  rendert aus der Registry, links/rechts nach `default_side`/`order`.
- Layout-Vertrag (`docs/architecture.md`): Statusbar links = Panel-Toggles,
  rechts = Info-Dropdowns.
- Referenz-Verhalten: `reference-src/src/modules/statusbar/` +
  `barItems.ts` (Panel-Toggle-Bar-Items) — die Idee „Panel-Toggle als
  Bar-Item" stammt von dort; hier fest auf die linke Statusbar-Seite gelegt.
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/dock.rs` `PanelButtons`
  (Dock-Rand-Buttons) — Konzept, aber bei uns in der Statusbar gebündelt.

## Anweisungen zur Umsetzung
1. **`PanelTogglesStatusItem`** (ein `StatusItem`, `default_side = Left`,
   niedriger `order`, damit ganz links):
   - Iteriert `workspace.panel_registry().iter()` in einer stabilen
     Reihenfolge (Registrierungsreihenfolge oder `order`-Feld je Panel).
   - Pro Panel: `IconButton` mit `Panel::icon()`, Tooltip = `Panel::title()`,
     aktiver Zustand = „Panel offen **und** im Dock aktiv".
   - Klick → `workspace.toggle_panel(name)` (öffnet+aktiviert, oder schließt
     wenn schon aktiv).
   - Rechtsklick auf einen Panel-Toggle → kleines Kontextmenü:
     `Nach links / rechts / unten andocken` (→ `Workspace::move_panel`),
     `Ausblenden` (Panel aus der Toggle-Leiste nehmen — Persistenz in T18-007
     „Panel-Sichtbarkeit").
2. **Kompakt-Modus**: bei schmalem Fenster nur Icons (kein Label); die
   `compact`-Logik aus dem alten `render_bar_item` übernehmen/vereinfachen.
3. **Keybind-Hinweis**: Tooltip zeigt zusätzlich den Keybind
   (`effective_binding` für das jeweilige „Toggle Panel: X"-Command aus
   T17-007), falls gesetzt.
4. **Alte Teile entfernen**: 44px-Activity-Rail, `render_panel_toggle`,
   `render_ai_toggle` (AI ist normales Panel), Sidebar-Umschaltleiste —
   restlos.
5. **Default-Sichtbarkeit**: alle fünf Panels (Explorer, SCM, Git-Graph,
   Snippets, AI) erscheinen initial als Toggle. `git-graph` (Bottom-Dock)
   ebenfalls — sein Toggle schaltet den Bottom-Dock. **Kein Hosts-Toggle**
   (Host-Manager ist kein Panel — `docs/architecture.md §8.1`).
6. `cargo run`: links in der Statusbar fünf Panel-Toggles; Klick öffnet/
   schließt das jeweilige Panel im richtigen Dock; aktives Panel visuell
   markiert; Rechtsklick → Andock-Menü; Tooltips mit Titel + Keybind;
   Kompakt-Modus bei schmalem Fenster.

## Akzeptanzkriterien
- [ ] Links in der Statusbar steht **ein** `PanelTogglesStatusItem`, das aus
      der `PanelRegistry` alle Panels als Toggles rendert.
- [ ] Toggle öffnet/schließt/aktiviert das Panel im korrekten Dock; aktiver
      Zustand korrekt hervorgehoben.
- [ ] Rechtsklick auf einen Toggle: Andocken links/rechts/unten +
      Ausblenden.
- [ ] 44px-Activity-Rail, `render_panel_toggle`, `render_ai_toggle`,
      Sidebar-Umschaltleiste sind entfernt.
- [ ] Kompakt-Modus (nur Icons) bei schmalem Fenster.
- [ ] Tooltip zeigt Panel-Titel + (falls vorhanden) Keybind.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Ob ein einziges Aggregat-`StatusItem` oder je Panel eines: Aggregat ist
  einfacher und hält die Panel-Toggles zusammen (sie sollen nicht per
  RMB einzeln nach rechts wandern — das gilt nur für Info-Items, T18-005).
  In `docs/architecture.md` festhalten: Panel-Toggles sind **fix links**.
- Label-Text der Toggles: kurz (`Explorer`, `Git`, `AI`, …) oder nur Icon —
  Design-Punkt für Nutzer-Sichtprüfung im PR.

## Warnungen
- ⚠️ „Aktiv" heißt: Dock offen **und** dieses Panel das aktive im Dock. Zwei
  Panels im selben Dock → nur eines ist aktiv; beide Toggles zeigen „offen",
  aber nur eines „aktiv". Visuell unterscheidbar machen.
- ⚠️ Der Bottom-Dock-Toggle (Git-Graph) darf den zentralen Workspace-Bereich
  nicht überlagern — das regelt das Dock-Layout aus T17-002, hier nur den
  Toggle korrekt anbinden.

## Weiterführende Tasks
- [T18-004: Statusbar rechts — Info-Dropdowns](./T18-004-statusbar-right-info-dropdowns.md)
- [T18-005: Statusbar-Item-Personalisierung](./T18-005-statusbar-item-personalization.md)
