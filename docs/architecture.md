# Labonair-rust — Target Architecture

**Status:** authoritative. Every task of the architecture rework (T16-002 …
T22-001) references this document as the single source of truth. If a later task
is unclear, look here first — do not re-decide. The crate graph may still be
refined by a later task (e.g. a panel that ends up as a tab view), but every
deviation is written back into this file and noted in `handshake.md`.

Sources: `bericht-architektur-rework-roadmap.md` (planning report, §1–§2) and
`vergleichsbericht-zed-vs-rust.md` (Zed reference comparison). Zed pattern
sources are under `zed-refrence/zed/crates/`.

---

## 1. Philosophy

> **"The most efficient way to get your work done in Labonair — with maximum
> performance and modularity for personalization."**

Binding for every rework task:

1. **Simple, fixed base structure.** The app has exactly four visible zones plus
   one overlay layer — nothing more:
   * **Titlebar** — tabs only, plus **one** icon button on the right (dropdown:
     Settings / Profile / future entries).
   * **Workspace** — tab content plus a recursive split layout.
   * **Side Panels** — docks left / right / bottom, each holding several
     switchable panels.
   * **Statusbar** — left: the **panel controls** (one toggle per panel); right:
     the **info dropdowns** (Notifications with badge, CWD breadcrumb, Updater,
     Transfers, Agent-Access).
   * **Modal / Overlay layer** — command palette, dialogs, transient search,
     toasts.
2. **Personalization is first-class.** Statusbar items are movable between
   left/right and can be hidden via **right-click → context menu**. Panels are
   movable between docks. Themes and keymap are editable files. Settings apply
   globally **and** per project folder.
3. **Modularity in the code = modularity in the product.** Every feature unit is
   its own crate with a clear API. New panels / statusbar items / settings
   register themselves via traits — no central god-object touches them.
4. **Performance is measurable.** No per-frame work that per-event would
   suffice; no `cx.notify` without a state change; startup and build time are
   documented before and after the rework.

Parity with the reference app remains mandatory, but from here on it is the
*minimum*, not the goal.

---

## 2. Target crate graph (7 → ~22 crates)

```
labonair-app            (bin)   – only main(): runtime, backend init, window bootstrap

── Foundation ────────────────────────────────────────────────────────────────
labonair-gpui-ext               – prelude re-exports, GPUI helper traits, shared newtypes
labonair-ui-kit                 – design system: Button, IconButton, List, Dropdown,
                                  Select, Dialog, Popover, ContextMenu, Disclosure,
                                  Table, Tabs, Tooltip, Divider, Indicator, Badge,
                                  Banner, KeybindingHint, Kbd, Icon/IconName, file_icon,
                                  markdown renderer
labonair-theme        (extended) – ThemeRegistry, JSON theme families, icon themes,
                                  theme_settings layer (density / font / radius)
labonair-notifications          – NotificationCenter + toast rendering
labonair-command-palette        – palette UI + command / keybind model

── Settings track ────────────────────────────────────────────────────────────
labonair-settings-content       – typed SettingsContent tree + MergeFrom
labonair-settings               – SettingsStore (layer merge), Settings trait +
                                  registration, keymap.json loader, JSON surgical edit,
                                  schema generation
labonair-settings-ui            – settings window, pages, generated field renderers

── Workspace track ──────────────────────────────────────────────────────────
labonair-panel                  – contracts: Panel trait, PanelRegistry,
                                  StatusItem trait, StatusItemRegistry (breaks the cycle)
labonair-workspace              – Workspace, Pane, PaneGroup (split tree), Dock (L/R/B),
                                  StatusBar host, ModalLayer, ToastLayer host, persistence
labonair-shell                  – AppShell: composes titlebar + docks + workspace +
                                  statusbar + modal layer. Thin, no feature code.

── Panels (one crate each) ──────────────────────────────────────────────────
labonair-panel-explorer  · labonair-panel-scm  · labonair-panel-git-graph
labonair-panel-snippets  · labonair-panel-ai

── Host access (NOT a dock panel — see §8) ──────────────────────────────────
labonair-hosts-ui               – host connect list + host / credential editing
                                  UI. Management surface embedded by
                                  labonair-settings-ui (Settings › Hosts);
                                  connect surface fed to the command palette
                                  as data. No dock panel, no tab.

── Unchanged (possible later split) ─────────────────────────────────────────
labonair-terminal (engine) · labonair-editor · labonair-backend · labonair-ai
```

### 2.1 Per-crate purpose + source files

Today's monolith is `crates/ui/src/` (~40 files, ~48k lines; `settings.rs`
5 957, `workspace.rs` 4 076, `app_shell.rs` 2 983, `hosts.rs` / `ai_chat.rs` /
`git.rs` four-digit). The table below assigns each of today's
`crates/ui/src/*.rs` to its target crate.

