# Zed Parity Working Records

**Status:** Working records for R07-000; incomplete
**Owner:** Product architecture and capability owners

This directory separates the independent Labonair requirements package from
source-side research. It records current evidence without claiming parity.

## Records

- [`feature-crosswalk.md`](feature-crosswalk.md) — draft user-workflow coverage
  map with proposed/current owners and explicit missing areas. It is not an
  exhaustive settings or runtime inventory yet.
- [`action-requirements.md`](action-requirements.md) — stable requirement IDs
  for all 595 action identifiers across the default macOS, Linux, and Windows
  keymaps, with 2,731 binding/context records and four null entries. It records
  five deprecated-alias resolutions from the pinned Linux action registry.
  The 585 macOS IDs remain stable. Independent static intent and owner-route
  packets currently cover 23 Search actions, 22 Pane actions, two Workspace
  collaborator-follow actions, one Workspace worktree-security action,
  four Workspace zoom actions, five Image Viewer zoom actions, five
  Application/window lifecycle actions (plus the related unbound Show All
  action), one Extensions action, two Settings entry actions (plus three
  related unbound Settings actions), two Keymap entry actions (plus one
  related unbound default-keymap action), one task-definition entry action
  (plus four related unbound global/project configuration actions),
  four Git worktree navigation/creation actions, six Git stash workflow
  actions (plus six related registered-unbound stash actions), eight Git
  staging actions (plus two related registered-unbound section actions), five
  Git restore/discard actions, six Git commit-lifecycle actions (plus three
  related registered-unbound actions), five Git remote-sync actions (plus three
  related registered-unbound actions), seven Git branch selection/rename/copy/
  filter actions, twelve GitPanel actions, nine Git Graph actions, two Git
  diff/review actions (plus nine related registered-unbound
  actions), two Git file/history actions (plus six related registered-unbound
  actions), three Git add/ignore registered-unbound actions, five Git
  repository/remote-management registered-unbound actions, 25 ProjectPanel
  tree/open/navigation/visibility/remote-download actions (15 action IDs in
  17 Vim/Helix profile bindings), 20 default-bound ProjectPanel actions
  (10 IDs in 10 Vim/Helix entries; 16 IDs across 41 macOS-profile entries and
  14 IDs across 20 Linux/Windows-profile entries), six generic Menu actions
  (20 macOS-profile entries and two shared Linux/Windows Atom entries), two
  Theme mode/selector actions, one Terminal Clear action, three Terminal
  clipboard actions, six Terminal scroll actions (eight JetBrains profile
  entries), two Terminal character-palette/vi-mode actions, two Task rerun
  actions (two JetBrains profile entries), two Terminal text/keystroke actions,
  one Terminal panel toggle and one registered-unbound Terminal panel focus
  action, five registered-unbound Terminal actions (rename, two half-page
  scroll declarations, search-test declaration, and select-all declaration),
  four registered-unbound TextFinder actions (four macOS-specific and four shared Linux/Windows override entries),
  19 Outline/OutlinePanel actions (six
  default-macOS-bound IDs and 13 registered-unbound panel actions; six of the
  latter occur in seven Vim/Helix entries), one Editor Go to
  Line action,
  one File Finder action, two Workspace Open/OpenFiles actions, one Workspace
  New Window action,
  one Workspace Add Folder to
  Project action, one Workspace New File action,
  one Workspace New Terminal action, one Workspace Open With System action,
  six Workspace dock-sizing actions, five Workspace dock visibility actions,
  two Workspace major-region focus actions, nine Workspace pane
  activation/swap actions, two Workspace bulk item/pane close actions, two
  Git worktree deletion actions, two font-size decrease actions, two increase
  actions and four reset actions, one Workspace Close Window action, one
  Workspace Reopen Last Picker action,
  two Workspace path-copy actions, and four Workspace save actions;
  pinned dispatch and native interaction acceptance remain open for those and
  all other actions.
- [`action-classification-search.md`](action-classification-search.md) —
  independently stated user goals, static Labonair owner routes, code coverage,
  and runtime/evidence gaps for 23 default-macOS Search-family actions. No
  action is marked runtime-observed.
- [`action-classification-text-finder-unbound.md`](action-classification-text-finder-unbound.md) —
  four registered-unbound TextFinder actions with eight macOS and shared
  Linux/Windows profile entries; classifies grouped-result folding, modal to
  tab transfer, query persistence, and the selection-only seed-setting override.
