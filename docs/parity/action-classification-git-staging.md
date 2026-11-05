# Git Staging and Selection Action Classification

**Status:** Static action and owner-route classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns repository/index mutations. Workspace hosts Editor and
Project Diff. Keymap owns default bindings.

## Evidence boundary

This packet independently states the visible stage/unstage workflows and
current Labonair routes. Pinned action declarations, platform keymaps, source
inspection, and Labonair Git contracts establish static requirements only;
they do not prove native dispatch or interactive parity. This work is
same-session independent redevelopment, not separated-room clean-room
implementation. Paired macOS interaction and visual evidence remain pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Stage or unstage all | From GitPanel or GitDiff, run StageAll or UnstageAll against all repository changes. Labonair's Source Control header exposes Stage All when unstaged/untracked changes exist and Unstage All otherwise; no matching default shortcut is present. |
| Stage or unstage files | In the change list, apply the operation to the selected file or effective selected file set. The single-file controls also appear in file-diff UI. Labonair supports row checkboxes and a file context menu, but not the pinned keyboard actions. |
| Toggle current state | In the Git change list, toggle the selected entry or selected set. If a multi-selection contains any unstaged entry, stage all selected entries; otherwise unstage them. In an Editor diff, toggle the selected hunk ranges. Labonair has row and hunk context controls, but no shared context-sensitive ToggleStaged action or shortcut. |
| Stage or unstage a range | Use the current selection anchor and cursor in the Git change list to stage or unstage the contiguous status-entry range. Do not resolve or unresolve merge conflicts as a side effect of sweeping a range. Labonair has no equivalent range-selection action. |
| Move through diff hunks | Stage or unstage the current diff hunk, then advance to the next hunk. The action is enabled only when the current hunk can perform that operation. Labonair offers explicit Stage Hunk and Unstage Hunk commands from the Editor's Git hunk context menu and can stage hunks through Project Diff; it has no matching next-hunk action or default key binding. |
| Stage or unstage a section | The unbound StageSection and UnstageSection actions act on the section containing the selected GitPanel entry. Labonair provides section checkboxes and directory aggregate checkboxes, but no matching action identifiers or keyboard bindings. |
| Platform keymap | StageAll: `cmd-ctrl-y` in GitPanel/GitDiff on macOS; `ctrl-space` in both contexts on Linux and Windows. UnstageAll: `cmd-ctrl-shift-y` on macOS; `ctrl-shift-space` on Linux and Windows. StageFile/UnstageFile in ChangesList: `cmd-y`/`cmd-shift-y` on macOS, `alt-y`/`alt-shift-y` on Linux, `alt-y`/`shift-alt-y` on Windows. StageRange: `shift-space` in ChangesList on all three platforms. |
| Hunk keymap | StageAndNext/UnstageAndNext in `Editor && !agent_diff` use `cmd-y`/`cmd-shift-y` on macOS, `alt-y`/`alt-shift-y` on Linux, and `alt-y`/`shift-alt-y` on Windows. ToggleStaged uses `cmd-alt-y` in Editor and GitPanel on macOS, `ctrl-alt-y` in those contexts on Linux, and is not Editor-bound on Windows; in ChangesList it also uses Space on all platforms. macOS additionally binds `cmd-alt-y` in ChangesList. |
| State, persistence, and ownership | Git owns the index and asynchronous operation state. Selection, range anchor, editor focus, and visible diff are transient UI state. Labonair's canonical operation contract is `labonair-git`, with local/remote execution in `labonair-git-transport`; `labonair-panel-scm` is a Git-owned sibling. Workspace and Editor host diff interactions. No Settings value or dependency is added. |
| Events, registry, and notifications | This classification adds no event, registry, command, action, or keymap entry. Labonair operation errors and refresh messages must use the canonical Notifications owner. Existing Editor Git hunk intents and typed Git methods remain the local contract. |
| UI kit and current Labonair entry points | Source Control uses shared file rows, section/directory checkboxes, Stage All/Unstage All controls, and file menus from `labonair-ui-kit`. Editor exposes Stage Hunk/Unstage Hunk in the Git hunk context menu; Project Diff routes hunk changes through Git. There are no default Labonair shortcuts or section/range selection commands for this family. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-git-StageAll`](action-requirements.md) | Stage every repository change; `cmd-ctrl-y` in GitPanel/GitDiff on macOS and `ctrl-space` on Linux/Windows. | Source Control Stage All button and async Git operation exist; the pinned shortcut and action dispatch are absent. |
| [`ACT-MACOS-git-UnstageAll`](action-requirements.md) | Unstage every repository change; `cmd-ctrl-shift-y` in GitPanel/GitDiff on macOS and `ctrl-shift-space` on Linux/Windows. | Source Control Unstage All button and async Git operation exist; the pinned shortcut and action dispatch are absent. |
| [`ACT-MACOS-git-StageFile`](action-requirements.md) | Stage the selected file set in ChangesList; macOS `cmd-y`, Linux/Windows `alt-y`. | Row toggle and file menu stage through Git, but no matching default key binding or action exists. |
| [`ACT-MACOS-git-UnstageFile`](action-requirements.md) | Unstage the selected file set in ChangesList; macOS `cmd-shift-y`, Linux `alt-shift-y`, Windows `shift-alt-y`. | Row toggle and file menu unstage through Git, but no matching default key binding or action exists. |
| [`ACT-MACOS-git-ToggleStaged`](action-requirements.md) | Toggle selected status entries or selected diff hunks; see the platform contexts and multi-selection rule above. | File rows and hunk context menu expose explicit stage/unstage, but not the unified action or its default shortcuts. |
| [`ACT-MACOS-git-StageRange`](action-requirements.md) | `shift-space` stages or unstages status entries between the range anchor and cursor in ChangesList; unresolved conflicts are excluded. | No range anchor/selection stage command or shortcut. |
| [`ACT-MACOS-git-StageAndNext`](action-requirements.md) | Stage current diff hunk then move to the next hunk; macOS `cmd-y`, Linux/Windows `alt-y` in Editor. | Hunk stage exists in the Editor context menu and Project Diff; no next-hunk action or shortcut. |
| [`ACT-MACOS-git-UnstageAndNext`](action-requirements.md) | Unstage current diff hunk then move to the next hunk; macOS `cmd-shift-y`, Linux `alt-shift-y`, Windows `shift-alt-y` in Editor. | Hunk unstage exists in the Editor context menu and Project Diff; no next-hunk action or shortcut. |
| [`ACT-REGISTRY-LINUX-git-StageSection`](registered-actions-unbound.md) | Stage each entry in the selected GitPanel section; registered without a default binding. | Section checkboxes cover pointer use, but no matching action or keyboard route. |
| [`ACT-REGISTRY-LINUX-git-UnstageSection`](registered-actions-unbound.md) | Unstage each entry in the selected GitPanel section; registered without a default binding. | Section checkboxes cover pointer use, but no matching action or keyboard route. |

## Required user-flow and state acceptance

- Stage and unstage all changes from GitPanel and GitDiff. Verify staged,
  unstaged, untracked, mixed, conflicted, clean, and no-repository states,
  button enablement, async progress, refresh, and operation failures.
- Stage and unstage one file and a multi-selection. Verify partially staged
  files and directories, selected-entry focus, status changes, row collapse,
  and recovery when a Git operation fails.
- Toggle a staged/unstaged mixture. Verify the selection-wide result, the
  distinct single-entry behavior, and Editor hunk toggles across one or more
  selected diff ranges.
- Select a range anchor and cursor, stage and unstage the range in both
  directions, and verify entries outside it and unresolved conflicts remain
  unchanged. Verify behavior when refresh, collapse, selection movement, or
  file deletion invalidates the anchor.
- Stage and unstage individual hunks from Editor and Project Diff. Verify
  partial-file preservation, hunk identity after edits/reload, empty and
  unparseable diff errors, pending operation state, and keyboard movement to
  the next available hunk for StageAndNext/UnstageAndNext.
- Apply StageSection/UnstageSection to every section kind and to no-selection,
  partially staged, and mixed selections. Verify no action crosses into an
  adjacent section.
- Capture paired macOS GitPanel and GitDiff all-stage controls, selected file
  and range behavior, mixed/partial status, hunk context controls, disabled,
  loading, and error states. Record repository contents/index, active
  selection, window size, theme, scale, and focus. Verify Linux/Windows
  keyboard dispatch separately.

## Acceptance evidence still required

Static review establishes action meaning, key contexts, range and hunk staging
targets, current Labonair file/section/hunk APIs, and visible Source Control
controls. It does not establish native dispatch, range-anchor behavior under
interaction, hunk navigation, operation recovery, or paired visual parity.
Labonair has meaningful all/file/section and per-hunk support, but the pinned
shortcut/action surface, range action, and stage/unstage-and-next workflows
remain incomplete or unverified.
