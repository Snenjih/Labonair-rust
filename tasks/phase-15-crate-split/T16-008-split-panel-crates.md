# T16-008: Panel-Crates ausgliedern

## Status
📋 Geplant

## Phase
15 — Crate-Zerlegung & Fundament

## Abhängigkeiten
T16-006 (`labonair-workspace`), T16-005 (`labonair-panel` Contracts)

## Ziel
Die fünf Seiten-Panel-Views aus `crates/ui` in je einen eigenen Crate lösen:
`labonair-panel-explorer`, `-panel-scm`, `-panel-git-graph`,
`-panel-snippets`, `-panel-ai`. **Plus** die Host-Manager-View in ein eigenes,
**kein-Panel**-Crate `labonair-hosts-ui` (siehe
[`docs/architecture.md §8.1`](../../docs/architecture.md) — der Host-Manager
ist weder Tab noch Dock-Panel mehr; die Verbindungs-Oberfläche liefert die
Command-Palette, die Verwaltung zieht in T19-010 in die Settings). In dieser
Task: reiner Move + Kompilierbarkeit. Das `impl Panel` für die fünf Panels
(Trait aus T16-005) folgt in T17-001 — `labonair-hosts-ui` bekommt **kein**
`impl Panel`.

## Kontext
- Heute in `crates/ui/src/`:
  - `explorer.rs` → `panel-explorer`
  - `git.rs` (Source-Control-Panel: Status/Staging) → `panel-scm`
  - `git_graph.rs` (Commit-Graph — die Panel-Nutzung; die Tab-Nutzung liegt
    seit T16-006 in `labonair-workspace::views`) → `panel-git-graph`
  - `hosts.rs` + `ssh_connection.rs` (Host-Manager, seit T16-006 als
    `views/hosts.rs` in `labonair-workspace` temporär) → **`labonair-hosts-ui`**
    (kein Panel-Crate)
  - `snippets.rs` → `panel-snippets`
  - `ai_chat.rs` + `ai_composer.rs` + `agent_access.rs` → `panel-ai`
  - `bookmarks.rs` (Path-Bookmarks) — Zuordnung offen: entweder in
    `panel-explorer` oder eigener kleiner `panel-bookmarks`. Default:
    **in `panel-explorer`** einbetten (Bookmarks sind verzeichnisnah).
- Gemeinsame Abhängigkeiten dieser Views: `labonair-backend` (fs, git, ssh,
  hosts, snippets, ai, bookmarks-Module), `labonair-ui-kit`, `labonair-theme`,
  `labonair-workspace` (für `WorkspaceLiveBridge`, aktive CWD),
  `labonair-notifications`.
- **`labonair-hosts-ui` weicht ab** (Abhängigkeitsregel 9,
  `docs/architecture.md §3`): Deps nur `labonair-backend` (hosts, ssh,
  keyring) + `labonair-ui-kit` + `labonair-theme` + `labonair-notifications`.
  **Kein** `labonair-workspace`, **kein** `labonair-panel`. Das Öffnen eines
  SSH-/SFTP-Tabs geht über hereingereichte Callbacks (`Fn(HostId)` o.ä.), die
  der Aufrufer (heute `Workspace`/`shell`, ab T19-010 auch `settings-ui`)
  stellt.
- Zed-Vorbild: `zed-refrence/zed/crates/{project_panel,outline_panel,git_ui,
  agent_ui}` — je Panel ein Crate, hängt von `workspace` + `ui` + Projekt/
  Backend, nie voneinander.

## Anweisungen zur Umsetzung
1. Für jeden der fünf Panel-Crates:
   - `crates/panel-<name>/` anlegen (`labonair-panel-<name>`,
     `src/panel_<name>.rs` Lib-Root, `[lib] path` explizit).
   - Die zugehörige(n) `crates/ui/src/*.rs` per `git mv` hinein, Modulpfade
     anpassen.
   - Dependencies exakt so weit, wie der Code sie braucht — **nie** ein
     anderer `panel-*`, **nie** `labonair-shell`, **nie** `labonair-ui`.
   - Öffentliche API: den View-Typ + dessen `…Event`-Enum + `…::new`
     unverändert re-exportieren (`ExplorerView`, `GitPanelView`,
     `GitGraphView`, `SnippetsView`,
     `AiChatView` + `AiChatEvent` + `AgentAccessStore` + `AiChatStore`).
   Zusätzlich:
   - `crates/hosts-ui/` anlegen (`labonair-hosts-ui`, `src/hosts_ui.rs`
     Lib-Root, `[lib] path` explizit). `hosts.rs` + `ssh_connection.rs` per
     `git mv` (aus `labonair-workspace::views`) hinein. Re-exportiert
     `HostManagerView`/`HostsView` + `…::new` + die für den Connect-Pfad
     nötigen Datentypen (`KnownHost`-Sicht o.ä.). **Kein** `impl Panel`,
     **kein** `labonair-panel`/`labonair-workspace` in den Deps.
