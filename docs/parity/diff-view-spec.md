# DiffView Behavior and UI Specification

**Status:** Behavioral specification and Labonair target mapping; implementation phases tracked in §11
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80` (`zed-refrence/zed`)
**Owner:** Git owns diff production and staging. Editor owns the diff presentation model. Workspace hosts the DiffView item. UI-kit owns controls.

## Evidence boundary

This document states what the pinned reference *does* (observable behavior, layout, settings, actions, state transitions) in Labonair's own words and structure. Implementation is written from this specification, not from the reference source; no reference code is copied (`CLAUDE.md`). Research and specification were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation. Items marked **(unobserved)** come from static reading only and need native confirmation.

## 1. Surface inventory (reference)

| Surface | Reference purpose | Editable | Staging | Base |
|---|---|---|---|---|
| Project Diff ("Uncommitted Changes") | Every changed file in one scrolling document, worktree against HEAD, each hunk tagged staged/unstaged | yes | stage/unstage/restore per hunk, per file, all | HEAD |
| Unstaged Diff ("Unstaged Changes") | Worktree against index | yes | stage, restore | Index |
| Staged Diff ("Staged Changes") | Index against HEAD | read-only | unstage | HEAD↔Index |
| Branch Diff ("Changes since `<ref>`") | Worktree against merge base with a chosen ref; toolbar branch picker, diff stat, review-to-agent | yes | none | merge base |
| Solo Diff | One file's diff, "Show Full File" / "Show Changes Only" toggle, status icon | yes | stage/unstage/restore + next | HEAD |
| Commit View | One commit: header (author, date, SHA copy, message), file excerpts, shallow-clone notice, buffer-search / Git Graph / provider buttons | read-only | none | parent |
| Text / Multi Diff | Compare two texts / selected file pairs (clipboard vs selection, file vs file) | n/a | none | n/a |

All of the first six share one document renderer and one editor; they differ only in base, capability, empty label, tab title, and toolbar.

## 2. Document model

- A diff document is an ordered list of **file sections**. Each section = one **file header** + a list of **excerpts**; each excerpt = context lines + changed lines around one or more hunks.
- **Excerpt context:** `excerpt_context_lines` (default 2) unchanged lines around each hunk. Adjacent excerpts that touch or overlap merge. Gaps between excerpts show an expand control revealing `expand_excerpt_lines` (default 5) more lines up or down. Solo diff with `file_diff.show_full_file = true` (default) shows the full file instead.
- **File order:** conflicted files first, then tracked, then new/untracked (grouping applies only when the panel groups by status). Within a group: by path (default), or file name first when `sort_by = name`, or folder-first when `tree_view`. The order is a pure function of (path, status, settings); it never depends on which other files are present.
- **Auto-fold on first appearance:** deleted files, and untracked files when `git_panel.collapse_untracked_diff` is true. Later refreshes never re-fold a section the user opened.
- **Hunk kinds:** Added, Deleted, Modified. A modified hunk renders its deleted lines then its added lines.
- **Hunk stage state** (Project/Unstaged diffs): `Unstaged` (has secondary hunk), `Staged` (no secondary hunk), `Overlapping` (part staged), plus a *pending* flag while a stage operation is in flight (controls render at 0.66 alpha).
- **Word diff:** within a modified hunk, changed words are highlighted using the word-added/word-deleted colors; disabled by `word_diff_enabled` (global and per language).
- **Stats:** per file `added`/`removed` line counts (header) and totals across the document (toolbar).
- **Empty state:** centered label (per surface: "No uncommitted changes", "No unstaged changes", "No staged changes", "No changes"), then either the remote-sync button or the text "Remote up to date", then a **Close** button with the close-item key binding. While loading with no content, a rotating accent spinner replaces the label.
- **Refresh:** file-list change or diff-base change re-runs refresh. Diff-base change clears the pending scroll target. Changes to sort/group/tree/collapse-untracked settings refresh. A buffer whose diff changed re-registers its excerpts unless the buffer is dirty. Removed files drop their sections; new files insert at their sorted position.

## 3. File header (one per file section)

Height = 1 line box, padded; full width; bordered 1px (`border`, `border_focused` when selected+folded+focused); rounded-small; background `editor_subheader_background`; hover tint `element_hover`; when sticky it gains a shadow and a vertical fade above it.

Left to right:
1. **Fold chevron** (28px square transparent button, hover fill `element_selected`). Click folds/unfolds this file. Alt-click folds/unfolds all. Tooltip: "Fold Excerpt"/"Unfold Excerpt" with meta "⌥ click to toggle all".
2. **Stage checkbox** (surfaces with staging only): checked = file fully staged, indeterminate = partially, disabled without write access, click toggles the file's staged state and does not select the row.
3. **Dirty/conflict dot** (editable surfaces): warning color when conflicted, accent when dirty.
4. **File icon** (when file icons enabled) + **file name** in buffer font, colored by file status (added/modified/deleted/conflict/renamed); deleted files strike through. Click opens the file at the excerpt's jump position (secondary-modifier click opens in a split).
5. **Parent directory** in buffer font, muted, truncated at the start; deleted files use disabled color.
6. **Lock icon** when read-only.
7. Right group: **diff stat** (`+N −M`, only when N or M > 0), and an **Open File** outlined button shown only when the header is hovered or selected (with the open-excerpts key binding when selected).

Sticky: the header of the file under the viewport top stays pinned and is pushed up by the next file's header.

## 4. Hunks and gutter

- **Row backgrounds:** added rows `editor_diff_hunk_added_background`, deleted rows `editor_diff_hunk_deleted_background`, full row including gutter.
- **Hollow vs filled** (`git.hunk_style`, default `staged_hollow`): a staged hunk renders hollow (hollow background + 1px hollow border) and an unstaged hunk filled; `unstaged_hollow` inverts. Surfaces whose hunks are all one state use a fixed rule (Staged diff renders all as staged; Unstaged renders none as staged).
- **Gutter:** old/new line numbers; deleted lines are numbered with their base line number in split mode; a narrow colored bar (added/modified/deleted `version_control_*`) marks hunk rows.
- **Word diff** overlays at 100% of the word color on top of the row background.
- **Deleted-hunk placeholder:** a pure-deletion hunk at an unchanged position shows its removed lines as a block above the next line.

## 5. Hunk controls

Shown at the end of the hunk's first row (anchored to the right edge, hanging below the hunk start), on hover of the hunk or when the cursor is inside it. A compact bordered bar with bottom radius, border `border_variant`, background `editor_background`, shadow; mouse events blocked except scroll. Contents, in order, each conditional:
1. **Stage** (hunk is unstaged and the surface supports staging) or **Unstage** (hunk is staged and the surface supports unstaging). Tooltip "Stage Hunk"/"Unstage Hunk" + `git::ToggleStaged` binding. 0.66 alpha while pending. Both gated by `git.show_stage_restore_buttons` (default true).
2. **Restore** (surface supports restore; disabled for a created file). Tooltip "Restore Hunk".
3. **Next / Previous hunk** square icon buttons — only when hunks are *not* all expanded (i.e. never in the diff views, where all hunks are always expanded).

The Staged diff offers only **Unstage**. The Branch diff offers none of the staging controls. In split mode the controls on the left (base) side delegate to the right editor's renderer and act on the right side's hunk.

## 6. Unified and Split styles

- Setting `diff_view_style` ∈ {`unified`, `split`}, default **split**. Unified shows one column: deleted lines (red) above their replacement lines (green). Split shows two columns: left = base text (read-only), right = current text (editable where the surface is editable), aligned so equal lines sit on the same row; the shorter side of a changed hunk is padded with filler rows (hatched/empty background).
- **Minimum width:** `minimum_split_diff_width` (default 100 columns, in em-advances of the editor font; 0 disables). When the container is narrower, the split is *suspended* (renders unified) while the style stays `split`; widening restores split automatically. The split toolbar icon changes to the "auto" variant while suspended and its tooltip reads "Split when wider than N columns" with the hint "⌘ click to change min width" (secondary-modifier click opens the setting).
- **Synchronization:** vertical scroll is shared; horizontal scroll is independent per side. Moving the cursor on one side moves the other to the corresponding display position without scrolling. Pane-left/right activation switches focus between sides (and propagates out when already at the edge). Soft-wrap toggles apply to both. The left side hides its vertical scrollbar, minimap, diagnostics, and LSP data.
- **Left side:** read-only, deleted lines numbered, open-excerpt requests are translated to the right side's file position, expand-excerpt requests are translated to the right side's anchors.
- **Style toggle control** (`DiffStyleControls`, shared by every diff toolbar): two small icon buttons — Unified, Split — the active one toggled. Selecting writes `diff_view_style` to the user settings file *and* switches the live view. A settings change from elsewhere switches the live view too.
- Branch, Solo, and Project diffs all show the same control.

## 7. Toolbars

**Project Diff toolbar** (primary right, hidden unless a Project Diff is active), single wrapped row, gap 6px, vertical dividers between groups:
1. Diff stat `+N −M` and a divider (only when the document is non-empty).
2. **Up / Down** icon buttons (Previous/Next Hunk), disabled when the document has no hunks. Tooltips carry the action binding. They exist "because staging has no undo" so the user can navigate back.
3. **Stage group**: when the selection is non-empty → one **Toggle Staged** button (disabled unless the selection touches any hunk); with no selection → **Stage** (disabled unless an unstaged hunk is at the cursor; tooltip "Stage and Go to Next Hunk") and **Unstage** (disabled unless a staged hunk is at the cursor; tooltip "Unstage and Go to Next Hunk").
4. **Stage All / Unstage All** (fixed 5rem width): shows *Unstage All* when everything is staged (can_unstage_all && !can_stage_all), otherwise *Stage All* (disabled unless can_stage_all).
5. **Commit** button (dispatches commit, opens the commit surface).
6. **Send Review to Agent (N)** — only when N > 0 review comments exist.
7. The **DiffStyleControls** (see §6).

**Unstaged** toolbar = (1)(2) + Stage / Stage All + Commit. **Staged** toolbar = (1)(2) + Unstage + Unstage All + Commit. **Branch** toolbar = diff stat + branch picker ("Base: `<ref>`", popover) + review button + style controls. **Solo** toolbar = left group (Show Full File / Show Changes Only chevron toggle, style controls, divider, file status icon unless status style is label-color) and right group (diff stat from the file's status entry, Up/Down, Stage/Unstage + next, Restore, Commit).

## 8. Actions and key bindings

Namespace `git` (diff-relevant): `Diff` (follows `git.diff_base`), `DiffHead`, `DiffBranch`, `ToggleDiffBase`, `CompareWithBranch`, `ViewUncommittedChanges`, `ViewUnstagedChanges`, `ViewStagedChanges`, `OpenFileDiff`, `ToggleStaged`, `StageAndNext`, `UnstageAndNext`, `Restore`, `RestoreAndNext`, `StageAll`, `UnstageAll`, `Commit`, `Amend`, `ReviewDiff`. Editor: `GoToHunk`, `GoToPreviousHunk`, `ToggleSelectedDiffHunks`, `ExpandAllDiffHunks`, `ToggleSplitDiff`, `ToggleFold`, `ToggleFoldAll`, `OpenExcerpts`, `ExpandExcerpts`.

macOS bindings (context `GitDiff > Editor` unless noted): `cmd-enter` Commit · `cmd-shift-enter` Amend · `cmd-ctrl-y` StageAll · `cmd-ctrl-shift-y` UnstageAll · `cmd-alt-z` RestoreAndNext · Editor: `cmd-alt-y` ToggleStaged · `cmd-y` StageAndNext · `cmd-f8` GoToHunk · `cmd-'` ToggleSelectedDiffHunks · `cmd-"` ExpandAllDiffHunks · `cmd-alt-g r` ReviewDiff · Git panel: `ctrl-g d` Diff · panel `space`/`cmd-alt-y` ToggleStaged.

