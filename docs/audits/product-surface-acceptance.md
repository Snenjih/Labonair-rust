# Product Surface Acceptance Matrix

**Status:** Working acceptance evidence for R07-001
**Date:** 2026-09-07

This document is evidence for the current product-surface audit. It is not a
replacement for the normative product or architecture contracts. A structural
status is based on the current source tree and registry/dependency checks; a
visual status is not complete until the surface has been inspected in the
native Rust application at the listed states.

## Evidence vocabulary

| Status | Meaning |
|---|---|
| `Verified` | Source ownership and focused behavior evidence are present. |
| `Partial` | The owner exists, but a documented compatibility or composition edge remains. |
| `Pending` | Required source, workflow, or visual evidence is still missing. |
| `N/A` | The state is not meaningful for that surface; the reason is recorded. |

## Structural acceptance

| Surface | Owner / canonical crate | Canonical entry point | Source evidence | Structural status |
|---|---|---|---|---|
| Titlebar and global menu | application shell / `labonair-shell` | titlebar global-menu button | `crates/shell/src/titlebar.rs`, `crates/shell/src/bootstrap.rs` | Verified; feature destinations are typed composition events |
| Workspace and empty state | Workspace / `labonair-workspace` | workspace surface, project picker, standalone launch | `crates/workspace/src/workspace.rs`, `crates/workspace/src/context.rs` | Partial; feature-view extraction remains in progress |
| Docks and panels | Workspace contracts / `labonair-panel` plus panel owners | dock panel controls | `crates/panel/src/`, `crates/workspace/src/dock.rs` | Verified; panel contributions are owner-registered |
| Statusbar | Workspace host / `labonair-workspace` plus feature owners | permanent statusbar zones | `crates/workspace/src/status_bar.rs`, `crates/shell/src/status_items.rs` | Verified; shell only composes typed registrations |
| Command Palette | Command Palette / `labonair-command-palette-core` | titlebar menu or global shortcut | `crates/command-palette-core/src/`, `crates/command-palette/src/`, `crates/command-palette-runtime/src/`, `crates/shell/src/actions.rs` | Partial; command metadata and ordinary handlers are owner-contributed, but dynamic submenu actions are still interpreted by shell composition |
| Keymap | Keymap / `labonair-keymap` | titlebar menu → Keymap | `crates/keymap/src/`, `crates/keymap-ui/src/` | Partial; GPUI installation/watch remains a shell adapter |
| Settings values | Settings / `labonair-settings` | Settings window | `crates/settings-content/src/areas.rs`, `crates/settings-ui/src/` | Verified; no Hosts/Themes/Icon Themes/Shortcuts management categories |
| Hosts management and picker | Hosts / `labonair-hosts` and `labonair-hosts-ui` | titlebar menu → Hosts; palette Hosts page | `crates/hosts/src/`, `crates/hosts-ui/src/command_provider.rs`, `crates/shell/src/actions.rs` | Partial; SSH primary and SFTP secondary actions are typed, but palette selection is still routed by shell composition |
| Themes and icon themes | Themes / `labonair-theme` | titlebar menu → palette theme pages | `crates/theme/src/`, `crates/shell/src/actions.rs` | Partial; catalogs and preview state are owner-owned, but selection/preview event execution remains shell-wired |
| Notifications | Notifications / `labonair-notifications-core` and `labonair-notifications` | statusbar notification dropdown | `crates/notifications-core/src/`, `crates/notifications/src/status_item.rs` | Verified; no toast renderer or second global surface; rows use the shared UI-kit `ListItem` |
| Transfers | Transfers / `labonair-transfers` and `labonair-transfers-ui` | statusbar Transfers item | `crates/transfers/src/`, `crates/transfers-ui/src/` | Verified; queue state and presentation are separate from SFTP browsing |
| Search overlay | Workspace / `labonair-workspace` | `Find` command | `crates/workspace/src/search_overlay.rs`, `crates/workspace/src/command_provider.rs` | Verified; shell injects only modal-host behavior |
| Updater | Updater / `labonair-updater` and `labonair-updater-ui` | statusbar item or update command | `crates/updater/src/`, `crates/updater-ui/src/` | Verified; UI and command contribution are owner-owned |

## Required visual state matrix

`Pending` means the state still needs a native-bundle inspection record. A
state may be marked `Verified` only together with a date, viewport/window
condition, and a capture or reproducible inspection note in the task/handshake.

| Surface | Normal | Narrow | Focused | Empty | Loading | Error | Long list | Overlay / anchoring |
|---|---|---|---|---|---|---|---|---|
| Titlebar / global menu | Pending | Pending | Pending | N/A | N/A | N/A | N/A | Pending |
| Workspace / docks | Pending | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| Statusbar | Pending | Pending | Pending | Pending | Pending | N/A | N/A | Pending |
| Command Palette | Pending | Pending | Pending | Pending | N/A | Pending | Pending | Pending |
| Keymap surface | Pending | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| Settings | Pending | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| Hosts management / picker | Pending | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| Themes / icon themes | Pending | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| Notifications dropdown | Pending | Pending | Pending | Pending | N/A | Pending | Pending | Pending |
| Transfers dropdown | Pending | Pending | Pending | Pending | Pending | Pending | Pending | Pending |
| Search overlay | Pending | Pending | Pending | N/A | N/A | Pending | N/A | Pending |
| Updater dialog | Pending | Pending | Pending | N/A | Pending | Pending | N/A | Pending |

## Required evidence procedure

1. Build and run the native application with `cargo run -p labonair`; never use
   the legacy installed Tauri application.
2. Inspect the surface at its applicable matrix states, including a narrow
   window and keyboard focus path.
3. Record the result with the surface, state, date, and capture or exact
   reproduction note. Do not mark a state based only on a successful compile.
4. If a state exposes a layout or ownership defect, create a bounded rework
   task before changing the matrix to `Verified`.

The matrix is complete only when every applicable state is verified and every
`Partial` structural row has either a documented removal condition or an
accepted composition-only rationale.

## Native inspection log

| Date | Native process | Window | Result | Follow-up |
|---|---|---|---|---|
| 2026-09-07 | `/Users/niklas/Developer/active/Labonair/Labonair-rust/target/debug/labonair` (PID 54142) | 76849 | The native window was found, but macOS denied `screencapture` because Screen Recording permission is unavailable to the runner. No screenshot was accepted as evidence. | Grant Screen Recording permission to the runner or capture the same matrix manually; keep all visual cells pending until then. |

The same native launch emitted warnings for legacy persisted keys that are no
longer part of the typed Settings model, including `hosts`, `hostsMigrated`,
`statusBarItemPlacements`, and several removed workspace/bookmark values. The
observed file is already marked `schemaVersion: 2` and `sparsified: true`, so
the V1-to-V2 migration returns early and does not revisit those old root keys;
the later schema walk then reports them. This is a migration/audit finding,
not visual evidence: R07-003 must define an idempotent cleanup/classification
step without weakening warnings for genuinely unknown future keys.
