# Extensions Action Classification

**Status:** Static action, owner, and view requirements; native dispatch,
catalog behavior, and paired acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** The planned Extensions capability owns extension discovery and
management. Workspace hosts its page; Keymap owns the action binding. A
canonical Labonair Extensions crate has not been assigned.

## Evidence boundary

This packet independently states the Extensions entry workflow and current
Labonair gap. Static action metadata, default keymaps, the Extensions page
contract, and local capability inventory establish the context, entry points,
view activation, and missing route; they do not establish runtime dispatch,
network recovery, installation, or visual behavior. Work in this session is
independent redevelopment, not separated-room clean-room implementation, as
recorded in the [parity README](README.md#provenance-and-source-boundary).
Native macOS and catalog acceptance remain pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Extensions owns the catalog, extension-management state, and Extensions page/action. Workspace hosts the page as an active-pane item and owns Workspace selection; Keymap owns `Workspace` bindings. Other owners may invoke the typed action with a category or ID to reach the same Extensions surface. The canonical Labonair crate and public catalog contract remain unassigned. |
| Canonical entry | `zed::Extensions` is bound in the `Workspace` context: `cmd-shift-x` on macOS and `ctrl-shift-x` on Linux/Windows. It is also reached from the application menu, titlebar global menu, and Welcome's Explore Extensions row. Category-owner routes may pass a preselected category or extension ID. |
| Action contract | The default action has no arguments and opens the unfiltered Extensions page. The typed action accepts an optional category filter (`Themes`, `IconThemes`, `Languages`, `Grammars`, `LanguageServers`, `ContextServers`, `Snippets`, or `DebugAdapters`) and an optional extension ID to focus. In the active pane, reuse and activate an existing Extensions page; update its category only when a filter is supplied and focus the matching extension only when an ID is supplied. Otherwise create and activate a page. A default invocation on an existing page leaves its current filter and search state intact. |
| Page workflow | The page provides extension search, `All`/`Installed`/`Not Installed` filters, category filters, and a scrollable extension list. Opening the page has loading, failed-fetch, and empty/no-match states. The failure state tells the user to check the connection and try again; an explicit retry control was not found and recovery needs acceptance. |
| State and persistence | Opening the page does not change Settings or install an extension. Search, filter, and focus are view state; no explicit Extensions-page serialization hook was located, so item restore is unverified. Installed extension records, package data, trust, and updates belong to the Extensions owner and are outside this action packet. |
| Events and registries | Extensions owns catalog data and the shared manager. Before implementation, decide whether its actual provider/consumer count needs a registry; a single provider/consumer should use a direct typed contract. This packet selects or adds no Labonair registry. The action delegates category-specific caller routes to the same view. |
| Settings | None identified for opening the manager or preselecting a category/ID. Extension catalog/install state does not become a Settings category. |
| Notifications and recovery | Opening the view is local navigation. Catalog fetch failure is shown in the Extensions surface; install/update failures and passive user messages must follow the future Extensions and notification-center contracts. Retry behavior remains open. |
| UI kit | A Labonair Extensions view must compose the shared UI-kit search/input, toggle group, category buttons, list, extension rows, and error/empty presentation. Workspace hosts the view and must not own a local copy of extension state or controls. |
| Current Labonair route | No Extensions capability crate, management page, catalog store, typed action, menu contribution, or keymap route exists. The `extensions` crosswalk remains Missing; the roadmap defers marketplace/distribution work until the core is stable. Theme catalog and import helpers do not provide an Extensions manager. |

## Action requirement and current route

| Requirement ID | Binding and entry | User-visible outcome | Current Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-zed-Extensions`](action-requirements.md) | `cmd-shift-x` (`Workspace`) on macOS; `ctrl-shift-x` (`Workspace`) on Linux/Windows. Also App menu, titlebar global menu, and Welcome's Explore Extensions. | Open or activate the Extensions manager in the active pane; default to all categories, with typed category/ID variants for focused entry. | No Extensions page/action/catalog owner or local shortcut/menu route exists. Assign the canonical Extensions crate and registry contract before implementation. |

## Required user-flow and state acceptance

- Open Extensions from each entry point with no page present, then with an
  existing filtered page in the active pane. Confirm it activates the existing
  item and a default action preserves its current query/category state.
- Invoke category-filter and extension-ID variants from Theme, language,
  snippet, debugger, context-server, and agent callers. Confirm each reaches
  the same Extensions owner and the intended result set/focus.
- Verify search, All/Installed/Not Installed, category filtering, scrolling,
  loading, empty/no-match, offline/fetch failure, retry, and install/update
  result states. Confirm long lists remain responsive and bounded.
- Close and restore a Workspace containing Extensions. Record whether the
  view and its query/filter state return; do not claim persistence without
  observed evidence.
- Capture paired native macOS states for App/titlebar/Welcome entry, active
  pane tab creation and reuse, category filters, search, loading, empty,
  failed fetch, narrow layout, and installation result. Record build, OS,
  window/pane state, category, extension ID, focus, and network state.

## Acceptance evidence still required

Static metadata and code review establish only the action parameters,
bindings, entry routes, and view activation contract. Extension discovery,
search, loading/retry, installation permissions, update behavior, persistence,
native dispatch, and paired visual acceptance remain unverified. The action
packet does not approve implementing marketplace or distribution workflows
before the core is stable.
