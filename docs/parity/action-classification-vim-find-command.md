# Vim FindCommand Action Classification

**Status:** Static Ex-search route and local owner gap classified; palette dispatch, platform behavior, and end-to-end acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Requirement IDs:** `ACT-REGISTRY-LINUX-vim-FindCommand`; colon entry `KEYBIND-VIM-HELIX-8a40bf95c177`; direct interactive search entries `KEYBIND-VIM-HELIX-3a0758fa56d6` and `KEYBIND-VIM-HELIX-fbe3f183edbe`

**Owner:** Vim owns Ex parsing and search command dispatch; Editor owns the active buffer/searchable item, BufferSearchBar, matches, and selection; Keymap owns the shared colon and search bindings; Settings owns smartcase and search-wrap values; Command Palette hosts the query and invokes the command interceptor.

## Evidence boundary

The pinned Linux action registry declares `vim::FindCommand` with two required fields, `query: string` and `backwards: boolean`, no extra fields, and no deprecated aliases. No platform-default map or shared profile row directly binds this parameterized action. The shared Vim/Helix profile instead binds `:` in Vim Normal mode to `command_palette::Toggle`. The Vim command interceptor strips leading colons, parses an optional range prefix, and returns `FindCommand` when the remaining Ex query starts with `/` or `?`.

This is distinct from the direct Vim Normal `/` and `?` profile bindings to `vim::Search`, which open the interactive search route. The FindCommand action is constructed with the typed query and direction by the Ex interceptor. Since both action fields are required, it cannot be instantiated as an argument-free action-list entry; that does not make the Ex route unreachable. Counted colon input is a separate `vim::CountCommand` range-prefill route and is classified in the [CountCommand packet](action-classification-vim-count-command.md).

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-vim-FindCommand](registered-actions-unbound.md) | Required-query action returned by the Vim command interceptor for `:/pattern` or `:?pattern`; the `?` form searches backward. | Labonair has an interactive Vim `/` and `?` search, but no Ex `FindCommand`, command interceptor, or BufferSearchBar handoff. |
| [KEYBIND-VIM-HELIX-8a40bf95c177](keymap-overlay-requirements.md) | Shared `:` binding in Vim Normal opens Command Palette input; the interceptor turns a typed Ex search into a parameterized action result. | Labonair has a generic Command Palette but no Vim colon-to-Ex-search result route. |
| [KEYBIND-VIM-HELIX-3a0758fa56d6](keymap-overlay-requirements.md) and [KEYBIND-VIM-HELIX-fbe3f183edbe](keymap-overlay-requirements.md) | Direct `/` and `?` bindings dispatch `vim::Search`, a separate interactive search action with direction/regex parameters. | Labonair's direct Vim search is partial and does not implement the pinned BufferSearchBar or Ex-interceptor flow. |
| [SET-keymap-vim-mode](settings-requirements.md) | Vim mode enables the shared profile and command interceptor. Editor `use_smartcase_search` selects smartcase behavior; `search_wrap` controls search-bar boundary wrapping. | Labonair has local Vim smartcase and `hlsearch` options, but its search route and option behavior are not equivalent. |

## Pinned Ex-search workflow

In Vim Normal mode, `:` opens Command Palette input. Typing `:/pattern` or `:?pattern` enters the Vim command-interceptor route. After removing leading colons and any parsed range prefix, the interceptor constructs `FindCommand` from the text after `/` or `?`; the latter sets `backwards = true`. The slash is syntax and is not part of the search query. The `query` and `backwards` fields are required by direct action serialization.

The action looks up the current pane and its `BufferSearchBar`; if either the pane or search bar is unavailable, it returns. It calls `show()`, which must find an active searchable item. For an empty action query, it reuses the current search-bar query. Otherwise it replaces the query editor contents. The action explicitly requests regex and case-sensitive search. When `use_smartcase_search` is enabled, case sensitivity is overridden to true only if the effective query contains an uppercase character; with smartcase disabled, the action remains case-sensitive.

Search runs asynchronously through the active searchable item and adds the query to that BufferSearchBar's bounded history. The history holds at most 50 entries and replaces its previous entry when the new query contains that previous query. It is held in the BufferSearchBar rather than written to durable search settings or WorkspaceDb. When the search completes, the action selects one next match for `/` or one previous match for `?`. The Editor `search_wrap` setting determines whether reaching the end wraps; with wrapping disabled, the search bar reports no more matches. No matches leave no active match. An invalid regular expression sets the search bar's query error, clears active matches, and prevents the follow-up selection.

