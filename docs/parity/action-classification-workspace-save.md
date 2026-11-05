# Workspace Save Action Classification

**Status:** Static intent, save scope, typed save-intent behavior, and current
Labonair Editor route; native dispatch, file-picker/prompt interaction,
partial-failure behavior, and paired visual acceptance remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns the four save actions and
selects the target item(s). Editor (`labonair-editor`) owns document content,
formatting, dirty state, file path, and disk writes.

## Evidence boundary

This packet distinguishes saving the active center-pane item, saving all
eligible dirty files in the Workspace, choosing a new path, and saving once
without automatic formatting. It maps save intents to the existing
format-on-save settings and current Labonair Editor flow. Static inspection
does not establish native keyboard dispatch, prompt appearance, or end-to-end
save behavior.

The requirements are independently stated for Labonair's Workspace and Editor
contracts. This is same-session independent redevelopment, not separated-room
clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns Save, Save All, Save As, and Save Without Format. Editor (`labonair-editor`) executes Editor document saves; other view owners define their own save support. |
| Canonical user entry points | All four pinned actions use Workspace context. macOS: Save `Cmd+S`, Save All `Cmd+Alt+S`, Save As `Cmd+Shift+S`, Save Without Format `Cmd+K S`. Linux: `Save` or `Ctrl+S`, `Alt+Save` or `Ctrl+Alt+S`, `Shift+Save` or `Ctrl+Shift+S`, `Ctrl+K S`. Windows: `Ctrl+S`, `Ctrl+K S`, `Ctrl+Shift+S`, `Ctrl+K Ctrl+Shift+S`. Labonair exposes active Editor Save through the File menu and command palette. The command descriptor has no published default key binding; native menu key-equivalent dispatch remains to be checked. |
| Save | Target the active item in the active center pane. The default `SaveIntent::Save` writes even when the file is clean, applies the effective format-on-save policy, and prompts before overwriting an external disk change. A pathless singleton can ask for a new destination. With no saveable active item, there is no file to write. |
| Save All | Target eligible dirty file items across all center panes. The default `SaveIntent::SaveAll` saves items with local changes and applies each item's effective format-on-save policy. It does not mean “save every clean file.” Per-file conflict or error handling can leave partial progress; record which files were saved before cancellation/failure. |
| Save As | Target the active saveable item and always ask for a new path, even if it already has a path. A pathless buffer receives a destination before it becomes a saved file. Cancelling path selection or a later failure leaves the item open and its unsaved content available. |
| Save Without Format | Target the active item and skip the automatic formatter for this save, even when the effective language setting enables format-on-save. It still writes the document and follows normal path, conflict, and write-error handling. |
| Save intent arguments | Save and Save All accept an optional typed `save_intent`; defaults are `Save` and `SaveAll`. Supported values are `Save` (write even when unchanged), `FormatAndSave` (force formatting), `SaveWithoutFormat` (skip formatting), `SaveAll` (write dirty files), `SaveAs` (always request a new path), `Close` (normal dirty-close prompt), `Overwrite` (write dirty files without a conflict prompt), and `Skip` (skip saving). The standalone Save As and Save Without Format actions select their corresponding intents directly. |
| Format settings | The pinned language-level `format_on_save` requirement is [`SET-LANGUAGE-FIELD-format-on-save`](settings-requirements.md), with packaged default `on` and per-language/user/project behavior to verify. Save follows the effective mode; FormatAndSave forces a formatter run; SaveWithoutFormat bypasses it. Labonair has [`LAB-SET-EDITOR-FORMAT-ON-SAVE`](settings-requirements.md), a global Boolean defaulting to `false`, with no per-language modes or explicit one-shot bypass. |
| State and persistence | On success, the file contents and accepted file snapshot update, the dirty indicator clears for the saved revision, and Save As changes the document's path. Preserve encoding, BOM, line endings, external-change identity, undo history, and unsaved content according to the Editor file lifecycle contract. No new settings or schema are introduced. |
| Conflict and failure behavior | A file changed on disk can prompt Overwrite, Discard Edits, or Cancel. A deleted singleton can prompt Save, Close, or Cancel. Cancel preserves the in-memory edits. Save/Save As/Save Without Format route task errors through the save error surface; Save All's static action handler detaches and logs errors, so verify how users see a per-file write failure and whether the batch stops or continues. |
| Events and registry contributions | Keep one Workspace action owner and the existing Editor save contract. Add no parallel save registry or shell-wide save handler. |
| UI-kit | Reuse the shared command/menu surfaces, file picker, prompts, and notification center. Add no local Save As dialog or duplicate save button in this inventory slice. |

## Action requirements