- [`action-classification-pane.md`](action-classification-pane.md) —
  independently stated user goals, Workspace/Editor owner boundaries, current
  tab/pane routes, and evidence gaps for 22 default-macOS Pane actions. The
  split between universal tabs, outer panes, and Editor groups remains explicit;
  runtime acceptance is pending.
- [`action-classification-project-panel-actions.md`](action-classification-project-panel-actions.md) —
  independently stated behavior, Workspace/Explorer/Editor/Git/Settings
  ownership, visibility persistence, and local routes for 25 registered-
  unbound ProjectPanel actions; 15 action IDs have 17 Vim/Helix profile
  bindings. macOS registration and runtime acceptance remain pending.
- [`action-classification-project-panel-bound-actions.md`](action-classification-project-panel-bound-actions.md) —
  independently stated behavior, preview and auto-open Settings effects, and
  Labonair routes for 20 default-macOS-bound ProjectPanel actions. Native
  dispatch, filesystem recovery, and paired macOS acceptance remain open.
- [`action-classification-menu-navigation.md`](action-classification-menu-navigation.md) —
  independently stated generic Menu behavior and local UI-kit routes for six
  default-bound actions, including the unresolved pinned Restart handler and
  provider-specific SecondaryConfirm behavior. Native dispatch and paired
  keyboard/menu acceptance remain open.
- [`action-classification-theme-mode-selector.md`](action-classification-theme-mode-selector.md) —
  pinned Theme mode/selector behavior, ten related Theme and Icon Theme Settings
  effects, Labonair palette and persistence routes, and native acceptance gaps.
- [`action-classification-terminal-clear.md`](action-classification-terminal-clear.md) —
  the pinned saved-screen/cwd-history effect, platform bindings, Labonair's ANSI
  PTY command route, and native scrollback/runtime acceptance gaps.
- [`action-classification-terminal-clipboard.md`](action-classification-terminal-clipboard.md) —
  Copy/Paste/PasteText clipboard-type behavior, paste sanitation, two related
  Settings effects, and Labonair's clipboard routes and gaps.
- [`action-classification-terminal-scroll.md`](action-classification-terminal-scroll.md) —
  six line/page/top/bottom actions, alternate-screen and inline-block behavior,
  eight JetBrains profile entries, history settings, and native evidence gaps.
- [`action-classification-terminal-character-palette-vi-mode.md`](action-classification-terminal-character-palette-vi-mode.md) —
  native palette and alternate-screen routing, transient vi-motion mode,
  missing local actions, and macOS acceptance states.
- [`action-classification-task-rerun.md`](action-classification-task-rerun.md) —
  Workspace last-task fallback, Terminal TaskId rerun overrides, shortcut
  context overlap, JetBrains entries, and Labonair's missing Tasks owner.
- [`action-classification-terminal-input.md`](action-classification-terminal-input.md) —
  raw text versus mode-aware keystroke payloads, platform bindings, the
  `option_as_meta` effect, and missing local action routes.
- [`action-classification-terminal-panel.md`](action-classification-terminal-panel.md) —
  TerminalPanel open/focus/close behavior, the separate `ToggleFocus` action,
  dock Settings, duplicate handler registration, and Labonair's tab-owner gap.
- [`action-classification-terminal-unbound.md`](action-classification-terminal-unbound.md) —
  five Linux-registered Terminal actions without default bindings; distinguishes
  the Terminal tab rename handler, unhandled half-page/SearchTest declarations,
  editor-owned SelectAll, and Labonair's generic rename/search/pointer routes.
- [`action-classification-outline-panel-actions.md`](action-classification-outline-panel-actions.md) —
  independently stated Outline picker and OutlinePanel behavior, four pinned
  Settings effects, six default-macOS-bound actions, and 13 registered-unbound
  panel actions; six of the unbound IDs occur in seven Vim/Helix entries, and
  the same layer maps the default-bound collapse/expand actions to `h`/`l`. The
  local EditorShowOutline list is a separate single-document affordance;
  OutlinePanel and modal action routes remain absent, with native dispatch and
  visual acceptance pending.
- [`action-classification-go-to-line.md`](action-classification-go-to-line.md) —
  independently stated intent, Editor owner, current line-jump route, and
  evidence gap for the default-macOS Go to Line action. The arbitrary
  line/column prompt and its runtime acceptance remain open.
- [`action-classification-file-finder.md`](action-classification-file-finder.md) —
  independently stated intent, Editor/Workspace ownership, current Open File
  route, related Settings targets, and runtime/evidence gaps for the single
  default-macOS File Finder action.
- [`action-classification-image-viewer-zoom.md`](action-classification-image-viewer-zoom.md) —
  independently stated Image Viewer zoom/fit/reset actions, toolbar and
  percentage editing, wheel/pinch/drag input, view-state persistence, and the
  missing Labonair Preview owner and route.
