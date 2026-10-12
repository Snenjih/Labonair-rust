# TextFinder Group and Handoff Action Classification

**Status:** Pinned grouped-result behavior, profile overrides, persistence, and Labonair owner routes classified; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Search owns TextFinder query, matches, group selection, and collapse state; Workspace hosts its modal and ProjectSearch tab. In Labonair, Editor owns query/project-result/document-fold state and Workspace hosts SearchOverlay and project-search integration.

## Evidence boundary

The four registered-unbound action IDs are `ToProjectSearch`, `Fold`, `Unfold`, and `ToggleFoldAll`. None has a default macOS, Linux, or Windows key binding. All four appear in the macOS-specific override layer and the shared Linux/Windows override layer, with the TextFinder context:

`TextFinder || (TextFinder > Picker > Editor) || (TextFinder > Picker > menu)`

The macOS layer binds `alt-cmd-f`, `alt-cmd-[`, `alt-cmd-]`, and `cmd-shift-enter`. The shared Linux/Windows layer binds `ctrl-alt-f`, `ctrl-{`, `ctrl-}`, and `ctrl-shift-enter`. That is four action entries in each of the two override layers. The same `alt-cmd-f` / `ctrl-alt-f` chord is used by the separate, default-bound `text_finder::Toggle` action in its opening context; native context precedence has not been observed.

The macOS override block sets `use_key_equivalents=true`; the shared Linux/Windows block does not set that field.

These action IDs and profile rows are linked from the [registered-unbound action inventory](registered-actions-unbound.md), [macOS layer register](keymap-overlay-requirements.md), and [Linux/Windows layer register](keymap-nonmacos-requirements.md). Static bindings do not prove the selected profile loads or the shortcut dispatches.

## Pinned action behavior

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-text_finder-ToProjectSearch](registered-actions-unbound.md) | From the TextFinder action menu, “Open as Tab” invokes this action. It carries the current query, options, results, and any active search stream into a focused ProjectSearch item without rerunning the search, dismisses the modal, and completes transfer of result excerpts. It is the modal-to-tab direction; the existing Search packet covers the separate ProjectSearch-to-TextFinder action. | SearchOverlay can show Editor project results inline, but there is no TextFinder modal, separate ProjectSearch result tab, or action to move current results between them. |
| [ACT-REGISTRY-LINUX-text_finder-Fold](registered-actions-unbound.md) | Collapse the file group containing the selected result or group header. A separator/no selection is a no-op. After folding, selection moves to that group header. | Editor exposes symbol-derived code-fold markers and persists Editor fold ranges. SearchOverlay has no file-group collapse state or matching action. These document folds are a different workflow. |
| [ACT-REGISTRY-LINUX-text_finder-Unfold](registered-actions-unbound.md) | Expand the file group containing the selected result or group header. A separator/no selection is a no-op. After expanding, selection moves to the first match in that group. | No equivalent grouped search-result expansion action was found. Editor code folding remains a separate state. |
| [ACT-REGISTRY-LINUX-text_finder-ToggleFoldAll](registered-actions-unbound.md) | If no result groups are collapsed, collapse every group with matches. If any group is already collapsed, clear all collapsed state and expand every group. This is all-or-none when starting from a partially collapsed result list. | No project-search result group state or expand/collapse-all action was found in SearchOverlay. |

The TextFinder Delegate groups matches by project path and stores collapsed paths separately from query/match state. Fold/Unfold update that group state and rebuild picker entries; ToggleFoldAll derives its behavior from whether the collapsed-path set is empty.

## Query seeding and persistence

The Search & Files setting [SET-search-files-seed-search-query-from-cursor](settings-requirements.md) defaults to `always` for its consumers. TextFinder explicitly asks a searchable item for a `SeedQuerySetting::Selection` suggestion. This override accepts an explicit selection, ignores a bare word under the cursor, and does not use the global setting value. If no active item query is available, TextFinder can seed from its persisted last query. TextFinder is not a consumer of the generic setting; do not add it to that setting's consumer list.

Pinned TextFinder stores the last query and search-option bits per persisted Workspace in TextFinderDb when the modal is dismissed. The database row cascades when its Workspace is deleted. Active result groups, selection, and current asynchronous task are transient; ToProjectSearch transfers the current task/results into the tab path.

