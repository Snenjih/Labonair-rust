# T16-008: Panel-Crates ausgliedern

## Status
📋 Geplant

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-006 (`labonair-workspace`), T16-005 (`labonair-panel` Contracts)

## Ziel
Die sechs Seiten-Panel-Views aus `crates/ui` in je einen eigenen Crate lösen:
`labonair-panel-explorer`, `-panel-scm`, `-panel-git-graph`, `-panel-hosts`,
`-panel-snippets`, `-panel-ai`. In dieser Task: reiner Move + Kompilierbarkeit.
Das `impl Panel` für jeden (Trait aus T16-005) folgt in T17-001 — hier zunächst
nur die Datei-/Crate-Heimat.

## Kontext
- Heute in `crates/ui/src/`:
  - `explorer.rs` → `panel-explorer`
  - `git.rs` (Source-Control-Panel: Status/Staging) → `panel-scm`
  - `git_graph.rs` (Commit-Graph — die Panel-Nutzung; die Tab-Nutzung liegt
    seit T16-006 in `labonair-workspace::views`) → `panel-git-graph`
  - `hosts.rs` (Host-Manager) → `panel-hosts`
  - `snippets.rs` → `panel-snippets`
  - `ai_chat.rs` + `ai_composer.rs` + `agent_access.rs` → `panel-ai`
  - `bookmarks.rs` (Path-Bookmarks) — Zuordnung offen: entweder in
    `panel-explorer` oder eigener kleiner `panel-bookmarks`. Default:
    **in `panel-explorer`** einbetten (Bookmarks sind verzeichnisnah).
- Gemeinsame Abhängigkeiten dieser Views: `labonair-backend` (fs, git, ssh,
  hosts, snippets, ai, bookmarks-Module), `labonair-ui-kit`, `labonair-theme`,
  `labonair-workspace` (für `WorkspaceLiveBridge`, aktive CWD),
  `labonair-notifications`.
- Zed-Vorbild: `zed-refrence/zed/crates/{project_panel,outline_panel,git_ui,
  agent_ui}` — je Panel ein Crate, hängt von `workspace` + `ui` + Projekt/
  Backend, nie voneinander.

## Anweisungen zur Umsetzung
1. Für jeden der sechs Crates:
   - `crates/panel-<name>/` anlegen (`labonair-panel-<name>`,
     `src/panel_<name>.rs` Lib-Root, `[lib] path` explizit).
   - Die zugehörige(n) `crates/ui/src/*.rs` per `git mv` hinein, Modulpfade
     anpassen.
   - Dependencies exakt so weit, wie der Code sie braucht — **nie** ein
     anderer `panel-*`, **nie** `labonair-shell`, **nie** `labonair-ui`.
   - Öffentliche API: den View-Typ + dessen `…Event`-Enum + `…::new`
     unverändert re-exportieren (`ExplorerView`, `GitPanelView`,
     `GitGraphView`, `HostsView`/`hosts`-Einstieg, `SnippetsView`,
     `AiChatView` + `AiChatEvent` + `AgentAccessStore` + `AiChatStore`).
2. **Zyklen prüfen**: `git_graph` und `scm` teilen Git-Backend-Zugriff — dieser
   läuft über `labonair-backend::modules::git`, nicht über gegenseitige
   Panel-Deps. `panel-ai` nutzt `WorkspaceLiveBridge` aus
   `labonair-workspace` (ok, gerichtete Kante).
3. Workspace-`Cargo.toml`: alle sechs als Member + Dep-Einträge.
4. `crates/ui`: die verschobenen `mod …;` raus; die Importe in `app_shell.rs`
   (`crate::explorer::ExplorerView`, `crate::git::GitPanelView`,
   `crate::git_graph::GitGraphView`, `crate::snippets::SnippetsView`,
   `crate::ai_chat::{…}`, `crate::agent_access::{…}`, `crate::bookmarks::{…}`)
   → `labonair_panel_*::…`.
5. `cargo run`: jedes Panel per Sidebar-Toggle öffnen; Explorer-Baum,
   SCM-Staging/Diff, Git-Graph, Host-Manager (Verbinden), Snippets
   (ausführen), AI-Chat (Nachricht senden, Slash/`@`-Popup, Plan-Mode) —
   alles unverändert.

## Akzeptanzkriterien
- [ ] Sechs neue `crates/panel-*/`, alle Workspace-Members, mit explizitem
      `[lib] path`.
- [ ] `crates/ui` enthält keine der Dateien `explorer.rs`, `git.rs`,
      `git_graph.rs`, `hosts.rs`, `snippets.rs`, `ai_chat.rs`,
      `ai_composer.rs`, `agent_access.rs`, `bookmarks.rs` mehr.
- [ ] `cargo tree` zeigt für **keinen** `panel-*` eine Kante zu einem anderen
      `panel-*`, zu `labonair-shell` oder zu `labonair-ui`.
- [ ] `cargo run`: alle sechs Panels funktional identisch zu vor der Task
      (manuelle Prüfung je Panel).
- [ ] Bestehende Panel-Tests (Explorer, Git, AI-Composer, Snippets, …) laufen
      in den jeweiligen neuen Crates.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Reihenfolge innerhalb der Task: mit dem einfachsten Panel anfangen
  (`snippets` oder `explorer`), grün machen, committen; dann die übrigen. Alle
  sechs innerhalb dieser Task, aber gern in mehreren Commits.
- `panel-ai` ist der größte (Chat + Composer + Agent-Access + Plan-Mode +
  ModelPicker). Falls er den Rahmen sprengt: interne Modulaufteilung
  (`chat.rs`, `composer.rs`, `plan.rs`, `model_picker.rs`, `agent_access.rs`)
  gleich beim Move — aber kein Logik-Refactor.

## Warnungen
- ⚠️ `bookmarks.rs` wird laut `app_shell.rs` als eigenständiges Overlay-View
  gehalten (`self.bookmarks.clone()` im `render`). Beim Einbetten in
  `panel-explorer` die Overlay-Semantik nicht verlieren — oder doch als
  `panel-bookmarks` separat führen und in `docs/architecture.md` nachziehen.
- ⚠️ `agent_access` teilt `AgentAccessStore` mit `Workspace` (T11-006). Der
  Store muss von `labonair-workspace` **und** `labonair-panel-ai` erreichbar
  sein — ihn nach `labonair-workspace` oder `labonair-backend` legen und von
  `panel-ai` importieren, nicht duplizieren.

## Weiterführende Tasks
- [T16-009: `labonair-shell` + `labonair-app` schlank](./T16-009-shell-and-app-slim.md)
- [T17-001: `Panel`-Trait & `PanelRegistry` verdrahten](../phase-16-registries/T17-001-panel-trait-and-registry.md)