- [`action-classification-app-window-lifecycle.md`](action-classification-app-window-lifecycle.md) —
  independently stated Hide, Hide Others, Minimize, Quit, and fullscreen
  workflows, quit confirmation and Workspace preparation, fullscreen mode,
  current native menu routes, and the missing or unverified Labonair behavior.
- [`action-classification-extensions.md`](action-classification-extensions.md) —
  independently stated Extensions page entry, Workspace action parameters,
  active-pane activation/reuse, filtered entry points, and the missing
  Labonair Extensions owner and route.
- [`action-classification-settings-entry.md`](action-classification-settings-entry.md) —
  independently stated Settings UI, user Settings JSON, project Settings
  UI/file, and bundled-default entry routes, with current Labonair command and
  menu gaps.
- [`action-classification-keymap-entry.md`](action-classification-keymap-entry.md) —
  independently stated Keymap management, editable user keymap JSON, and
  bundled-default entry routes, with current Labonair command and menu gaps.
- [`action-classification-task-definition-entry.md`](action-classification-task-definition-entry.md) —
  independently stated global, project, and worktree task/debug configuration
  file routes, with missing Task/Debugger owners and local entry points.
- [`action-classification-git-worktree-navigation.md`](action-classification-git-worktree-navigation.md) —
  independently stated Worktree Picker, create, switch, and new-window flows,
  with current Git owner/API gaps and Workspace-state acceptance.
- [`action-classification-git-stash.md`](action-classification-git-stash.md) —
  independently stated stash creation variants, latest and selected-entry
  apply/pop, picker and stash-diff routes, confirmation/conflict states, and
  current Labonair Source Control coverage and action gaps.
- [`action-classification-git-staging.md`](action-classification-git-staging.md) —
  independently stated repository, file, section, selection-range, and hunk
  stage/unstage behavior with per-platform bindings and current Labonair
  Source Control/Editor routes.
- [`action-classification-git-restore-discard.md`](action-classification-git-restore-discard.md) —
  independently stated hunk and file restore, tracked-change scope,
  untracked-file Trash behavior, prompt arguments, and Labonair destructive
  recovery gaps.
- [`action-classification-git-commit-lifecycle.md`](action-classification-git-commit-lifecycle.md) —
  independently stated commit/amend, AI message generation, composer,
  cancel, signoff, hook-skip, last-commit undo, and current Git/Source
  Control contract gaps.
- [`action-classification-git-remote-sync.md`](action-classification-git-remote-sync.md) —
  independently stated fetch, pull/rebase, push/publish, force-with-lease,
  remote selection, upstream state, and current transport/UI mismatches.
- [`action-classification-git-branch-selection.md`](action-classification-git-branch-selection.md) —
  independently stated branch picker filters, branch selection/create, rename
  argument behavior, branch-name clipboard action, and current Labonair
  dropdown/palette routes.
- [`action-classification-git-panel-actions.md`](action-classification-git-panel-actions.md) —
  independently stated GitPanel close/toggle/menu, persisted view options,
  transient co-author toggle, selected-entry staged/unstaged diff actions, and
  current Source Control setting/route gaps.
- [`action-classification-git-graph-actions.md`](action-classification-git-graph-actions.md) —
  independently stated Git Graph open/search/commit navigation, selected SHA
  and tag copy, CommitView, half-page selection, and changed-file layout actions.
- [`action-classification-git-diff-review.md`](action-classification-git-diff-review.md) —
  independently stated configured/HEAD/branch diffs, staged and unstaged
  views, active-file diff, diff-base persistence, and Agent branch review.
- [`action-classification-git-file-history.md`](action-classification-git-file-history.md) —
  independently stated blame, bulk modified-file opening, path history, ref
  commit view, project-file navigation, and Git-host permalink actions.
- [`action-classification-git-add-ignore.md`](action-classification-git-add-ignore.md) —
  independently stated Git Add dispatch discrepancy and created-file
  `.gitignore`/`.git/info/exclude` workflows.
- [`action-classification-git-repository-management.md`](action-classification-git-repository-management.md) —
  independently stated clone/init/project-selection routes, the fallback
  branch setting, and unresolved registered remote actions.
- [`action-classification-workspace-open.md`](action-classification-workspace-open.md) —
  independently stated intent, Workspace/Editor ownership, distinct general
  Open and platform Open Files routes, related Settings targets, and runtime
  gaps for the macOS-bound Workspace Open action and Linux/Windows-only
  Workspace OpenFiles action.
