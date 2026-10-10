# File Finder Action Classification

**Status:** Static semantic and Labonair-owner crosswalk for one default-macOS
action; pinned runtime, native interaction, and paired visual acceptance remain
pending  
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`  
**Owner:** Editor owns file-query and ranking state; Workspace composes the
current view and filesystem adapter

## Evidence boundary

This packet states the user outcome behind the pinned `file_finder::Toggle`
action and compares it with Labonair's current Open File workflow. The [action
register](action-requirements.md) keeps the exact default key, context, and
arguments. Catalog and source inspection support the static description; they
do not prove key dispatch or rendered behavior.

This record is `Source checked`, not `Runtime observed`. The current host is
Linux without `DISPLAY` or `WAYLAND_DISPLAY`; pinned macOS dispatch, native
Labonair interaction, and paired captures remain unavailable. Provenance is
the same-session independent redevelopment documented in the [parity README](README.md#provenance-and-source-boundary).

## Owner and workflow contract

| Contract field | Current Labonair record |
|---|---|
| Owner module and canonical crate | Editor — `labonair-editor` owns `FileFinderQuery`, matching, ranking, result limits, and generation-bound session state. |
| View, listing, and composition | Workspace — `labonair-workspace` hosts the modal view, runs bounded filesystem listing off the foreground thread, and validates a selected path before opening it. Shell (`labonair-shell`) supplies the existing modal host and command composition only. |
| Canonical user entry point | Current `CommandId::OpenFile` / File > Open File, bound locally to `Cmd+Shift+O`. It opens the keyboard-first FileFinderView. The pinned `Cmd+P` route and repeated-action selection behavior are not mapped. |
| Public typed contract | Editor `FileFinderQuery`, `FileFinderCandidate`, `FileFinderResult`, `FileFinderSession`, `FileFinderSnapshot`, and `FileFinderStatus`; Workspace exposes a validated candidate-open operation. |
| Query and listing | The local query starts empty, uses smart-case fuzzy matching, ranks basename matches ahead of directory-only matches, and defaults to 100 results (maximum 500). The Workspace adapter enumerates a bounded set of files under its explicit root. |
| Selection and errors | The view supports keyboard movement and pointer selection. State distinguishes idle, loading, ready, empty, and error. Query generations reject stale results; dismissing cancels in-flight work. |
| Persistence and preview | Query/results are modal-local and transient. Candidate activation opens a regular Editor tab. There is no File Finder-specific Settings value currently wired into this route. |
| Settings targets | The [Settings requirements register](settings-requirements.md) lists the four `USER`-scoped values `file_finder.include_ignored` (`"smart"`), `file_finder.file_icons` (`true`), `file_finder.skip_focus_for_active_in_search` (`true`), and `preview_tabs.enable_preview_from_file_finder` (`false`). Their target effects, validation/reset/reload behavior, and Labonair parity remain unaccepted. |
| Notifications and failure | A missing root, failed listing, deleted candidate, or path outside the active workspace must remain an explicit error; opening revalidates candidate containment and file existence. |
| UI-kit | FileFinderView uses the shared input, button, and list-row controls. Native focus, pointer, disabled, narrow-window, and visual states remain pending. |

## Required states, input, and persistence checks

The pinned default is `Cmd+P` in the Workspace context. The action has no
arguments in that binding; its schema also exposes optional separate-history
and include-ignored values, whose defaults and setting interaction need
acceptance. Source inspection indicates an invocation while the picker is
already open cycles its selection. Verify all of this against the pinned
runtime. Required target states include:

- **Open and focus:** no active project, local and remote project, open file,
  closed modal, already-open modal, initial focus, repeated `Cmd+P`, and focus
  return after dismiss.
- **Query:** empty query, exact/basename/path matches, lowercase smart-case,
  uppercase case-sensitive input, clear, no results, and a truncated result
  set.
- **Results:** loading, stale responses, cancellation, unavailable root,
  filesystem failure, deleted file, out-of-root candidate, keyboard wrap,
  pointer selection, opening the active candidate, and preview versus permanent
  tab behavior.
- **Settings:** verify the four linked target values, scopes, reset behavior,
  and effects for ignored files, file icons, active-item focus, and preview-tab
  opening. Labonair has no matching File Finder value wiring today.
- **Persistence and recovery:** query and result state clear when dismissed;
  repeated invocation, failed load, project-root change, and reopen must not
  surface stale candidates. File navigation history is a separate target
  behavior and is not implemented by the current query model.

## Per-action semantic crosswalk

`Partial` means a related Labonair workflow exists but a pinned entry point,
state, setting, or repeat behavior differs. This is a static classification,
not runtime acceptance.

| Requirement | Labonair owner / canonical crate | Pinned user goal, stated independently | Labonair static route, coverage, and gap |
|---|---|---|---|
| [`ACT-MACOS-file_finder-Toggle`](action-requirements.md) | Editor (`labonair-editor`); Workspace (`labonair-workspace`) composes the view and listing | Open a project file picker, search files by name or path, choose a result, and return to the active Editor with that file open. | **Partial.** `CommandId::OpenFile` opens a shared-UI-kit modal with an empty query, bounded async listing, smart-case fuzzy ranking, loading/empty/error states, and root-safe file opening. Its local `Cmd+Shift+O` route does not match pinned `Cmd+P`; there is no per-action key descriptor, ignored-file argument/settings wiring, recent-file navigation history, file-icon setting, or preview setting. Evidence: [`command_provider.rs`](../../crates/editor/src/command_provider.rs), [`file_finder.rs`](../../crates/editor/src/file_finder.rs), [`workspace.rs`](../../crates/workspace/src/command_provider.rs), [`FileFinderView`](../../crates/workspace/src/file_finder.rs), [`actions.rs`](../../crates/shell/src/actions.rs). |

## Existing test traceability

| Related local tests | Requirement ID | Coverage limit |
|---|---|---|
| `basename_matches_beat_directory_only_matches`, `ranking_is_case_aware_and_uses_path_tie_breaks`, `empty_query_matches_every_candidate_and_respects_cap`, `no_match_produces_empty_session_state`, `stale_results_and_cancel_are_ignored`, and `selection_movement_wraps` in [`file_finder.rs`](../../crates/editor/src/file_finder.rs) | `ACT-MACOS-file_finder-Toggle` | Covers Editor matching/session primitives, not the action binding, modal focus, pointer/keyboard UI, or selected-file lifecycle. |
| `adapter_lists_only_root_safe_files_and_preserves_generation` and `adapter_reports_an_unavailable_root_as_an_error` in [`file_finder.rs`](../../crates/workspace/src/file_finder.rs) | `ACT-MACOS-file_finder-Toggle` | Covers local enumeration and root errors, not native modal interaction or pinned repeat behavior. |
| `open_file_is_not_a_second_workspace_descriptor` in [`command_provider.rs`](../../crates/workspace/src/command_provider.rs) | `ACT-MACOS-file_finder-Toggle` | Verifies that Open File remains an Editor descriptor; it does not exercise the registered modal handler or pinned key. |

No Labonair test exercises the pinned `Cmd+P` dispatch or the complete picker
workflow. There are no native interaction or paired visual artifacts for this
ID. Related model/adapter tests are not action acceptance.

## Next work

Continue with the remaining default-macOS action families and the
registered-but-unbound catalog. Keep query/ranking ownership in Editor,
filesystem and view composition in Workspace, and modal hosting in Shell.
Verify each linked Settings value through the Settings owner before treating
File Finder parity as accepted.
