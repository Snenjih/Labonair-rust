# Outline and OutlinePanel Action Classification

**Status:** Static outline, panel, keymap, and Settings classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned OutlinePanel owns its project/editor tree and active-item presentation with Workspace action adapters. Pinned `outline::Toggle` owns a searchable Editor outline picker. Labonair Editor owns document-symbol values and its inline outline view; Workspace composes Editor and dock state; Settings owns persisted values.

## Evidence boundary

This packet classifies 13 registered-unbound `outline_panel` actions, five macOS-bound `outline_panel` actions, and the macOS-bound `outline::Toggle` action. The five panel actions also appear in the default Linux and Windows maps. Six of the 13 unbound panel action IDs have seven conditional Vim/Helix bindings. Action and map records are static evidence; they do not establish live macOS registration, dispatch, Settings reload, or visible interaction. No action is marked runtime-observed.

Same-session source analysis and independently authored requirements are redevelopment, not separated-room clean-room implementation. The pinned OutlinePanel and the local Editor symbol list are distinct surfaces.

## Owner and workflow contract

| Contract | Pinned behavior | Labonair route and gap |
|---|---|---|
| OutlinePanel contents | The panel combines active-project filesystem entries, files/excerpts and document outline items from the active editor/multi-buffer, and search results. It tracks one selected panel entry and can navigate between those entry kinds. | No OutlinePanel dock or multi-buffer tree exists. Editor produces document symbols and can render a separate code-symbol list inside the Editor view; it does not include the pinned filesystem/excerpt/search tree. |
| Outline picker | `outline::Toggle` opens a searchable modal picker for the active full-mode, single-buffer Editor's outline. Toggling again dismisses the open picker; missing editor, non-singleton buffer, or empty outline produces no picker. | Labonair has a syntax-derived `DocumentSymbol` model and an Editor-local list controlled by `editorShowOutline`. No matching `outline::Toggle` modal action or Cmd+Shift+O dispatch was found. `Workspace::active_editor_symbols` is documented for Go to Symbol, but the bounded caller search found no in-repository invocation. |
| Selected-entry collapse/expand | `CollapseSelectedEntry` collapses the selected filesystem/file/excerpt/symbol entry and folds associated editor buffers where applicable; when it cannot newly collapse the row it moves to the parent. `ExpandSelectedEntry` expands a collapsed row; when already expanded it moves to the next row. | Editor code folding and clicks in the inline symbol list exist, but no OutlinePanel selection model or matching Left/Right action routes exist. |
| Collapse/expand all | `CollapseAllEntries` and `ExpandAllEntries` collapse or expand the cached OutlinePanel filesystem, open-buffer, excerpt, and symbol entries and associated buffer folds. They require an active editor. | No unified multi-buffer OutlinePanel tree exists. Editor folds apply to the current text buffer only. |
| Fold/unfold directories | `FoldDirectory` and `UnfoldDirectory` act on selected filesystem directory or compact folded-directory entries. The context menu offers them when `outline_panel.auto_fold_dirs` is enabled and the row is eligible. These are compact project-folder chains, not code-symbol folds. | Explorer can automatically compress single-child filesystem paths under `file_manager.explorerFoldSingleChildDirs`; there is no OutlinePanel row or selected-entry Fold/Unfold action. |
| Open selected entry | `OpenSelectedEntry` navigates from a selected outline symbol, excerpt, or eligible open-file entry into the associated Editor item and scrolls/selects/focuses according to entry kind. Filesystem directories and entries without an editor anchor do not create an Editor target. | The Editor-local symbol rows set the caret and reveal it on click. There is no OutlinePanel Space action, file/excerpt tree, or selected-entry focus contract. |
| Reveal filesystem entry | `RevealInFileManager` resolves a selected filesystem path and asks the project to reveal it in the platform file manager; symbol, search, or ineligible entries have no path effect. | Explorer's row menu has a Finder reveal route, but there is no OutlinePanel selection to feed it and no matching OutlinePanel action. |
| Parent, scroll, and selection | `SelectParent` moves to the containing directory, file, excerpt, or enclosing outline item when one exists. `ScrollUp`/`ScrollDown` change selection by half the rendered row count. Cursor Top/Center/Bottom align the selected row without changing selection. | Labonair's Editor list permits pointer symbol selection and click-to-caret; it has no OutlinePanel parent/half-page selection or selected-row alignment actions. |
| Active item and symbols | `ToggleActiveEditorPin` transiently pins the current Editor item so the panel does not follow active-item changes; unpin resumes following according to `auto_reveal_entries`. `ToggleSymbols` only operates for multi-buffer content and transiently overrides `multi_buffer_hide_symbols`, changing symbol/excerpt/search rows. A Settings change to that value clears the override. | The Editor-local symbol list is controlled by a persisted Editor setting. It has no multi-buffer symbol visibility action or pin-active-editor state. |
| Panel focus and visibility | Workspace `ToggleFocus` toggles OutlinePanel focus. The separate `Toggle` attempts that focus toggle and closes the panel if it returns false. They are not the same action. | Workspace docks and Explorer controls exist, but no OutlinePanel dock or matching focus/toggle action route was found. |

