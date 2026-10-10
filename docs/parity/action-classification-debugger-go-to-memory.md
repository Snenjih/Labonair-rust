# Debugger Go To Memory Action Classification

**Status:** Static handler, context-menu route, memory navigation, persistence
boundary, and Labonair owner gap classified; native debugger acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned `debugger_ui` owns the Variables-list action and Memory View;
`project` owns the DAP session and memory request. Labonair's planned Debugger
capability is the intended owner, but no canonical Debugger crate or public
contract exists yet.

## Evidence boundary

`variable_list::GoToMemory` is registered in the inspected Linux runtime action
catalog and has no binding in the inspected default macOS, Linux, or Windows
keymaps or alternate profile overlays. Linux registration does not prove
macOS registration, Command Palette discovery, or native dispatch. The
canonical source route found is the “Go To Memory” item in a selected variable
row's context menu, gated by the adapter's `supports_read_memory_request`
capability. The menu does not test whether that variable has a memory
reference, so the item can be visible while invoking it has no effect.

## Requirement crosswalk

| Requirement | Pinned behavior and entry point | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-variable_list-GoToMemory](registered-actions-unbound.md) | The Linux-registered `GoToMemory` action is attached to `VariableList`'s action handler and offered only from a variable row's context menu when the session advertises `supports_read_memory_request`. Invoking it requires a currently selected entry that resolves to a DAP variable with `memory_reference`; missing selection, non-variable, or missing reference silently returns. | Labonair has no Debugger crate, DAP memory-read contract, Variables list, or Memory View. The canonical Debugger owner and its typed adapter boundary remain unresolved. |

## Pinned workflow and state

1. In an active Debugger Variables list, secondary-click a variable row. The
   row handler first stores that row's `EntryPath` as the current selection,
   then opens its context menu. The menu includes “Go To Memory” only if the
   adapter reports `supports_read_memory_request`; the menu's variable check
   does not require a non-empty `memory_reference`.
2. The handler resolves the current list selection back to an entry and
   requires that it be a DAP variable with a memory reference. It chooses an
   optional size expression from the variable type when the type consists
   only of whitespace, alphabetic characters, and `*`; otherwise it uses the
   evaluate name, removing a leading `/nat ` prefix.
3. `MemoryView::go_to_memory_reference` parses the DAP reference as an integer
   address. A parse failure leaves the Memory View address and selection
   unchanged. If a size expression exists, the DAP session requests its data
   access size in the selected stack frame; missing or failed size lookup
   falls back to one byte.
4. The view selects the inclusive address range from the starting address
   through `address + size - 1`, aligns the displayed rows within the current
   4 KiB page, scrolls the target address into view, and reads memory through
   the DAP session. The handler defers activation of the Debugger `MemoryView`
   pane item, including when the reference parse did not update the view.

The route has no keybinding in the inspected default or profile assets. The
Memory View's own address/expression query bar is a separate entry point and
does not establish a direct binding for `GoToMemory`.

The debugger layout serializer records pane arrangement and active pane item,
including the `MemoryView` item. The inspected `MemoryView` address, row width,
scroll handle, and selected byte range live in an in-memory `ViewState`; those
values are not part of the debugger layout record. No Settings value was
found for this route. No notification is emitted for a missing reference or
unparseable address in the inspected handler.

## Labonair owner and current route

The capability crosswalk assigns debugger workflows to a planned Debugger
owner and marks launch protocols, lifecycle, settings, and failure behavior
undefined. Labonair currently has no debugger runtime, DAP session, Variables
list, or memory surface. Do not place this behavior in Editor or Shell. Before
implementation, R07-001 must establish the canonical Debugger crate, DAP
transport/session contract, project-trust boundary, pane contribution,
notification route, and persistence policy.

## Acceptance still required

- Verify macOS action registration, native dispatch, and Command Palette
  discoverability separately from the Linux catalog declaration.
- Exercise adapters that do and do not support memory reads; confirm menu
  visibility for variable rows and the no-op behavior for missing references.
- Verify supported DAP memory-reference formats, parse failure behavior,
  selected-frame size lookup, one-byte fallback, inclusive highlighting,
  address alignment, DAP reads, and surfaced adapter errors.
- Characterize stale selection, stopped and historic sessions, missing or
  replaced variables, async completion after switching panes, and retry.
- Restore a Debugger layout with MemoryView active and compare persisted pane
  membership/selection against the transient address, selection, and row-width
  state. Determine whether retaining those transient values is required by the
  accepted Labonair workflow.
- Compare keyboard focus, context-menu placement, memory grid selection,
  loading/empty/error states, narrow layout, and restored layout in paired
  native macOS captures.
- Define the local DAP/project trust and permission boundary before issuing
  any memory request; capability advertisement alone is not a user grant.

## Source records inspected

- Pinned action declaration, menu, handler, and action attachment:
  `zed-refrence/zed/crates/debugger_ui/src/session/running/variable_list.rs`.
- Pinned memory-reference parsing, data-size lookup, address navigation, and
  transient view state:
  `zed-refrence/zed/crates/debugger_ui/src/session/running/memory_view.rs`.
- Pinned Debugger pane-item capability and layout serialization:
  `zed-refrence/zed/crates/debugger_ui/src/persistence.rs`.
- Linux runtime action metadata: [captured registry evidence](evidence/zed-actions-3569541-linux-x86_64.json).
- Default and profile keymaps: `zed-refrence/zed/assets/keymaps/`.
- Labonair planned owner and current gap: [`feature-crosswalk.md`](feature-crosswalk.md),
  [`docs/capabilities.md`](../capabilities.md), and the active R07-000 task.
