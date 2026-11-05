# Keymap Entry Action Classification

**Status:** Static entry, target, and owner requirements; native dispatch,
document behavior, and paired Keymap acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Keymap owns binding state and the management workflow. Workspace
hosts the management surface and opens editable or bundled keymap documents.
The application root composes native menu entries.

## Evidence boundary

This packet independently states the three Keymap entry workflows and current
Labonair routes. Static action metadata, keymaps, menu entries, and local
providers establish which surface or file each route targets; they do not
establish native dispatch, focus, or visual acceptance. Work in this session
is independent redevelopment, not separated-room clean-room implementation,
as recorded in the [parity README](README.md#provenance-and-source-boundary).
Paired macOS Keymap-menu and document evidence remains pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Keymap management surface | `OpenKeymap` opens or activates the Keymap management item in the active Workspace pane. Pinned routes include the App > Settings menu, Workspace shortcut, Welcome “Customize Keymaps,” and titlebar/global menu. Labonair's `labonair-keymap` owns binding data and composition; `labonair-keymap-ui` renders its management view; Workspace owns the tab lifecycle. |
| Editable user keymap JSON | `OpenKeymapFile` opens the user keymap JSON as a normal Workspace Editor item and initializes it when absent. It is separate from the Keymap management view and bundled defaults. Labonair exposes the file through the management view's “Edit keymap.json” control; there is no matching App-menu entry or `cmd-e`/`ctrl-e` binding. |
| Bundled default key bindings | `OpenDefaultKeymap` opens the bundled baseline JSON document from App > Settings > Open Default Key Bindings. It is distinct from the user's editable overrides and the Keymap management view. Labonair includes default keymap assets for runtime composition but has no matching bundled-document route. |
| Events and registries | No new event or registry is needed. Reuse the Keymap command provider, Workspace handler, existing Keymap Settings-surface contribution, and native menu composition. |
| State and persistence | User bindings persist in the user keymap file and combine with platform and optional profile/mode layers. Opening the management view does not change bindings. The bundled defaults remain distinct from editable user overrides. |
| Settings | No setting is added. Profile and Vim/Helix mode values and layer precedence remain in the keymap-layer requirements. |
| Notifications and recovery | Malformed or unwritable user keymap data uses the Keymap notification route; Workspace Editor failures use the existing Workspace error path. Do not add a second JSON editor or duplicate passive notification. |
| UI kit | The management surface uses shared `labonair-ui-kit` controls. Editable and bundled JSON documents use the Workspace Editor. Native menu entries use the application menu integration. |
| Current Labonair route | The `OpenKeymapJson` command and `zed::OpenKeymap` alias open the management tab through Workspace; `cmd-k cmd-s`/`ctrl-k ctrl-s` also map there. The provider and platform keymap additionally declare `cmd-shift-/`/`ctrl-shift-/`, unlike the pinned defaults. The App menu places Keymap under Window, while Settings and the titlebar/global menu provide separate routes. User JSON is available from the Keymap view's editor button, but not through the pinned `cmd-e`/`ctrl-e` action or App Settings menu. Bundled defaults have no open-document route. |

## Action and related menu requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-zed-OpenKeymap`](action-requirements.md) | `cmd-k cmd-s` (`Workspace`) on macOS; `ctrl-k ctrl-s` (`Workspace`) on Linux/Windows. Open or activate the Keymap management item. Menu, Welcome, and titlebar/global routes converge on this surface. | Management view, Settings link, titlebar/global menu, Window menu, palette, and matching chord exist. The `cmd-shift-/`/`ctrl-shift-/` shortcut is an additional local default; native dispatch and active-pane reuse are unverified. |
| [`ACT-MACOS-zed-OpenKeymapFile`](action-requirements.md) | `cmd-e` (`KeymapEditor`) on macOS; `ctrl-e` (`KeymapEditor`) on Linux/Windows. Open the editable user keymap JSON in a Workspace Editor item. | The Keymap view's “Edit keymap.json” control opens/creates the user JSON. The pinned key is not bound, and the App Settings > Open Keymap File menu action is missing. |
| [`ACT-REGISTRY-LINUX-zed-OpenDefaultKeymap`](registered-actions-unbound.md) | App > Settings > Open Default Key Bindings. Open the bundled baseline JSON document; no default shortcut. | Built-in default keymap assets feed runtime composition, but there is no matching menu item or bundled-document route. |

## Required user-flow and state acceptance

- Open Keymap from the App menu, Workspace shortcut, Welcome, titlebar/global
  menu, Settings link, and command palette. Verify activation of an existing
  Keymap item in the active pane and creation in the selected pane when none is
  present.
- Open user keymap JSON from the menu, `cmd-e`/`ctrl-e`, and the Keymap view's
  JSON control. Verify an absent file gets its initial scaffold, existing
  edits persist, and the document opens as a normal Workspace Editor item.
- Open bundled default key bindings from the menu. Verify the document is the
  bundled baseline and remains distinct from the user override file and the
  interactive management surface.
- Close and reopen the Keymap surface and raw JSON item. Verify saved user
  bindings and file contents persist, and confirm whether view filters or
  selections are transient or restored.
- Capture paired native macOS App Settings menu, Keymap management view,
  existing-item activation, user JSON, bundled JSON, and file-error states.
  Record OS, active Workspace/pane, target document, and focus state. Verify
  non-macOS bindings separately.

## Acceptance evidence still required

Static metadata and code review establish action targets and local entry
routes; they do not establish key dispatch, native menu layout, active-pane
activation, user-file creation behavior, bundled-document behavior, or paired
visual acceptance. Those interactions and the current local action gaps remain
open.
