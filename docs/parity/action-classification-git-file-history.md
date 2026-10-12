# Git File History and Navigation Action Classification

**Status:** Static action, file-history, and provider-route classification; native interaction and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Git owns blame, history, commit, and hosting-link data. Editor presents blame. Workspace owns file navigation and permalink entry points. Git Graph presents commit history.

## Evidence boundary

This packet independently states the pinned Blame, OpenModifiedFiles, FileHistory, ViewCommit, OpenFileAtHead, ViewFile, and file-permalink workflows and compares Labonair's Git services, Source Control history, Git Graph, and Workspace routes. Source establishes action contexts, file/commit targets, provider resolution, and current local contracts; native rendering, ref parsing, hosting-provider results, clipboard/browser behavior, and failure recovery remain unobserved. Research and requirements were produced in the same session, so this is independent redevelopment, not separated-room clean-room implementation.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Blame gutter | Editor-bound `git::Blame` toggles the Git blame gutter for the current buffer and starts asynchronous blame lookup when needed. This is separate from the inline current-line blame setting/popover. Labonair's Editor consumes no blame contract; `GitService` has no blame operation or blame data type. |
| Inline blame settings | The pinned current-line blame is controlled by `git.inline_blame.enabled`, `location` (`inline` or `status_bar`), `delay_ms`, `padding`, `min_column`, and `show_commit_summary`; author avatar visibility is `git.blame.show_avatar`. Defaults are true, inline, 0 ms (no delay), 7 columns, column 0, false, and true respectively. These settings describe current-line/status-bar details and do not replace the `Blame` gutter action. Labonair's inventory has these setting rows but has not verified rendering or value effects. |
| Open modified files | Editor-bound `git::OpenModifiedFiles` requires a GitPanel and active repository. It reads cached status entries classified as modified and opens each corresponding project path in the Workspace. Missing panel or repository yields no paths and no visible result. Labonair has no Git bulk-open action or modified-file tab route found. |
| File history | Registered `git::FileHistory` targets the selected GitPanel file when the panel has focus; otherwise it resolves the file at the active Editor selection. It opens or reuses a GitGraph item scoped to that repository and path. The panel context menu offers it for eligible changed-file rows. Labonair has a repository History list and global GitGraph, but its Git log contracts do not carry a path-scoped log source and no FileHistory action route was found. |
| View commit by ref | Registered `git::ViewCommit` opens a ref-entry modal for the active repository. Typing a ref schedules a delayed commit-details preview; confirming a non-empty valid ref opens CommitView, while empty input or Cancel dismisses. A failed lookup shows an error toast; no active repository returns without a modal. Labonair's history row opens a Commit-source ProjectDiff but has no ref-entry modal or full CommitView route. |
| Open file at HEAD | In CommitView, `git::OpenFileAtHead` appears on a file context menu only when that path exists in the current project. It opens the current project file in a preview tab; it does not switch the commit diff buffer to an older version. Labonair has no corresponding CommitView or action route. |
| View file | Registered `git::ViewFile` acts on the selected GitPanel status entry and opens its current project path in a preview tab. Labonair's changed-file selection opens ProjectDiff with the path selected; it does not expose a matching ViewFile action/preview route in Source Control. |
| Open/copy permalink | Workspace file-tab context menus offer OpenFilePermalink and CopyFilePermalink when the project path belongs to a Git repository. The project asks the Git hosting-provider registry for a URL; Open launches it externally and Copy writes it to the clipboard. Provider errors are logged and shown in a toast. Labonair has no hosting-provider registry/permalink operation or Git file-permalink menu route. |
| Ownership, contract, and persistence | Labonair's canonical Git capability is `labonair-git`; `labonair-git-transport` provides local/remote adapters. `GitService` exposes repository status, repo-wide log, per-file diff, and commit diff. `GitGraphService` supplies repo-wide log and commit detail/diff operations; `labonair-panel-git-graph` renders the graph. `labonair-panel-scm` owns the current Source Control History list and emits a Commit-source ProjectDiff on selection. There is no local blame, path-log, arbitrary ref show, or hosting-permalink contract. |
| Current Labonair routes | Source Control loads up to 100 repository commits into History and opens the selected commit as a read-only ProjectDiff. A separate GitGraph tab presents repository commit history and can open a selected commit diff. Changed-file selection targets a path in the WorkingTree ProjectDiff. The inspected routes do not cover inline blame, per-file history, bulk opening modified paths, arbitrary ref CommitView, host permalinks, or the pinned action discovery paths. |