- [`action-classification-workspace-add-folder.md`](action-classification-workspace-add-folder.md) —
  independently stated multi-directory add-to-current-project behavior,
  collaboration restriction, root persistence, and Labonair's single-root
  Open Project distinction.
- [`action-classification-workspace-collaborator-follow.md`](action-classification-workspace-collaborator-follow.md) —
  independently stated active-pane collaborator cycling and Unfollow behavior,
  Vim/Helix overlay keys, transient state, and the missing Collaboration owner.
- [`action-classification-workspace-worktree-security.md`](action-classification-workspace-worktree-security.md) —
  independently stated Restricted Mode consent, path-scoped trust, the global
  auto-trust setting, reset behavior, and the missing Labonair trust workflow.
- [`action-classification-workspace-zoom.md`](action-classification-workspace-zoom.md) —
  independently stated focused Pane/Panel zoom, Workspace overlay behavior,
  padding and persistence, the Git CommitEditor key override, and Labonair's
  separate debug dock and native window zoom routes; it also classifies the
  three registered unbound Editor-maximize and direct Zoom In/Out actions.
- [`action-classification-git-worktree-delete.md`](action-classification-git-worktree-delete.md) —
  independently stated normal and forced worktree removal, protection for
  main/open worktrees, dirty-tree confirmation, data-loss effects, and the
  missing Labonair worktree lifecycle route.
- [`action-classification-font-size-decrease.md`](action-classification-font-size-decrease.md) —
  independently stated shared-buffer and app-UI decrease effects, transient
  versus persistent arguments, context precedence, the gated primary-modifier
  wheel gesture, Settings values, and Labonair's removed no-op route and
  duplicate font-size paths.
- [`action-classification-font-size-increase-reset.md`](action-classification-font-size-increase-reset.md) —
  independently stated buffer/UI increase and reset behavior, the four-scope
  Reset All menu route, Agent-only ResetAgentZoom behavior, transient versus
  persistent effects, Settings values, context precedence, and Labonair's
  removed no-op Zoom routes.
- [`action-classification-workspace-new-window.md`](action-classification-workspace-new-window.md) —
  independently stated intent, Workspace/application-window ownership,
  `On New Window` setting behavior, the missing Labonair action route, and
  runtime/evidence requirements for the default-macOS New Window action.
- [`action-classification-workspace-new-file.md`](action-classification-workspace-new-file.md) —
  independently stated intent, Editor/Workspace ownership, New File contexts,
  local universal-tab adaptation, unsaved-buffer Settings, and action/runtime
  gaps for the default-macOS untitled-document action.
- [`action-classification-workspace-new-terminal.md`](action-classification-workspace-new-terminal.md) —
  independently stated intent, Terminal/Workspace ownership, shell/cwd
  Settings, local/remote execution, focus-dependent placement, and runtime
  gaps for the default-macOS New Terminal action.
- [`action-classification-workspace-open-with-system.md`](action-classification-workspace-open-with-system.md) —
  independently stated intent, Workspace ownership, ProjectPanel and
  InvalidBuffer contexts, the Windows deprecated-alias route, and Labonair's
  missing selected-path default-app action.
- [`action-classification-workspace-dock-sizing.md`](action-classification-workspace-dock-sizing.md) —
  independently stated Workspace dock resize/reset behavior, six action
  bindings, the JSON-only resize-all-panels setting, and Labonair's existing
  pointer route and per-edge sizing model.
- [`action-classification-workspace-dock-visibility.md`](action-classification-workspace-dock-visibility.md) —
  independently stated active-dock close, per-edge toggles, all-dock
  restoration, keybindings, and Labonair's panel/statusbar and primary-dock
  routes.
- [`action-classification-workspace-major-region-focus.md`](action-classification-workspace-major-region-focus.md) —
  independently stated forward/backward region order, wrap and closed-dock
  behavior, accessibility focus targets, and Labonair's pane/statusbar-only
  routes.
- [`action-classification-workspace-pane-activation-swap.md`](action-classification-workspace-pane-activation-swap.md) —
  independently stated zero-based pane activation, directional pane/dock
  focus, adjacent-pane swaps, and Labonair's tab-number shortcut conflict.
- [`action-classification-workspace-bulk-close.md`](action-classification-workspace-bulk-close.md) —
  independently stated cross-pane close-all and close-inactive behavior,
  pinned-item retention, save intent, partial cancellation, and Labonair's
  Space-scoped tab close routes.
