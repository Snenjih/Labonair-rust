# Workspace New File Action Classification

**Status:** Static intent, owner, and route crosswalk for one default-macOS
action; pinned dispatch, native interaction, and paired visual acceptance
remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Editor (`labonair-editor`) owns the untitled document and file
lifecycle; Workspace (`labonair-workspace`) hosts and focuses its Editor view

## Evidence boundary

This packet specifies the user flow for creating a new, empty document that
has no filesystem path until the user saves it. The action IDs, bindings, and
contexts remain in the [action requirements register](action-requirements.md).
Static source checks support the behavior summary; they do not prove key
dispatch, new-window behavior, dirty-close prompts, or session restoration.

The requirements are written as user outcomes for Labonair's accepted
universal-tab architecture. This is same-session independent redevelopment,
not separated-room clean-room work, as recorded in the
[parity README](README.md#provenance-and-source-boundary). Pinned macOS
interaction and paired captures are unavailable on this Linux host without
`DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Editor — `labonair-editor` owns the untitled document, initial editing state, and save lifecycle. Workspace — `labonair-workspace` hosts the Editor view and contributes universal-tab presentation. |
| Composition | Editor contributes the New Untitled Document intent through its owner command surface. Workspace supplies a typed host contract to open and focus one new Editor tab. Shell composes menus and command providers only. |
| Canonical user entry points | Pinned File > New and `Cmd+N` in `Workspace && !Terminal`; `Cmd+N` is also bound in `Welcome`. The Welcome surface has a New File button. Labonair currently exposes File > New Editor Tab and `Cmd+E`. |
| Public contract | Reuse the Editor-owned untitled-document intent and the Workspace typed Editor-view host contract. The Labonair route creates one new universal Editor tab in the active Workspace. It does not create a second application window. Do not add a shell-owned duplicate action table. |
| Initial state | The new document is empty and pathless, with the caret focused and ready for input. Creating it does not create a named filesystem entry. It becomes a regular file after the user chooses a path through Save As. |
| Distinct actions | Keep this action separate from `project_panel::NewFile`, which creates a named filesystem file, and from `workspace::NewWindow`, which creates a window using the `On New Window` preference. The New File action explicitly asks for an empty Editor even when the `On New Window` preference is `launchpad`. |
| Registry contributions | Editor owns the New Untitled Document action and its keybinding. Workspace contributes only the typed view-host operation. This inventory slice changes no command, action, event, registry, or binding. |
| Dependencies | Editor owns the document and save lifecycle; Workspace hosts the universal Editor tab; Save/Save As owns path selection and persistence; Settings supplies unsaved-buffer/session behavior. |
| Persistence | A pathless buffer remains untitled until saved. Verify dirty-buffer close and restart behavior against `restore_unsaved_buffers` and `restore_on_startup`. A successfully saved document follows normal Editor document persistence. |
| Notifications and failure | A failed buffer creation or Save As operation must leave existing tabs intact, report a recoverable error through the notification center, and permit retry. Dirty-close cancellation preserves the buffer; confirmed discard removes only the new untitled document. |
| Security | No path is selected at creation time. Save As must follow the existing path permission, overwrite confirmation, and project trust rules. Do not infer or grant project trust from an empty buffer. |
| UI-kit | Reuse the shared Editor view, universal tab strip, Save As prompt, and dirty-close dialog. Do not add a separate local blank-document surface. |

The pinned implementation opens the untitled Editor in a fresh application
window. Labonair's accepted universal-tab architecture presents the same new
document flow as a new focused Editor tab in the active Workspace. This
documented adaptation preserves the user's ability to create, edit, and save
an untitled file. Its interaction and persistence behavior remain unaccepted
until end-to-end verification.

## Action requirement

| Requirement ID | Default route and context | User goal and pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-NewFile`](action-requirements.md) | macOS: `Cmd+N` (`Workspace && !Terminal`); `Cmd+N` (`Welcome`). Linux: `New` and `Ctrl+N` (`Workspace`); `Ctrl+N` (`Welcome`). Windows: `Ctrl+N` (`Workspace`); `Ctrl+N` (`Welcome`). | Create an empty, untitled Editor document, focus it for typing, and keep it pathless until Save As. The pinned action opens a fresh window with this document. | **Partial.** The Workspace-owned `CommandId::NewEditorTab` creates a new empty pathless Editor view in the active universal-tab workspace. It uses local File > New Editor Tab and `Cmd+E`; there is no `Cmd+N` New File route or Welcome New File entry. The blank-editor result is statically similar, but window/tab persistence and save lifecycle are not action-accepted. |