2. **Zyklen prüfen**: `git_graph` und `scm` teilen Git-Backend-Zugriff — dieser
   läuft über `labonair-backend::modules::git`, nicht über gegenseitige
   Panel-Deps. `panel-ai` nutzt `WorkspaceLiveBridge` aus
   `labonair-workspace` (ok, gerichtete Kante).
3. Workspace-`Cargo.toml`: alle fünf `panel-*` + `hosts-ui` als Member +
   Dep-Einträge.
4. `crates/ui`: die verschobenen `mod …;` raus; die Importe in `app_shell.rs`
   (`crate::explorer::ExplorerView`, `crate::git::GitPanelView`,
   `crate::git_graph::GitGraphView`, `crate::snippets::SnippetsView`,
   `crate::ai_chat::{…}`, `crate::agent_access::{…}`, `crate::bookmarks::{…}`)
   → `labonair_panel_*::…`; die Host-Importe → `labonair_hosts_ui::…`.
5. `cargo run`: jedes Panel per Sidebar-Toggle öffnen; Explorer-Baum,
   SCM-Staging/Diff, Git-Graph, Host-Manager-Tab (Verbinden — noch als Tab,
   T17-009 entfernt ihn), Snippets (ausführen), AI-Chat (Nachricht senden,
   Slash/`@`-Popup, Plan-Mode) — alles unverändert.

## Akzeptanzkriterien
- [ ] Fünf neue `crates/panel-*/` + `crates/hosts-ui/`, alle Workspace-Members,
      mit explizitem `[lib] path`.
- [ ] `crates/ui` enthält keine der Dateien `explorer.rs`, `git.rs`,
      `git_graph.rs`, `hosts.rs`, `ssh_connection.rs`, `snippets.rs`,
      `ai_chat.rs`, `ai_composer.rs`, `agent_access.rs`, `bookmarks.rs` mehr
      (bzw. sie sind aus `labonair-workspace::views` raus).
- [ ] `cargo tree` zeigt für **keinen** `panel-*` eine Kante zu einem anderen
      `panel-*`, zu `labonair-shell` oder zu `labonair-ui`.
- [ ] `cargo tree -p labonair-hosts-ui` zeigt **keine** Kante zu
      `labonair-panel`, `labonair-workspace`, `labonair-shell`.
- [ ] `cargo run`: alle fünf Panels + der Host-Manager-Tab funktional
      identisch zu vor der Task (manuelle Prüfung je View).
- [ ] Bestehende Panel-/Host-Tests laufen in den jeweiligen neuen Crates.
- [ ] Gates grün: `cargo fmt --check`, `cargo check --workspace --all-targets`,
      `cargo clippy --workspace --all-targets -- -D warnings`,
      `cargo test --workspace`.

## Notizen
- Reihenfolge innerhalb der Task: mit dem einfachsten Panel anfangen
  (`snippets` oder `explorer`), grün machen, committen; dann die übrigen. Alle
  fünf `panel-*` + `hosts-ui` innerhalb dieser Task, aber gern in mehreren
  Commits.
- `labonair-hosts-ui` ist bewusst **kein** `panel-*`: siehe
  `docs/architecture.md §8.1`. Es wird in T19-010 von `labonair-settings-ui`
  eingebettet und ist der einzige Ort der Host-/Credential-Bearbeitung.
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
- ⚠️ `labonair-hosts-ui` darf **nicht** auf `labonair-workspace` zeigen
  (Regel 9). Wenn `hosts.rs` heute `WorkspaceLiveBridge` / Tab-Öffnen direkt
  aufruft, diese Aufrufe hinter Callback-Parameter (`on_open_ssh(HostId)`,
  `on_open_sftp(HostId)`) legen, die der Aufrufer stellt. Das ist der einzige
  nicht rein mechanische Teil dieses Moves — sauber trennen, kein Logik-Refactor
  darüber hinaus.

## Weiterführende Tasks
- [T16-009: `labonair-shell` + `labonair-app` schlank](./T16-009-shell-and-app-slim.md)
- [T17-001: `Panel`-Trait & `PanelRegistry` verdrahten](../phase-16-registries/T17-001-panel-trait-and-registry.md)
- [T17-009: Tabs optional & Host-Manager-Tab entfernen](../phase-16-registries/T17-009-optional-tabs-empty-workspace.md)
- [T19-010: Settings › Hosts](../phase-18-settings-core/T19-010-hosts-settings-category.md)