| Requirement ID | Pinned bindings | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-Save`](action-requirements.md) | macOS `Cmd+S`; Linux `Save` or `Ctrl+S`; Windows `Ctrl+S` | Save the active center-pane item using the supplied `save_intent`, defaulting to Save; format according to effective language settings. | File menu and command palette route to the active Editor only. No published default command binding; native menu shortcut is unverified. Pathless Save reports no file path instead of opening Save As. |
| [`ACT-MACOS-workspace-SaveAll`](action-requirements.md) | macOS `Cmd+Alt+S`; Linux `Alt+Save` or `Ctrl+Alt+S`; Windows `Ctrl+K S` | Save eligible dirty file items across Workspace center panes, defaulting to SaveAll. | No Save All command or menu route. |
| [`ACT-MACOS-workspace-SaveAs`](action-requirements.md) | macOS `Cmd+Shift+S`; Linux `Shift+Save` or `Ctrl+Shift+S`; Windows `Ctrl+Shift+S` | Always request a new path for the active saveable item, even if it already has one. | No Save As command or path-selection route. |
| [`ACT-MACOS-workspace-SaveWithoutFormat`](action-requirements.md) | macOS `Cmd+K S`; Linux `Ctrl+K S`; Windows `Ctrl+K Ctrl+Shift+S` | Save the active item while bypassing format-on-save for that invocation. | No one-shot formatting bypass; local Editor Save follows its global format-on-save Boolean. |

The complete action register contains the exact key sequences and Workspace
contexts. The pinned Save handler is only active for an editable current item;
Save As has its own availability rules. Verify read-only items, non-Editor
active views, and empty panes rather than assuming the key always saves.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`CommandId::Save` registration and descriptor](../../crates/workspace/src/command_provider.rs) | Registers “Save” in the palette and dispatches it to the active Workspace Editor. Its descriptor has no default binding. | It does not save a Terminal or every open Editor; there are no local Save All, Save As, or Save Without Format commands. |
| [`menu::Save` handler](../../crates/shell/src/commands.rs) | Routes the File menu Save item to `CommandId::Save`. | Native menu key-equivalent dispatch has not been checked on macOS. |
| [`Workspace::save_active`](../../crates/workspace/src/workspace.rs) | Looks up the active tab's Editor and calls its save method. | A pathless Editor does not open a Save As picker; the Editor reports that it has no file path. |
| [`EditorView::save`](../../crates/workspace/src/views/editor.rs) and the existing Settings record | Checks the current file identity before writing, preserves the file lifecycle metadata, and optionally requests local asynchronous formatting under `editor.editorFormatOnSave`. | The setting is one global Boolean defaulting false. There is no per-language format mode or SaveWithoutFormat bypass; Save All and Save As are absent. |

The Settings register already records both the pinned per-language format
field and Labonair's partial global Boolean counterpart. Those records do not
establish save-key, formatter, or file-picker parity.

## Required user-flow and state acceptance

- Save a clean and dirty file in the active pane while another pane contains a
  different dirty file. Verify Save affects only the active item, then Save
  All affects every eligible dirty file across panes and leaves clean files
  alone.
- Save a pathless singleton through Save and Save As. Verify the picker opens,
  its suggested name and root are appropriate, cancellation preserves the
  buffer, and success changes the path and clears the dirty indicator.
- Save an already-pathed item with Save As to a new path, the current path, and
  a path that is already open. Verify overwrite/collision handling, active tab
  identity, old/new file contents, and cancellation recovery.
- Exercise each effective language `format_on_save` mode. Compare Save,
  FormatAndSave, SaveWithoutFormat, and Save All; test formatter available,
  unavailable, error, and delayed response states. Confirm a one-shot bypass
  does not mutate the saved setting.
- Change the file externally, delete it, or make it read-only before saving.
  Record Overwrite/Discard/Cancel and Save/Close/Cancel choices, preserve edits
  on cancellation, and verify retry/recovery after filesystem errors.
- Save All with several dirty files and a conflict or write failure partway
  through. Record the completion order, partial disk writes, remaining dirty
  indicators, user-visible error, and whether later items are attempted.
- Verify Workspace context dispatch and all platform bindings, including
  user overrides and conflicts. Compare the File menu and native `Cmd+S` path
  with the action binding on macOS; do not infer one from the other.
- Compare paired native macOS captures for clean/dirty indicators, Save As
  picker, conflict/deleted-file prompts, formatting in progress, cancellation,
  and post-save state. Record binary, file fixture, settings, theme, scale, and
  window bounds.

## Acceptance evidence still required

For each trace, record build and commit, platform, action and active context,
active item/pane, file identity and dirty state, effective language settings,
formatter result, dialog choice and destination path, disk content, remaining
dirty items, errors/notifications, and session/layout effects. Include paired
macOS captures for menu/key, picker, conflicts, and post-save indicators.
Labonair's active-file Save and global format Boolean do not establish the
Save All, Save As, per-language, or one-shot no-format workflows.
