# T17-001: `Panel`-Trait & `PanelRegistry` verdrahten

## Status
📋 Geplant

## Phase
16 — Root-Objekt & Registries

## Abhängigkeiten
T16-008 (Panel-Crates), T16-005 (`labonair-panel` Contracts), T16-006 (`labonair-workspace`)

## Ziel
Das `enum SidebarPanel` + die `render_panel_body`-`match`-Kaskade abschaffen.
Stattdessen implementiert jeder Panel-Crate das `Panel`-Trait aus
`labonair-panel`, und `labonair-shell` registriert die konkreten Panels **einmal**
in einer `PanelRegistry`. Docks und Statusbar-Toggles lesen ab dann nur noch die
Registry.

**Es sind fünf Panels: Explorer, SCM, Git-Graph, Snippets, AI.** Der frühere
`SidebarPanel::Hosts`-Eintrag entfällt **ersatzlos** (der Host-Manager ist kein
Panel und kein Tab mehr — `docs/architecture.md §8.1`; Verbinden läuft über die
Command-Palette, Verwalten über Settings › Hosts). `labonair-hosts-ui` bekommt
**kein** `impl Panel`.

## Kontext
- Contracts aus T16-005: `trait Panel`, `PanelEvent`, `DockPosition`,
  `PanelRegistry` in `crates/panel/src/panel.rs`.
- Abzulösen: `crates/shell/src/app_shell.rs` — `enum SidebarPanel`
  (`Explorer|Snippets|SourceControl|Tabs|Hosts|Ai`), `SidebarPanel::{label,
  slug,from_slug}`, `render_panel_body(panel, cx)` `match`, `left_slot`/
  `right_slot` (`sidebar_slot::SidebarSlot`), sowie die kompakte
  Hosts-in-Sidebar-Liste (`render_hosts_sidebar` / `SidebarPanel::Hosts`,
  `app_shell.rs:2748`+) — die entfällt ganz.
- Panel-Crates (nach T16-008): `labonair-panel-explorer` (`ExplorerView`),
  `-panel-scm` (`GitPanelView`), `-panel-git-graph` (`GitGraphView`),
  `-panel-snippets` (`SnippetsView`), `-panel-ai` (`AiChatView`).
  `labonair-hosts-ui` ist **kein** Panel-Crate.
- Zed-Vorbild: `zed-refrence/zed/crates/workspace/src/dock.rs`
  (`Panel`-Trait, `Dock::add_panel`, `PanelHandle`,
  `dyn PanelHandle`-Erasure), `zed-refrence/zed/crates/project_panel/src/
  project_panel.rs` (ein konkretes `impl Panel`).

## Anweisungen zur Umsetzung
1. **`Panel`-Trait finalisieren** in `labonair-panel` (aus T16-005-Skelett):
   Methoden gemäß T16-005 + object-safe halten. Einen `PanelHandle`-Wrapper
   (analog Zeds `dyn PanelHandle`) einführen, der `Entity<T: Panel>` typlos
   hält und `title`/`icon`/`position`/`render`/Focus weiterreicht.
2. **`PanelRegistry` fertig bauen**:
   - `register(&mut self, meta: PanelMeta, ctor: PanelCtor)` — `PanelMeta`
     = `{ persistent_name, default_position, default_visible }`;
     `PanelCtor` = `Box<dyn Fn(&mut Window, &mut App) -> PanelHandle>`.
   - `iter()` / `by_name()` / `for_position(DockPosition)`.
   - Als GPUI-Global **oder** als Feld im `Workspace` — Entscheidung: Feld im
     `Workspace` (wie Zed die Docks am Workspace hängt), Zugriff über
     `workspace.panel_registry()`.
3. **`impl Panel` je Panel-Crate**:
   - `labonair-panel-explorer`: `impl Panel for ExplorerView` —
     `persistent_name() = "explorer"`, `default_position = Left`,
     `icon = PanelIcon::Files`, `default_size = 260px`, `min_size = 180px`.
   - `-panel-scm`: `"source-control"`, `Left`, `icon = GitBranch`.
   - `-panel-git-graph`: `"git-graph"`, `Bottom`, `icon = GitGraph`.
   - `-panel-snippets`: `"snippets"`, `Left`, `icon = Code`.
   - `-panel-ai`: `"ai"`, `Right`, `icon = MessageSquare`,
     `default_size = 380px`.
   - Werte für `default_position`/`default_size` aus dem heutigen Verhalten +
     `reference-src` ableiten, in Doc-Kommentaren begründen.
   - **Kein** `impl Panel` für `labonair-hosts-ui`. Die `PanelIcon::Hosts`-
     Variante (aus T16-005, `crates/panel/src/dock.rs`) hier entfernen —
     kein Panel nutzt sie mehr.