| Target crate | Purpose (one sentence) | Today's source files |
|---|---|---|
| `labonair-app` (bin) | Only `main()`: tokio runtime, backend init, window bootstrap — no feature code. | `ui/assets.rs`, `ui/window_state.rs` (window part); today's `crates/app` |
| `labonair-gpui-ext` | Prelude re-exports, GPUI helper traits and shared newtypes so downstream crates import one path. | new (extracted helpers currently inlined across `ui/*.rs`) |
| `labonair-ui-kit` | The design system: every reusable primitive, bound to theme tokens; nothing feature-specific. | `ui/components/*` (`button.rs`, `context_menu.rs`, `icon.rs`, `text_field.rs`, `mod.rs`), `ui/markdown.rs` |
| `labonair-theme` (extended) | `ThemeRegistry`, JSON theme families, icon themes, syntax themes, terminal background images, density/font/radius layer. | `ui/theme.rs`, `ui/syntax_theme.rs`, `ui/background.rs`; today's `crates/theme` |
| `labonair-notifications` | `NotificationCenter` + toast rendering; usable from panels and shell. | `ui/notifications.rs` |
| `labonair-command-palette` | Palette UI plus the command / keybind model shared with the keymap. | `ui/command_palette.rs` |
| `labonair-settings-content` | Typed `SettingsContent` tree + `MergeFrom` trait (derives / replaces `Preferences`). | new, derived from the `Preferences` types currently in `ui/settings.rs` |
| `labonair-settings` | `SettingsStore` (layer merge), `Settings` trait + registration, `keymap.json` loader, comment-preserving JSON surgical edit, schema generation. | new, extracted from the store logic in `ui/settings.rs` |
| `labonair-settings-ui` | Settings window, pages, generated per-type field renderers. | `ui/settings.rs` (UI part: pages, `FIELDS`/`SECTION_GROUPS` stay unchanged initially) |
| `labonair-panel` | Contracts only: `Panel` trait, `PanelRegistry`, `StatusItem` trait, `StatusItemRegistry` — signatures ported from Zed, initially unused. Breaks the Panel ↔ Workspace cycle. | new; trait extraction target of `ui/sidebar_slot.rs` and the `BarItemId` model in `ui/bar_items.rs` |
| `labonair-workspace` | `Workspace`, `Pane`, `PaneGroup` (recursive split tree), `Dock` (L/R/B), `StatusBar` host, `ModalLayer`, `ToastLayer` host, layout/session persistence. | `ui/workspace.rs`, `ui/pane.rs`, `ui/tabs.rs`, `ui/sidebar_slot.rs`, `ui/session.rs`, `ui/window_state.rs` (persistence part), `ui/preview.rs`, `ui/sftp.rs`, `ui/terminal.rs` (view), `ui/editor.rs` (view), `ui/diff.rs` (view) — terminal/editor/sftp views stay here for now (§7 of the report); `ui/bookmarks.rs` moved on to `labonair-panel-explorer` in T16-008 (§8.4) |
| `labonair-shell` | `AppShell`: composes titlebar + docks + workspace + statusbar + modal layer; the only crate that knows concrete panel types (registration). Thin, no feature code. | `ui/app_shell.rs`, `ui/lib.rs`, `ui/menu.rs`, `ui/bar_items.rs` (concrete items), `ui/cwd_breadcrumb.rs`, `ui/transfers.rs` (statusbar item + progress UI), `ui/updater.rs`, `ui/agent_access.rs` |
| `labonair-panel-explorer` | File-explorer panel (also hosts the path-bookmarks overlay view — see §8.4). | `ui/explorer.rs`, `ui/bookmarks.rs` |
| `labonair-panel-scm` | Source-control (status / staging) panel. | `ui/git.rs` |
| `labonair-panel-git-graph` | Commit-graph panel. | `ui/git_graph.rs` |
| `labonair-hosts-ui` | Host connect list + host / credential editing UI. **Not a dock panel and not a tab** (see §8): the connect surface is rendered by the command palette (`Enter` = SSH, `Shift+Enter` = SFTP), the management surface is embedded in **Settings › Hosts** (a first-class top-level category). | `ui/hosts.rs`, `ui/ssh_connection.rs` |
| `labonair-panel-snippets` | Command-snippets panel. | `ui/snippets.rs` |
| `labonair-panel-ai` | AI-chat panel. | `ui/ai_chat.rs`, `ui/ai_composer.rs`, `ui/live_bridge.rs` |
| `labonair-terminal` | Terminal engine (alacritty), unchanged; audible bell. | today's `crates/terminal`, `ui/bell.rs` |
| `labonair-editor` | TreeSitter editor core, unchanged. | today's `crates/editor` |
| `labonair-backend` | SSH / SFTP / Git / PTY / SQLite / keyring, unchanged; no UI deps. | today's `crates/backend` |
| `labonair-ai` | AI providers / streaming / tools, unchanged; no UI deps. | today's `crates/ai` |

> **T16-006 outcome note.** `labonair-workspace` was extracted with its full
> structural closure: `workspace.rs` (lib root), `pane.rs` + `pane_group.rs`
> (recursive split tree split out), `session.rs`, `live_bridge.rs`, `tabs.rs`,
> `agent_access.rs`, `background.rs`, `bell.rs`, `markdown.rs`, `syntax_theme.rs`,
> `drag.rs` (extracted from `explorer.rs`), `prefs.rs` (`GlobalPreferences`
> newtype extracted from `ui/settings.rs`), the `AskAboutSelection` action, and
> the tab-content views under `src/views/` (`terminal`, `editor`, `sftp`,
> `preview`, `ssh_connection`, `git_graph`, `diff`, `hosts`). **`views/hosts.rs`
> and `transfers.rs` are acknowledged *temporary* residents** — `Workspace` owns
> `Entity<HostManagerView>` / `Entity<TransfersView>` today; `hosts` moves to
> `labonair-hosts-ui` (T16-008 — **not** a panel crate; see §8) and `transfers`
> to `labonair-shell` later (see `TODO(T16-008)` / `TODO(shell)` at those module
> heads). The `TabKind::Home` host-manager tab is removed in T17-009; the
> management UI re-surfaces in **Settings › Hosts** (T19-010). The runtime
> `ThemeStore` (+ `ThemeMode`, `FontOverrides`, `GlobalTheme`, `init`,
> `init_fonts`, `theme_store`, `active_theme`, `modal_scrim`, `SCROLLBAR_SIZE`,
> `menu_metrics`) moved from `ui/theme.rs` into `labonair_theme::store`; the
> `impl labonair_ui_kit::UiTheme for ThemeStore` lives in `crates/ui-kit`
> (orphan rule — `labonair-theme` must not depend on `labonair-ui-kit`).
> `crates/ui` keeps thin `pub use` shims for every moved symbol.

> **T16-009 outcome note.** `crates/ui` is **deleted**. `labonair-shell`
> (`crates/shell/`, lib root `src/shell.rs`) now owns the last real files that
> lived in the monolith: `app_shell.rs` (unchanged — its diet is T17-006),
> `menu.rs` (native macOS menu bar + `apply_keybinds`), `window_state.rs`
> (window-geometry persistence), `assets.rs` + the `assets/icons/` SVG bundle
> (`include_bytes!` paths are crate-relative — moved together), and the
> shell-near helpers `updater.rs`, `cwd_breadcrumb.rs`, `sidebar_slot.rs`
> (Phase 17 may re-home these as statusbar items). `no_pictograph_icons.rs`
> moved to `crates/shell/tests/`.
>
> **Theme-store home (task instruction 3): decided = `labonair-theme`.** No
> further move — T16-006 already relocated the runtime `ThemeStore` closure to
> `labonair_theme::store`, and `background`/`syntax_theme`/`markdown`/`tabs`
> already live in `labonair-workspace` (T16-006). Task instructions 3–4
> proposed relocations that T16-006 superseded; T16-009 only confirms the
> current homes. `crates/app` reaches every init hook through one
> `labonair_shell::` import root — `labonair-shell` re-exports
> `labonair_theme::{init_fonts, init_theme}`,
> `labonair_notifications::init as init_notifications` and
> `labonair_workspace::background::init as init_background`, so `main.rs`
> changed import paths only (no bootstrap logic). Inside `labonair-shell`,
> `crate::{background,bar_items,live_bridge,pane,session,theme,workspace}` are
> `pub(crate)` re-export shims forwarding to `labonair-workspace` /
> `labonair-theme`, so `app_shell.rs` / `updater.rs` moved byte-for-byte.
>
> `transfers.rs` / `views/hosts.rs` stay in `labonair-workspace` for now
> (`Workspace` still owns `Entity<TransfersView>` / `Entity<HostManagerView>`);
> decoupling them into `labonair-shell` is deferred to the T17 workspace/shell
> rework rather than done here (would be a logic refactor, out of T16-009
> scope). `labonair-shell` deps: `labonair-workspace` + `labonair-settings-ui`
> + all five `labonair-panel-*` + `labonair-command-palette` +
> `labonair-notifications` + `labonair-theme` + `labonair-ui-kit` +
> `labonair-gpui-ext` + `labonair-terminal` + `labonair-backend`. `crates/app`
> now depends on `labonair-shell` (not `labonair-ui`); no crate has a
> `labonair-ui` edge.

