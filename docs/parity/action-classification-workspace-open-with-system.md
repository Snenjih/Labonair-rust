# Workspace Open With System Action Classification

**Status:** Static intent, owner, context, and route crosswalk for one
default-macOS action; Windows alias dispatch, native interaction, and paired
visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the generic local-path
open-with-system operation. Explorer owns selected-entry state; Editor
contributes the local invalid-item context. The platform layer hands the path
to the operating system's default application.

## Evidence boundary

This packet describes the user outcome of opening a selected local file or
folder outside Labonair with the operating system's default application. The
same action can be invoked from a local invalid-item view. Static checks
identify menu and key contexts, alias metadata, and current Labonair helper
code. They do not prove key dispatch, OS association behavior, launched app,
or the outcome on any native platform.

The requirements are independently stated for Labonair's existing Explorer,
Editor, and Preview surfaces. This is same-session independent redevelopment,
not separated-room clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux
host without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace — `labonair-workspace` owns the generic local-path handoff to a system application. Explorer — `labonair-panel-explorer` owns selected paths and its context menu. Editor supplies its local invalid-item view. |
| Composition | Explorer or Editor supplies a local path to the Workspace open-with-system operation. The platform service delegates to the OS association. Keep `Reveal in Finder`, in-app `Open`, and system-default launch as separate outcomes. |
| Canonical user entry points | Pinned `Ctrl+Shift+Enter` in ProjectPanel and InvalidBuffer on macOS and Linux. Windows ProjectPanel binds the old spelling `project_panel::OpenWithSystem`; the pinned Linux action metadata lists that spelling as a deprecated alias for `workspace::OpenWithSystem`. Windows has no recorded InvalidBuffer binding. Pinned ProjectPanel offers `Open in Default App` for local entries. Labonair has no matching command, binding, or Explorer/Editor default-app action. |
| Public typed contract | The pinned action is `workspace::OpenWithSystem`; ProjectPanel resolves its selected worktree entry to an absolute path and the invalid-item view supplies its local path to the native `open_with_system(Path)` service. Labonair's Workspace Preview has a separate `open_external(&str)` helper for its own preview flow, but no shared Explorer/Editor contract. If the action is implemented, use a typed local `PathBuf` operation and keep external process/platform behavior in its owner; no discovery registry is needed. |
| Input and outcome | Launch the selected local file or folder in the OS default application. Do not open a second Labonair Editor tab or merely reveal the parent directory. The invalid-item action retries the file through the OS app after Labonair could not display it. |
| Availability | The pinned ProjectPanel menu offers the action for local selections in its editable menu state, including directories; remote worktrees do not show it. The InvalidBuffer button appears only for local paths. Verify whether the OS opens a selected directory in the file manager and how a read-only ProjectPanel state exposes alternate routes. |
| Registry contributions | The pinned action is Workspace-owned and its selected-entry contribution comes from ProjectPanel; InvalidBuffer attaches the action keybinding to its local fallback button. This inventory changes no action, binding, registry, or menu. |
| Dependencies | Workspace owns the path-to-system-app operation and native platform handoff. Explorer and Editor supply a user-selected local path through their owner boundary. `labonair-filesystem` remains responsible for filesystem access, not launching applications. |
| Settings and persistence | No Settings value or Labonair persistence change is required. The OS may remember application associations; that state remains outside Labonair. Opening a path must leave current selection, project identity, tabs, and editor buffers unchanged. |
| Notifications and failure | A failed platform handoff should report a recoverable error through the notification center. Labonair's Preview helper only logs a warning on spawn failure; its result is not connected to a notification. |
| Security | Offer local paths only. Do not pass remote SFTP/SSH paths or credentials to local applications. Use a native path API or a process argument, never shell interpolation. The user's explicit action must not change project trust or write file contents. |
| UI-kit | Reuse the Explorer shared context menu and the existing invalid-item/error view action surface. Add no feature-local menu, button, or status style. |

`OpenWithSystem` is separate from `workspace::Open`, which invokes a Labonair
path chooser, from the Explorer's in-app `Open`, and from `Reveal in Finder`,
which reveals an item in the file manager without opening it in its associated
application. The pinned Windows spelling is captured in the action alias
table; its metadata resolution comes from the Linux registry dump, so Windows
runtime dispatch remains unverified.