Behavior: stage/unstage acts on the selection's hunks, or the single hunk under a collapsed cursor (it never widens to the whole excerpt). Staging a dirty buffer saves it first. `…AndNext` then runs `GoToHunk` (wrapping across files). `GoToHunk`/`GoToPreviousHunk` move the cursor to the next/previous hunk start across file sections and scroll it into view.

## 9. Selection, focus, and navigation

- Opening Project Diff from the Git panel with an entry scrolls/selects that file's first excerpt; if the file's excerpt is not yet loaded the target is kept as *pending scroll* and applied when it registers, then cleared at the end of refresh.
- Re-opening an existing Project Diff activates it, moves the cursor to the document start, and re-points the repository when the active repository changed (one Project Diff per workspace).
- Cursor movement in the focused diff selects the matching entry in the Git panel (only when the diff is focused, so background diffs never hijack it).
- When the document becomes empty while the editor is focused, focus moves to the empty-state container; when it becomes non-empty again, focus returns to the editor.
- Tab: git-branch icon, titles listed in §1, tooltip = title. Dirty and conflict states come from the editor. The item can split, saves through the editor, and is not serialized beyond its kind and diff base (it rebuilds from git on restore).

## 10. Settings (reference names → Labonair keys)

| Reference | Default | Labonair key (target) |
|---|---|---|
| `diff_view_style` | `split` | `editor.diffViewStyle` (`unified`\|`split`) |
| `minimum_split_diff_width` | 100 | `editor.minimumSplitDiffWidth` |
| `excerpt_context_lines` | 2 | `editor.excerptContextLines` |
| `expand_excerpt_lines` | 5 | `editor.expandExcerptLines` |
| `word_diff_enabled` | true | existing `editor.gitWordDiff` |
| `git.hunk_style` | `staged_hollow` | `git.hunkStyle` |
| `git.show_stage_restore_buttons` | true | `git.showStageRestoreButtons` |
| `git.diff_base` | `head` | `git.diffBase` (`head`\|`defaultBranch`) |
| `git.file_diff.show_full_file` | true | `git.fileDiff.showFullFile` |
| `git_panel.collapse_untracked_diff` | false | `scm.collapseUntrackedDiff` |
| `git_panel.sort_by/group_by/tree_view` | path/status/false | existing SCM tree/sort settings where present |

