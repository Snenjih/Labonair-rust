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
  * **Extensibility.** The bar renders purely from `StatusItemRegistry`
    (`labonair-panel`), keyed by an arbitrary `&'static str` id + an `Arc`
    constructor — it has *no* dependency on the panel system.
    `labonair-shell::register_builtin_status_items` is just the built-in list;
    any crate can register its own `StatusItem` entity on the workspace's
    registry to add a widget. The per-dock panel-button groups
    (`dock-buttons-*`) are the only status items that touch `PanelRegistry`.
  * **One right-click menu per item.** `StatusBar` owns the single context menu:
    the shared "Move left / Move right / Hide" block, with the item's own rows
    (from `StatusItem::status_menu_entries`) merged in above it. A widget with
    bespoke actions (the CWD breadcrumb's "Copy path", …) contributes through
    that hook instead of opening a competing menu of its own.
  * **Dock-button menu.** Right-clicking a panel toggle lists every dock the
    panel supports as a checkable row (current dock ticked → flip directly),
    then "Hide Button".
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
  Titlebar-scoped items map to the statusbar default (migrator T18-006, §8.6).
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

#### Deviation (UI-optimization pass, post-T21): sidebar sub-navigation

`docs/settings-guidelines.md` rule 1 as written puts *only* top-level
categories in the left rail and makes the right-hand page's section headers
**user-collapsible disclosures** with a scroll-spy jump bar. The
UI-optimization pass replaces that with:

* **Left rail is two levels.** Each top-level category row carries a
  disclosure chevron; expanding it lists that category's section labels as
  **sub-level entries**. Clicking a sub-level entry navigates to the
  category (if needed) and scrolls the content area to that section. Sub-level
  entries are scroll anchors, **not** pages/sub-pages — the content shown is
  still wholly determined by the top-level category.
* **Section headers are static.** The right-hand page still renders the same
  curated section headers (e.g. "Typography"), but they are plain labels with
  a hairline — no disclosure chevron, no collapse. The scroll-spy jump-bar
  chip row (`render_jump_bar`) is removed entirely.
* **Header chrome trimmed.** The settings window header is no longer a
  toolbar: no in-window close button (the OS traffic lights close it), left
  edge padded clear of the traffic lights, and the single "Edit in
  config.json" action moved to the right edge.
* **Sidebar surface.** The rail is painted on the `--sidebar` token, distinct
  from the `--card` content area.
* **Schema warnings suppressed.** Unknown-/legacy-key schema *warnings* are no
  longer shown as a top-of-page banner (only hard type/enum errors are).
  `TODO(notification-system)` in `crates/settings-ui/src/view.rs` marks where
  they should re-surface as dismissible notifications.

Rationale: the flat rail + in-page disclosures did not scale visually once
categories grew several sections deep, and the jump-bar chip row read as
stray buttons at the top of every page. Rules 2–9 of the contract are
unaffected (typed fields, generated UI, custom-pane chrome, origin+reset,
global search, deep links). Scope: `crates/settings-ui/*`,
`crates/ui-kit/src/palette.rs` (new `sidebar*` tokens).

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

### 8.6 `BarLoc` / bar-item blob — resolved in T18-005 / T18-006

T17-003 replaced the shell's `render_bar_item` `match` (plus
`render_simple_bar_button`, every `render_*_item`, `render_bar_menu`,
`build_bar_bucket`, `move_bar_item`, `persist_placement`,
`panel_for_item`/`item_for_panel`) with a `StatusItemRegistry` (a `Workspace`
field, mirroring `PanelRegistry` from T17-001), self-describing `StatusItem`
views in `crates/shell/src/status_items.rs`, and a `StatusBar` component in
`labonair-workspace` (`status_bar.rs`) that renders **only** from the registry
(sorted per side by `order`). Header/titlebar carries no bar items.

