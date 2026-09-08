# Remaining Boundary Backlog

**Status:** Working migration backlog
**Date:** 2026-09-07
**Authority:** Evidence for the normative rules in [`../architecture.md`](../architecture.md)

This document turns the explicitly allowed dependency edges in the
[architecture inventory](architecture-inventory.md) into an ordered follow-up
backlog. It is not permission to add abstractions speculatively. A boundary is
changed only when the target contract has a real consumer and the active task
queue contains a bounded task for it.

## Decision vocabulary

| Decision | Meaning |
|---|---|
| **Done** | The edge has been removed; the row records the resolving task and evidence. |
| **Extract next** | The current edge represents feature-view or state coupling that should be removed after the acceptance gate. |
| **Narrow when needed** | The edge is currently typed and legitimate, but should become narrower only when a concrete second consumer or implementation boundary exists. |
| **Retain by design** | The edge is an intentional composition or public-contract edge and is not a violation to remove. |

## Ordered backlog

| ID | Current edge | Evidence | Decision | Target outcome / removal condition |
|---|---|---|---|---|
| B01 | ~~`panel-explorer → workspace`~~ | **Resolved (R07-004).** `crates/panel-explorer/src/panel_explorer.rs` now holds a `labonair_explorer_host::ExplorerHost` (four injected callbacks) instead of `Entity<Workspace>`; `DraggedPaths` / `quote_paths` / `shell_quote` / `is_previewable` / `PREVIEW_EXTENSIONS` moved to the leaf `labonair-explorer-host` crate consumed by both `panel-explorer` and `workspace`. The shell builds the Workspace-backed `ExplorerHost` and re-notifies the panel on active-editor changes. The direct `panel-explorer → workspace` edge is gone from Cargo metadata and the verifier allow-list. | **Done** | Native Explorer visual-state recording rolls into the R07-001 visual matrix. |
| B02 | ~~`workspace → background`~~ | **Resolved (R07-005).** `crates/workspace/src/workspace.rs` and `views/terminal.rs` now hold a `labonair_background_host::BackgroundHost` (a layer-rendering callback plus a `BackgroundPulse` repaint entity) instead of `Entity<BackgroundStore>`. `labonair-background::host()` builds the host from the concrete store and wires the pulse to `cx.observe`; the shell injects it at composition. The direct `workspace → background` edge is gone from Cargo metadata and the verifier allow-list. | **Done** | Native background-layer visual-state recording rolls into the R07-001 visual matrix. |
| B03 | `workspace → ai` | `crates/workspace/src/live_bridge.rs` implements the AI live bridge and Workspace owns the GPUI reaction path. | **Narrow when needed** | Keep Workspace orchestration, but move AI-specific context and lifecycle details behind a typed AI bridge contract when the AI UI/core boundary is actively rebuilt. No AI state belongs in shell or generic Workspace state. |
| B04 | `workspace → settings` | Workspace and its editor/terminal views read typed `SettingsStore` values for behavior and presentation. | **Narrow when needed** | Retain typed settings value access; replace only implementation-level global access with a narrow capability snapshot when a concrete consumer boundary exists. Settings remains a value owner, not a Workspace feature owner. |
| B05 | `panel-scm → editor`, `panel-scm → settings` | SCM consumes the public unified-diff contract and typed SCM presentation settings. | **Narrow when needed** | Keep the public contracts. Extract a shared diff or preference contract only after a real second non-SCM consumer requires it; there is no benefit in creating a facade now. |
| B06 | `command-palette → settings`, `command-palette → filesystem` | Palette reads typed palette settings and owns recent-command persistence using the filesystem boundary. | **Narrow when needed** | Keep the palette owner responsible for search and recent history. Replace direct implementation access only if another host needs the same provider or storage behavior; do not create a second registry. |
| B07 | `shell composition → integration siblings` | `labonair-shell` constructs concrete platform integrations and injects them into capability contracts. | **Retain by design** | Keep construction in the composition root. The edge is correct as long as the shell performs no feature behavior, owns no feature state, and exposes no aggregate backend facade. |

**Re-reviewed 2026-09-07** (after B01/B02 resolved): B03–B06 were checked
against their current real consumers (`live_bridge.rs`'s single typed
`LiveBridge` impl for B03; typed `SettingsStore` value reads only, no
Settings management UI, for B04; the public unified-diff/preference
contracts for B05; the palette's own search/recent-history ownership for
B06). Every edge is still exactly what the table describes — no
implementation-detail or feature-state leak was found. All four `Narrow when
needed` decisions stand unchanged; none has a second consumer yet, so none is
extracted now. See `docs/rework-roadmap.md`'s "Post-B02 audit" note for the
accompanying Keymap/Workspace-Terminal-Editor/AI-UI review.

## Execution order

1. Complete the native visual acceptance in `R07-001`.
2. B01 (`panel-explorer → workspace`) is resolved by `R07-004`: the contract,
   adapter, consumer migration, dependency removal, and focused tests have
   landed. Only the native Explorer visual-state recording is outstanding and
   folds into the R07-001 visual matrix.
3. B02 (`workspace → background`) is resolved by `R07-005`, mirroring B01's
   `labonair-explorer-host` pattern: the `labonair-background-host` contract,
   the `BackgroundHost` composition adapter, consumer migration, dependency
   removal, and focused tests have landed. Only the native background-layer
   visual-state recording is outstanding and folds into the R07-001 visual
   matrix.
4. Re-evaluate B03–B06 at the beginning of the next feature rebuild. They are
   review points, not automatic extraction mandates.
5. Keep B07 as a permanent composition invariant and do not replace it with a
   new all-feature facade.

## Completion evidence

A backlog item is complete only when its active task records the owner,
contract, consumer migration, removed dependency, focused tests, full gates,
and visual evidence where applicable. Removing an edge from the dependency
allow-list without moving the consumer is not completion. Keeping an edge must
instead be justified as a typed public-contract or composition edge in the
architecture inventory.