---

## 3. Dependency rules (binding — CI-checked in T16-010)

These are stated so T16-010 can derive a mechanical check (e.g. `cargo-depgraph`
+ an allow-list assertion):

> **T16-010:** these rules are now enforced mechanically by
> [`scripts/check-crate-deps.sh`](../scripts/check-crate-deps.sh) (a per-crate
> allow-list over `cargo metadata` + a transitive traversal for acyclicity and
> the "must-not-reach" invariants). CI runs it as the `crate-deps` job in
> `.github/workflows/ci.yml`; a PR that adds a forbidden edge goes red. Two
> pre-existing, accepted deviations are encoded in the allow-list with inline
> comments and listed in §8.5. The realised graph is in §9.

1. **`labonair-panel` depends on no crate of the workspace track** — not on
   `labonair-workspace`, not on `labonair-shell`. This breaks the cycle "panels
   need workspace types, workspace needs the panel trait".
2. **Panel crates** (`labonair-panel-*`) depend only on `labonair-panel` +
   `labonair-ui-kit` + `labonair-theme` + `labonair-backend` (and optionally
   `labonair-terminal` / `labonair-editor` / `labonair-ai`). They depend
   **never on each other**, **never on `labonair-shell`**, **never on
   `labonair-workspace`**.
3. **`labonair-shell` is the only crate that knows concrete panel types.** It
   depends on `labonair-workspace` + `labonair-panel` + every `labonair-panel-*`
   + `labonair-settings-ui` + `labonair-notifications` +
   `labonair-command-palette`, and performs registration.
4. **`labonair-backend`, `labonair-ai`, `labonair-terminal` (engine),
   `labonair-editor` depend on no UI crate** — no `gpui`, no `gpui-component`,
   no `labonair-ui-kit`/`-theme`/`-workspace`/`-shell`.
5. **`labonair-ui-kit` depends only on** `gpui`, `gpui-component`,
   `labonair-theme`, `labonair-gpui-ext`. No feature crate, no workspace crate,
   no panel crate.
6. **`labonair-gpui-ext` depends only on** `gpui` / `gpui-component` (and small
   utility crates). It is a leaf below `ui-kit`.
7. **Settings track:** `labonair-settings-content` has no UI dep;
   `labonair-settings` depends on `labonair-settings-content` (+ `gpui` for the
   store handle) but not on `labonair-settings-ui`; `labonair-settings-ui`
   depends on `labonair-settings` + `labonair-ui-kit` (+ `labonair-hosts-ui`
   for the Hosts category, see rule 9).