## Action requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-git-Blame`](action-requirements.md) | `cmd-alt-g b` in Editor on macOS (and `alt-g b` on Linux/Windows); toggle blame gutter and fetch entries if needed. | No Git blame type, API, Editor renderer, or matching action route found. Related inline blame settings are inventoried but not runtime-verified. |
| [`ACT-MACOS-git-OpenModifiedFiles`](action-requirements.md) | `cmd-alt-g m` in Editor on macOS (and `alt-g m` on Linux/Windows); open all cached status entries classified as modified. | No modified-file bulk-open action or route found. |
| [`ACT-REGISTRY-LINUX-git-CopyFilePermalink`](registered-actions-unbound.md) | Registered without a default binding; copy the Git host permalink for the file represented by a Workspace tab. | No hosting-provider registry or permalink operation/menu route found. |
| [`ACT-REGISTRY-LINUX-git-FileHistory`](registered-actions-unbound.md) | Registered without a default binding; show commit history for the selected GitPanel file or active Editor file. | Global history and GitGraph exist, but no path-scoped log input or FileHistory action route. |
| [`ACT-REGISTRY-LINUX-git-OpenFileAtHead`](registered-actions-unbound.md) | Registered without a default binding; from a CommitView file, open the same path in the current project when it exists. | No CommitView file action route. |
| [`ACT-REGISTRY-LINUX-git-OpenFilePermalink`](registered-actions-unbound.md) | Registered without a default binding; open the Git host permalink for the Workspace file tab in the external browser. | No hosting-provider registry or permalink operation/menu route found. |
| [`ACT-REGISTRY-LINUX-git-ViewCommit`](registered-actions-unbound.md) | Registered without a default binding; enter a Git ref, preview commit details, and open CommitView. | Selecting an existing History/GitGraph row opens a ProjectDiff; arbitrary ref lookup and CommitView are absent. |
| [`ACT-REGISTRY-LINUX-git-ViewFile`](registered-actions-unbound.md) | Registered without a default binding; open the selected GitPanel status path as a project file preview. | Source Control selects the path in ProjectDiff; no matching open-file action route found. |

## Required user-flow and state acceptance

- Toggle blame for tracked, untracked, historic, remote, and missing files; verify asynchronous loading, stale-result handling, gutter content, inline current-line display, popover delay, author/avatar/summary settings, and action repeat behavior.
- Invoke OpenModifiedFiles with no panel/repository, no changes, modified tracked files, deleted or renamed files, untracked/created entries, multiple repositories, and many files. Verify the exact modified-status filter, open-tab ordering/reuse, focus, partial failures, and cancellation.
- Open FileHistory from the focused Editor and focused GitPanel row; verify repository/path targeting, graph reuse, filtering and pagination, empty history, rename/move history, detached commits, deleted files, no active target, and asynchronous errors.
- Enter a branch, tag, short hash, full hash, invalid ref, ambiguous ref, and empty value in ViewCommit. Verify delayed preview, confirmation, Escape/cancel, lookup failure recovery, resulting CommitView tabs, and no active repository.
- From a commit file invoke OpenFileAtHead when the file exists or was deleted/renamed; verify it opens the current project version in preview and preserves the commit view. Test missing files, remote worktrees, path conversion failure, and repeated open.
- Invoke ViewFile on tracked, untracked, deleted, renamed, and no-longer-present GitPanel entries. Verify preview activation, focus, path identity, and recoverable open errors.
- Open and copy file permalinks for each supported hosting provider, branch/ref state, nested repository, remote project, and unsupported/private host. Verify repository/provider selection, URL path and ref safety, clipboard contents, external browser opening, credential exclusion, and statusbar notification/error behavior.
- Verify exact keymap contexts and command discoverability; compare normal/narrow Editor, GitPanel, CommitView and tab-context states. Capture paired native macOS results with pinned and Labonair builds, repository state, provider/remote, file/ref, window size, scale, theme, and action outcome.

## Acceptance evidence still required

Static review establishes two bound and six registered-unbound actions, blame settings/defaults, editor and panel file targeting, GitGraph path-history selection, ref-picker/CommitView behavior, and host-provider permalink dispatch. Labonair has repository history/graph and commit-diff display, but no blame, path-scoped history, ref-entry CommitView, Git bulk-open, or file-permalink contracts/routes in the inspected crates. This does not establish native action dispatch, blame/settings rendering, exact modified status results, provider outcomes, clipboard/browser effects, or paired visual parity. Runtime, implementation, and macOS acceptance remain open.
