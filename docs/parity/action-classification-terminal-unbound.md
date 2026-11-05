# Terminal Registered-Unbound Action Classification

**Status:** Static action metadata, discovered handlers, alternate routes, and Labonair gaps classified; native discovery and interaction pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned TerminalView owns Terminal tab rename and Terminal view actions; the Terminal model owns buffer selection and scroll/search state. Labonair `labonair-terminal` owns Terminal session behavior and `labonair-workspace` hosts the Terminal view and universal tab rename/search surfaces.

## Evidence boundary

These five stable Linux-runtime IDs are registered but have no binding in any of the default macOS, Linux, or Windows keymaps: `RenameTerminal`, `ScrollHalfPageUp`, `ScrollHalfPageDown`, `SearchTest`, and `SelectAll`. No matching occurrence was found in the inspected alternate keymap/profile overlays. Linux registration alone does not establish registration on macOS, discoverability, a handler, or a runtime effect. The rows remain in the [registered-unbound action inventory](registered-actions-unbound.md).

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-terminal-RenameTerminal](registered-actions-unbound.md) | `terminal::RenameTerminal` has a TerminalView handler and a tab-context-menu “Rename” entry only when the Terminal has no task; the handler also refuses task-backed Terminals. It opens an inline single-line editor seeded from custom title or the dynamic terminal title. Enter saves trimmed input; Escape or losing focus cancels. Empty input or text equal to the dynamic title clears the custom title. | Workspace exposes a generic tab-context-menu Rename flow for its universal tabs, including Terminal workspace tabs; Enter commits, Escape cancels, and blank input clears the custom label. This updates Workspace tab metadata and snapshots. There is no Terminal-specific action ID or task-based restriction. The generic route is an owner-compatible workflow, not evidence of the named action's reachability or exact behavior. |
| [ACT-REGISTRY-LINUX-terminal-ScrollHalfPageUp](registered-actions-unbound.md) | The action declaration describes scrolling up by half a page. No matching handler, call site, default binding, or alternate profile binding was found in the pinned source. A runtime effect is therefore unresolved. | Local Terminal supports wheel-driven scrolling and session scroll operations, but no named half-page action or matching local binding was found. |
| [ACT-REGISTRY-LINUX-terminal-ScrollHalfPageDown](registered-actions-unbound.md) | The action declaration describes scrolling down by half a page. No matching handler, call site, default binding, or alternate profile binding was found in the pinned source. A runtime effect is therefore unresolved. | Same local wheel/session scroll routes; no named half-page action or matching local binding was found. |
| [ACT-REGISTRY-LINUX-terminal-SearchTest](registered-actions-unbound.md) | Its action documentation says it searches Terminal text, but no matching handler or call site was found. The implemented Terminal search entry is the separate `buffer_search::Deploy` route; do not attribute it to `terminal::SearchTest`. | Workspace `SearchOverlay` can target the active Terminal and delegates matching to its Terminal view/session. This is an existing search workflow, with no matching local `SearchTest` action. |
| [ACT-REGISTRY-LINUX-terminal-SelectAll](registered-actions-unbound.md) | The action declaration describes selecting all Terminal text, but no handler or call site for `terminal::SelectAll` was found. TerminalView instead handles the distinct `editor::SelectAll` action by selecting all Terminal text; its context-menu item also uses `editor::SelectAll`. | Local Terminal supports pointer-drag range selection and copy, but no select-all API, action route, or matching binding was found in the Terminal view/session. |

## Persistence and settings

Pinned Terminal rename marks the Terminal item for serialization; its serialized item record saves the custom title by item and Workspace identity and restores it during deserialization. Labonair's generic custom tab title is included in its Workspace tab/session snapshot. These storage paths serve similar label persistence but are not interchangeable contracts. No setting affecting the five action declarations or routes was found; this classification adds no Settings requirement.

The local SearchOverlay keeps its last query in a process-local static slot. Match, selection, and scroll position are live view/session state; no durable persistence behavior for those states is asserted here.

## Acceptance still required

- Confirm action discoverability and native dispatch for all five Linux-registered IDs; establish whether the two half-page and SearchTest declarations without handlers are inert, stale, or dynamically dispatched elsewhere, and whether `terminal::SelectAll` has any route beyond the separate `editor::SelectAll` handler.
- Exercise Rename from the Terminal tab context menu on normal and task-backed terminals; verify title seeding, Enter, Escape, blur, whitespace trimming, dynamic-title reset, persistence after Workspace restore, and the separate generic Labonair tab rename route.
- Verify Terminal search through `buffer_search::Deploy` independently of `SearchTest`; compare query navigation, wraparound, empty results, and active-match rendering with the local Workspace SearchOverlay Terminal target.
- Establish the pinned effect, if any, for each half-page action. Compare against local wheel and scrollbar movement without treating pointer scrolling as a keyboard-action equivalent.
- Verify `editor::SelectAll` while the pinned Terminal is focused and its context-menu entry; establish whether `terminal::SelectAll` can be invoked independently. In Labonair, verify pointer selection/copy and determine the missing keyboard-select-all behavior.
- Capture paired native macOS Terminal tab, context-menu, rename-edit, search-overlay, selection, and scroll states. Static source review does not prove runtime reachability or visual parity.

## Source records inspected

- Pinned action declarations: `zed-refrence/zed/crates/terminal/src/terminal.rs`; TerminalView rename, editor-owned SelectAll, context-menu action, search, and serialized-item paths: `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs` and the Terminal search action/keymap records.
- Pinned default and profile keymaps: `zed-refrence/zed/assets/keymaps/`; Linux action metadata: [captured registry evidence](evidence/zed-actions-3569541-linux-x86_64.json).
- Labonair Terminal selection/search/session routes: `crates/workspace/src/views/terminal.rs`, `crates/workspace/src/search_overlay.rs`, and `crates/terminal/src/`.
- Labonair generic tab title editor and restore snapshot: `crates/workspace/src/workspace.rs` and `crates/workspace/src/tabs.rs`.
- Related registers: [registered-unbound actions](registered-actions-unbound.md), [action requirements](action-requirements.md), and [capability crosswalk](feature-crosswalk.md).
