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
| **Extract next** | The current edge represents feature-view or state coupling that should be removed after the acceptance gate. |
| **Narrow when needed** | The edge is currently typed and legitimate, but should become narrower only when a concrete second consumer or implementation boundary exists. |
| **Retain by design** | The edge is an intentional composition or public-contract edge and is not a violation to remove. |

## Ordered backlog

| ID | Current edge | Evidence | Decision | Target outcome / removal condition |
|---|---|---|---|---|
| B01 | `panel-explorer → workspace` | `crates/panel-explorer/src/panel_explorer.rs` stores `Entity<Workspace>` and calls Workspace open-file, open-terminal, and preview callbacks; drag/preview types are re-exported from Workspace. | **Extract next** | Define an Explorer host contract for those intents, move shared drag/preview values below both modules, inject the host at composition, and delete the direct Workspace dependency. Planned task: [`R07-004`](../../tasks/rework/R07-004-explorer-host-contract.md). |
| B02 | `workspace → background` | `crates/workspace/src/workspace.rs` and `views/terminal.rs` hold `Entity<BackgroundStore>` to mount the background layer. | **Extract next** | Introduce a narrow background presentation capability at the view boundary, then remove the Workspace dependency on the Background entity/store. Background persistence and rendering remain owned by Backgrounds. |
| B03 | `workspace → ai` | `crates/workspace/src/live_bridge.rs` implements the AI live bridge and Workspace owns the GPUI reaction path. | **Narrow when needed** | Keep Workspace orchestration, but move AI-specific context and lifecycle details behind a typed AI bridge contract when the AI UI/core boundary is actively rebuilt. No AI state belongs in shell or generic Workspace state. |
| B04 | `workspace → settings` | Workspace and its editor/terminal views read typed `SettingsStore` values for behavior and presentation. | **Narrow when needed** | Retain typed settings value access; replace only implementation-level global access with a narrow capability snapshot when a concrete consumer boundary exists. Settings remains a value owner, not a Workspace feature owner. |
| B05 | `panel-scm → editor`, `panel-scm → settings` | SCM consumes the public unified-diff contract and typed SCM presentation settings. | **Narrow when needed** | Keep the public contracts. Extract a shared diff or preference contract only after a real second non-SCM consumer requires it; there is no benefit in creating a facade now. |
| B06 | `command-palette → settings`, `command-palette → filesystem` | Palette reads typed palette settings and owns recent-command persistence using the filesystem boundary. | **Narrow when needed** | Keep the palette owner responsible for search and recent history. Replace direct implementation access only if another host needs the same provider or storage behavior; do not create a second registry. |
| B07 | `shell composition → integration siblings` | `labonair-shell` constructs concrete platform integrations and injects them into capability contracts. | **Retain by design** | Keep construction in the composition root. The edge is correct as long as the shell performs no feature behavior, owns no feature state, and exposes no aggregate backend facade. |

## Execution order

1. Complete the native visual acceptance in `R07-001`.
2. Create one bounded task for B01 and complete its contract, adapter,
   consumer, dependency, and visual checks before starting B02.
3. Create B02 only after its presentation contract is demonstrated by the
   current Background consumer; do not split a crate merely to make the graph
   look symmetrical.
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