8. **The crate graph is acyclic.** Verified in CI (T16-010).
9. **`labonair-hosts-ui` is not a panel crate.** It depends only on
   `labonair-backend` (hosts, ssh, keyring) + `labonair-ui-kit` +
   `labonair-theme`. It depends on **no** `labonair-workspace` / `-shell` /
   `-panel*` — opening a tab is passed in as a callback. `labonair-settings-ui`
   depends on it (embeds the management page). `labonair-command-palette` does
   **not** depend on it: the shell injects the host list + connect callbacks
   into the palette as data (the reference's `RegistryCallbacks` pattern).

---

## 4. Layout contract (binding)

```
┌─ Titlebar ────────────────────────────────────────────────────────────────┐
│  [Tab] [Tab] [Tab] [＋▾]                                          [◉ ▾]     │  ← tabs + new-tab menu (left) + 1 button (right)
├─ Docks + Workspace ──────────────────────────────────────────────────────┤
│ ┌ left dock ┐                                              ┌ right dock ┐ │
│ │  Panel    │            Workspace (split tree)            │   Panel    │ │
│ └───────────┘                                              └────────────┘ │
│ ┌ bottom dock ─────────────────────────────────────────────────────────┐ │
│ │  Panel                                                                │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
├─ Statusbar ──────────────────────────────────────────────────────────────┤
│ [Explorer][SCM][Git][Snippets][AI]  ················  [⟳][CWD ▸][🔔³] │
│  └─ panel toggles (left, default) ──┘   └─ info dropdowns (right) ────────┘│
└──────────────────────────────────────────────────────────────────────────┘
   Overlay layer: command palette · dialogs · Cmd+F search · toasts
```

* **Titlebar** — tabs plus, at the left end of the tab strip, a **`＋▾`
  new-tab menu button** (Terminal / Editor / Preview / Git Graph · separator ·
  `SSH ▸` recent-hosts submenu · `SFTP ▸` recent-hosts submenu · `Host
  settings…`). On the right, exactly **one** icon button `[◉ ▾]` → dropdown:
  `Settings…`, `Profile` (placeholder), separator, room for planned entries.
  The `＋▾` menu counts as part of the tab strip, not a second right-hand
  button — the contract still holds. The macOS traffic-light inset stays.
* **Workspace** — the active tab's content plus the recursive `Member::Axis`
  split tree. **May hold zero tabs** — then it renders the empty surface:
  a centred hint with the key shortcuts; double-click opens a local terminal;
  a dropped file opens an editor tab.
* **Side Panels** — `Dock` on left / right / bottom; each dock holds several
  registered panels, one active, resizable, zoomable, persisted.
* **Statusbar** — left: one toggle per registered panel (from `PanelRegistry`;
  five — Explorer, SCM, Git-Graph, Snippets, AI), active highlighted. Right:
  info dropdowns — Notifications (badge dropdown), CWD breadcrumb, Updater,
  Transfers, Agent-Access, Jump-Hosts — each a `StatusItem`.
* **Modal / Overlay layer** — `ModalLayer` (command palette, dialogs, bookmarks,
  updater modal, transient `Cmd+F` search) and `ToastLayer` (toasts). Nothing
  else renders overlays.

### What is removed

* **Header inline search** in the titlebar — gone. Replaced by a transient
  `Cmd+F` overlay in the modal/overlay layer (no permanent chrome surface).
* **The `⋯` app-menu button** — gone. Replaced by the single `[◉ ▾]` titlebar
  dropdown.
* **The 44 px "activity rail"** (an invention flagged in `subagent-1.md`) —
  gone. Panel switching runs through the statusbar toggles.
* **The titlebar scope of bar items.** The old `barItemPlacements` schema had a
  titlebar side; the new `statusBarItemPlacements`
  (`{ itemId: { side, hidden } }`) has only `left` / `right` + `hidden`.
  Titlebar-scoped items map to the statusbar default (migrator T18-006).
* **`drain_pending_*` frame buffers** in `render()` — gone; events are handled
  directly via `cx.subscribe_in` / `window.defer`.
* **The Host-Manager tab and the `SidebarPanel::Hosts` list** — gone. Not a
  tab, not a dock panel. Connecting to a host runs through the command-palette
  **Hosts** page (`Enter` = SSH, `Shift+Enter` = SFTP), quick-connect rows at
  palette root, the titlebar `＋▾` submenu, and the native menu; `Cmd+Shift+N`
  opens the Hosts page. Adding / editing hosts and credentials lives in
  **Settings › Hosts**, a first-class top-level category. (§8.1)
* **The "always at least one tab" rule** — gone. Closing every tab yields the
  empty workspace surface; `TabKind::Home` is deleted. `startup_tab` gains an
  `empty` value; an empty last session restores empty. (§8.2)

The native macOS menu bar stays (parity); the titlebar dropdown is the
cross-platform, discoverable second path.

---

## 5. Pattern catalog — what we copy 1:1 from Zed

| Area | Zed source | What we take |
|---|---|---|
| Panel / Dock | `zed-refrence/zed/crates/workspace/src/dock.rs` | `Panel` trait (`position`, `set_position`, `default_size`, `min_size`, `PanelEvent`), `DockPosition` |
| Statusbar items | `zed-refrence/zed/crates/workspace/src/status_bar.rs` | `StatusItemView` + `HideStatusItem` — the item describes hiding itself |
| Split layout | `zed-refrence/zed/crates/workspace/src/pane_group.rs` | recursive `Member::Axis` tree + persistence |
| Overlay layers | `zed-refrence/zed/crates/workspace/src/modal_layer.rs`, `zed-refrence/zed/crates/workspace/src/toast_layer.rs` | reusable `ModalLayer` / `ToastLayer` types |
| Settings model | `zed-refrence/zed/crates/settings_content/`, `zed-refrence/zed/crates/settings/src/merge_from.rs` | typed tree + `MergeFrom` |
| Settings store | `zed-refrence/zed/crates/settings/src/settings_store.rs`, `zed-refrence/zed/crates/settings_macros/` | `Settings` trait + `RegisterSetting` derive + `inventory`, layer merge |
| JSON edit | `zed-refrence/zed/crates/settings_json/` (`update_value_in_json_text`) | comment- and format-preserving surgical edits |
| Settings-UI generation | `zed-refrence/zed/crates/settings_ui/src/settings_ui.rs`, `zed-refrence/zed/crates/settings_ui/src/page_data.rs` | `SettingField<T>{ pick }` + a `SettingFieldRenderer` registry per type, `SettingsPageItem` |
| Keymap | `zed-refrence/zed/crates/settings/src/keymap_file.rs`, `zed-refrence/zed/crates/settings/src/base_keymap_setting.rs`, `zed-refrence/zed/assets/keymaps/` | `keymap.json` with contexts + chords, validators |
| Theme | `zed-refrence/zed/crates/theme/src/registry.rs`, `zed-refrence/zed/crates/theme/src/theme.rs`, `zed-refrence/zed/crates/theme/src/icon_theme.rs`, `zed-refrence/zed/crates/theme/src/settings.rs` | `ThemeRegistry`, JSON families, icon themes, density layer |
| UI-kit + gallery | `zed-refrence/zed/crates/ui/src/components/`, `zed-refrence/zed/crates/component/`, `zed-refrence/zed/crates/component_preview/` | primitive set + live-preview page |

---

## 6. Settings layers (overview — detail in Phase 18)

The `SettingsStore` merges configuration in this fixed order, later layers
overriding earlier ones:

```
default  →  user  →  OS  →  project  →  language
```

* **default** — compiled-in defaults (the `SettingsContent` defaults).
* **user** — the global `settings.json` in the user config dir.
* **OS** — platform overrides (e.g. macOS-specific keys).
* **project** — `.labonair/settings.json` in an opened folder (a whitelisted
  subset of keys; use cases: per-repo default host, start layout, snippet set).
* **language** — per-language overrides for editor keys.

`keymap.json` is a parallel file with the same layering (default keymap →
user keymap). `enum ShortcutId` is only the default source.

---

## 7. Naming convention

* Every new crate is named **`labonair-<name>`** (e.g. `labonair-ui-kit`,
  `labonair-panel-explorer`).
* Its directory is **`crates/<name>/`** (without the `labonair-` prefix, e.g.
  `crates/ui-kit/`, `crates/panel-explorer/`).
* `[lib] path` is set **explicitly** to `crates/<name>/src/<name>.rs`
  (e.g. `crates/ui-kit/src/ui_kit.rs`), following the Zed `CLAUDE.md`
  recommendation — the crate root file is named after the crate, not `lib.rs`.
* Existing crates (`app`, `terminal`, `editor`, `backend`, `ai`, `theme`) are
  renamed to the `labonair-<name>` package name over the rework; `crates/ui`
  becomes a shrinking facade and is deleted in T16-009.

---

## 8. Deviations recorded after T16-001 (workflow rework)

The header rule of this file: *every deviation from the T16-001 plan is written
back here.* These three come from the tab / host-access / settings workflow
rework agreed after T16-005. They **extend** the plan — the registry patterns,
dependency rules and acyclic graph are unchanged. Rationale:
[`bericht-workflow-rework.md`](../bericht-workflow-rework.md).

### 8.1 Host access — no Host-Manager tab, no hosts panel

`labonair-panel-hosts` is **dropped** from the crate graph. The SSH host
manager splits into two surfaces:

* **Connect** — the command palette gains a single **Hosts** page (replacing
  the separate `HostsSsh` / `HostsSftp` pages): one row per host,
  `Enter` = open SSH terminal, `Shift+Enter` = open SFTP, with a footer hint
  bar. The most-recent hosts also appear as quick-connect rows at palette root.
  `Cmd+Shift+N` opens the Hosts page. The titlebar `＋▾` menu and the native
  menu keep an `SSH ▸` / `SFTP ▸` recent-hosts submenu.
* **Manage** — **Settings › Hosts**, a first-class top-level settings category
  (peer of Themes): add / edit / delete / duplicate hosts, credentials via
  keyring, jump-hosts, tunnels, SSH-config import/export, availability polling.

View code (`ui/hosts.rs`, `ui/ssh_connection.rs`) moves to **`labonair-hosts-ui`**
in T16-008 — a plain view crate, **not** a `labonair-panel-*` crate, with no
`impl Panel`. `labonair-settings-ui` depends on it and embeds the management
page (T19-010). The palette receives the host list + connect callbacks as
injected data from the shell (no `labonair-command-palette` → `labonair-hosts-ui`
edge). `enum SidebarPanel` (incl. `Hosts`) and the `TabKind::Home` host tab are
deleted (T17-001 / T17-009).

Tasks: T16-007 (palette Hosts page + secondary action), T16-008
(`labonair-hosts-ui`), T17-001 (drop from `PanelRegistry`), T19-001
(`hosts` area in `SettingsContent`), T19-010 (Settings › Hosts page).

### 8.2 Tabs are optional — empty workspace surface

`TabKind::Home` is removed. The workspace may hold **zero** tabs and then
renders the empty surface (centred shortcut hint; double-click → local
terminal; dropped file → editor tab). `close`/`close_all` no longer stop at
one tab. `PaneGroup`'s root is optional (empty = no split tree). `startup_tab`
gains `empty`; session-restore of an empty last state stays empty (respecting
`session_restore`). The titlebar `＋▾` menu and the command palette re-open any
tab type.

Tasks: T17-004 (optional `PaneGroup` root), T17-006 (empty render path),
T17-009 (the `Option<ActiveTab>` audit + `TabKind::Home` removal + empty
surface), T18-001 (`＋▾` menu + empty-surface visuals).

### 8.3 Settings design contract

A written contract — [`docs/settings-guidelines.md`](./settings-guidelines.md),
created in **T19-000** before any other Phase 18 task — governs every settings
page so they cannot drift the way the Tauri version did:

1. One navigation model: **top-level category → in-page disclosure section →
   optional sub-page** (`SubPageLink`). No category deviates.
2. Every setting is a typed field in `SettingsContent` with metadata (title,
   description, unit, range, `requires_restart`). No setting exists only in UI.
3. The field UI is **generated from the Rust type** via the renderer registry
   (T19-004). No bespoke toggles.
4. **Custom panes** (Themes, Hosts, Shortcuts, AI providers, MCP) are a
   sanctioned first-class registration path — a `SettingsPage { kind: Custom }`
   that can also be a **top-level category** — not a hack around the registry.
   They still render inside the standard page chrome (header, search, origin
   badges).
5. Every field shows its effective layer (default / user / project) + a reset
   affordance. Search covers every page (T19-007). Every category and section
   has a stable deep-link slug.

Tasks: T19-000 (contract doc + `CLAUDE.md` rule), T19-001 (`hosts` +
`personalization` as areas; custom-category marker), T19-004 (disclosure nav +
custom top-level category path), T19-010 (Hosts as the first new custom
top-level category).

### 8.4 Bookmarks live in `labonair-panel-explorer` (no `panel-bookmarks` crate)

T16-008 default taken: the path-bookmarks overlay (`ui/bookmarks.rs` →
`BookmarksView` / `BookmarkEvent`) moves into **`labonair-panel-explorer`** as a
`bookmarks` submodule rather than its own crate — bookmarks are directory-near
and the view already couples to `ExplorerView` (needs the local explorer root).
It stays an `EventEmitter` overlay view: `AppShell` keeps `self.bookmarks:
Entity<BookmarksView>` and renders it as an overlay (unchanged semantics); the
crate boundary is the only thing that moved.

Directed edges added in T16-008 (all acyclic): `labonair-workspace` →
`labonair-hosts-ui` and → `labonair-panel-git-graph` (it owns the tab-view
entities); `labonair-panel-{explorer,snippets,ai}` → `labonair-workspace`
(`Workspace`, `markdown`, `syntax_theme`, `agent_access`). `labonair-hosts-ui`
and `labonair-panel-git-graph` do **not** depend on `labonair-workspace`, so no
cycle. `AgentAccessStore` stays in `labonair-workspace::agent_access` and is
re-exported by `labonair-panel-ai`.

### 8.5 Accepted deviations from the §3 allow-list (T16-010)

`scripts/check-crate-deps.sh` encodes two edges that the literal §3 wording
would reject but that predate the rework and are kept deliberately (each is
commented `[deviation]` in the script):

1. **`labonair-terminal → labonair-theme`.** Rule 4 says the terminal *engine*
   pulls no UI crate. `labonair-terminal` today also renders GPUI cells and
   reads its ANSI palette from `labonair-theme` (a leaf token crate, no
   transitive UI). It reaches nothing else. A deeper engine/renderer split is
   left as future work (tracked in `docs/perf-baseline.md`).
2. **`labonair-panel-ai → labonair-command-palette` and `→ labonair-editor`.**
   Rule 2's optional list covers `terminal`/`editor`/`ai`; `panel-ai` also uses
   the command-palette's slash-command model and the editor buffer for its
   composer. No panel→panel or panel→shell edge is introduced.

The panel crates transitively reach `labonair-panel-git-graph` *through*
`labonair-workspace` (§8.4 — workspace owns that tab-view entity). That
indirection is sanctioned; the check only forbids a **direct** panel→panel
edge and reaching `labonair-shell` by **any** path.

### 8.6 `BarLoc` / bar-item blob kept transitionally through T17-003

T17-003 replaced the shell's `render_bar_item` `match` (plus
`render_simple_bar_button`, every `render_*_item`, `render_bar_menu`,
`build_bar_bucket`, `move_bar_item`, `persist_placement`,
`panel_for_item`/`item_for_panel`) with a `StatusItemRegistry` (a `Workspace`
field, mirroring `PanelRegistry` from T17-001), self-describing `StatusItem`
views in `crates/shell/src/status_items.rs`, and a `StatusBar` component in
`labonair-workspace` (`status_bar.rs`) that renders **only** from the registry
(sorted per side by `order`). Header/titlebar carries no bar items.