Theme tokens used: `editor_diff_hunk_{added,deleted}_{background,hollow_background,hollow_border}`, `version_control_{added,deleted,modified,renamed,conflict,ignored}`, `version_control_word_{added,deleted}`, `editor_subheader_background`, `element_hover`, `element_selected`, `border`, `border_variant`, `border_focused`.

## 11. Labonair target mapping and phases

Current state: `ProjectDiffView` (`crates/workspace/src/views/project_diff.rs`) renders one file at a time from a 220px rail plus a virtualized list of patch lines; one Unified/Split text button; hunk fold + Stage/Unstage buttons; WorkingTree/Commit/Stash/Branch sources; no multi-file document, no file headers, no stats, no toolbar, no hunk navigation, no restore, no staged/unstaged tagging, no empty/loading chrome, no persisted style, no auto-unsplit, no synchronized split.

Architecture decisions:
- **D1 Model in Editor.** A pure `DiffDocument` (`labonair-editor`, next to `unified.rs`): files → hunks → rows, per-file/total stats, hunk list with stage state, next/prev hunk navigation, unified and split row builders (aligned with filler rows), word-diff ranges. No GPUI, unit-tested.
- **D2 Contract stays in `labonair-panel`.** `ProjectDiffRequest` gains `DiffSource::{Staged, Unstaged}` and `ProjectDiffMode` stays Unified/Split; the producer remains Git/SCM. The view never infers actions from paths (existing contract rule).
- **D3 View in Workspace.** `ProjectDiffView` renders the document in one virtualized scroll with sticky-less file headers, hunk controls, toolbar, empty/loading states.
- **D4 Settings.** `diffViewStyle`, `minimumSplitDiffWidth`, `hunkStyle`, `showStageRestoreButtons` added to settings-content, schema, defaults, and docs.
- **Known deviation:** the reference tags HEAD→worktree hunks with a secondary stage state. Labonair's `GitService` exposes only patches (no per-revision file content), so the Uncommitted diff merges the file's *staged* patch (index↔HEAD, tagged Staged) and *unstaged* patch (worktree↔index, tagged Unstaged) into one section ordered by position; overlapping edits appear as two hunks. Closing this needs a `GitService` content call (`file_at(rev)`); tracked as follow-up.
- **Not in scope of the first pass (tracked, not dropped):** editable excerpts inside the diff (needs editor-ui MultiBuffer diff mode), review comments / Send Review to Agent, Commit View header chrome, Text/Multi Diff, Branch picker toolbar, `ToggleDiffBase`, persisted fold state, conflict-marker UI.

