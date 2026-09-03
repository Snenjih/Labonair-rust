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
labonair-panel-hosts     · labonair-panel-snippets  · labonair-panel-ai

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
| `labonair-workspace` | `Workspace`, `Pane`, `PaneGroup` (recursive split tree), `Dock` (L/R/B), `StatusBar` host, `ModalLayer`, `ToastLayer` host, layout/session persistence. | `ui/workspace.rs`, `ui/pane.rs`, `ui/tabs.rs`, `ui/sidebar_slot.rs`, `ui/session.rs`, `ui/window_state.rs` (persistence part), `ui/bookmarks.rs`, `ui/preview.rs`, `ui/sftp.rs`, `ui/terminal.rs` (view), `ui/editor.rs` (view), `ui/diff.rs` (view) — terminal/editor/sftp views stay here for now (§7 of the report) |
| `labonair-shell` | `AppShell`: composes titlebar + docks + workspace + statusbar + modal layer; the only crate that knows concrete panel types (registration). Thin, no feature code. | `ui/app_shell.rs`, `ui/lib.rs`, `ui/menu.rs`, `ui/bar_items.rs` (concrete items), `ui/cwd_breadcrumb.rs`, `ui/transfers.rs` (statusbar item + progress UI), `ui/updater.rs`, `ui/agent_access.rs` |
| `labonair-panel-explorer` | File-explorer panel. | `ui/explorer.rs` |
| `labonair-panel-scm` | Source-control (status / staging) panel. | `ui/git.rs` |
| `labonair-panel-git-graph` | Commit-graph panel. | `ui/git_graph.rs` |
| `labonair-panel-hosts` | SSH host-manager panel. | `ui/hosts.rs`, `ui/ssh_connection.rs` |
| `labonair-panel-snippets` | Command-snippets panel. | `ui/snippets.rs` |
| `labonair-panel-ai` | AI-chat panel. | `ui/ai_chat.rs`, `ui/ai_composer.rs`, `ui/live_bridge.rs` |
| `labonair-terminal` | Terminal engine (alacritty), unchanged; audible bell. | today's `crates/terminal`, `ui/bell.rs` |
| `labonair-editor` | TreeSitter editor core, unchanged. | today's `crates/editor` |
| `labonair-backend` | SSH / SFTP / Git / PTY / SQLite / keyring, unchanged; no UI deps. | today's `crates/backend` |
| `labonair-ai` | AI providers / streaming / tools, unchanged; no UI deps. | today's `crates/ai` |

---

## 3. Dependency rules (binding — CI-checked in T16-010)

These are stated so T16-010 can derive a mechanical check (e.g. `cargo-depgraph`
+ an allow-list assertion):

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
   depends on `labonair-settings` + `labonair-ui-kit`.
8. **The crate graph is acyclic.** Verified in CI (T16-010).

---

## 4. Layout contract (binding)

```
┌─ Titlebar ────────────────────────────────────────────────────────────────┐
│  [Tab] [Tab] [Tab] [+]                                            [◉ ▾]     │  ← tabs + 1 button only
├─ Docks + Workspace ──────────────────────────────────────────────────────┤
│ ┌ left dock ┐                                              ┌ right dock ┐ │
│ │  Panel    │            Workspace (split tree)            │   Panel    │ │
│ └───────────┘                                              └────────────┘ │
│ ┌ bottom dock ─────────────────────────────────────────────────────────┐ │
│ │  Panel                                                                │ │
│ └──────────────────────────────────────────────────────────────────────┘ │
├─ Statusbar ──────────────────────────────────────────────────────────────┤
│ [Explorer][SCM][Git][Hosts][Snippets][AI]  ·············  [⟳][CWD ▸][🔔³] │
│  └─ panel toggles (left, default) ───────┘   └─ info dropdowns (right) ───┘│
└──────────────────────────────────────────────────────────────────────────┘
   Overlay layer: command palette · dialogs · Cmd+F search · toasts
```

* **Titlebar** — tabs plus the split-tree tab content region. On the right,
  exactly **one** icon button `[◉ ▾]` → dropdown: `Settings…`, `Profile`
  (placeholder), separator, room for planned entries. The macOS traffic-light
  inset stays.
* **Workspace** — the active tab's content plus the recursive `Member::Axis`
  split tree.
* **Side Panels** — `Dock` on left / right / bottom; each dock holds several
  registered panels, one active, resizable, zoomable, persisted.
* **Statusbar** — left: one toggle per registered panel (from `PanelRegistry`),
  active highlighted. Right: info dropdowns — Notifications (badge dropdown),
  CWD breadcrumb, Updater, Transfers, Agent-Access — each a `StatusItem`.
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