The pinned `Cmd+N` binding excludes Terminal, where a different action creates
a terminal. Verify that the New File binding does not capture terminal input
and that Welcome and Workspace contexts both open the empty Editor flow.

## Settings and persistence requirements

| Requirement | Target behavior | Current Labonair status |
|---|---|---|
| [`SET-general-session-restore-unsaved-buffers`](settings-requirements.md) | `session.restore_unsaved_buffers` is user-scoped and defaults to `true`. Verify whether the untitled buffer is restored after restart when enabled and omitted when disabled. | The Settings inventory contains the target row; no matching local setting is wired for this flow. |
| [`SET-general-restore-on-startup`](settings-requirements.md) | `restore_on_startup` is user-scoped and defaults to `last_session`. Verify the interaction between session restore and `restore_unsaved_buffers` for empty and edited untitled buffers. | The Settings inventory contains the target row; local restore behavior for a pathless Editor tab is unverified. |

The `On New Window` preference belongs to the separate
[`New Window action](action-classification-workspace-new-window.md). The
New File action's explicit empty-document request must not silently produce a
Launchpad instead. Record both Settings values when testing persistence.

## Required user-flow and state acceptance

- Activate File > New and `Cmd+N` in the Workspace context. Verify `Cmd+N`
  remains available from Welcome and does not dispatch while Terminal owns the
  same key. Verify keyboard discoverability and the pointer-accessible Welcome
  New File action.
- Confirm every invocation creates exactly one fresh empty document, focuses
  its Editor, places the caret at the beginning, and leaves existing documents,
  selections, tabs, project roots, and dirty state unchanged.
- Type text, use undo/redo, and verify the document remains untitled until
  Save As. Save it to a new path and an existing path; verify name, extension,
  overwrite confirmation, active tab, and dirty state. Exercise cancel and
  permission/write errors without losing text.
- Close the new document before and after edits. Verify clean close, dirty
  confirmation, cancel, discard, Save, and Save As outcomes. Confirm closing
  the new document does not close or replace the prior Workspace tabs.
- Test one and repeated invocations, no active project, standalone mode, local
  project, remote project, Welcome, and Terminal focus. In Labonair, verify the
  accepted universal-tab adaptation opens in the current Workspace rather
  than creating an OS window.
- Test `restore_unsaved_buffers` enabled and disabled with
  `restore_on_startup = last_session`. Verify save/reload/restart/reset of
  both values, restored buffer contents, focus, duplicate prevention, and
  recovery when the original Workspace path is missing.
- Test buffer-creation failure, slow initialization, empty state, initial
  keyboard focus, pointer focus, accessibility labels, narrow window layout,
  and error recovery. The empty document must remain usable while any unrelated
  project or service is unavailable.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| `CommandId::NewEditorTab` descriptor and handler in [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), with `new_editor_tab` in [`workspace.rs`](../../crates/workspace/src/workspace.rs) | Related new untitled Editor view | Creates one empty pathless Editor tab and focuses it. Its `Cmd+E` route differs from pinned `Cmd+N`; no test covers the pinned contexts, first input, save, dirty close, or session restore. |
| Editor view creation through `new_editor_view` in [`workspace.rs`](../../crates/workspace/src/workspace.rs) | Editor content surface | Proves a local Editor view can be created; it does not establish unsaved-document persistence or pinned action behavior. |
| [`settings-requirements.md`](settings-requirements.md) rows for unsaved-buffer restore and startup restore | Settings target inventory | Records IDs/defaults/scope targets; no Labonair setting wiring or runtime effect has been verified for a pathless Editor tab. |

No Labonair test exercises `ACT-MACOS-workspace-NewFile` end to end. The
existing New Editor Tab descriptor and Workspace view creation are not action,
save, close, or restore acceptance. Pinned macOS dispatch, native interaction,
and paired captures remain pending.

## Acceptance evidence still required

For each pinned and Labonair trace, record build, OS, window/tab dimensions,
scale, theme, active context, exact key/menu route, first typed content,
pathless and saved state, dirty-close decision, both restore Settings values,
and resulting tabs/windows after restart. Attach paired native captures and
interaction evidence for empty, edited, saved, dirty-close, and restored
states. Linux static checks do not satisfy the macOS-first gate.
