# ADR 0004 — Keep editor-internal splits inside the Editor capability

* **Status:** Accepted
* **Date:** 2026-09-12
* **Deciders:** project owner and implementation team
* **Related:** [`capabilities.md`](../capabilities.md), [`modules.md`](../modules.md), [`feature-lifecycle.md`](../feature-lifecycle.md), [`0003-modular-product-architecture.md`](0003-modular-product-architecture.md)

## Context

The Editor is currently hosted as a Workspace tab and its GPUI adapter is in
`labonair-workspace`. The Workspace also has a separate recursive pane tree
for cross-capability layout. A Zed-like editor needs more than one editing
surface in one tab: users must be able to compare files, keep related views
side by side, and focus an editor group without turning every editor action
into a shell concern.

Reusing the Workspace pane tree for this purpose would mix two different
lifecycles. Workspace panes host product capabilities such as Terminal,
Editor, SFTP, and Diff; editor groups host views of the Editor capability and
share editor-owned buffers, selections, and display state. Creating a second
shell split surface would also add permanent chrome and a competing layout
owner.

## Decision

1. Editor-internal splits are owned by the `editor` module and are available
   inside an Editor tab or standalone Editor surface. The Workspace remains
   responsible only for opening, placing, focusing, and closing the outer
   Editor tab/pane.
2. `labonair-editor` owns the UI-free split domain contract and lifecycle:
   `EditorGroupId`, a versioned `EditorSplitTree`, leaf editor-view state,
   split orientation, focus, and close/merge transitions. The contract exposes
   immutable snapshots and typed events; it never exposes
   `Entity<Workspace>` or Workspace-private state.
3. The editor view boundary owns rendering and interaction for the tree. It
   may remain in `labonair-editor` while the implementation is small. A
   sibling `labonair-editor-ui` crate is permitted only if a real GPUI
   dependency boundary is demonstrated; it remains part of the Editor owner.
4. Split dividers, focus indicators, disclosure controls, menus, and other
   reusable controls use `labonair-ui-kit`. Editor-specific text rendering,
   gutters, minimap, and split composition remain feature-owned surfaces and
   consume theme tokens rather than defining local global styles.
5. Split actions are Editor-owned command contributions in the canonical
   command and keymap registries. Initial stable identities are
   `editor.split-right`, `editor.split-down`, `editor.focus-next-group`,
   `editor.focus-previous-group`, `editor.close-group`, and
   `editor.close-other-groups`. Default bindings are attached to the owner
   descriptors; no shell action table or second editor keymap is introduced.
6. The existing Workspace pane tree remains the canonical mechanism for
   cross-capability splits. An Editor-internal split must not create a new
   permanent shell zone, duplicate the titlebar/statusbar, or change the
   canonical Project Diff review surface.
7. The Editor owns persistence of its internal split payload. Workspace
   session persistence owns the outer tab identity and stores the Editor
   payload through a typed snapshot boundary. Version 1 payloads with one
   leaf migrate to the equivalent one-leaf tree. Invalid or stale layout data
   is discarded with a notification and the file remains open in one leaf;
   user file contents are never discarded by layout recovery.
8. The first implementation is local and single-window. Remote editor views,
   collaboration, and cross-window split synchronization are explicitly
   deferred; they must not influence the initial contract.

## Consequences

Positive consequences:

- editor interactions stay with the Editor owner instead of expanding
  Workspace conditionals;
- outer capability layout and inner editor layout have unambiguous owners;
- shared buffers and independent view state can support multi-cursor,
  folding, search, and LSP work without duplicating documents;
- the user gets comparison and focused editing workflows without new shell
  chrome.

Costs and constraints:

- the Editor needs a small tree/state contract before the visual work starts;
- Workspace session serialization needs a typed adapter and a migration;
- the editor must define focus, close, empty, and narrow-layout behavior;
- this decision does not authorize copying Zed's implementation or importing
  its dependency graph or license obligations.

## Rejected alternatives

### Put all editor groups in the Workspace pane tree

Rejected. It would make Workspace the owner of editor-specific lifecycle and
view state, and would couple editor actions to a cross-capability layout
implementation.

### Add a second shell-wide split/layout registry

Rejected. It would duplicate the existing Workspace layout owner and add a
permanent product surface without a separate user workflow.

### Copy Zed's editor/display-map implementation

Rejected. Zed is a clean-room behavioral and architectural reference only.
Labonair will implement the required contracts independently under its own
module, token, dependency, and licensing rules.

## Exit condition

This ADR is implemented when editor-internal split state, commands, focus,
rendering, and persistence are owned by the Editor capability; Workspace only
hosts the outer surface through typed contracts; the command/keymap and
notification paths are owner-registered; the one-leaf migration is tested;
and no second shell layout or duplicate review surface exists.