Phases (each ends with the §12 gates):
1. **P1 — done.** `DiffDocument` model + tests (`crates/editor/src/diff_document.rs`).
2. **P2 — done.** Multi-file view: file headers (§3), hunk rows, tints, word diff, hollow/filled (§4), hunk controls (§5), empty/loading states (§2).
3. **P3 — done.** Toolbar (§7), hunk navigation, stage/unstage/restore/all, Commit (reveals Source Control) (§8).
4. **P4 — done.** Split suspension by width, persisted style and minimum width (`editorDiffViewStyle`, `editorMinimumSplitDiffWidth`) (§6, §10). Horizontal scroll sync is moot: lines clip instead of scrolling (see gaps).
5. **P5 — partly done.** Done: Staged-only / Unstaged-only surfaces (`DiffSource::{Staged, Unstaged}`, per-surface toolbar and hunk controls, Source Control overflow menu + `git: View Staged/Unstaged/Uncommitted Changes` commands), key context `Diff` with owner-registered commands and default bindings (`editor::GoToHunk` `cmd-f8`, `editor::GoToPreviousHunk` `cmd-shift-f8`, `git::ToggleStaged` `cmd-alt-y`, `git::StageAndNext` `cmd-y`, `git::UnstageAndNext`, `git::StageAll` `cmd-ctrl-y`, `git::UnstageAll` `cmd-ctrl-shift-y`, `editor::ToggleSplitDiff`), `editorDiffHunkStyle` / `editorDiffShowStageRestoreButtons`, hollow staged hunks with a 1px outline, sticky file header, excerpt-style hunk separators. Solo file diff (`DiffSource::File`, `git: Open File Diff`, "Show Full File" / "Show Changes Only" toggle, `editorDiffSoloShowFullFile`), per-file header Open File button and lock icon, empty-state Close button, rotating loading spinner, Source Control selection follows the DiffView cursor (`WorkspaceEvent::DiffCursorFile` → `GitPanelView::reveal_path`). Open: `git.diffBase`, `ToggleDiffBase`, Solo status icon / Restore buttons in the solo toolbar.