## Settings effects

| Requirement | Pinned effect | Labonair relationship |
|---|---|---|
| `SET-editor-outline-panel-expand-outlines-with-depth` | `outline_panel.expand_outlines_with_depth` (default `100`) sets initial symbol expansion depth. Settings observation reapplies default expansion when the value changes. | No matching per-depth setting for an OutlinePanel symbol tree. |
| `SET-panels-outline-panel-auto-fold-dirs` | `outline_panel.auto_fold_dirs` (default `true`) compresses eligible single-child filesystem chains and gates Fold/Unfold menu items. | Explorer has a separate filesystem setting; it does not add an OutlinePanel action route. |
| `SET-panels-outline-panel-auto-reveal-entries` | `outline_panel.auto_reveal_entries` (default `true`) controls following/revealing the active item when the panel is not pinned. | No pinned active-editor item or OutlinePanel follow setting. |
| `SET-panels-outline-panel-multi-buffer-hide-symbols` | `outline_panel.multi_buffer_hide_symbols` (default `false`) sets initial multi-buffer symbol visibility; changing it clears the transient ToggleSymbols override. | No multi-buffer OutlinePanel or matching symbol-visibility action. |

Labonair's `editorShowOutline` is a separate global Editor value (default `false`). `EditorSettings::show_outline` is consumed by `Workspace`'s Editor view to render a 220px side list of up to 80 document symbols; clicking a row sets the caret and reveals it. This is an Editor-local single-document affordance, not the pinned OutlinePanel or modal action. The Settings inventory previously grouped this value with fields marked no-consumer/Remove; the corrected inventory splits it out as a current Editor consumer.

## Default and profile keymaps

The default macOS OutlinePanel context binds `left` to CollapseSelectedEntry, `right` to ExpandSelectedEntry, `space` to OpenSelectedEntry, and `alt-cmd-r` to RevealInFileManager under `OutlinePanel && not_editing`. `cmd-shift-b` in Workspace invokes ToggleFocus. The distinct Outline picker action `outline::Toggle` uses `cmd-shift-o` in BufferSearchBar and full-mode Editor contexts. These same five OutlinePanel action IDs are present in the default Linux and Windows maps; their platform-specific chords remain listed in [`action-requirements.md`](action-requirements.md).

In the conditional Vim/Helix `OutlinePanel && not_editing` layer, six otherwise-unbound action IDs have seven entries: `-` selects the parent; `Ctrl-U`/`Ctrl-D` move selection by half a page; `z t`, `z z`/`z c`, and `z b` align the selected row at top, center, and bottom. The center action has two chords. That layer also remaps the default-bound `CollapseSelectedEntry` and `ExpandSelectedEntry` actions to `h` and `l`; all OutlinePanel Vim/Helix bindings therefore cover eight action IDs over nine entries.

