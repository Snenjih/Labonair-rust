# R09-006 — Editor E5: Git gutter and review bridge

## Status

⏳ Planned

## Implementation evidence (E5 core)

The editor-owned Git projection core is implemented while the task remains
formally Planned because its required R07-001 dependency is still open.

- Added `crates/editor/src/git.rs` and public exports in `crates/editor/src/lib.rs`.
- Added typed `GitGutterSnapshot`, `GitHunk`, `GitLineDecoration`, explicit
  unavailable/loading/no-repository/empty/error states, revision-bound display
  projection, changed-line navigation, hunk lookup, and typed stage/unstage/
  discard-preview/Project-Diff intents.
- Added `GitDecorationProvider` and `HunkActionSink` as asynchronous editor
  boundaries. The editor does not depend on `labonair-git` or Project Diff
  view entities and does not mutate repository state.
- Integrated revision-safe Git marker painting and typed hunk accessors into
  `crates/workspace/src/views/editor.rs`. Workspace remains a thin rendering
  adapter; no second Git cache or review surface was added.
- Added conservative, Unicode-safe inline/intra-line diff projection for
  compatible adjacent delete/add runs, revision-safe display mapping, and a
  `editorGitWordDiff` setting that independently controls those decorations.
- Extended the existing hunk context menu with status, line range, counts, and
  a compact patch preview while keeping Project Diff as the canonical review
  surface.
- Added focused tests for unified-diff projection, added/modified/deleted and
  untracked hunks, Unicode/CRLF line mapping, changed-line navigation, stale
  revisions, typed action boundaries, discard previews, and explicit empty /
  unavailable states.

Verification for this E5 core increment:

- `cargo fmt --all -- --check` — passed
- `cargo test -p labonair-editor` — passed, 109 tests
- `cargo test -p labonair-workspace --all-targets` — passed, 121 tests
- `cargo check --workspace --all-targets` — passed
- `cargo clippy --workspace --all-targets -- -D warnings` — passed
- `bash scripts/check-crate-deps.sh` — passed
- `python3 scripts/check_rework_queue.py` — passed
- `python3 scripts/check_documentation.py` — passed
- `git diff --check` — passed
- `cargo test --workspace --no-fail-fast` — E5 targets passed, but the
  repository-wide run has two unrelated pre-existing environment-sensitive
  failures: `labonair-ai::client::tests::end_to_end_streams_openai_sse_over_http`
  (`PermissionDenied`) and `labonair-settings::watch::tests::watcher_never_observed_the_rename`.
  No AI or Settings code was changed for E5.

The typed Workspace Git bridge, visible hunk actions, Project-Diff navigation,
and `editorGitWordDiff` settings consumer are now present. Inline blame remains
explicitly deferred because the current GitService contract has no blame
capability; no dead setting or fake provider was added. Native visual matrix
evidence remains under the open R07-001 acceptance dependency.

## Owner

- Module: editor for presentation; git for repository state and operations
- Capability-matrix row: [Editor](../../docs/capabilities.md) and [Git / source control](../../docs/capabilities.md)
- Composition entry point: labonair-shell injects the Git snapshot/action bridge into the Editor view

## Dependencies

- R09-001-editor-e0-text-model-contract
- R09-004-editor-e3-display-search-navigation
- R09-005-editor-e4-local-language-services
- R07-001-product-surface-acceptance must be complete.

## Goal

Provide responsive inline Git hunk decorations and safe navigation from the
Editor to source-control actions while keeping Git state and review operations
owned by the Git capability and Project Diff as the canonical review surface.

## Scope

- In scope: line-level added/modified/deleted gutter markers, hunk snapshots,
  word-diff highlights where available, hunk hover/details, refresh/invalidation,
  stage/unstage hunk intents, open Project Diff at a hunk, blame preview,
  selection-safe updates, and untracked/ignored behavior.
- Out of scope: a second diff/review editor, commit UI redesign, Git history
  graph changes, remote Git transport, and inline review comments.

## Contracts and ownership

- Public domain values: GitHunk, GitHunkStatus, GitLineDecoration,
  GitGutterSnapshot, GitHunkAction, and typed GitGutterEvent in the Editor/Git
  boundary.
- Service traits or typed events: labonair-git provides a read-only
  GitDecorationProvider snapshot and a typed HunkActionSink for stage,
  unstage, and open-review intents. Git owns repository state and mutations;
  Editor owns placement and rendering.