The task's AC1 also asked to delete `BarLoc`. That is **deferred to
T18-005 / T18-006**: `BarItemId` / `BarLoc` / `BarSide` / `Placements` /
`BAR_ITEM_ORDER` / `BarLayoutTick` in `labonair-workspace::bar_items` stay
untouched because `labonair-settings-ui` (`view.rs`, `panes/themes.rs` — the
titlebar/statusbar bar-item layout editor) still consumes them, and that
editor + the `barItemPlacements → statusBarItemPlacements` migrator are
explicitly the subject of T18-005 / T18-006 — collapsing `BarLoc` now would
fold those tasks forward. The `BarLayoutTick` `observe_global` in `AppShell`
stays wired (now a plain `cx.notify()`) so a settings-window edit still
refreshes the live bar; T18-005 repoints it at
`StatusItemRegistry::resolve_side`. Dock-layout persistence moved off
`AppShell` onto `Workspace` (a `set_dock_persist_hook` callback, since
`labonair-workspace` cannot depend on `labonair-settings-ui`'s
`PreferencesStore`). No new crate edges (`shell`/`workspace` → `panel` and
`shell` → `workspace` already existed).

### 8.7 `PaneGroup` — n-ary split tree done in T17-004

`labonair-workspace::pane_group` was rebuilt from the pre-T17-004 flat binary
tree (`enum PaneNode { Pane, Split{ratio,first,second} }`) into a Zed-style
n-ary model: `enum Member { Pane(PaneId), Axis(PaneAxis) }`,
`struct PaneAxis { id, axis, members: Vec<Member>, flexes: Vec<f32> }` with
`sum(flexes) == 1.0` held invariant (resize only ever redistributes *within*
one adjacent pair), and `struct PaneGroup { root: Option<Member> }`. Removing
the last pane sets `root = None` — a valid state, not an error; the empty
surface itself is T17-009 / T18-001 (the `render_content` Terminal arm still
shows the old placeholder for `root == None`). `WorkspaceLayout` keeps its
name and per-tab role but now wraps `PaneGroup` + `active: Option<PaneId>`
(the full `Option` audit across `Workspace` is T17-009; only the signature
lands here).

