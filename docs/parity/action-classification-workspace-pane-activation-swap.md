# Workspace Pane Activation and Swap Action Classification

**Status:** Static action, pane ownership, and local-route crosswalk for nine
actions; geometric dispatch, view cloning, topology restore, and paired native
interaction remain pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns outer pane focus, pane order,
directional focus, and split-tree position. The active view owner determines
whether its content can be cloned into a new pane. Editor-internal groups are a
separate Editor capability.

## Evidence boundary

This packet distinguishes three different operations: activate a numbered
center pane, move focus in a direction between panes and adjoining docks, and
swap the position of two center panes. A missing numeric slot can create one
cloned split when the active view supports it. Static inspection identifies
the action contract and Labonair's tab/pane routes; it does not prove native
key dispatch, geometry choices, clone support, or restored topology.

The requirements are independently stated for Labonair's Workspace model.
This is same-session independent redevelopment, not separated-room clean-room
work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
Pinned macOS interaction and paired captures are unavailable on this Linux host
without `DISPLAY` or `WAYLAND_DISPLAY`.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner module and canonical crate | Workspace (`labonair-workspace`) owns center-pane indices, split topology, active-pane focus, directional focus, and pane swaps. Each view owner provides a focus target and clone/split capability. Editor owns editor-internal groups. |
| Canonical user entry points | Numbered pane activation: macOS `Cmd+1` through `Cmd+9` with action arguments 0 through 8; Linux/Windows `Alt+1` through `Alt+9` with the same zero-based arguments. Directional focus: macOS `Cmd+K` then `Cmd+Arrow`; Linux/Windows `Ctrl+K` then `Ctrl+Arrow`. Directional swap: macOS `Cmd+K` then `Shift+Arrow`; Linux/Windows `Ctrl+K` then `Shift+Arrow`. All actions use Workspace context. |
| Numbered activation | Focus the center pane at the requested zero-based position in current pane order. If no pane occupies that position, attempt one right-side split that clones the active view. If the active pane has no active item or that item cannot be cloned, do nothing. One invocation creates at most one split. |
| Directional activation | Move focus to a neighboring center pane in the requested direction when one exists. At the center boundary, focus an open adjoining dock or applicable sidebar in that direction. From a dock/sidebar, focus a Workspace neighbor in that direction; with no eligible target, do nothing. This changes focus, not pane topology. |
| Directional swap | Exchange the Workspace's active center pane position with the geometrically adjacent center pane in the requested direction, regardless of whether focus currently rests in a dock. A dock is not a swap target. If no adjacent center pane exists, do nothing. Keep pane identity and its view content intact as its position changes; verify the focused control remains usable. |
| State and persistence | Existing-index and directional activation change focus only. Swapping changes center split positions without opening/closing docks or changing view contents. An out-of-range numeric request can add one split and cloned view. Persist any resulting topology and verify restore. |
| Events and registry | Workspace handles the nine actions using the existing pane/dock contracts. No new event, provider registry, or dependency is required for this inventory slice. |
| Settings | No new setting. Current pane order and split layout remain Workspace state; user keymap overrides are covered by keymap acceptance. |
| Notifications and failure | No adjacent target is a no-op. An unavailable clone makes an out-of-range numeric request a no-op. Do not report normal boundary navigation as an error. |
| UI-kit and input | Reuse existing pane, split, dock, and focus surfaces. Add no local directional buttons or pane-specific keybinding style. Keep this Workspace pane tree distinct from Editor's internal group controls. |

## Action requirements