- [`action-classification-workspace-close-window.md`](action-classification-workspace-close-window.md) —
  independently stated per-window close preparation, context-specific
  bindings, dirty-item/call prompts, session handling, and Labonair's native
  close and tab/pane shortcut differences.
- [`action-classification-workspace-path-copy.md`](action-classification-workspace-path-copy.md) —
  independently stated absolute and worktree-relative clipboard paths,
  owner-specific selection cardinality, path formatting, and Labonair's
  Explorer-only equivalent.
- [`action-classification-workspace-reopen-last-picker.md`](action-classification-workspace-reopen-last-picker.md) —
  independently stated eligible-picker behavior, per-window latest-view
  retention, state and focus restoration, and Labonair's missing stash route.
- [`action-classification-workspace-save.md`](action-classification-workspace-save.md) —
  independently stated active-item and Workspace-wide save scope, typed save
  intents, format-on-save settings, conflict and partial-failure behavior,
  and Labonair's active-Editor-only route.
- [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md) — stable
  IDs for 1,402 bindings across seven macOS base-profile overlays, conditional
  Vim/Helix bindings, and macOS-specific overrides. Profile behavior, action
  semantics, and native macOS dispatch remain open. It also records 133
  default-map context-block flags for key-equivalent handling.
- [`keymap-nonmacos-requirements.md`](keymap-nonmacos-requirements.md) — stable
  IDs for 446 shared Linux/Windows profile and override bindings, plus 259
  default-map key-equivalent block records. Windows runtime acceptance and
  both platforms' action semantics and dispatch remain open.
- [`registered-actions-unbound.md`](registered-actions-unbound.md) — stable IDs
  for 813 actions registered by the pinned Linux runtime but absent from all
  three default keymaps. Reachability, semantic classification, Labonair owner
  and route, and runtime behavior remain open. The complete pinned registry
  metadata and schemas are in [`evidence/zed-actions-3569541-linux-x86_64.json`](evidence/zed-actions-3569541-linux-x86_64.json).
  The JSON under `evidence/` is machine-generated reference metadata, not an
  implementation brief or proof of user-visible behavior.
- [`titlebar-tab-strip.md`](titlebar-tab-strip.md) — independent titlebar tab
  appearance, input/state matrix, ownership, and evidence packet.
- [`settings-requirements.md`](settings-requirements.md) — 488 stable IDs for
  384 static field/value rows, including five JSON-only settings, per-language
  values, shipped profiles, dynamic setup pages, and core Settings flows.
  Field effects, exact owners/routes, runtime behavior, and visual acceptance
  remain open.
- [`settings-crosswalk.md`](settings-crosswalk.md) — independent Settings
  page-to-owner map and implementation order. It records page ownership;
  field-level acceptance remains incomplete.
- [`../reports/zed-parity-settings-analysis-2026-10-09.md`](../reports/zed-parity-settings-analysis-2026-10-09.md)
  — research-side Settings source and commit review. Do not use this report as
  an implementation brief.
- [`../reports/zed-titlebar-tabbar-analysis-2026-10-10.md`](../reports/zed-titlebar-tabbar-analysis-2026-10-10.md)
  — source-side analysis of the supplied Editor tab-bar reference and current
  Labonair titlebar. The implementation packet above is the user-facing
  requirements record.
- [`../zed-parity.md`](../zed-parity.md) — normative target, clean-room rules,
  required feature-record fields, and exit criteria.

## Evidence vocabulary

| Label | Meaning |
|---|---|
| `Source checked` | The pinned tree or current Labonair source was inspected. It proves structure, not rendered behavior. |
| `Runtime observed` | The exact pinned runtime was opened and the named behavior was inspected. |
| `Capture verified` | An inspected native capture records the exact build and matching environment. |
| `Public-doc only` | A current public documentation page describes the behavior; its match to the pinned build is unverified. |
| `Pending` | Evidence is missing or the comparison has not been completed. |

## Provenance and source boundary

The current R07-000 analysis and implementation were performed in the same
contributor session. A source-blind implementation checkout was not used, so
this work is recorded as independent redevelopment and makes no claim of
separated-room clean-room provenance. That limitation applies to prior R07-000
implementation changes too; it cannot be changed retroactively by creating a
new checkout. Do not copy Zed code, comments, assets, fonts, or implementation
details.

The source-side reports remain research records and are not implementation
briefs. Future work may use a source-blind checkout when its contributor has
not received source-level reports. Implementation packets in this directory
must contain independently written user goals, observable flows, states,
owner/contracts, settings and persistence, dependencies, and acceptance
evidence. The independent package is not complete while any field is missing
or marked pending. Runtime evidence and action-level parity remain open under
R07-000.