* **Splits are central-area only.** `PaneGroup` applies to the workspace
  tab content; **docks stay single-panel** (T17-002). No per-dock split tree.
* **Directions.** `PaneGroup::split` / `WorkspaceLayout::split` take a
  `SplitDirection` (`Up`/`Down`/`Left`/`Right`); `Workspace::split` too. The
  shell's two existing actions map to `Right` / `Down`; `Left` / `Up` are
  wired only through the API for now (user-facing bindings are T17-007's
  `CommandRegistry`).
* **Persistence.** `session.rs` gained `SerializedPaneGroup` (recursive serde
  enum, legacy `split` variant retained for read-only migration to `Axis`)
  and `SerializedLayout { root: Option<..>, active: Option<PaneId> }`.
  `WorkspaceTabSnapshot.layout` changed type from `WorkspaceLayout` to
  `SerializedLayout` but **kept its field name and nesting**, so
  pre-T17-004 `session.json` files still deserialise with **no
  `SNAPSHOT_VERSION` bump** (the new format is a superset).

### 8.8 `ModalLayer` / `ToastLayer` — T17-005

`labonair-workspace` gained `modal_layer.rs` (`trait ModalView: ManagedView`,
`struct ModalLayer` — one active, focus-trapping modal; `toggle_modal` /
`open_modal` / `hide_modal`; `on_focus_out` + `dismiss_on_focus_lost`) and
`toast_layer.rs` (`ToastLayer<Th: UiTheme>` — observes
`labonair_notifications::NotificationCenter` and renders `render_overlay`; the
stacking + `background_executor().timer()` auto-dismiss already lived in the
center). `AppShell::render` now ends with **exactly two** overlay children:
`.child(modal_layer).child(toast_layer)`. `pending_commands` /
`pending_bookmarks` / `drain_pending_commands` / `drain_pending_bookmarks` are
gone — palette + bookmark picks are serviced immediately from a
`cx.subscribe_in` set up in `AppShell::new` (which has the `&mut Window`).

Deviations from the literal task text, all accepted:

* **The three existing overlays keep their own scrim + centering.** The
  command palette, path-bookmarks popover and updater dialog each already
  paint a full-screen scrim, position their own card and handle their own
  `Esc` / overlay-click, so they set `ModalView::render_bare() == true`: the
  layer hosts them for lifecycle + focus identity only and renders them
  as-is. `ModalLayer::render` still implements the generic non-bare path
  (`modal_scrim()` backdrop with `occlude()` + `on_mouse_down` → `hide_modal`,
  a centered `occlude()`d panel with `track_focus`) for future modals — the
  `Cmd+F` search overlay (T18-002) is the first planned consumer. Stripping
  the palette's position-preference-aware scrim was judged higher-risk than
  keeping it, given no runtime verification is possible on this headless VPS.
* **`impl ModalView` lives on shell-local wrapper newtypes, not the views.**
  `labonair-command-palette` / `labonair-panel-explorer` cannot depend on
  `labonair-workspace` (cycle), and the orphan rule bars
  `impl ModalView for CommandPalette` anywhere else, so
  `crates/shell/src/app_shell.rs` defines `CommandPaletteModal`,
  `BookmarksModal`, `UpdaterModal` — thin `Render`+`Focusable`+
  `EventEmitter<DismissEvent>` wrappers that delegate to the inner entity.
  `CommandPalette` / `BookmarksView` gained `impl EventEmitter<DismissEvent>`
  and emit it from `close()` so the wrapper (and hence the layer) drops on a
  self-close.
* **Bookmarks + updater are driven modals; the palette is explicit.** The
  updater dialog (`dialog_open` flipped by the async check) and the bookmarks
  popover (its status-bar button flips `open` directly) are mirrored into the
  layer by `sync_updater_modal` / `sync_bookmarks_modal`, called from
  `render`. The palette, only ever opened from action handlers, uses
  `ModalLayer::open_modal` / `hide_modal` directly.
* No new crate-graph edges (shell already reaches workspace + notifications;
  workspace already reaches notifications + ui-kit).

Bar / breadcrumb context menus were **not** forced into the modal layer (per
the task): they remain `PopoverMenu`/`StatusItem`-local, pending T20-001.

### 8.9 `AppShell` → composition-only — T17-006

`crates/shell/src/app_shell.rs` dropped from 2 072 to **272 lines**. The
2 000-line startup body + action cascade split into shell submodules and one
`Workspace` addition:

* **`crate::bootstrap`** — the whole former `AppShell::new` body: builds every
  child entity, runs the ordered startup sequence (MCP-prefs hydrate → session
  snapshot → theme preference → `apply_prefs_to_theme` → keybinds → settings
  deps → updater check), wires the reactive edges, returns
  `AppShell::from_parts(…)`. The ~15 `cx.observe(&x, |_,_,cx| cx.notify())`
  lines collapsed to the handful that still do real work (`background`, `prefs`,
  `workspace` for `.when(can_split)` staleness, the two CWD-feed closures, the
  live-snapshot refresh, the `BarLayoutTick` global).
* **`crate::actions`** — every `act_*` handler + `build_palette_data` +
  `run_palette_command` + `handle_palette_event` / `handle_bookmark_event` as an
  `impl AppShell` block. The `.on_action(...)` registration list stays in
  `render`; only the bodies moved. `build_palette_data` is now snapshotted when
  the palette opens, not every frame. T17-007 replaces this module with a
  `CommandRegistry`.
* **`crate::titlebar`** — `Titlebar` entity: tab strip + transient inline search
  + `⋯` app-menu, ported verbatim from `render_header` / `render_app_menu` /
  `render_search`. Owns its own `zen_mode_show_header` reactivity (renders an
  empty element when hidden) and its search state. The redesign is T18-001.
* **`crate::modals`** — the three `ModalView` wrapper newtypes (moved out of
  `app_shell.rs`, unchanged).
* **`Workspace::render_dock` + `Workspace::render` (`labonair-workspace`)** — the
  three edge docks, the `DockResize` drag payload and the drag-to-resize handler
  moved off `AppShell` into `Workspace`, which now composes
  `[left dock | (pane group + bottom dock) | right dock]` itself (Zed's
  `Workspace::render` model). The shell just does `.child(workspace.clone())`.
  `set_dock_size` / `move_panel_persist` moved with it. **No new crate edge**
  (87 internal edges, unchanged).
