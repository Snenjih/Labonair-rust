# Git Graph Action Classification

**Status:** Static Git Graph navigation, selection, clipboard, and detail classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns graph and commit data. Workspace hosts the Git Graph tab and commit views; GitPanel provides related entry points.

## Evidence boundary

This packet classifies nine registered-unbound `git_graph` actions and compares pinned action handlers with Labonair's GitGraphView and Workspace routes. Source establishes selected-commit clipboard behavior, SHA navigation, commit-view opening, search focus, half-page selection, and changed-file layout toggling. It does not establish native key dispatch, clipboard/browser effects, graph loading/recovery, or paired macOS visual parity. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Open the graph | `Open` requires an active repository and opens or reuses Git Graph for all repository commits. `OpenAtCommit` also requires an active repository, takes a `sha` argument, and opens/reuses the graph positioned at that commit. Labonair has an `OpenGitGraph` Workspace command and Source Control menu route that open/focus the single Git Graph tab, but no SHA-argument action or selected-commit route. |
| Search and select commits | `FocusSearch` focuses the graph search editor. `ScrollUp` and `ScrollDown` select a commit roughly half the visible rows above or below, scrolling the selection into view and clamping at graph ends. Labonair has graph commit rows and a separate branch prompt; no matching commit-search focus or half-page selection actions were found. |
| Copy commit identity | `CopyCommitSha` copies the selected commit's full SHA and is a no-op without a selected commit. `CopyCommitTag` is a no-op without a selected commit or tags; it copies one tag directly and opens CommitTagPicker when several tags are present. Git Graph and GitPanel commit context menus also provide related copy routes. Labonair's Graph row menu copies full/short hashes, but no typed SHA action or tag-copy route was found. |
| Open a commit view or change layout | `OpenCommitView` opens a CommitView for the selected commit and does nothing without a selected commit/repository. `ToggleChangedFilesView` switches the selected commit's changed-file list between flat and tree layouts, resets that list to the top, and keeps the view mode transient. Labonair shows selected commit details and changed paths in a side panel; View Changes opens a read-only Commit ProjectDiff. It has no CommitView action or changed-file tree/flat toggle. |
| Ownership, contract, and persistence | Pinned GitGraph owns action state and resolves commits through the active repository. Labonair's `labonair-git` supplies `GitGraphService`, `labonair-panel-git-graph` renders the graph, and Workspace owns the tab and `GitGraphEvent::OpenCommitDiff` consumer. Local copy-hash and View Changes pointer routes do not constitute action dispatch. No Settings or persistent graph view mode was found or proposed. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-REGISTRY-LINUX-git_graph-CopyCommitSha`](registered-actions-unbound.md) | Copy the selected commit's full SHA; no selected commit is a no-op. | Full/short hash copy is available from the local row menu, but no matching action route. |
| [`ACT-REGISTRY-LINUX-git_graph-CopyCommitTag`](registered-actions-unbound.md) | Copy one selected tag; open a picker for several; no tag/selection is a no-op. | No tag-copy route or action was found. |
| [`ACT-REGISTRY-LINUX-git_graph-FocusSearch`](registered-actions-unbound.md) | Focus the Git Graph search editor. | No matching graph search-focus action; the local branch prompt is separate. |
| [`ACT-REGISTRY-LINUX-git_graph-Open`](registered-actions-unbound.md) | Open/reuse Git Graph for the active repository's full log. | Workspace opens/focuses a Git Graph tab through a command/menu route, but no matching action dispatch. |
| [`ACT-REGISTRY-LINUX-git_graph-OpenAtCommit`](registered-actions-unbound.md) | Open/reuse Git Graph at the SHA argument in the active repository. | No SHA-argument graph-opening contract or matching action. |
| [`ACT-REGISTRY-LINUX-git_graph-OpenCommitView`](registered-actions-unbound.md) | Open CommitView for the selected graph commit. | Local selected-commit details and Commit ProjectDiff are different surfaces; no matching CommitView action. |
| [`ACT-REGISTRY-LINUX-git_graph-ScrollDown`](registered-actions-unbound.md) | Select about half a viewport of commits below the selection and reveal it. | Pointer/viewport scrolling exists; no matching action for half-page commit selection. |
| [`ACT-REGISTRY-LINUX-git_graph-ScrollUp`](registered-actions-unbound.md) | Select about half a viewport of commits above the selection and reveal it. | Pointer/viewport scrolling exists; no matching action for half-page commit selection. |
| [`ACT-REGISTRY-LINUX-git_graph-ToggleChangedFilesView`](registered-actions-unbound.md) | Toggle the selected commit's changed-file list between flat/tree layouts and reset its scroll position. | Local commit detail file list has no matching flat/tree view action. |

## Required user-flow and state acceptance

- Open/reuse Git Graph with an active repository, no repository, and multiple repository contexts; open it at a valid, missing, malformed, and non-visible commit SHA and verify selection, loading, and recovery.
- Focus search, enter and clear queries, and exercise half-page up/down around the newest, oldest, and empty graph states. Verify focus, match behavior, selection stability, and scroll placement.
- Copy a selected commit SHA; copy commits with no tag, one tag, and several tags. Verify exact clipboard contents, picker choice, unchanged clipboard on no-op, and errors.
- Open CommitView from the selected graph commit and from its context menu; compare with Labonair's side details and Commit ProjectDiff. Verify item identity/reuse, navigation, and stale commit recovery.
- Toggle changed files between flat and tree views with nested paths, empty changes, and long lists. Verify scroll reset and whether the selected commit remains stable.
- Capture paired native macOS graph, search, context menu, clipboard/tag picker, CommitView, and changed-file states; record repository graph, selected commit/tags, query, window size, scale, theme, focus, and outcome.

## Acceptance evidence still required

Static review establishes nine registered-unbound action IDs, the active-repository Open/OpenAtCommit routes, selected SHA/tag behavior, CommitView, search focus, half-page selection, and transient changed-file layout toggle. Labonair can open/focus its Git Graph tab, display commit details, copy full/short SHA through a pointer menu, and open Commit ProjectDiff, but has no matching action route, tag-copy route, CommitView action, SHA positioning action, or layout toggle. Native dispatch, graph errors/loading, clipboard effects, focus, and paired macOS visual parity remain open.