Labonair SearchOverlay uses an Editor selection as its query seed and otherwise falls back to a process-local last query. No matching cursor-seed setting was found. Labonair project results are owned by Editor and rendered inline in SearchOverlay; they do not have TextFinder's per-path picker groups or persisted query/options record. Editor document-fold ranges are included in Editor session snapshots but are not search-result group state.

## Ownership and entry points

| Contract field | Pinned route | Labonair route |
|---|---|---|
| Canonical owner | Search crate owns TextFinder and the grouped project-result picker; Workspace owns modal/tab composition. | Editor owns query, project-result and document-fold state; Workspace hosts SearchOverlay and filesystem adaptation. |
| Opening and conversion | The separate default-bound `text_finder::Toggle` opens the modal. The picker action menu exposes “Open as Tab”; ToProjectSearch carries results into the ProjectSearch view. | Command Palette/menu Find opens SearchOverlay. Search scope switches between current Editor and Editor project search; results remain in the overlay. |
| State and persistence | TextFinderDelegate owns query, matches, selection, and collapsed paths. TextFinderDb saves query/options per persisted Workspace on dismissal. | Editor owns typed search/fold state. Workspace remembers only the last search query in process memory; no TextFinder result group state or conversion action exists. |
| Settings | The generic query-seed setting is overridden with Selection for TextFinder. | No matching setting. SearchOverlay seeds an explicit Editor selection only. |
| Commands and keymap | Four actions are unbound in platform default maps and bound in eight platform-specific override rows; preserve contexts and same-chord Toggle/ToProjectSearch layering. | No corresponding TextFinder action IDs or profile-keymap route were found. |

## Acceptance still required

- Verify all four profile actions with macOS and Linux/Windows override layers selected; verify context activation, same-chord Toggle/ToProjectSearch precedence, profile disabling, user overrides, and platform dispatch separately.
- Exercise Fold and Unfold with a selected match, selected header, separator, empty results, and multiple matches in a file. Verify the selection destination after collapse/expand.
- Exercise ToggleFoldAll from expanded, partially collapsed, and fully collapsed result groups; verify the all-or-none behavior and result updates during an active search.
- Invoke ToProjectSearch with completed results, an active stream, no current query, cancellation, and a superseding query. Verify query/options, selection, results, focus, modal dismissal, and continuation without a second search.
- Exercise the reverse ProjectSearch-to-TextFinder action independently; it removes the old ProjectSearch tab and opens the modal with its query and filters. Verify this pair without conflating the two action IDs.
- Verify the seed-search setting’s scope and all enum values on its actual consumer surfaces. Confirm TextFinder continues to use explicit-selection override and then persisted-query fallback regardless of that generic value.
- Compare Labonair’s current Editor/Terminal SearchOverlay, inline project results, persisted document code folds, and process-local query retention. Do not count these as TextFinder group-fold or modal-to-tab parity.
- Capture paired native macOS TextFinder open, grouped results, selected/folded/unfolded states, query-seed choices, and modal-to-tab transfer. Static inspection does not prove rendered layout or shortcut reachability.

## Source records inspected

- Pinned action and database behavior: `zed-refrence/zed/crates/search/src/text_finder.rs`, `text_finder/delegate.rs`, and `text_finder/render.rs`.
- Pinned profile overrides: `zed-refrence/zed/assets/keymaps/specific-overrides-macos.json` and `specific-overrides.json`; opening action in the default platform maps.
- Pinned query seed: `zed-refrence/zed/crates/editor/src/items.rs`, Editor settings, and [Settings requirement](settings-requirements.md).
- Labonair search and folding: `crates/workspace/src/search_overlay.rs`, `crates/workspace/src/workspace.rs`, `crates/workspace/src/views/editor.rs`, and `crates/editor/src/search.rs`.
- Related records: [Search action classification](action-classification-search.md), [macOS keymap overlays](keymap-overlay-requirements.md), [Linux/Windows keymap overlays](keymap-nonmacos-requirements.md), and [Search & Files settings crosswalk](settings-crosswalk.md).
