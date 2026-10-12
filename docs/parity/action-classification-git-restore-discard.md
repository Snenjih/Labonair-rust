# Git Restore and Discard Action Classification

**Status:** Static action and destructive-scope classification; native interaction, recovery, and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns repository state and path mutations. Editor owns selected
hunk buffer edits. Workspace hosts project items and Trash. Keymap owns
default bindings.

## Evidence boundary

This packet independently states the hunk, file, tracked-change, and
untracked-file restore workflows. It records the current Labonair controls and
two source-level mismatches that require runtime confirmation: the scope of
RestoreTrackedFiles and the multi-file RestoreFile deletion route. Pinned
action metadata, keymaps, source, and local contracts establish static
behavior, not native dispatch, user recovery, or visual parity. Research and
requirements were produced in the same session, so this is independent
redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Restore selected hunks | `Restore` replaces selected diff hunks with their diff-base text in the writable Editor buffer. It is an Editor edit and follows normal buffer save/undo behavior; a read-only buffer does nothing. `cmd-alt-z` is bound in the macOS Editor context; Linux and Windows use `ctrl-k ctrl-r`. Labonair supports per-hunk stage/unstage but has no working hunk-restore action. Its visible Discard Hunk route is rejected by the Workspace Git bridge. |
| Restore and continue | `RestoreAndNext` applies the selected-hunk restore and moves the cursor to the next diff hunk; it wraps when collapsed hunks make the remaining range incomplete. It is bound in GitDiff Editor context (`cmd-alt-z` on macOS; `ctrl-k ctrl-r` on Linux/Windows). Labonair has neither this action nor next-hunk restore navigation. |
| Restore selected files | `RestoreFile` works on the selected status-entry set. For tracked entries it confirms by default, then unstages staged state and checks the file out from HEAD, removing both staged and unstaged changes. The prompt distinguishes restoring a deleted file from discarding other edits. On macOS, `cmd-backspace` and `cmd-delete` pass `skip_prompt: true`; Backspace/Delete pass false. Linux and Windows bindings pass false. An untracked single-file path still asks a separate Trash confirmation even when `skip_prompt` is true. |
| Selected untracked files | The multi-entry `RestoreFile` branch labels the confirmation "Trash these files?", but statically calls the permanent project delete path for created entries. This differs from its single-entry route, which sends the file to system Trash. With `skip_prompt: true`, the multi-entry deletion bypasses its confirmation. Record this as a pinned source discrepancy and verify its user-visible effect; do not silently substitute the safer Trash action. |
| Restore tracked files action | `RestoreTrackedFiles` is bound to `ctrl-g backspace` in GitPanel on all platforms. Static code filters the selected directory descendants (when invoked from a directory context) or the visible change list to entries that are both tracked and staged. Zero entries is a no-op; one entry restores directly; multiple entries prompt before checkout. This is narrower than the registered action description "all tracked files" and excludes tracked entries with only unstaged changes. Confirm that scope and the one-entry prompt difference at runtime. |
| Trash untracked files | `TrashUntrackedFiles`, bound to `ctrl-g shift-backspace` in GitPanel on all platforms, collects the visible created/untracked entries, prompts with names, unstages staged additions, and moves files to system Trash. Empty selection is a no-op; a single item uses its own Trash prompt. Ignored files are not part of the visible change list. Labonair's Clean Untracked Files menu confirms intent, then runs `git clean -fd`; this deletes untracked files/directories rather than moving them to recoverable Trash. |
| Ownership and state | Git (`labonair-git`) owns repository/index operations; `labonair-git-transport` handles local/remote Git. `labonair-panel-scm` owns the current Source Control controls. Editor hunk edits remain Editor-owned; Workspace hosts file items and Trash. No Settings, persistence, event, registry, action, or dependency change is proposed. |
| Notifications and UI kit | Labonair failures route through the Notifications owner. Existing Source Control rows and confirmation controls use `labonair-ui-kit`; destructive prompt controls must use shared UI-kit components. |
| Current Labonair routes | Source Control file menus expose Discard Changes for unstaged/conflicted rows without a confirmation prompt. A panel overflow menu offers confirmed Discard All Changes for tracked files and Clean Untracked Files. Its Editor Git hunk menu exposes Discard Hunk, but execution returns an explicit unsupported-operation error. No equivalent default keyboard actions or RestoreTrackedFiles/TrashUntrackedFiles commands exist. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-git-Restore`](action-requirements.md) | Restore selected diff hunks to their base in Editor; `cmd-alt-z` on macOS, `ctrl-k ctrl-r` on Linux/Windows. | Hunk staging exists, but hunk restore is absent; Discard Hunk reaches an unsupported Git bridge operation. |
| [`ACT-MACOS-git-RestoreAndNext`](action-requirements.md) | Restore selected hunk(s), then move to next diff hunk; macOS `cmd-alt-z`, Linux/Windows `ctrl-k ctrl-r` in GitDiff Editor. | No hunk restore-and-next action or default binding. |
| [`ACT-MACOS-git-RestoreFile`](action-requirements.md) | Selected-file restore/discard with platform-specific `skip_prompt` arguments and a second Trash confirmation for a single untracked entry; see source discrepancy above for multi-entry untracked files. | Per-row Discard Changes exists without confirmation and uses `git restore -- <path>`; no matching combined tracked/index restore or safe selected-set route. |
| [`ACT-MACOS-git-RestoreTrackedFiles`](action-requirements.md) | `ctrl-g backspace` from GitPanel. Static handler targets staged tracked entries in a selected directory or visible list, with no prompt for a single entry and a prompt for several. | Discard All Changes has broader all-tracked scope and confirms; there is no selected-section/specific tracked set action. |
| [`ACT-MACOS-git-TrashUntrackedFiles`](action-requirements.md) | `ctrl-g shift-backspace` from GitPanel; prompt, unstage created entries, then move them to system Trash. | Clean Untracked Files confirms but calls permanent `git clean -fd`; it has no trash recovery and no pinned shortcut. |

## Required user-flow and state acceptance

- Restore one and multiple hunks in Editor and GitDiff. Verify diff-base
  selection, read-only no-op, buffer dirty/undo/save behavior, selections
  spanning multiple hunks, and RestoreAndNext movement/wrap behavior.
- Restore one tracked file with staged and unstaged changes, a deleted file,
  and a multi-selection. Verify prompt text, cancel, both `skip_prompt` values,
  index/worktree results, and recovery after checkout failure.
- Test one and several untracked created entries through RestoreFile. Compare
  the single-file Trash path with the multi-file "Trash" prompt and permanent
  delete path, including `skip_prompt: true`. Record this source discrepancy
  explicitly if runtime agrees with the static call path.
- Invoke RestoreTrackedFiles with no selection, a directory, one staged
  tracked file, several staged tracked files, an unstaged-only tracked file,
  a mixed staged/unstaged file, and a staged new file. Verify actual scope,
  prompts, cancellation, index/worktree result, and failure recovery.
- Invoke TrashUntrackedFiles with none, one, multiple, staged additions,
  ignored files, nested directories, and partial Trash failure. Verify prompt
  names, unstaging, Trash recovery, and refresh.
- Compare Labonair's row discard, tracked Discard All, Clean Untracked, and
  Editor Discard Hunk against the pinned safety and recovery outcomes. Capture
  whether each mutation is undoable, recoverable from Trash, or permanent.
- Capture paired native macOS hunk/file/section destructive surfaces, prompts,
  skip-prompt variants, loading/error states, and Labonair equivalents. Record
  repository contents/index, file statuses, selection count, prompt choice,
  Trash state, window size, theme, scale, and focus. Verify Linux/Windows
  bindings separately.

## Acceptance evidence still required

Static review establishes five bound action targets, keymaps, selected-entry
and tracked/untracked behavior, the Labonair hunk-discard error, and local
Git/Workspace operation boundaries. It does not establish key dispatch,
confirmation visibility, system Trash recovery, the pinned multi-delete
source discrepancy at runtime, or paired visual parity. Labonair currently
has partial whole-file discard and cleanup routes, but not the pinned hunk
restore, selected tracked restore, or recoverable untracked Trash workflows.