* **`drain_pending_ai` / `sync_live_bridge` deleted.** AI "run in terminal" is
  serviced straight from a `cx.subscribe_in(&ai_chat, window, …)` in
  `bootstrap` (no `pending_ai` buffer). The `WorkspaceLiveBridge` **snapshot**
  is refreshed event-driven via `cx.observe` on the workspace + explorer
  (`bootstrap::refresh_live_snapshot`); the bridge's **command queue** is
  drained by a light `cx.spawn` + `background_executor().timer(120 ms)` loop
  (`AppShell::_live_drain`) applying each via `Workspace::apply_live_command` —
  the same async→main idiom `Workspace` already uses for its SSH / transfer
  bridges, and no longer a per-frame `render` call.

**Deviation from the task's "≤ 8 fields / panel entities move to `Workspace`"
acceptance criterion (§8.4 wins).** `struct AppShell` keeps **13 fields**; the
eight concrete panel / feature entities (`explorer`, `bookmarks`, `git_panel`,
`snippets`, `ai_chat`, `updater`, `command_palette`, `prefs`) stay in
`labonair-shell` — grouped in `ShellPanels` — because
`labonair-panel-{explorer,scm,snippets,ai}` already depend on
`labonair-workspace` (§8.4), so storing their concrete `Entity<…>` on
`Workspace` would be a dependency cycle, and `PreferencesStore` /
`CommandPalette<PreferencesStore, …>` / `UpdaterView` cannot even be named from
`labonair-workspace`. `docs/architecture.md` §8.4 explicitly says "`AppShell`
keeps `self.bookmarks: Entity<BookmarksView>`" — the crate boundary is the only
thing that could move, and it can't here. A full panel↔workspace dependency
inversion (a new `labonair-prefs` contracts crate + registry `build` closures
that let `Workspace` own the panels type-erased) is a candidate future task,
**not** T17-006. `AppShell::new` itself has exactly one `cx.observe` (theme);
the functional observes live in `bootstrap`. `render` also carries the
pre-existing full-window `background.layer(App)` wallpaper overlay as a child
(not feature logic) alongside Titlebar / Workspace / StatusBar / ModalLayer /
ToastLayer.

### 8.10 `CommandRegistry` — T17-007

The ~50-entry `.on_action(cx.listener(Self::act_*))` chain on the shell root
plus the parallel `run_palette_command` match are gone. Every command is now
one [`CommandRegistry::register`] call in
`crate::commands::register_builtin_commands` (the single definition site); the
native menu bar, the key bindings and the command palette all dispatch the same
`labonair_command_palette::CommandId` through `AppShell::dispatch_command` →
`registry.run_for(id)`.

* **`crate::commands`** (new, `crates/shell/src/commands.rs`) — `CommandRegistry`
  (`register` / `run_for` / `iter` / `visible_in`), `register_builtin_commands`
  (all former `act_*` bodies as `CommandFn` closures over `&mut AppShell`), and
  `attach_action_handlers` — the Action → `CommandId` bridge that puts one
  `.on_action` per `menu::` action on the shell root (context-gated
  `SplitRight/Down` + `ClosePane` unchanged). `app_shell.rs::render` keeps only
  **3** genuine window `.on_action`s (`ToggleFullScreen`, `Minimize`,
  `ZoomWindow`) + one `attach_action_handlers(root, …)` call.
* **`crate::actions`** slimmed to: the 3 window actions, the `AppShell` helper
  methods the command closures call (`select_panel`, `toggle_zen_pref`,
  `show_command_palette`, …), the modal-layer mirrors, `handle_palette_event`
  (its `Run(id)` arm is now `self.dispatch_command(id, …)`),
  `handle_bookmark_event`, and a **slimmed** `build_palette_data`.

**Deviation 1 — registry lives in `labonair-shell`, not
`labonair-command-palette` / a new `labonair-commands` crate.** A `CommandFn`
body needs `&mut AppShell` to reach the shell-owned panel / feature entities
(`panels.ai_chat`, `panels.updater`, `panels.command_palette`, `titlebar`, …)
that §8.4 / §8.9 keep out of `Workspace` on a crate-cycle argument. `AppShell`
is only nameable in `labonair-shell`, so `CommandFn` and the registry are too.
Palette and keymap still *share* the registry: the shared vocabulary is the
`CommandId` enum owned by `labonair-command-palette`. **No new crate edge** (87
internal edges, unchanged).

**Deviation 2 — `build_palette_data` is slimmed, not deleted.** The
pref/theme-derived scalars (`color_mode`, `editor_theme`, `terminal_font_size`,
the nine `Toggle: …` bools) plus the user keybind overrides moved to
`PalettePrefs` trait reads — the palette holds `Entity<PreferencesStore>` and
reads them itself, and renders shortcut hints through `effective_keys`
(override-aware). What still flows through `CommandPalette::set_data` at
palette-open is the genuinely panel-/workspace-/settings-sourced choice lists
(`snippet_choices`, `session_choices`, `branch_choices`, `known_hosts`,
`recent_hosts`, `active_editor_symbols`, `theme_choices`): the palette crate
cannot pull those without a `labonair-panel-* → labonair-command-palette`
back-edge (cycle) or a `labonair-settings-ui` dependency. `PaletteData` shrank
from 12 fields to 7.

**Deviation 3 — placeholder `CommandId`s stay unregistered.** `ZoomIn` /
`ZoomOut` / `ZoomReset` / `OpenShortcuts` / `FormatDocument` and every palette
sub-page navigator id have no `register` call; `dispatch_command` no-ops for
them — byte-for-byte the pre-T17-007 behaviour (they were no-op `dispatch_action`
calls). Their wiring lands with the phases that implement them.

---

### 8.11 `AppEvent` bus kept + `BackendEventBridge` — T17-008

Decision recorded in full in [`docs/adr/0002-app-event-bus.md`](adr/0002-app-event-bus.md):
**keep** `labonair_backend::EventBus` / `AppEvent`, connect it through one
GPUI-side entity.

* **`labonair_workspace::backend_event_bridge::BackendEventBridge`** (new) is the
  single foreground subscriber to `backend.events`. One `cx.spawn` loop
  (`tokio::sync::broadcast::Receiver::recv` is runtime-agnostic) decodes each
  `RawEvent` → `TransferBusEvent` / `AppEvent` and pushes it straight into the
  `Workspace` entity via `entity.update`. `Lagged` → warn + resync; `Closed` /
  workspace-dropped → stop. **No `tokio::spawn` + `mpsc` + poll-drain hop.**
* `Workspace` gains `apply_transfer_bus_event`; `handle_ssh_event` is now
  `pub(crate)`. The former 40 ms `ssh_poll` loop keeps only
  `refresh_active_tunnels` (a genuine state poll).
* **Reference consumer:** SFTP transfer progress (`transfer_progress` /
  `transfer_step` / `transfer_completed` → `TransfersView::apply`) is
  event-driven end-to-end through the bridge; the UI never polls it.
* `spawn_event_logger` (`crates/app/src/main.rs`) is now `#[cfg(debug_assertions)]`
  only — a developer trace, not a product path.
* **No new crate edge** — the bridge is inside `labonair-workspace`, which
  already depends on `labonair-backend` (87 internal edges, unchanged, acyclic).