The action does not edit the buffer. It updates the active search UI and selection. The search bar stays visible after `show()`; the action does not add a separate notification or toast route. Static source inspection establishes these calls but does not establish focus, result ordering, status feedback, or native command-palette dispatch.

## Labonair owner and current route

The canonical owner for a complete route is `labonair-editor`; `labonair-keymap` should own the Vim Normal colon profile entry, and the existing Command Palette/interceptor boundary should carry typed Ex results. Editor settings own the smartcase and wrap values. No new shell surface or dependency is implied.

Labonair's [`vim.rs`](../../crates/editor/src/vim.rs) handles direct `/` and `?` keystrokes by opening its Vim command line. On Enter it stores a single in-memory `last_search` and calls `run_search`; `n`/`N` repeat in the remembered direction. `run_search` builds a `SearchQuery` with the default literal-regex setting, applies local smartcase, optionally stores all hits for `hlsearch`, then moves the caret to the next/previous match and wraps to the first/last match at a boundary. It does not show the generic Workspace SearchOverlay or a BufferSearchBar.

The local Vim command parser handles `:set` and substitution, among other Ex tokens, but has no `:/pattern` or `:?pattern` dispatch. There is no typed FindCommand action, interceptor result, or handoff into the active pane's search bar/history. Labonair's generic `Find` SearchOverlay is a separate Workspace workflow, not this Vim command route.

## Pinned fixtures inspected

- `command.rs::test_command_search` checks `:/b` moving to the next `b` and `:?a` moving backward to the prior `a`.
- `normal/search.rs` has focused interactive-search and BufferSearchBar option paths for the separate `vim::Search` action.

Pinned fixtures and source were inspected, not run. The command fixture does not establish regex edge cases, smartcase, empty-query reuse, history, wrap settings, invalid-pattern UI state, focus behavior, or cross-platform dispatch.

## Acceptance still required

- Dispatch `:` from Vim Normal on macOS, Linux, and Windows. Verify `:/pattern` and `:?pattern` become the dynamic FindCommand route, and verify direct `/` and `?` remain the separate interactive `vim::Search` route. Check counted colon against the [CountCommand packet](action-classification-vim-count-command.md).
- Verify required action arguments and result selection, including direct invocation with missing/malformed fields, a missing pane/search bar/active searchable item, and an already visible BufferSearchBar.
- Verify literal metacharacters and regex patterns, invalid regex feedback, empty-query reuse and empty-with-no-prior-query behavior, uppercase/lowercase smartcase with the setting enabled and disabled, and the interaction with configured search defaults.
- Verify forward and backward selection from different cursor positions, no match, first/last match, wrapping on and off, repeated results, and the no-more-matches feedback. Record the selected range and caret after each route.
- Verify query-editor updates, search-bar visibility and focus, acceptance/cancellation, modal behavior, focus restoration, history capacity and replacement behavior, and whether history survives pane closure/recreation or application restart.
- Capture paired native macOS Editor, Keymap, and Command Palette evidence with platform, Vim/profile setting, query, direction, match state, case/wrap settings, theme, scale, and viewport recorded.

## Source records inspected

- Action schema and registry metadata: `docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`.
- Shared colon and direct `/`/`?` bindings: `zed-refrence/zed/assets/keymaps/vim.json` and `docs/parity/keymap-overlay-requirements.md`.
- Action handler, Ex interceptor, and `test_command_search`: `zed-refrence/zed/crates/vim/src/normal/search.rs` and `zed-refrence/zed/crates/vim/src/command.rs`.
- Search bar visibility, option/query update, async match selection, errors, and bounded history: `zed-refrence/zed/crates/search/src/buffer_search.rs`.
- History insertion behavior: `zed-refrence/zed/crates/project/src/search_history.rs`.
- Search smartcase/wrap Settings: `zed-refrence/zed/crates/editor/src/editor_settings.rs` and `zed-refrence/zed/crates/search/src/buffer_search.rs`.
- Local Vim command-line search and query options: `crates/editor/src/vim.rs` and `crates/editor/src/search.rs`.

Same-session pinned-source research and independently written requirements are redevelopment, not separated-room clean-room implementation. This packet records static evidence only and does not claim runtime acceptance.