Other static profile overlays include `outline::Toggle` on macOS: Atom `Cmd-R`, Sublime Text `Cmd-R`, JetBrains `Cmd-F12`, TextMate `Cmd-Shift-T`, Vim `g s`/`g Shift-O`, and Helix `Space s`. The Linux/Windows profiles map Atom `Ctrl-R`, Sublime Text `Ctrl-R`, and JetBrains `Ctrl-F12`; JetBrains `ToggleFocus` maps `Cmd-7` on macOS and `Alt-7` on Linux/Windows in both Workspace and Pane contexts. These are overlay records, not runtime dispatch evidence. The 13 registered-unbound panel actions are absent from the three default maps; Linux registry data alone does not prove macOS registration. See the [macOS overlay register](keymap-overlay-requirements.md) and [non-macOS layer register](keymap-nonmacos-requirements.md).

OutlinePanel Copy Path is classified separately in the [Workspace path-copy packet](action-classification-workspace-path-copy.md). Its Linux/Windows platform action is an alias to `workspace::CopyPath`; it has no default macOS binding and does not imply a local OutlinePanel route.

## Action requirements

| Requirement | Pinned intent | Labonair route and current gap |
|---|---|---|
| [ACT-MACOS-outline-Toggle](action-requirements.md) | Open or dismiss the searchable active-Editor outline modal; empty/non-single-buffer states are no-ops. | Editor can display a Settings-controlled inline symbol list; no matching modal action or Cmd+Shift+O route. |
| [ACT-MACOS-outline_panel-CollapseSelectedEntry](action-requirements.md) | Collapse the selected tree/symbol entry and related buffer folds, or move to its parent when it is not newly collapsible. | No OutlinePanel selection/action route; Editor code folding is a separate text-buffer surface. |
| [ACT-REGISTRY-LINUX-outline_panel-CollapseAllEntries](registered-actions-unbound.md) | Collapse all cached panel entries and applicable buffer folds. | No unified OutlinePanel tree or collapse-all route. |
| [ACT-MACOS-outline_panel-ExpandSelectedEntry](action-requirements.md) | Expand the selected collapsed row; if already expanded, select the next row. | No OutlinePanel selection/action route. |
| [ACT-REGISTRY-LINUX-outline_panel-ExpandAllEntries](registered-actions-unbound.md) | Expand all cached filesystem, buffer, excerpt, and symbol entries and applicable buffer folds. | No unified OutlinePanel tree or expand-all route. |
| [ACT-REGISTRY-LINUX-outline_panel-FoldDirectory](registered-actions-unbound.md) | Fold an eligible selected filesystem directory chain when automatic directory folding is enabled. | Explorer has separate filesystem chain compression, but no OutlinePanel action. |
| [ACT-MACOS-outline_panel-OpenSelectedEntry](action-requirements.md) | Navigate to the selected symbol, excerpt, or eligible open-file entry in the active Editor. | Inline Editor symbol clicks jump to a caret; no selected-entry action or multi-buffer tree. |
| [ACT-MACOS-outline_panel-RevealInFileManager](action-requirements.md) | Reveal the selected eligible filesystem path in the system file manager. | Explorer has a row-menu reveal action, but no OutlinePanel selection or key route. |
| [ACT-REGISTRY-LINUX-outline_panel-ScrollCursorBottom](registered-actions-unbound.md) | Align the selected row at the bottom of the panel viewport. | No OutlinePanel selected-row alignment route. |
| [ACT-REGISTRY-LINUX-outline_panel-ScrollCursorCenter](registered-actions-unbound.md) | Center the selected row in the panel viewport. | No OutlinePanel selected-row alignment route. |
| [ACT-REGISTRY-LINUX-outline_panel-ScrollCursorTop](registered-actions-unbound.md) | Align the selected row at the top of the panel viewport. | No OutlinePanel selected-row alignment route. |
| [ACT-REGISTRY-LINUX-outline_panel-ScrollDown](registered-actions-unbound.md) | Move selection down by half the rendered entry count. | No OutlinePanel keyboard selection route. |
| [ACT-REGISTRY-LINUX-outline_panel-ScrollUp](registered-actions-unbound.md) | Move selection up by half the rendered entry count. | No OutlinePanel keyboard selection route. |
| [ACT-REGISTRY-LINUX-outline_panel-SelectParent](registered-actions-unbound.md) | Select the containing directory, file, excerpt, or enclosing outline entry when present. | No hierarchical symbol/file selection action in the local Editor view. |
| [ACT-MACOS-outline_panel-ToggleFocus](action-requirements.md) | Toggle focus on the OutlinePanel without applying Toggle's close-on-unfocus behavior. | No OutlinePanel dock or matching focus target. |
| [ACT-REGISTRY-LINUX-outline_panel-Toggle](registered-actions-unbound.md) | Toggle OutlinePanel focus and close the panel if focus toggling returns false. | Workspace has other dock controls, but no OutlinePanel toggle action. |
| [ACT-REGISTRY-LINUX-outline_panel-ToggleActiveEditorPin](registered-actions-unbound.md) | Pin/unpin the active Editor item followed by the OutlinePanel. | No pinned active Editor item behavior in the local outline list. |
| [ACT-REGISTRY-LINUX-outline_panel-ToggleSymbols](registered-actions-unbound.md) | Toggle symbols, excerpts, and search-match rows for an active multi-buffer view. | No multi-buffer OutlinePanel or corresponding visibility action. |
| [ACT-REGISTRY-LINUX-outline_panel-UnfoldDirectory](registered-actions-unbound.md) | Unfold the selected compact filesystem directory chain. | Explorer has separate filesystem chain compression, but no OutlinePanel action. |