* **Follow-ups (not in T17-008):** `fs:dir-changed` → explorer auto-refresh;
  git-status change → scm auto-refresh; move `panel-snippets`' run-log
  subscription onto the same foreground pattern; route `menu:activated` through
  `AppShell::dispatch_command`.

### 8.12 Tabs optional — realised in T17-009

Implements §8.2. Notes / small deviations:

* **`TabKind::Home` → `TabKind::Hosts`.** The un-closable landing tab is gone.
  The host-manager dashboard is now a normal, closable, non-persisted
  `TabKind::Hosts` tab opened on demand via `open_host_manager` (menu /
  `CommandId::OpenHostManager` / `＋▾` "All hosts…"). T19-010 removes this tab
  kind and moves the entry point to Settings.
* **`TabStore` close rules.** `close` dropped the `len() <= 1` guard and the
  `Home` special-case; `close_others` / `close_by_kind` may reach zero; new
  `close_all`. `activate_fallback` resets `active_id` to `0` (no active tab)
  when the store empties.
* **`Option<ActiveTab>` audit.** `TabStore::active()` already returned
  `Option`; every `Workspace` call site already used `?` / `let Some` /
  `unwrap_or_default`, so no signature changes were needed — the audit was a
  read-through confirmation, not a refactor. Tab-dependent actions at zero tabs
  are clean no-ops (`split_active`, `search_active`, `duplicate_active_tab`,
  breadcrumb `send_cd`, …); `WorkspaceLiveBridge` reports `has_terminal: false`
  / `None` fields (unchanged default path).
* **Startup.** `Workspace::new`: a passed-in snapshot is replayed verbatim
  (zero restored tabs → stay empty); with no snapshot the `startup_tab` pref
  decides — `terminal` → one local terminal, `empty` → nothing. No automatic
  tab is ever opened now.
* **`StartupTab`.** `HostManager` variant replaced by `Empty` (the new
  `#[default]`); `#[serde(alias = "host-manager")]` migrates the old value.
  Settings-UI select is `["terminal", "empty"]`. Truly unknown enum strings
  still fall back through the existing whole-`Preferences`→`Default` path
  (pre-existing coarse behaviour, not changed here).
* **Legacy session snapshots.** `TabSnapshot::Home` / `RestoreAction::Home` are
  kept **deserialise-only** (removing the variant would fail the whole
  `serde_json::from_str` and lose the session). `plan_restore` still emits
  `RestoreAction::Home`; the executor now drops it silently (no tab, not
  counted as restored, not a `failed` entry).
* **Empty surface.** `Workspace::render_empty_surface` — minimal centred hint
  (`No tabs open · ⌘T new terminal · ⌘K commands`) with a
  double-click→`new_terminal_tab` handler. Styled version + `＋▾` menu +
  file-drop are T18-001.
* **Test-harness deviation.** The task asked for a "render builds at zero tabs"
  + "sweep every public `Workspace` action at zero tabs" test. `labonair-workspace`
  has **no** `Workspace` test harness (constructing one needs a live `Backend`,
  `TokioHandle`, `TerminalRegistry`, `Window`), and test binaries cannot link
  on this headless VPS anyway. Coverage is instead: `TabStore` unit tests
  (close-last-tab-empties, `close_all`), `session.rs` zero-tab disk round-trip
  + legacy-`home` deserialise, `preferences.rs` `host-manager`→`Empty`
  migration. The zero-tab render path is exercised structurally
  (`render_content` `None` arm → `render_empty_surface`, no `unwrap`).

---

## 9. Ist-Graph after Phase 15 (T16-010)

Regenerate with [`scripts/gen-crate-graph.sh`](../scripts/gen-crate-graph.sh)
(writes `docs/assets/crate-graph.dot` + `.svg` and prints this list).

![Crate graph](./assets/crate-graph.svg)

<!-- generated by scripts/gen-crate-graph.sh — do not edit by hand -->
20 workspace crates, 81 internal edges, 8 tiers.

- `labonair` → `labonair-ai`, `labonair-backend`, `labonair-editor`, `labonair-shell`, `labonair-terminal`, `labonair-theme`
- `labonair-ai` → `labonair-backend`
- `labonair-backend` → *(leaf)*
- `labonair-command-palette` → `labonair-backend`, `labonair-gpui-ext`, `labonair-theme`, `labonair-ui-kit`
- `labonair-editor` → *(leaf)*
- `labonair-gpui-ext` → *(leaf)*
- `labonair-hosts-ui` → `labonair-backend`, `labonair-notifications`, `labonair-theme`, `labonair-ui-kit`
- `labonair-notifications` → `labonair-gpui-ext`, `labonair-theme`, `labonair-ui-kit`
- `labonair-panel` → `labonair-gpui-ext`
- `labonair-panel-ai` → `labonair-ai`, `labonair-backend`, `labonair-command-palette`, `labonair-editor`, `labonair-theme`, `labonair-ui-kit`, `labonair-workspace`
- `labonair-panel-explorer` → `labonair-backend`, `labonair-notifications`, `labonair-theme`, `labonair-ui-kit`, `labonair-workspace`
- `labonair-panel-git-graph` → `labonair-backend`, `labonair-notifications`, `labonair-theme`, `labonair-ui-kit`
- `labonair-panel-scm` → `labonair-backend`, `labonair-notifications`, `labonair-theme`, `labonair-ui-kit`
- `labonair-panel-snippets` → `labonair-backend`, `labonair-notifications`, `labonair-theme`, `labonair-ui-kit`, `labonair-workspace`
- `labonair-settings-ui` → `labonair-ai`, `labonair-backend`, `labonair-command-palette`, `labonair-gpui-ext`, `labonair-notifications`, `labonair-theme`, `labonair-ui-kit`, `labonair-workspace`
- `labonair-shell` → `labonair-backend`, `labonair-command-palette`, `labonair-gpui-ext`, `labonair-notifications`, `labonair-panel-ai`, `labonair-panel-explorer`, `labonair-panel-git-graph`, `labonair-panel-scm`, `labonair-panel-snippets`, `labonair-settings-ui`, `labonair-terminal`, `labonair-theme`, `labonair-ui-kit`, `labonair-workspace`
- `labonair-terminal` → `labonair-theme`
- `labonair-theme` → *(leaf)*
- `labonair-ui-kit` → `labonair-gpui-ext`, `labonair-theme`
- `labonair-workspace` → `labonair-ai`, `labonair-backend`, `labonair-command-palette`, `labonair-editor`, `labonair-gpui-ext`, `labonair-hosts-ui`, `labonair-notifications`, `labonair-panel`, `labonair-panel-git-graph`, `labonair-terminal`, `labonair-theme`, `labonair-ui-kit`

`labonair-settings-content` / `labonair-settings` (§2, settings track) are not
yet created — they land in Phase 18. `labonair-panel` is present as the
contracts crate but is not yet consumed by any panel (`Panel` trait wiring is
T17-001).
