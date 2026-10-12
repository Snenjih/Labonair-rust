# Settings Entry Action Classification

**Status:** Static entry, target, and owner requirements; native menu/window
dispatch and paired Settings acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Settings owns its UI, typed values, and default content. Workspace
hosts Settings JSON files in Editor items. Keymap owns the two default
shortcuts; the app menu is a composition surface.

## Evidence boundary

This packet independently states the Settings entry workflows and current
Labonair routes. Static action metadata, keymaps, menu actions, and local
command providers establish which surface or file each route opens; they do
not establish native dispatch, focus behavior, or visual acceptance. Work in
this session is independent redevelopment, not separated-room clean-room
implementation, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Paired macOS Settings-window and menu evidence remains pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Settings UI | `OpenSettings` opens or focuses the Settings UI. Its `!SettingsWindow` context avoids invoking the same open action while the Settings window is active. Pinned menu, Welcome, and shortcut entry points converge on this owner surface. In Labonair, `labonair-settings-ui` owns a separate GPUI Settings window; `labonair-settings` owns values and `labonair-settings-content` their typed shape. |
| User Settings JSON | `OpenSettingsFile` opens the user's Settings JSON as a normal Workspace Editor item and initializes the file when absent. It does not open the Settings UI, a project file, or bundled defaults. Settings supplies the user file/default contents; Workspace/Editor owns displaying and editing the document. |
| Project Settings UI | The menu-only `OpenProjectSettings` action opens Settings targeted at the active project's visible root/worktree. Settings owns the surface and values; Workspace supplies the project target. Labonair's Settings window exposes a User/Project scope selector, but its separate `OpenProjectSettings` palette command currently opens the project's JSON file instead of directly targeting the Settings window. |
| Project Settings JSON | The menu-only `OpenProjectSettingsFile` action opens the project's Settings JSON in the Workspace Editor. It is distinct from the project Settings UI and from the user JSON file. Labonair's Settings header can open the selected Project JSON; the separate Workspace command opens or creates the project JSON directly. |
| Default Settings | The menu-only `OpenDefaultSettings` route opens the bundled baseline Settings document for inspection. It is distinct from editable user and project Settings files. Labonair has no matching bundled-default document route. |
| Events and registries | No new event or registry is needed. Settings contributes the UI command and value contract; Workspace opens JSON documents; Keymap supplies the two bound action defaults. The app root only composes native menu entries. |
| State and persistence | Settings values persist in their user/project layers; JSON file contents remain owned by those settings layers and Workspace Editor file lifecycle. Opening or focusing a Settings window adds no Settings value. The bundled defaults are not a user-editable Settings file. |
| Settings | No field is added. Preserve target scope when opening User or Project JSON, and preserve the distinction between the UI, user file, project UI/file, and bundled baseline. |
| Notifications and recovery | Window activation is navigation. File creation/open failures use the Settings/Workspace error route. Do not create a second Settings-only toast or duplicate JSON editor. |
| UI kit | The Settings window uses the shared Settings/UI-kit controls. JSON documents use the Workspace Editor. Native app-menu items use platform menu integration. |
| Current Labonair route | `OpenSettings` exists through the Settings command, App/Window menus, Settings window, and fixed macOS `cmd-,`; the pinned non-macOS `ctrl-,` default is absent. User Settings JSON exists as `OpenSettingsJson` in the command palette and as the Settings window's scope-aware JSON button, but lacks `cmd-alt-,` and an App-menu Open Settings File entry. Project JSON can be opened from the Settings button/Workspace command; there is no separate direct project-target Settings-window action or bundled-default JSON route. |

## Action and related menu requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-zed-OpenSettings`](action-requirements.md) | `cmd-,` on macOS; `ctrl-,` on Linux/Windows; `!SettingsWindow`. Open or focus the Settings UI. App Settings menu and Welcome row are additional routes. | Settings command, App/Window menu entries, Welcome/settings entry, and fixed `cmd-,` route exist. `ctrl-,` is missing. Verify existing-window focus and the `!SettingsWindow` exclusion. |
| [`ACT-MACOS-zed-OpenSettingsFile`](action-requirements.md) | `cmd-alt-,` on macOS; `ctrl-alt-,` on Linux/Windows; global. Open the editable user Settings JSON in a Workspace Editor item. | `OpenSettingsJson` command-palette route and scope-aware Settings JSON button exist. The default shortcut and App-menu Open Settings File entry are missing; command identity and file-creation behavior differ. |
| [`ACT-REGISTRY-LINUX-zed-OpenProjectSettings`](registered-actions-unbound.md) | App > Settings > Open Project Settings. Open the Settings UI targeted to the active project's visible root. | Settings scope selection exists; the similarly named Labonair command opens project JSON instead of directly targeting the Settings UI. |
| [`ACT-REGISTRY-LINUX-zed-OpenProjectSettingsFile`](registered-actions-unbound.md) | App > Settings > Open Project Settings File. Open the active project's JSON document in Editor. | Project JSON can be opened through the Settings window or Workspace command, but no matching menu action or action identity exists. |
| [`ACT-REGISTRY-LINUX-zed-OpenDefaultSettings`](registered-actions-unbound.md) | App > Settings > Open Default Settings. Open the bundled baseline document for inspection; no default shortcut. | No bundled default Settings document route or menu action exists. |

## Required user-flow and state acceptance

- Open Settings from App/Window menus, Welcome, command palette, and
  `cmd-,`/`ctrl-,`. Confirm the current Settings window is focused rather than
  duplicated, and verify the action is unavailable inside `SettingsWindow`.
- Open user Settings JSON from its shortcut, menu, palette route, and Settings
  header. Verify an absent file gets the initial scaffold, existing edits are
  retained, and the Editor opens as a normal Workspace item.
- With and without an active project, open Project Settings UI, Project JSON,
  and bundled defaults. Verify the selected project root and scope are
  preserved, and that the bundled document remains distinct from editable
  user/project files.
- Close and reopen the Settings window and Settings JSON document. Verify
  window visibility is transient while saved Settings values and file
  contents persist in their correct scopes.
- Capture paired native macOS Settings menus/window, User/Project scope,
  Settings JSON opening, existing-window focus, no-project, and file-error
  states. Record build, OS, active window/project, target file, and Settings
  scope. Verify non-macOS shortcuts separately.

## Acceptance evidence still required

Static metadata and code review establish the action targets and current
routes; they do not establish key dispatch, native menu layout, Settings
window focus, project targeting, default-file read-only behavior, or paired
visual acceptance. Those interactions and the current local action gaps remain
open.