The task's AC1 also asked to delete `BarLoc`. That was completed across two
follow-up tasks: **T18-005** deleted `BarItemId`/`BarLoc`/`BarSide`/
`Placements`/`BAR_ITEM_ORDER`/`BarLayoutTick` (`labonair-workspace::bar_items`)
and the `labonair-settings-ui` titlebar/statusbar bar-item layout editor that
had been their only remaining consumer, replacing them with
`labonair-workspace::status_placements` (`StatusPlacement { side, hidden }`,
a `StatusBarLayoutTick` global for cross-window sync) and a right-click
per-item context menu (move left/right, hide) on the new `StatusBar`. **T18-006**
then added `labonair_backend::modules::settings::migrations::migrate_bar_item_placements`,
a one-time idempotent migrator (run from `crates/shell/src/bootstrap.rs`,
before the first `StatusItemRegistry` build) that reads the legacy
`barItemPlacements` blob (`{ itemId: { itemId, bar: titlebar|statusbar, side,
hidden } }`) and writes the new `statusBarItemPlacements` blob (`{ itemId:
{ side, hidden } }`), renaming the old key to `barItemPlacements_legacy` as a
safety net and `.bak`-ing the whole settings file first. Old ids are
camelCase (the pre-T18 `BarItemId` enum's serde rename); new ids are the
kebab-case strings `StatusItem::id()` returns in `status_items.rs` — several
were renamed along the way, so this is an explicit id-remap table, not a
same-name carry-over:

| Old id | New id | Outcome |
|---|---|---|
| `agentAccess` | `agent-access` | kept, `side`/`hidden` carried over |
| `jumpHosts` | `jump-hosts` | kept |
| `cwdBreadcrumb` | `cwd` | kept (renamed) |
| `previewUrl` | `preview-url` | kept |
| `cursorPosition` | `cursor-position` | kept |
| `bookmarks`, `notifications`, `transfers`, `updater` | unchanged | kept |
| `ai`, `aiMini`, `aiPanel` | — | dropped — AI is a panel toggle now, not placeable |
| `explorerPanel`, `snippetsPanel`, `sourceControlPanel`, `tabsPanel` | — | dropped — panel toggles are fixed-left, not placeable; Tabs is a sidebar panel now, not a status item |
| any other/unrecognised id, or an entry with no `side` | — | dropped (falls back to the item's compiled-in `default_side`) |

An old `bar: titlebar` entry gets the same treatment as `bar: statusbar` —
`bar` is ignored, every surviving item just lands in the statusbar (the only
bar that exists now).

Dock-layout persistence moved off `AppShell` onto `Workspace` (a
`set_dock_persist_hook` callback, since `labonair-workspace` cannot depend on
`labonair-settings-ui`'s `PreferencesStore`). No new crate edges (`shell`/
`workspace` → `panel`, `shell` → `workspace`, `shell` → `backend` all already
existed).

**T18-007** re-added a settings-ui layout editor — deliberately *not* a
resurrection of the deleted `BarLoc` machinery above. The new "Personalization"
category (`crates/settings-ui/src/panes/personalization.rs`) is a thin
GUI over the exact same `Workspace` methods the in-app right-click menus call:
`set_status_bar_placement` (already existed, T18-005) and the new
`set_panel_toggle_visible` / `reset_status_bar_placements`. `labonair-settings-ui`
gained a direct `labonair-panel` dependency (to name `StatusSide`/`PanelIcon`/
`DockPosition`) and now holds the app's `Entity<Workspace>` (`SettingsDeps`),
published from `crates/shell/src/bootstrap.rs` alongside the existing
`prefs`/`backend`/`tokio` deps — no back-edge, `labonair-workspace` still
doesn't know `labonair-settings-ui` exists. A new `panelToggleVisibility`
blob (`{ panelName: bool }`, mirroring `statusBarItemPlacements`'s
read-merge-write pattern in `labonair_backend::modules::settings`) persists
which panels get a status-bar toggle; `PanelTogglesStatusItem` (T18-003) now
reads it and reloads on the same `StatusBarLayoutTick` global instead of the
session-only `HashSet` it used before this task.

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

### 8.13 Titlebar redesign — T18-001

Implements the §4 layout contract for the top chrome. Notes / deviations:

* **Removal targets had already moved.** The task text points at
  `app_shell.rs::render_header` (line ~1334), but T17-006 (§8.9) already
  extracted `render_header` / `render_app_menu` / `render_search` verbatim into
  `crates/shell/src/titlebar.rs`. So T18-001's removals happened in
  `titlebar.rs`, not `app_shell.rs`. `render_agent_badge` and the
  `agent_badge_open` / `app_menu_open` fields never existed in the post-T17-006
  code (the agent badge became a `StatusItem` in T17-003); nothing to delete.
* **`titlebar.rs` now = tab strip + one right-hand icon button.** The `⋯`
  app-menu (`render_app_menu`) is gone; the right button (`IconName::Ellipsis`
  — the bundle has no `Settings2` / `CircleUser` glyph; the task allowed a
  fallback) opens a hand-rolled dropdown (`absolute` under the button, same
  pattern the old app-menu used — `context_menu`'s full-screen overlay can't
  render from the 40 px titlebar container) with `Settings…` (→
  `open_settings_window(None)`) and `Profile` (placeholder → "coming soon"
  toast via `notification_center`; the `labonair-notifications → labonair-shell`
  edge already exists, no new crate edge).
* **`＋` new-tab menu was already there.** `Workspace::render_tab_bar` /
  `render_new_tab_menu` (built in the T17-009 groundwork) already port
  `NewTabDropdownItems` — Terminal / Editor / Preview / Git Graph, separator,
  `SSH` / `SFTP` recent-host rows (injected via `Workspace::recent_hosts`, no
  `titlebar → hosts-ui` edge), "All hosts…" → `open_host_manager`. No separate
  `▾` split-button: the whole `＋` opens the menu, `⌘T` is the quick action
  (task explicitly allowed this: "ganzer Button = Menü, ⌘T bleibt die
  Schnellaktion").
* **Inline search kept as a provisional floating overlay.** Instruction 2 says
  to keep the old search until T18-002 ships the real overlay. `render_search`
  is retained but now rendered `absolute` just below the titlebar (not an
  inline flex child), with `open_search` still driving the `⌘F` fallback from
  `commands.rs`. T18-002 removes it and the `search_*` fields.
* **Window chrome.** Titlebar root is `.window_control_area(Drag)` +
  `should_move` latch (`on_mouse_down` set → first `on_mouse_move` →
  `window.start_window_move()` → `on_mouse_up` clear), and `on_click` with
  `click_count == 2` → `window.titlebar_double_click()` — the Zed
  `platform_title_bar` mechanism. Interactive children consume their own
  clicks. `TRAFFIC_LIGHT_INSET` is now `#[cfg]`-split: 78 px on macOS, 8 px
  elsewhere (Linux has no traffic lights).
* **Empty surface final look.** `Workspace::render_empty_surface` — centred
  "Labonair" wordmark over a column of shortcut-hint rows (`⌘T` New Terminal,
  `⌘E` Editor, `⌘K` Commands, `⌘,` Settings, `⌘⇧N` Hosts), muted-foreground +
  bordered key chips. Double-click anywhere → `new_terminal_tab`; `on_drop`
  `ExternalPaths` → one `open_file(path, false, …)` per dropped file. No own
  state.
* **Not verifiable here.** Instruction 7 and several acceptance rows need a
  `cargo run` on macOS (traffic-light inset, live window-drag, double-click
  zoom, dropdown visuals, file-drop). This is a headless Linux VPS — not done;
  gates were the `cargo check/clippy --all-targets` + `check-crate-deps.sh`
  substitute (`cargo test` can't link here).

---

### 8.14 `Cmd+F` search overlay — T18-002

**Focus-trap decision (instruction 1).** No new `OverlayLayer` and no
`ModalView::traps_focus()` extension: [`SearchOverlay`]
(`crates/workspace/src/search_overlay.rs`) is a plain **bare**
[`ModalView`](../crates/workspace/src/modal_layer.rs) (`render_bare() ==
true`). `ModalLayer`'s bare path already renders the view directly with no
scrim and no `occlude()` wrapper — only `open_modal`/`toggle_modal` move
keyboard focus into it. Since the overlay itself doesn't `occlude()` its
container either (its `absolute` box only covers its own small footprint),
mouse wheel / drag on the rest of the active tab keeps working while the
overlay has keyboard focus. This is the same mechanism the command palette /
bookmarks popover already use for their own full-screen paint; the overlay
just doesn't paint a scrim.

**Scope widening vs. the task text (user decision, 2026-09-04).** The task
allowed keeping the editor's own find bar and a "current match only, no
scrollback" terminal search as acceptable minimums. The user asked for the
fuller version instead:

* **The overlay is the *only* find UI, including for editors.** The editor's
  old in-buffer find bar (`FindBar`, `render_find_bar`, its own `Cmd+F`
  binding, Tab-to-replace-field, Replace/Replace-All) was deleted from
  `crates/workspace/src/views/editor.rs`, not kept alongside the overlay.
  `EditorView` now exposes a minimal `EditorSearch` state (query + matches +
  active index, no replace) through `search_set` / `search_step` /
  `search_close` / `search_seed`; the active match is still shown as the
  ordinary editor selection (unchanged rendering path). `Document::replace_all`
  itself is untouched and still backs the vim `:s` command.
* **Terminal search is real scrollback search, not "first visible match".**
  `TerminalEmulator` (`crates/terminal/src/engine.rs`) gained literal search
  built on `alacritty_terminal::term::search::{RegexSearch, RegexIter}` over
  the whole buffer (`grid().topmost_line()..grid().bottommost_line()`), with
  `search_set` / `search_step` / `search_clear` / `search_count`. The query is
  escaped into a literal regex-automata pattern; **an explicit `(?-i)` /
  `(?i)` prefix is required** because `RegexSearch::new` derives its own
  case-(in)sensitivity from whether the *pattern text* contains an uppercase
  character (alacritty's built-in smart-case) — without the explicit prefix an
  all-lowercase query silently ignores an explicit case-sensitive request (hit
  in `search_is_case_sensitive_when_requested`, fixed before landing). The
  active match is mirrored into `term.selection` (so it paints through the
  existing selection-span code for free and `scroll_to_point` keeps it on
  screen); the other matches are exposed as a second `RenderableScreen::search`
  span list and painted by `TerminalView` in a dim overlay color between the
  cell runs and the selection layer. `SessionAccess` (`crates/terminal/src/
  session.rs`) grew the three methods so both local (`TerminalSession`) and
  SSH-backed (`RemoteSession`) sessions search identically.
* **Routing.** `Workspace::active_search_target` / `search_set` / `search_step`
  / `search_end` (`crates/workspace/src/workspace.rs`) pick editor vs. terminal
  vs. `SearchTarget::Unavailable` by tab kind (`self.editors` map vs.
  `active_pane_view`) — SFTP/git-graph/host-manager tabs get the "not
  available" message the task asked for, no crash.
* **Pre-fill without select-all.** `SearchOverlay` seeds the input from the
  editor selection or the last query via `InputState::default_value`, but does
  not select it — `InputState::select_all` (`gpui-component` 0.5.1) is
  `pub(super)`, not reachable from outside the crate, and there's no `SelectAll`
  action re-export to dispatch instead. The seed still runs the initial search
  immediately (so reopening the overlay shows results right away); a user who
  wants to replace the seed has to clear it manually first. Minor polish gap,
  not a correctness one.
* **Last query.** Kept as a single process-lifetime `static LAST_QUERY:
  Mutex<String>` in `search_overlay.rs` rather than on `Workspace` /
  `PreferencesStore` — the task only asked for it to survive across overlay
  opens within a session, not across app restarts, so no persistence layer was
  added.
* **Not verified with `cargo run`.** Same headless-VPS caveat as §8.13; typing
  / highlight / count / next-prev / Esc-close were exercised through the new
  `crates/terminal/src/engine.rs` unit tests (`search_set_counts_matches_and_selects_one`,
  `search_step_wraps_and_updates_the_selection`, `search_is_case_sensitive_when_requested`,
  `search_finds_matches_in_scrollback_history`, `search_clear_drops_matches_and_selection`,
  `empty_query_clears_search`) and the adjusted `views::editor::tests::find_navigates_matches`,
  not a live window.

### 8.15 `labonair-settings-content` / `labonair-settings-macros` — T19-001

Both new leaf crates land as specified in §2's settings track:
`labonair-settings-content` (the typed `SettingsContent` tree — `general`,
`appearance`, `terminal`, `editor`, `fileManager`, `connections`, `hosts`,
`workspace`, `ai`, `mcp`, `personalization`, `keymap` — plus `MergeFrom`, the
`AREAS` category registry, and the fault-tolerant `parse`) and
`labonair-settings-macros` (`#[derive(MergeFrom)]`, used only inside
`labonair-settings-content` itself). Neither depends on GPUI, any UI crate, or
`labonair-backend`.

**Deviation:** `labonair-backend` gains a new outgoing edge to
`labonair-settings-content` (§9's "`labonair-backend` → *(leaf)*" line no
longer holds). T19-001 asks for `impl From<&SettingsContent> for Preferences`
(`crates/backend/src/modules/settings/content_bridge.rs`) so every existing
call site that reads `Preferences` keeps working unchanged until `T19-002`
swaps the runtime store over. That impl needs both types in scope; putting it
in `labonair-settings-content` instead would require that crate to depend on
`labonair-backend` (forbidden — it must stay backend-agnostic, `T19-001`'s own
Anweisungen #1), so `labonair-backend` depends on `labonair-settings-content`
instead. This is a one-way edge (`labonair-settings-content` never depends
back), so no cycle; `labonair-ai`, `labonair-workspace`, and everything else
that already reaches `labonair-backend` picks it up transitively without any
edge of their own changing. `hosts.entries` in the new tree is a fresh,
non-secret-only model (`HostAuthMethod`/`HostEntry`/`HostTunnel`) rather than
a reuse of `labonair-backend::modules::hosts::db::Host` — that SQLite row
stays the authoritative runtime store; reconciling the two is `T19-010`'s
concern, not this task's.

### 8.16 UI-Kit primitive set — T20-001

**Inventur.** Grepping the panel / workspace / settings crates for repeated
hand-rolled `div()` shapes turned up nine recurring patterns. Each one is now a
`labonair-ui-kit` primitive; the "call sites" column lists what the inventory
found (✓ = migrated in T20-001 as the proof-of-API call site, the rest follow
in T20-002/T20-003).

| Hand-rolled pattern found | Primitive | Call sites found in the inventory |
|---|---|---|
| `div().h(px(1.0)).bg(border)` / `.w(px(1.0))` 1px rules | `divider(Axis, Hsla)` | `panel-ai` `MdBlock::Rule` ✓, `views/preview.rs` `MdBlock::Rule` ✓, `views/sftp.rs` splitter ✓, `shell/titlebar.rs`, `views/diff.rs`, `status_bar.rs` |
| "section heading with a `▸`/`▾` arrow that collapses its group" | `disclosure(..)` | `settings-ui/panes/generic.rs::render_section_header` ✓, `panel-explorer` tree rows |
| "muted group heading + rows of icon/label/trailing" | `list_header` / `ListItem` / `list_separator` | `settings-ui/view.rs::render_search_results` ✓, `command-palette` result rows, `hosts-ui` host list, `settings-ui` dropdown options |
| "button opens an absolutely-positioned card of actions" | `popover_menu(..)` (anchored sibling of `context_menu`) | `shell/titlebar.rs::render_account_menu` ✓, statusbar dock menus (already on `context_menu`) |
| "`−` / value / `+` stepper over a bounded number" | `number_field(..)` (+ pure `step_value`) | `settings-ui` `FieldControl::Int` ✓ and `FieldControl::Float` ✓ (the private `step_btn` + `slider_track` helpers are gone) |
| "trigger showing the current option + an anchored option list" | `select_trigger` / `select_popover` (+ pure `selected_label`) | `settings-ui` `FieldControl::Select` ✓, `FieldControl::FontFamily` ✓, `render_dropdown` ✓ |
| "`SquareCheck`/`Square` icon pair in a selectable row" | `checkbox(..)` | `hosts-ui` SSH-config import list ✓, `hosts-ui` host export list ✓ |
| "row of bordered pills, one of them active" | `segmented_control(..)` | `settings-ui` themes Installed/Community ✓, `settings-ui` theme variant picker ✓, `panel-ai` ModelPicker All/Favorites/Recent |
| "icon button with a sticky pressed fill" | `icon_toggle_button` / `toggle_base` | `shell/status_items.rs` panel toggles ✓, `panel-ai` AI/Shell composer toggle ✓, `panel-snippets` run-log tabs |
| "small coloured status dot" | `indicator(IndicatorSize, Hsla)` | `hosts-ui` host reachability ✓, `workspace` connection-log dot ✓, `workspace` unsaved-tab dot ✓, statusbar badges |
| "full-width tinted info/warn/error strip" | `banner(Severity, Palette)` | `settings-ui` JSON syntax-error banner ✓, `settings-ui` schema-validation banner ✓, `views/editor.rs` conflict banner, `panel-explorer` clipboard strip |
| "bordered key chip, sometimes with a label" | `kbd` / `kbd_row` / `keybinding_hint`, plus `MenuItem::keybind` | `command-palette` result keys ✓ + footer hints ✓, `workspace` tab context menu ✓, `shell/titlebar.rs` account menu ✓, `settings-ui` Shortcuts pane |
| `div().flex().flex_col()` / `.flex_row().items_center()` | `v_stack()` / `h_stack()` | ~400 occurrences workspace-wide; ✓ in `settings-ui/panes/generic.rs` and `workspace.rs` |

**`Palette` — the one styling parameter.** Every primitive is styled from
`labonair_ui_kit::Palette`, a `Copy` snapshot of the `labonair-theme` tokens
built once per render with `Palette::from_theme(theme)`. It exists because a
view's `render` cannot hold a `&ThemeStore` borrow across `cx.listener(..)`,
which is why `settings-ui` and `hosts-ui` each already had a private `Palette`
struct with the same six fields — both are now the shared one. `divider` and
`indicator` still take a bare `Hsla`, since a whole palette for a one-colour
line would be noise. `button`, `context_menu` and `popover` were switched from
`&impl UiTheme` to `Palette` so the whole crate has a single convention.

**`gpui-component` is not wrapped for these.** 0.5.1 ships `checkbox`,
`divider`, `kbd`, `select`, `tab`, … but every one of them styles itself from
*its own* `cx.theme()` global, which the app never syncs to `labonair-theme` —
wrapping them would silently bypass our tokens (Critical Rule 3). It stays in
use only where the behaviour is the hard part and the colours are incidental:
`InputState`/`Input` (caret, selection, IME, undo), `Badge`, `Switch`,
`Tooltip`.

**Deliberately not built (no ≥2 real call sites — T20-001 Notizen).**
* `Table` — the host list and the transfer queue are both card/row layouts with
  per-row actions, not column-aligned grids. No call site wanted a table.
* `Tab` / `TabBar` — the only real tab *bar* is `Workspace::render_tab_bar`
  (drag-reorder, close buttons, kind indicators, per-tab context menus); the
  other "tab" strips (settings themes, ModelPicker) are segmented controls and
  are covered by `SegmentedControl`. Extracting a `TabBar` from one bespoke
  call site would be speculative — it is folded into T20-002's workspace
  migration instead.
* `ToggleButton` (labelled convenience) and `Indicator::outline` — dropped
  during implementation once the inventory showed no second call site;
  `toggle_base(..).child(..)` covers the labelled case.

### 8.17 Component gallery — T20-004

`crates/ui-kit/src/gallery.rs` is a hand-maintained page (`struct Gallery:
Render`) that renders every primitive across its variants / sizes / states
plus a few realistic compositions, with a live **System / Light / Dark**
switch at the top that flips the shared `ThemeStore` preference so every
sample re-renders. It is the fastest way to answer "does this still look like
the reference?" after a T20-002/003 migration.

* **Access:** `cargo run` (debug), then command palette →
  *Debug: Open Component Gallery* (`CommandId::OpenComponentGallery`). It opens
  its **own** small window (`open_gallery_window`), not a workspace tab, so it
  never disturbs the working layout.
* **Not in release builds.** The module, its `pub use`, the palette row and the
  shell-side command registration are all `#[cfg(debug_assertions)]` /
  `#[cfg(any(debug_assertions, feature = "gallery"))]`. The `gallery` cargo
  feature (on `labonair-ui-kit`, forwarded by `labonair-shell` →
  `labonair-app`) force-compiles it for `cargo check --features gallery`; it
  adds no crate-graph edge.
* **Honest about state.** GPUI cannot force an element into its `:hover` /
  `:active` look, so those are only visible by mousing over the real window —
  the gallery says so at the top. `disabled` / `selected` / `pressed` /
  `checked` / severity tints / `NumberField` clamping / `Disclosure` chevron
  are all real.

### 8.18 `ThemeRegistry` + JSON theme families — T20-005

`crates/theme/src/registry.rs` adds a **registry** on top of the T02-003
single-custom-theme model: `ThemeFamilyContent` (`{ name, author?, themes:
[{ name, appearance, colors }] }`, a flat token→color map that inherits the
built-in default of the same `appearance` for any token it omits),
`ThemeRegistry` (`builtin()` from the embedded `assets/themes/labonair.json`
+ `load_user_themes(dir)`, `list() -> Vec<ThemeMeta>`, `get(id)` /
`resolve(id, appearance)` → `Result<Theme, ThemeNotFoundError>`,
`resolve_family_variant` for the per-mode override), and `ThemeMeta`
(`family_id` = source file stem / `"default"`, `family`, `variant_name`,
`appearance`, `builtin`).

`ThemeStore` holds the registry + `active_family` + `registry_variant`. It
resolves the active family through `resolve_family_variant` into the same
`custom` slot the legacy `import_theme_file` path uses, so
preference/appearance switching and font overrides keep working unchanged.
`set_active_theme(id)` / `set_registry_variant` / `reload_user_themes(dir)`
are the new entry points; `preview_registry_theme` is the palette-hover path.

**Deviations from the task's literal wording (deviation process, Critical
Rule 3 / settings-guidelines):**

* **No `JsonSchema` derive on `ThemeFamilyContent`.** `labonair-theme` is the
  zero-workspace-dep leaf crate (`scripts/check_crate_deps.py`
  `"labonair-theme": set()`); pulling `schemars` into it for a derive the
  acceptance criteria don't require was judged not worth the leaf-crate
  weight. A JSON-Schema for theme files can be added under T19-006's
  schema-generation umbrella if a need appears.
* **`set_active_theme` does not itself write `appearance.app_theme`.**
  `labonair-theme` cannot depend on `labonair-settings` (`SettingsStore`).
  Persistence stays where it already lived: `labonair-settings-ui`
  (`apply.rs` / the Themes pane) writes `appTheme` and then calls
  `ThemeStore::set_active_theme`. This is the same split the pre-T20-005 code
  used for the imported-theme id.
* **Built-in family JSON is full-color, generated.** `assets/themes/
  labonair.json` carries every `COLOR_TOKENS` hex for both variants,
  regenerated from `tokens.rs` with `REGEN_BUILTIN_THEME=1 cargo test -p
  labonair-theme builtin_json` — so "built-in JSON == hardcoded theme" is a
  real equality test (±1/255 per channel, matching the existing
  export-round-trip tolerance), not a tautology over an empty map.

**Live-reload** is driven from `labonair-shell` (it already depends on
`labonair-settings`): `settings::watch_dir(themes_dir, …)` (new sibling of
`watch_file`, filters on the `.json` extension) → `reload_theme_registry`
→ `ThemeStore::reload_user_themes` re-resolves the active family or falls
back to the built-in if its file vanished. `labonair-theme` gains no
`notify` dependency.

### 8.19 Icon themes — T20-006 (+ Zed-parity icon-set revision)

**Two strictly separated icon systems, mirroring Zed** (`zed-refrence/zed/crates/icons`
+ `zed-refrence/zed/crates/file_icons`):

1. **UI / chrome icons** — `labonair-ui-kit::IconName`, one variant per SVG in
   `crates/shell/assets/icons/*.svg`. That set is now a **verbatim vendored
   copy of Zed's full Lucide-derived set** (~297 SVGs, `snake_case` names, ISC —
   `assets/icons/LICENSES`) plus a small `// + Labonair addition` block for
   glyphs Zed has no equivalent for (`house`, `shield`, `square`, dock-panel
   toggles, `arrow_down_up`, `circle_check`, `palette`). `IconName` also carries
   back-compat **alias assoc-consts** (`Search = MagnifyingGlass`, `X = Close`,
   `Refresh = RotateCw`, …) so the port's earlier semantic call sites compile
   unchanged. The SVG bundle is embedded with `rust-embed`
   (`labonair_shell::EmbeddedAssets`), not a hand-maintained list.
2. **File / folder icons** — a swappable **JSON icon theme**
   (`crates/theme/src/icon_theme.rs`) transcribed 1:1 from Zed's
   `"Zed (Default)"`. `IconThemeContent` now has Zed's **two-level shape**:
   `file_stems` / `file_suffixes` map a name/suffix → an *icon key*
   (`"rust"`), and `file_icons` maps a key → an **asset path**
   (`"icons/file_icons/rust.svg"`, a vendored copy of Zed's per-language SVG
   set under `crates/shell/assets/icons/file_icons/`). `directory` / `chevron`
   / optional `named_directory_icons` are direct asset paths.
   `IconThemeContent::file_icon_path(name)` / `directory_icon_path(name, expanded)`
   / `chevron_icon_path(expanded)` return the resolved asset path; lookup order
   = whole (lower-cased) name in stems→suffixes → progressively shorter
   dot-suffixes (`archive.tar.gz` → `tar.gz` → `gz`; `.gitignore` → `gitignore`)
   → `default_file` key; a missing key falls back to `default_file` then to the
   literal `icons/file_icons/file.svg` (never blank / panic).

`IconThemeRegistry` (`builtin()` from the embedded
`assets/icon_themes/labonair.json` + `load_user_icon_themes(dir)` — now also
accepts a Zed-style **family** file `{ name, author, themes: [ … ] }`,
`list()`, `get(id)`) is unchanged in shape.

`labonair-ui-kit::icon` exposes `file_icon_path` / `folder_icon_path` /
`chevron_icon_path` / `icon_for_path(theme, name, is_dir, is_expanded)` (all
return a `SharedString` asset path) + `svg_path(path, color)` to render one.
The old `file_icon` / `folder_icon` / `glyph_icon` / `chevron_icon`
(`-> IconName`) and the hand-coded ~90-extension match table are **removed**.
`TreeRow` / `GitChangeRow` gained `icon_path(Option<SharedString>)` /
`chevron_path(…)` that win over the `IconName` setters. Explorer + SFTP rows +
the Settings icon-theme preview resolve through `ThemeStore::icon_theme()`.

`ThemeStore` holds `icon_registry` + `active_icon_theme` with
`set_active_icon_theme(id)` / `reload_user_icon_themes(dir)` / `icon_theme()`;
`appearance.icon_theme` (new `Preferences` / `AppearanceContent` field, default
`"default"`) persists the choice, written by `labonair-settings-ui`
(`apply.rs` + the Themes pane's new icon-theme picker with a live glyph
preview + Import / Open-folder). `labonair-shell` adds a second
`watch_dir(icon_themes_dir, …)` → `reload_icon_theme_registry` for live reload.

**Deviations / notes (deviation process, Critical Rule 3):**

* **No `JsonSchema` derive on `IconThemeContent`** — same reasoning as §8.18
  (`labonair-theme` is the zero-workspace-dep leaf crate). `serde` only.
* **Built-in icon theme is generated from Rust tables, now transcribed from
  Zed.** `DEFAULT_FILE_STEMS` / `DEFAULT_FILE_SUFFIXES` / `DEFAULT_FILE_ICONS`
  in `icon_theme.rs` are a 1:1 transcription of Zed's
  `FILE_STEMS_BY_ICON_KEY` / `FILE_SUFFIXES_BY_ICON_KEY` / `FILE_ICONS`, and
  the single source of truth for `assets/icon_themes/labonair.json` (regen with
  `REGEN_BUILTIN_ICON_THEME=1 cargo test -p labonair-theme builtin_icon`). The
  earlier "reproduce the legacy `file_icon` mapping" test is replaced by
  "resolves Zed's path for representative files" + "every referenced icon key
  resolves to a `file_icons/*.svg`".
* **UI icon names not one big rename.** Rather than sed-ing ~15 crates onto
  Zed's variant names, `IconName` keeps the port's names as alias assoc-consts
  pointing at the Zed-named variant that owns the glyph. All call sites are
  expression position (no `match` arms on `IconName` outside `ui-kit`), so this
  is sound. New code should prefer the Zed names.
* **Tab-bar / command-palette icons stay category glyphs**, not per-file —
  `TabKind::indicator()` etc. still return an `IconName` (now a Zed glyph via
  the aliases). Zed itself uses a category icon on editor tabs, so no per-file
  wiring was added there.
* **file_icons asset licensing.** `assets/icons/LICENSES` covers the Lucide UI
  set (ISC). Zed's `file_icons/*.svg` are vendored from the Zed repo
  (GPL-3.0-or-later / Apache-2.0); our repo is Apache-2.0. Provenance is
  recorded here; revisit if Zed clarifies the asset license as GPL-only.

### 8.20 `theme_settings` metric layer — T20-007

`crates/theme/src/theme_settings.rs` adds the *metric* half of the active
theme: `UiDensity` (`Compact` ×0.85 / `Default` ×1.0 / `Comfortable` ×1.15),
`ThemeMetrics` (UI + buffer font family/size/line-height, density,
`corner_radius_scale`, `reduce_motion`), and `ActiveTheme { colors, metrics }`
— colour from the `ThemeRegistry`, metric from settings, recomputed only on a
real change of either. `ThemeStore` owns the live `ThemeMetrics`
(`set_metrics`) and a cached `ActiveTheme`; `init_theme` mirrors it into the
`GlobalActiveTheme` global via a `cx.observe`, read app-wide through
`labonair_ui_kit::ActiveThemeExt` (`cx.active_theme()`).

Deviations from the task's letter:

* **`ActiveTheme.colors` is `Theme` by value, not `Arc<Theme>`.** A `Theme` is
  a few hundred bytes; keeping it inline drops the atomic traffic and keeps
  `ActiveTheme: Send + 'static` trivially. It is rebuilt only on change, never
  per frame (the `## Warnungen` concern), so the clone cost is irrelevant.
* **`GlobalActiveTheme` is a mirror, not a second source.** `ThemeStore` is
  the single owner; the global is refreshed by an observer. `cx.active_theme()`
  is the read path for code that isn't holding the store entity.
* **Metric settings live in the existing `appearance` area**, not a new
  `theme` area, and the `Settings` struct keeps the name `ThemeSettings`
  (`crates/settings/src/concrete.rs`). New leaves: `bufferFontFamily`,
  `bufferFontSize`, `bufferLineHeight`, `uiDensity`, `cornerRadiusScale`.
  `appFontSize` / `appFontFamily` / `appLineHeight` feed `ui_*`;
  `appCornerRadius` (px) is the retained `_legacy` key — a non-default value
  migrates to `cornerRadiusScale = px / 5` once, read-time, in
  `ThemeSettings::from_settings` (no on-disk migrator; Phase 18 is closed).
* **`uiDensity` renders as the generated `Select` dropdown**, not a bespoke
  `SegmentedControl` — the generated-field renderer (T19-004) has no segmented
  variant and a one-off widget for one field isn't worth it.
* **ui-kit primitive migration scope.** `Palette` gained `density` +
  `Palette::space(px)`; the nine primitives that take a `Palette`
  (`button`, `checkbox`, `context_menu`, `banner`, `kbd`, `number_field`,
  `segmented`, `select`, `toggle`) route every spacing/size `px(..)` literal
  through `space()`, and `Palette::radius` is the `corner_radius_scale`d scale.
  Kept as literals on purpose: `text_size`, icon glyph `.size(..)`, and 1px
  hairlines (typographic / non-spacing). `list`, `disclosure`, `indicator`,
  `divider` take bare colour params rather than a `Palette`, so they have no
  density channel — threading one means a constructor-signature change across
  ~30 call sites, deferred.
* **`buffer_*` metrics are new, seeded at the editor defaults.** Full
  editor-vs-terminal font-pref consolidation is deferred; the terminal keeps
  its own `FontOverrides` path.
* **`reduce_motion` zero-duration clamp.** `ActiveTheme::animation()` reports
  `Duration::ZERO` when reduce-motion is on (matches the acceptance wording),
  but the one `with_animation` call site (workspace tab-in) clamps to 10µs
  because GPUI divides by the duration.

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

`labonair-settings-content` and `labonair-settings-macros` (§2, settings
track) landed in T19-001 (§8.15) — `labonair-backend` now depends on
`labonair-settings-content` (a deviation from the graph above, which predates
Phase 18). `labonair-settings` (the `SettingsStore`, T19-002) is not yet
created. `labonair-panel` is present as the contracts crate but is not yet
consumed by any panel (`Panel` trait wiring is T17-001).