| Requirement ID | Platform binding and argument | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-ActivatePane`](action-requirements.md) | macOS `Cmd+1..9`, args `0..8`; Linux/Windows `Alt+1..9`, args `0..8` | Focus the indexed center pane; for a missing index, attempt a single right split cloning the active view. | `Cmd+1..9` selects tabs by index; there is no numeric pane activation action. |
| [`ACT-MACOS-workspace-ActivatePaneDown`](action-requirements.md) | macOS `Cmd+K Cmd+Down`; Linux/Windows `Ctrl+K Ctrl+Down` | Focus a pane below, or an eligible lower dock from the center boundary. | No directional pane/dock focus action. |
| [`ACT-MACOS-workspace-ActivatePaneLeft`](action-requirements.md) | macOS `Cmd+K Cmd+Left`; Linux/Windows `Ctrl+K Ctrl+Left` | Focus a pane to the left, or an eligible left dock/sidebar at the boundary. | No directional pane/dock focus action. |
| [`ACT-MACOS-workspace-ActivatePaneRight`](action-requirements.md) | macOS `Cmd+K Cmd+Right`; Linux/Windows `Ctrl+K Ctrl+Right` | Focus a pane to the right, or an eligible right dock/sidebar at the boundary. | No directional pane/dock focus action. |
| [`ACT-MACOS-workspace-ActivatePaneUp`](action-requirements.md) | macOS `Cmd+K Cmd+Up`; Linux/Windows `Ctrl+K Ctrl+Up` | Focus a pane above; no upper dock is implied at the center boundary. | No directional pane/dock focus action. |
| [`ACT-MACOS-workspace-SwapPaneDown`](action-requirements.md) | macOS `Cmd+K Shift+Down`; Linux/Windows `Ctrl+K Shift+Down` | Swap with the adjacent center pane below, if one exists. | No directional center-pane swap action. |
| [`ACT-MACOS-workspace-SwapPaneLeft`](action-requirements.md) | macOS `Cmd+K Shift+Left`; Linux/Windows `Ctrl+K Shift+Left` | Swap with the adjacent center pane to the left, if one exists. | No directional center-pane swap action. |
| [`ACT-MACOS-workspace-SwapPaneRight`](action-requirements.md) | macOS `Cmd+K Shift+Right`; Linux/Windows `Ctrl+K Shift+Right` | Swap with the adjacent center pane to the right, if one exists. | No directional center-pane swap action. |
| [`ACT-MACOS-workspace-SwapPaneUp`](action-requirements.md) | macOS `Cmd+K Shift+Up`; Linux/Windows `Ctrl+K Shift+Up` | Swap with the adjacent center pane above, if one exists. | No directional center-pane swap action. |

## Required user-flow and state acceptance

- With one through nine center panes, invoke each numbered key and verify its
  zero-based action argument selects the corresponding pane in the current
  pane order. Verify selection changes focus without changing tabs, dock
  visibility, contents, or split geometry.
- Request a pane index that does not exist. Verify at most one new pane is
  created to the right with a clone of the active view when cloning is
  supported. Verify empty and non-cloneable active views leave the layout
  unchanged. Repeat calls to check how the pane count and indices advance.
- In regular, nested, unequal, and cursor-offset splits, activate panes in
  each direction. Verify which adjacent pane receives focus. With no center
  neighbor, verify direction-appropriate dock/sidebar focus, closed/empty dock
  handling, sidebar on either side, and no-op behavior at unavailable edges.
- Swap in all four directions across simple and nested layouts. Verify only
  the adjacent pane positions exchange, view contents and active identity
  remain intact, docks are untouched, and a missing neighbor is a no-op. Invoke
  swaps while a dock has focus and verify the operation still targets the
  active center pane without stealing dock focus.
- Save and restore a layout after swaps and after numeric clone fallback.
  Verify pane order, split proportions, active item, view-owner state, and
  recovery when the cloned view no longer supports splitting.
- Verify every platform's multi-stroke sequence, held modifiers, keymap
  overrides, and inactive contexts. On macOS, check the `Cmd+1..9` collision
  with Labonair's current `SelectTab1..9` route; verify users can distinguish
  selecting a tab from activating a pane.
- Keep Workspace outer panes separate from Editor groups. Exercise views with
  different clone/split capabilities, dirty state, and restore behavior.
  Compare paired native macOS captures for pane identity, position, focus,
  docks, split geometry, and narrow-window constraints.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [`FocusNextPane` and `focus_next_pane`](../../crates/workspace/src/command_provider.rs) | Cycles split leaves in the active Workspace tab. | The current recursive outer pane tree is exercised for Terminal workspaces; this command does not select by index, move focus spatially, or exchange pane positions. |
| [`SelectTab1..9` handlers and descriptors](../../crates/workspace/src/command_provider.rs) | Select tab indices 0 through 8; descriptors bind `Cmd+1..9`. | These shortcuts conflict semantically with pinned numeric pane activation; they select different Workspace entities. |
| [`SplitRight` and `SplitDown` handlers](../../crates/workspace/src/command_provider.rs) | Create a new pane in two orientations in eligible Workspace contexts. | No missing-index clone fallback, four-way directional activation, or pane-swap action is registered. |
| [`WorkspaceLayout` pane tree](../../crates/workspace/src/pane_group.rs) | Represents nested horizontal/vertical pane splits and order. | Model support is not proof that a user can reach an action or that cross-capability views restore correctly. |
| [Pane and Editor-group crosswalk](action-classification-pane.md) | Defines the outer Workspace/Editor ownership boundary and known split limitations. | This packet adds action-specific index, directional-focus, and swap behavior; the pane matrix remains partial. |

No Labonair test or runtime trace covers the nine pinned action routes. The
existing pane model, tab-number commands, and right/down split commands do not
establish numeric pane activation, spatial focus, adjacent-pane swapping, or
their native key behavior.

## Acceptance evidence still required

For each trace, record build and commit, OS, exact action argument and key
sequence, focused view, pane order and geometry, open docks, whether the
requested neighbor/index existed, clone capability, before/after pane identity
and contents, focus result, and saved layout. Include paired native macOS
captures with scale, theme, and window bounds. Numeric tab selection, a split
tree model, or pane cycling alone does not prove action acceptance.
