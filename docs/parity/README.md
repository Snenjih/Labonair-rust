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
  one Workspace zoom action, one Editor Go to Line action, one File Finder
  action, two Workspace Open/OpenFiles actions, one Workspace New Window action,
  one Workspace Add Folder to
  Project action, one Workspace New File action,
  one Workspace New Terminal action, one Workspace Open With System action,
  six Workspace dock-sizing actions, five Workspace dock visibility actions,
  two Workspace major-region focus actions, nine Workspace pane
  activation/swap actions, two Workspace bulk item/pane close actions, two
  Git worktree deletion actions, one Workspace Close Window action, one
  Workspace Reopen Last Picker action,
  two Workspace path-copy actions, and four Workspace save actions;
  pinned dispatch and native interaction acceptance remain open for those and
  all other actions.
- [`action-classification-search.md`](action-classification-search.md) —
  independently stated user goals, static Labonair owner routes, code coverage,
  and runtime/evidence gaps for 23 default-macOS Search-family actions. No
  action is marked runtime-observed.
- [`action-classification-pane.md`](action-classification-pane.md) —
  independently stated user goals, Workspace/Editor owner boundaries, current
  tab/pane routes, and evidence gaps for 22 default-macOS Pane actions. The
  split between universal tabs, outer panes, and Editor groups remains explicit;
  runtime acceptance is pending.
- [`action-classification-go-to-line.md`](action-classification-go-to-line.md) —
  independently stated intent, Editor owner, current line-jump route, and
  evidence gap for the default-macOS Go to Line action. The arbitrary
  line/column prompt and its runtime acceptance remain open.
- [`action-classification-file-finder.md`](action-classification-file-finder.md) —
  independently stated intent, Editor/Workspace ownership, current Open File
  route, related Settings targets, and runtime/evidence gaps for the single
  default-macOS File Finder action.
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
  separate debug dock and native window zoom routes.
- [`action-classification-git-worktree-delete.md`](action-classification-git-worktree-delete.md) —
  independently stated normal and forced worktree removal, protection for
  main/open worktrees, dirty-tree confirmation, data-loss effects, and the
  missing Labonair worktree lifecycle route.
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
- [`settings-requirements.md`](settings-requirements.md) — 487 stable IDs for
  383 static field/value rows, including four JSON-only settings, per-language
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