- Registry contributions: Git commands remain Git-owned; Editor contributes
  only display toggles and navigation commands. No new Git registry is
  created for one provider. Existing Project Diff palette actions remain
  canonical.
- UI surface: Editor gutter/hover and the existing Project Diff tab. Hunk
  review and multi-hunk staging must delegate to Project Diff/SCM flows.
- Shared UI-kit components: tooltips, popovers, menus, buttons, badges,
  dialogs, list rows, and keyboard hints. Gutter markers and inline diff
  painting use Editor tokens and rendering.

## Commands and keymap

Editor registers Toggle Git Gutter, Next/Previous Change, Show Change,
Open Project Diff at Hunk, and Toggle Inline Blame if supported. Git owns
Stage Hunk, Unstage Hunk, Discard Hunk, and related confirmation actions.
Every action is owner-registered with stable IDs and Editor or Source Control
context. Discard always requires the existing actionable confirmation flow.

## Settings

These are proposed Editor presentation values:

| Key | Type / default | Scope | Migration and test |
|---|---|---|---|
| editorGitGutter | bool / true | Global + safe Project | New key; validate bool, reset true, test empty/untracked/repository states |
| editorInlineBlame | bool / false | Global + safe Project | New key; validate bool, reset false, test async refresh and narrow layout |
| editorGitWordDiff | bool / true | Global + safe Project | New key; validate bool, reset true, test hunk mapping and no-repository state |

Each key must be added to Settings content, project whitelist/scope rules,
generated UI, migration classification, and focused consumer tests in the
same implementation change. Git credentials, repository paths, branch state,
and staging data are capability-owned, not Settings values.

## Persistence and migration

- Storage: Git gutter snapshots are derived runtime state and are not persisted.
  Editor session persistence may store only the display-toggle values through
  Settings; Git state is re-read.
- Migration: absent new settings use the defaults above. Invalid hunk anchors
  after edits are dropped and refreshed from Git; file contents are untouched.
- Compatibility: a temporary adapter may map the current Project Diff hunk
  model to GitGutterSnapshot. Remove it when the Git capability publishes the
  typed decoration provider consumed by Editor.

## Notifications

Git operation failures, repository access failures, and stale worktree
refreshes use labonair-notifications-core with structured details and
deduplication. Hunk hover/details is an informational local surface; it must
not duplicate the notification dropdown. Discard confirmation remains a local
decision surface.

## Dependency restrictions

- Git remains the sole owner of repository state, diff calculation, staging,
  discard, blame data, and transport.
- Editor must not depend on Git internals or Project Diff view entities.
- Workspace/shell may inject a typed bridge but may not reinterpret hunk state
  or hold a second Git decoration cache.
- Do not add a new review surface or remote Git dependency in this task.

## Implementation plan

1. Define the read-only Git gutter snapshot and typed hunk-action bridge →
   verify mapping tests for insert/delete/rename/untracked and anchor shifts.
2. Implement asynchronous refresh and Editor gutter/hover rendering → verify
   dirty-buffer, external-change, large-file, no-repository, and narrow-state
   tests plus native visual checks.
3. Route stage/unstage/discard/open-review through Git and Project Diff →
   verify command ownership, confirmation, notification, and refresh tests.
4. Add settings and remove the temporary Project Diff adapter → verify
   settings migration, dependency audit, and source audit.

## Acceptance criteria

- [ ] Gutter markers are derived from Git snapshots and stay aligned after
      edits, reloads, and external changes.
- [ ] Stage/unstage/discard operations cross typed Git boundaries and never
      mutate Project Diff or Git state from Editor.
- [ ] Project Diff remains the only canonical review surface.
- [ ] Settings have complete owner, type, default, scope, validation, reset,
      migration, UI, and consumer-test records.
- [ ] Failures use notifications and no duplicate passive inline error exists.
- [ ] Focused Git mapping, Editor rendering, command, notification, and visual
      tests pass.
- [ ] Full repository, dependency, documentation, queue, and diff checks pass.

## Removal / exit condition

The task is complete when Editor no longer calculates or owns Git state, the
temporary Project Diff adapter is removed, all hunk actions use the typed Git
contract, and no second review implementation or hidden staging path remains.

## Notes and follow-ups

Remote Git and collaboration are deferred. E6 covers remaining Editor polish,
durable session persistence, autosave, format-on-save, and the deferred P2
editing improvements.
