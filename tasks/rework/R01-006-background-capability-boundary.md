# R01-006 — Extract the Background capability from Workspace

## Status

`✅ Done`

## Owner

- Module: `backgrounds`
- Canonical crate: `labonair-background`
- Related matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)

## Goal

Give background images a real capability owner and remove the final
`settings-ui → workspace` dependency. Settings UI may edit typed appearance
values, but it must not hold or render a workspace-owned entity.

## Scope

- Move `BackgroundStore` and its GPUI layer out of `labonair-workspace`.
- Move background image persistence and import/delete operations out of the
  broad backend module into the background capability.
- Keep Workspace responsible only for composing its injected background layer;
  it must not become the owner of background state.
- Remove the `labonair-workspace` dependency from `settings-ui`.
- Record the remaining synchronization boundary explicitly; do not add a
  second settings source of truth as part of this task.

## Acceptance criteria

- [x] `labonair-background` is the canonical owner of background state,
  storage, and rendering.
- [x] `settings-ui` has no dependency on or entity field from
  `labonair-workspace`.
- [x] `backend` no longer exports the background capability module.
- [x] Workspace and shell compile against the dedicated background crate.
- [x] Dependency inventory and verifier describe the new edge and its
  removal condition.
- [x] Full workspace verification gates pass.

## Removal condition

The temporary `workspace → background` composition edge can be removed when
the app-shell/background synchronization and standalone background surface
are exposed through a dedicated background capability contract.

## Outcome

`labonair-background` now owns the BackgroundStore, image persistence,
import/delete operations, decoding cache, and GPUI layers. Workspace and the
shell consume that capability for composition, while Settings UI remains
independent of both the workspace crate and the background entity. The shared
settings synchronization contract remains a separately tracked follow-up so
this extraction does not introduce a second source of truth.
