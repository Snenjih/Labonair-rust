# Search Action Classification

**Status:** Static semantic and local-owner crosswalk for 23 default-macOS
Search-family actions; pinned runtime, local native interaction, and paired
visual acceptance remain pending  
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`  
**Owners:** Each search operation stays with the capability for its active
context; Editor owns buffer and project-search state, Workspace composes the
cross-pane entry point and filesystem adapter

## Evidence boundary

This register independently states the user-visible intent described by the
pinned action catalog and compares it with the current Labonair contracts and
code. The exact default-profile bindings and contexts remain in the [action
register](action-requirements.md); action metadata is preserved in the [pinned
registry evidence](evidence/zed-actions-3569541-linux-x86_64.json). Neither
source establishes what a macOS user sees after pressing a key.

All 23 rows below are `Source checked`, not `Runtime observed`. No row has
verified pinned dispatch, focus, result selection, error, cancellation, or
recovery behavior in a running macOS build. Labonair source/tests establish
contracts and model behavior only; native Labonair interactions and paired
screenshots are also pending. Same-session analysis and implementation remain
documented in the [parity working-record provenance](README.md#provenance-and-source-boundary).

## Ownership and entry point

| Contract field | Current Labonair record |
|---|---|
| Owner module and canonical crate | Editor — `labonair-editor` owns buffer/project query, match, result, and replacement state. Terminal search stays with `labonair-terminal`; search fields in AI, Settings, and Keymap stay with those field-owning modules. No general-purpose Search owner is introduced. |
| Composition and view integration | Workspace — `labonair-workspace` contributes the executable `Find` handler, presents the transient overlay, routes to the active Editor or Terminal, and adapts the Editor's typed project request to the filesystem provider. |
| Canonical user route | Command Palette / menu `Find` (`CommandId::Find`, macOS default `Cmd+F`) opens Workspace's SearchOverlay for the active Editor or Terminal. Search-field controls are part of their owning view. |
| Public operation boundary | Editor-owned `SearchQuery`, `SearchOptions`, `ProjectSearchQuery`, `ProjectSearchSession`, and Editor view methods. The Workspace adapter does not own match or result state. |
| Registry coverage | Labonair has one `Find` command, not a one-to-one registration for the pinned `buffer_search::*`, `project_search::*`, and `search::*` action identities. Action ownership follows the pinned context: Editor/Terminal for buffer operations, Editor for project search, File Finder's Editor owner for ignored-path behavior, and each named view owner for generic field focus/options. The pinned action-specific sequences are not currently represented as equivalent Labonair keymap commands. |
| Settings and persistence | Search options are transient overlay state. `SearchOptions::default()` is case-insensitive smart-case, literal, whole-word off, multiline off, and wrapping on. Workspace retains only the last query in process memory; it does not provide the pinned previous/next query history or durable search settings. |
| Notifications and errors | Invalid buffer regex is shown inline and disables replacement. Project-search failures also publish through Notifications. |
| Async and security boundary | Workspace runs project grep on a blocking worker, carries an Editor generation to reject stale results, caps results, and checks opened results remain under the project root. Cancellation/recovery must still be accepted end to end. |
| UI-kit | SearchOverlay uses UI-kit search/text inputs, buttons, and list rows. Native focus, pointer, narrow-window, and visual behavior remain pending. |

## Per-action semantic crosswalk

`Partial` means a related user operation exists but at least one pinned
behavior, context, keyboard route, or state is missing. `Missing` means source
inspection found no corresponding user operation in the current Labonair
route. These are static classifications, not runtime acceptance.

| Requirement | Pinned user goal, stated independently | Labonair static route, coverage, and local evidence |
|---|---|---|
| [`ACT-MACOS-buffer_search-Deploy`](action-requirements.md) | Open in-buffer find for the active editor, terminal, or Markdown preview; one binding variant can request selection-based search. | **Partial.** Workspace owns the `CommandId::Find` handler and routes to Editor or Terminal. No Markdown-preview target or configurable deploy action is present. Route: [`command_provider.rs`](../../crates/workspace/src/command_provider.rs), [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-buffer_search-Dismiss`](action-requirements.md) | Close the active find bar and end its match state. | **Present in source.** Modal dismissal calls Workspace search-end, which clears Editor search state. Route: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs), [`editor.rs`](../../crates/workspace/src/views/editor.rs). Escape dispatch and focus restoration remain runtime-pending. |
| [`ACT-MACOS-buffer_search-FocusEditor`](action-requirements.md) | Return keyboard focus to the document while leaving the find surface active. | **Missing.** SearchOverlay's focus handle is the query field; no owner action transfers focus to the underlying Editor while retaining the overlay. Native focus behavior is unverified. |
| [`ACT-MACOS-buffer_search-UseSelectionForFind`](action-requirements.md) | Replace the current query with selected text, or use the caret word when there is no selection. | **Partial.** Opening Find seeds a non-empty, single-line selection. There is no explicit selection-update command and no word-at-caret fallback. Evidence: `EditorView::search_seed` in [`editor.rs`](../../crates/workspace/src/views/editor.rs). |
| [`ACT-MACOS-project_search-OpenTextFinder`](action-requirements.md) | Open the selected project hit in a separate focused text-result picker. | **Missing.** Project hits are listed inline; activating a hit opens its file in the Editor. There is no separate text-finder picker. Route: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs), [`workspace.rs`](../../crates/workspace/src/workspace.rs). |
| [`ACT-MACOS-project_search-SearchInNew`](action-requirements.md) | Preserve the current search and run another query in a separate project-search tab. | **Missing.** The current Editor has one transient project-search session and no multi-search result tabs. Evidence: [`search.rs`](../../crates/editor/src/search.rs), [`editor.rs`](../../crates/workspace/src/views/editor.rs). |
| [`ACT-MACOS-project_search-ToggleAllSearchResults`](action-requirements.md) | Collapse or expand the result excerpts as one operation. | **Missing.** The project results list has no grouped excerpt expansion state or corresponding action. Route inspected: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-project_search-ToggleFilters`](action-requirements.md) | Show or hide project-search filters so the user can narrow where results come from. | **Missing.** Project search exposes query matching only; there are no include/exclude path fields, ignored-file switch, or collapsible filter panel. Route: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs), [`project_search.rs`](../../crates/workspace/src/project_search.rs). |
| [`ACT-MACOS-project_search-ToggleFocus`](action-requirements.md) | Switch keyboard focus between project query controls and the result collection without closing search. | **Missing.** Results support pointer selection and opening, but no explicit result-focus mode or keyboard transfer action is implemented. Route: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-search-FocusSearch`](action-requirements.md) | Put typing focus into the relevant search field in each supported search surface. | **Partial; context owner varies.** Editor, AI (`AcpThread`), Settings, and Keymap own their respective fields; opening the Workspace overlay initially focuses its query input. A reusable focus action for an already-open field is absent. Native focus behavior is pending. |
| [`ACT-MACOS-search-NextHistoryQuery`](action-requirements.md) | Replace the current query with the next stored search-history item. | **Missing.** Labonair stores one process-local last-query string, not an ordered history. Evidence: `LAST_QUERY` in [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-search-PreviousHistoryQuery`](action-requirements.md) | Replace the current query with the preceding stored search-history item. | **Missing.** Same one-value process-local cache; no previous-query list or cursor. Evidence: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-search-ReplaceAll`](action-requirements.md) | Apply the replacement to every current-buffer match or every eligible project-search result. | **Partial.** Current-file replacement is one undoable document transaction. Project scope disables replacement, so the project result workflow is missing. Evidence: `search_replace_all` in [`editor.rs`](../../crates/workspace/src/views/editor.rs); `replacement_actions_enabled` in [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs); test `search_replace_actions_use_typed_state_and_one_undo_step`. |
| [`ACT-MACOS-search-ReplaceNext`](action-requirements.md) | Replace the active match and keep the user positioned in the updated result sequence. | **Partial.** The current-file overlay replaces the active match and refreshes matches, but does not expose the pinned action identity or prove its exact key/context behavior. Evidence: `search_replace_one` in [`editor.rs`](../../crates/workspace/src/views/editor.rs); test `search_replace_actions_use_typed_state_and_one_undo_step`. |
| [`ACT-MACOS-search-SelectAllMatches`](action-requirements.md) | Create selections for all matches so a subsequent edit can affect them together. | **Missing.** Find state tracks one active match and the current Editor search contract has no operation that creates a selection set from all matches. Evidence: [`editor.rs`](../../crates/workspace/src/views/editor.rs). |
| [`ACT-MACOS-search-SelectNextMatch`](action-requirements.md) | Move the active result forward, select it in the document, and honor the query's wrap policy. | **Partial.** Editor search navigation wraps and selects a single match; overlay Enter/buttons provide related input, but the pinned `Cmd+G` and action contexts are not registered as a matching operation. Evidence: `search_step` and test `find_navigates_matches` in [`editor.rs`](../../crates/workspace/src/views/editor.rs). |
| [`ACT-MACOS-search-SelectPreviousMatch`](action-requirements.md) | Move the active result backward, select it in the document, and honor the query's wrap policy. | **Partial.** Reverse Editor navigation exists; exact pinned Shift+Enter / `Cmd+Shift+G` dispatch and context behavior are not established in Labonair. Evidence: `search_step` and test `find_navigates_matches` in [`editor.rs`](../../crates/workspace/src/views/editor.rs). |
| [`ACT-MACOS-search-ToggleCaseSensitive`](action-requirements.md) | Change matching to distinguish uppercase and lowercase exactly. | **Partial; context owner varies.** Editor's SearchOverlay case control cycles Smart, Sensitive, and Insensitive modes and re-runs matching; the pinned action also applies in AI's `AcpThread` context. It is not the same action identity or key route. Evidence: `SearchCaseMode` in [`search.rs`](../../crates/editor/src/search.rs), `toggle_case` in [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-search-ToggleIncludeIgnored`](action-requirements.md) | Let file discovery include paths normally omitted by ignore rules. The pinned default binding is in FileFinder context. | **Partial.** The local file finder has no include-ignored state or toggle; the current project grep provider also has no query option to include ignored paths. Evidence: [`file_finder.rs`](../../crates/editor/src/file_finder.rs), [`project_search.rs`](../../crates/workspace/src/project_search.rs). |
| [`ACT-MACOS-search-ToggleRegex`](action-requirements.md) | Interpret the query as a regular expression, then refresh matching results. | **Partial; context owner varies.** Editor's query model and project adapter accept regex with invalid-pattern handling, but SearchOverlay shows its regex control only for Current File scope; the pinned action also runs in AI's `AcpThread` context. Local control/key action wiring and pinned context coverage are not equivalent. Evidence: [`search.rs`](../../crates/editor/src/search.rs), [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs); test `invalid_editor_regex_is_an_inline_error_and_disables_actions`. |
| [`ACT-MACOS-search-ToggleReplace`](action-requirements.md) | Show or hide the replacement field and its actions without ending the search. | **Partial.** The replacement row is always rendered for an Editor target; no toggle state or action exists to hide and restore it. Evidence: [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs). |
| [`ACT-MACOS-search-ToggleSelection`](action-requirements.md) | Restrict matching to the selected range and toggle back to the full buffer. | **Missing.** Selection text may seed a query, but the matcher searches the whole buffer and exposes no selection-range scope. Evidence: [`search.rs`](../../crates/editor/src/search.rs), [`editor.rs`](../../crates/workspace/src/views/editor.rs). |
| [`ACT-MACOS-search-ToggleWholeWord`](action-requirements.md) | Match only complete word boundaries around the query. | **Partial.** Whole-word matching exists in the Editor query model and the current-file control. The control is hidden in project scope, and the pinned key action is not registered in Labonair. Evidence: [`search.rs`](../../crates/editor/src/search.rs), [`search_overlay.rs`](../../crates/workspace/src/search_overlay.rs); test `literal_options_and_unicode_crlf_are_position_safe`. |

## Coverage and acceptance

| Evidence class | Current evidence | Remaining gate |
|---|---|---|
| Pinned action inventory | All 23 action IDs and default macOS bindings link to [`action-requirements.md`](action-requirements.md); catalog semantics are source-checked in the pinned evidence JSON. | Run the pinned macOS build and verify each key, context, visible result, inactive context, and failure/recovery path. |
| Labonair code mapping | Each row identifies its owner path and operation or states that source inspection found no route. | Add one-to-one typed owner actions and missing user workflows only in dependency-ordered implementation tasks after R07-000/R07-001. |
| Focused code tests | `find_navigates_matches`, `search_replace_actions_use_typed_state_and_one_undo_step`, `invalid_editor_regex_is_an_inline_error_and_disables_actions`, `project_session_rejects_stale_results_and_cancellation`, `project_session_navigation_wraps_and_empty_results_are_local`, and project adapter root checks. | Tests cover model/adapters, not all 23 action bindings or end-to-end UI states. Add action dispatch and UI workflow coverage with the owning implementation task. |
| Native interaction and paired visuals | None for this slice. | Pending native macOS windows and Screen Recording access; record platform, exact binaries, viewport, theme, scale, focus, and state for both apps. |
| Runtime status | No pinned macOS action has been classified as runtime-observed; no action in this register is accepted. | Keep R07-000 open until runtime and paired native acceptance are available. |

## Next work

Continue with the next default-macOS action family and the 813 registered but
unbound actions. Independently map semantics, owner, route, state, persistence,
and evidence; do not infer runtime dispatch from action names, registry
metadata, or a passing source-level test.