4. **Registrierung in `labonair-shell`**: eine Funktion
   `register_builtin_panels(workspace, cx)` ruft
   `registry.register(meta, |w, cx| PanelHandle::new(cx.new(|cx|
   ExplorerView::new(...))))` für alle fünf. Das ist der **einzige** Ort mit
   konkreten Panel-Typen.
5. **`app_shell.rs` umstellen**: `enum SidebarPanel` + `render_panel_body` +
   `SidebarPanel::{label,slug,from_slug}` + die Hosts-Sidebar-Liste löschen.
   `render_sidebar` liest `workspace.panel_registry()` + den aktuellen
   Dock-Zustand (Dock-Modell kommt in T17-002 — bis dahin einen minimalen
   „aktives Panel je Seite"-State im `Workspace` halten, den T17-002 durch
   `Dock` ersetzt).
6. **Persistenz-Schlüssel**: `sidebarActivePanel`/`sidebar…`-Prefs weiter über
   `persistent_name()` adressieren (kompatibel zu den heutigen `slug`s
   `explorer|snippets|source-control|ai`; `git-graph` neu). Ein persistierter
   `hosts`-Wert wird beim Laden auf `explorer` gemappt (Migration, in
   Doc-Kommentar notieren).
7. `cargo run`: jedes Panel per Toggle öffnen/schließen; korrekte
   Default-Seite; Breite persistiert über Neustart; kein `match` mehr im
   Shell-Code; kein Hosts-Panel/keine Hosts-Sidebar-Liste mehr.

## Akzeptanzkriterien
- [ ] `enum SidebarPanel` und `render_panel_body` existieren nicht mehr; die
      Hosts-in-Sidebar-Liste ist entfernt.
- [ ] Alle fünf Panel-Crates haben ein `impl Panel`; `labonair-shell` hat
      genau eine `register_builtin_panels`-Stelle. `labonair-hosts-ui` hat
      **kein** `impl Panel`.
- [ ] `PanelRegistry` liefert Panels nach Name + Position; `render_sidebar`
      nutzt ausschließlich die Registry.
- [ ] `cargo run`: Panel-Toggles, Default-Seiten (Explorer/SCM/Snippets
      links, AI rechts, Git-Graph unten sobald T17-002 den Bottom-Dock
      liefert — bis dahin an einer sinnvollen Seite), Breiten-Persistenz —
      alles funktioniert.
- [ ] Ein neues Panel hinzuzufügen erfordert: neuer Panel-Crate + `impl Panel`
      + eine Zeile in `register_builtin_panels` (im PR-Text kurz demonstrieren
      / dokumentieren).
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- `Tabs` war eine `SidebarPanel`-Variante — entfällt ersatzlos (Tab-Leiste ist
  Titlebar, Layout-Vertrag). `bookmarks` ist kein eigenes Panel (in
  `panel-explorer` eingebettet, T16-008).
- `Hosts` entfällt ebenfalls ersatzlos als Panel: der Host-Manager-*Tab*
  (`TabKind::Home`) bleibt in dieser Task noch als on-demand-Tab bestehen und
  wird erst in T17-009 entfernt; die Verwaltung zieht in T19-010 in die
  Settings. Der Connect-Pfad (Command-Palette `Page::Hosts`) ist seit T16-007
  da. `docs/architecture.md §8.1`.
- Zoom/Close-`PanelEvent`s dürfen hier noch No-ops sein — T17-002 verdrahtet
  sie mit dem Dock.

## Warnungen
- ⚠️ Object-Safety: `Panel`-Methoden mit `impl Trait` im Rückgabetyp oder
  generischen Parametern brechen `dyn Panel`. In `zed/crates/workspace/src/
  dock.rs` prüfen, wie Zed `Render` über den Handle statt über das Trait-Objekt
  auflöst.
- ⚠️ Keine gegenseitigen Panel-Deps einschleusen — die Registrierung lebt in
  `shell`, nicht in einem Panel-Crate.

## Weiterführende Tasks
- [T17-002: `Dock`-Modell (L/R/B)](./T17-002-dock-model-lrb.md)
- [T17-003: `StatusItem`-Trait & `StatusItemRegistry`](./T17-003-statusitem-trait-and-registry.md)
