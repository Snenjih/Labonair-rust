# ADR 0005 — Universal panes: every tab is a pane group

* **Status:** Accepted
* **Date:** 2026-11-05
* **Deciders:** project owner and implementation team
* **Supersedes:** [`0004-editor-internal-splits.md`](0004-editor-internal-splits.md)
* **Related:** [`capabilities.md`](../capabilities.md), [`modules.md`](../modules.md), [`../design-system.md`](../design-system.md)

## Context

Only Terminal tabs have a pane tree (`labonair-workspace` `pane_group.rs`).
Editor, Preview, and SFTP tabs are single surfaces keyed by tab id, and the
Editor owns a second, binary split tree whose non-focused groups are read-only
mirrors. Users cannot tile arbitrary tab kinds, drag tabs into panes, or move
panes back into tabs, and there is no pane chrome (title bar, split, zoom,
close) shared across kinds.

## Decision

1. A Workspace tab owns a pane tree (a pane group). A single-pane tab renders
   full-bleed with no frame and no pane bar. From two panes on, every pane
   renders inside a frame with a pane bar.
2. `labonair-workspace` is the only owner of layout: tiling, focus, zoom,
   drag and drop, and session persistence. Each tab kind is adapted to a pane
   by a workspace-owned adapter that supplies title, icon, focus handle,
   optional pane-bar slots, `can_split`, `split_clone`, and a snapshot.
   Capability crates keep owning their view state and behaviour.
3. Editor-internal splits are removed. Splitting an editor pane creates a new
   Editor pane with its own view state on the same editor-owned document
   (shared buffer, independent selections, scroll, and folds). The
   `EditorSplitTree` domain, its `editor.*` split commands, and the
   "Editor group N · preview" mirrors are replaced by the generic `pane::*`
   commands. Preview and Markdown side panes become sibling Preview panes.
4. The pane bar is generic shared chrome in `labonair-ui-kit` (icon, title,
   leading/trailing slots, split horizontal, split vertical, zoom, close). It
   knows no tab kinds. It is not new permanent shell chrome: it is part of the
   pane surface and is hidden for single panes.
5. Layout style is a setting, `appearance.paneLayout`: `comfortable`
   (floating, rounded panes with a gap) or `compact` (edge to edge, square,
   hairline dividers).
6. Zoom is a per-tab overlay of one pane; sibling panes stay laid out so
   terminal sizes do not change. Zoom is not persisted.
7. Singleton kinds (Git Graph, Keymap, Hosts, Project Diff) cannot be split
   but can be moved between panes and tabs; opening one again focuses the
   existing pane.
8. Session snapshots move to version 3: a leaf is `{kind, state}`. Version 2
   snapshots migrate (terminal layout becomes a terminal group, other kinds a
   single leaf). Unknown or invalid leaves are dropped at restore and the tree
   collapses; user file contents are never discarded by layout recovery.
9. Split and focus commands are `pane::*` contributions with stable IDs in the
   canonical command and keymap registries, valid in every pane kind.

## Consequences

- One split engine replaces two; Editor split persistence (R09-003/R09-007
  payloads) is folded into the pane snapshot.
- The per-tab-id view maps in `Workspace` are re-keyed by pane id.
- R09-003's "Editor-internal splits" acceptance is superseded by this ADR; its
  multi-selection work is unaffected.