## Required user-flow and state acceptance

- Exercise Cmd+Shift+O in BufferSearchBar and full Editor contexts; open, filter, select, dismiss, empty, and no-outline states; verify Editor caret, scroll, focus, and modal restoration.
- Exercise OutlinePanel Left/Right/Space/Reveal and Vim/Helix parent/half-page/viewport chords in `OutlinePanel && not_editing`. Confirm editing context, key precedence, and unchanged text-input behavior.
- Test filesystem file/directory/folded rows, open-buffer rows, excerpts, outline symbols, search matches, external files, and no selection across collapse/expand/all, Fold/Unfold, Open, Reveal, and SelectParent.
- Pin the active Editor item while changing workspace tabs and buffers; unpin and verify follow behavior with auto-reveal enabled/disabled. Toggle multi-buffer symbols, change Settings while overridden, and verify the default is reapplied.
- Change outline expansion depth and auto-fold Settings; verify saved values, reload, initial expansion, visible compact chains, menu eligibility, and expanded/collapsed recovery.
- Compare the local EditorShowOutline side list with the pinned modal and dock, including supported language, 80-row limit, long names, no symbols, click-to-caret, focus, Settings toggle, and narrow Editor layout.
- Capture direct macOS action registration and paired OutlinePanel/Outline modal states. Linux action metadata and static maps do not satisfy this gate.

## Acceptance evidence still required

Static source review establishes 19 outline-related action intents, six default-macOS-bound IDs, 13 registered-unbound panel actions, six unbound Vim/Helix IDs across seven entries plus two default-bound OutlinePanel IDs across two entries, four pinned Settings effects, a separate local EditorShowOutline consumer, and the existing path-copy classification. Labonair has no OutlinePanel dock or action dispatch; the inline Editor symbol list is a partial single-buffer affordance. Native macOS registration/dispatch, modal and panel keyboard behavior, Settings save/reload, buffer/focus transitions, and paired visual evidence remain open.

## Source records inspected

- Pinned actions and handlers: `zed-refrence/zed/crates/outline_panel/src/outline_panel.rs`, `outline_panel_settings.rs`, and `zed-refrence/zed/crates/outline/src/outline.rs`.
- Pinned Settings values and bindings: `docs/parity/settings-requirements.md`, `docs/parity/action-requirements.md`, `docs/parity/keymap-overlay-requirements.md`, and `zed-refrence/zed/assets/keymaps/default-macos.json` / `vim.json`.
- Labonair symbol extraction and UI: `crates/editor/src/symbols.rs`, `crates/workspace/src/views/editor.rs`, `crates/settings/src/concrete.rs`, and `crates/settings-ui/src/schema.rs`.
- Existing path-copy action route: [`Workspace path-copy packet`](action-classification-workspace-path-copy.md).
- Settings disposition requiring correction: `docs/settings-inventory.md`.