Implementation deviations and gaps (tracked, not hidden):
- Restore of a hunk is immediate like the reference, but undo is the view's own: the last restored hunk is re-applied by `git::UndoRestore` (`cmd-z` in the diff, toolbar "Undo Restore"; one level). Only whole-file restores (new / deleted files) stay two-step because they cannot be undone. `git::RestoreAndNext` is `cmd-alt-z`.
- Excerpts are driven by a whole-file diff (`GitService::diff_full`, `git diff -U1000000`, regrouped into Git-style hunks for staging): `editorDiffExcerptContextLines` (default 2) and `editorDiffExpandExcerptLines` (default 5) work, with expand-up / expand-down rows. Files whose full diff exceeds 2 MB fall back to the plain 3-line patch without expansion. Hunk separators carry no enclosing-symbol text for regrouped hunks (the symbol Git prints is lost). Staged and unstaged hunks of one file stay separate excerpt sets (the known merge deviation), so expansion never crosses between them.
- Hunk controls omit the next/previous icon buttons (the toolbar has them); only Stage/Unstage/Restore appear.
- Rows are syntax highlighted per hunk side (tree-sitter on the hunk's old/new snippet, so constructs opening outside the hunk may be classified loosely); no horizontal scrolling, no editable excerpts, no cursor/selection model beyond the active hunk. The pinned (sticky) file header is not pushed up by the next file's header.
- Theme tokens: `diff_{added,deleted}_{background,hollow_background,hollow_border,word}` are optional theme keys (`Theme::diff`). Unset roles derive from the status success/error colors (stronger fills in light themes), so a theme that only recolors status still gets matching diff tones. The built-in JSON does not pin them. The gutter marker bar always uses the status color; the word-diff role is per side, not per language.
- `git.fileDiff.showFullFile`, `scm.collapseUntrackedDiff`: not implemented. The Labonair keys for `git.hunkStyle` / `git.showStageRestoreButtons` are `editorDiffHunkStyle` (`stagedHollow` | `unstagedHollow`) and `editorDiffShowStageRestoreButtons`.
- Built since: Commit View header (author, date, message, SHA copy; no shallow-clone notice, buffer-search or provider buttons) and Text Diff (`DiffSource::Text`, `editor::DiffClipboardWithSelection`; file-vs-file via `editor::SelectForCompare` + `editor::CompareWithSelected` on the active editor tabs (local files, one pair; no multi-file selection or explorer integration).
- Built since: "Changes since <ref>" (`DiffSource::Since`, `git::DiffBranch`, toolbar "Base" picker). It is read-only and omits untracked files, and the base is chosen per view (no `git.diffBase` setting). `git::Diff` follows the new `editorDiffBase` setting (`head` | `defaultBranch`) and `git::ToggleDiffBase` flips the open diff between the two.
- Built since: conflict handling in the file list (conflicted files sort first, warning-colored name and dot) and `editorDiffCollapseUntracked` (auto-fold of untracked files). Conflict blocks of local conflicted files are listed with "Use Ours / Use Theirs / Use Both" buttons (`labonair_editor::conflict` parses and rewrites the file; staging the result stays manual); remote sessions show the combined diff text only. Checked natively on a real merge conflict.
- Not built: `ToggleDiffBase`, key bindings/actions (`OpenFileDiff`, `RestoreAndNext`, `Amend`, `ReviewDiff` still missing), persisted fold state, conflict UI.
- The merged staged/unstaged hunk model (see "Known deviation" above) is verified against a scripted Git; native pixel-level acceptance is still open.

## 12. Acceptance (per phase)

Visual check tool: `cargo run -p labonair-shell --example diff_preview -- <repo> [staged|unstaged|file:<path>|since:<ref>|commit:<hash>] [light] [narrow]` opens the DiffView alone in a window on a real repository. Checked natively on 2026-11-09 (dark theme, split style, modified + staged + deleted + untracked files): file headers, fold chevrons, split alignment with line numbers, filled and outlined (staged) hunks, word diff, syntax colors, expand rows, hunk controls on the cursor hunk, struck-through collapsed deleted file. Also checked natively: light theme, narrow-width split suspension (unified fallback), and the Commit source with its header (subject, author, date, SHA copy button), read-only lock icons and git's own hunk context labels. The Since source (worktree against a merge base, "Base: <ref>" button, uncommitted edits included) was checked too. The Text source (two strings, Rust highlighting from the title's extension) was checked too. Not yet checked natively: error state, the Base picker popover, unified style at normal width.


`cargo fmt --check`, `cargo check --workspace --all-targets`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, `python3 scripts/check_documentation.py`; native `cargo run -p labonair` visual check of: many files, one file, empty, loading, error, deleted, untracked, binary, staged-only, mixed, narrow width (split suspension), light/dark themes.