## Action requirement

| Requirement ID | Default route and context | User goal and pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-OpenWithSystem`](action-requirements.md) | macOS/Linux: `Ctrl+Shift+Enter` (`ProjectPanel`, `InvalidBuffer`). Windows default map: `Ctrl+Shift+Enter` (`ProjectPanel`) on deprecated alias `project_panel::OpenWithSystem`; no InvalidBuffer route is recorded. | Open the selected local file or folder with its OS default application. In a local InvalidBuffer state, open the failed item path externally. Keep remote paths out of the local launcher. | **Missing canonical route.** Labonair's Explorer context menu offers in-app `Open`, Preview, and `Reveal in Finder`, not Open With System. Preview's external button handles content classified as external, but it is not the Explorer selected-item or Editor invalid-item action. No matching command or binding exists. |

The registered-action alias table maps the Windows spelling to the canonical
Workspace action based on pinned Linux registry metadata. The Windows keymap
contains only the ProjectPanel spelling; do not claim alias dispatch works in
the Windows runtime until it is checked there.

## Required user-flow and state acceptance

- In a local ProjectPanel, invoke `Open in Default App` by pointer and the
  `Ctrl+Shift+Enter` binding for a regular file. Verify that the selected path
  opens in the OS-associated application and that Labonair's active tabs,
  project, selection, and dirty buffers do not change.
- Repeat with a directory, symlink, extension without an association, and a
  file whose default association is a browser or another already-running app.
  Record whether the OS opens, focuses, or reports an error.
- Trigger the action from InvalidBuffer for an unreadable local file. Verify
  the same absolute path is handed off and the failed view remains recoverable.
- Confirm the action is absent for remote ProjectPanel entries and remote
  invalid paths. Do not attempt to open an SFTP/SSH path in a local app.
- Verify the `Open` item still opens through Labonair, `Reveal in Finder`
  still shows the containing location, and `Open in Default App` launches the
  associated app. Test read-only projects and folders for correct menu state.
- Exercise missing application association, OS denial, launch failure, and
  repeated activation. Existing Labonair state must remain intact and an
  actionable notification must explain how to retry or recover.
- On Windows, verify whether `project_panel::OpenWithSystem` resolves to the
  Workspace action. Test its absence in InvalidBuffer. On macOS and Linux,
  verify both canonical Workspace contexts and user keymap overrides.
- Inspect keyboard and pointer reachability, accessibility names, light/dark
  themes, narrow menu layout, and native OS handoff with paired captures.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| Explorer context menu in [`panel_explorer.rs`](../../crates/panel-explorer/src/panel_explorer.rs) | Select/open files, open Preview, reveal in Finder, and other Explorer actions | No system-default action or matching key binding appears. `Open` remains an in-app route. |
| `open_external` in [`preview.rs`](../../crates/workspace/src/views/preview.rs) | Opens preview paths or URLs classified as external | Uses `/usr/bin/open` on macOS and `xdg-open` on non-macOS. It is not connected to Explorer selection or Editor invalid-item state; Windows behavior is unverified and the helper logs spawn failure only. |
| `cx.reveal_path` in Explorer and SFTP views | Reveal an item in the system file manager | Reveals a path rather than opening it with the default associated application. Remote path behavior is not evidence for this local action. |
| Editor `FileState::Error` and other unavailable-file states | Display file load failures | No matching default-app action was found on the invalid local Editor path. UI state and error recovery are otherwise separate from external launch. |

No Labonair test or runtime trace exercises
`ACT-MACOS-workspace-OpenWithSystem`. Preview's external button and Explorer's
reveal action do not establish a selected-item default-app route. Pinned
dispatch, Windows alias behavior, macOS interaction, native OS associations,
failure notification, and paired visual evidence remain pending.

## Acceptance evidence still required

For each pinned and Labonair trace, record build and commit, platform, exact
key/menu/button route, active surface and file state, local/remote origin,
selected file or directory, OS association outcome, resulting Labonair tabs and
selection, launched application/error, and whether buffers remain unchanged.
Attach paired native captures for ProjectPanel file selection and InvalidBuffer
recovery on macOS. Linux source and platform-service inspection does not prove
Windows alias resolution or native dispatch.
