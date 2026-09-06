# Architecture Inventory — Reset Baseline

**Status:** Working migration inventory
**Date:** 2026-09-06

This document records the current repository shape during the module migration. It is evidence for the rework; it is not a target design.

## Current workspace crates

| Current crate | Current role | Target owner | Migration note |
|---|---|---|---|
| `app` | Binary/bootstrap | application composition | Keep small; remove feature logic. |
| `backend` | Mixed filesystem, PTY, SSH, SFTP, Git, hosts, settings, updater, MCP, persistence | split across platform services and feature modules | Highest-priority god-object boundary. |
| `ai` | AI providers, sessions, tools | AI module | Keep backend-facing core; rebuild UI later. |
| `command-palette` | Palette UI, static commands, some keymap behavior | command-palette module + keymap module | Split registry/core from GPUI view. |
| `editor` | Editor engine | editor module | Separate core from workspace view. |
| `filesystem` | Local file access, traversal, mutation, and search | foundation/platform service | First extracted service boundary; watcher integration remains in `backend` temporarily. |
| `gpui-ext` | Shared GPUI helpers | foundation | Keep dependency-free from features. |
| `hosts-ui` | Host management UI and host-related dependencies | hosts module | Remove settings and notification coupling. |
| `notifications` | Notification state plus toast renderer | notifications module | Remove toast rendering; keep dropdown consumer. |
| `panel` | Panel/status contracts | workspace foundation | Keep contracts-only. |
| `panel-explorer` | Explorer panel | explorer module | Remove workspace dependency through contracts. |
| `panel-git-graph` | Git graph panel | git module | Keep UI-specific graph surface. |
| `panel-scm` | Source-control panel | git module | Separate Git service contract from UI. |
| `panel-snippets` | Snippet panel and execution UI | snippets module | Separate persistence/execution from view. |
| `settings` | Layered settings store | settings module | Keep as core after removing misplaced categories. |
| `settings-content` | Typed settings data | settings module | Keep only actual configuration values. |
| `settings-json` | JSON editing | settings module | Keep as persistence adapter. |
| `settings-macros` | Settings derives | settings module | Keep implementation detail. |
| `settings-ui` | Settings views and generated fields | settings module | Remove Hosts, Themes, Icon Themes, Shortcuts management. |
| `shell` | App shell, menus, commands, status items, updater | application composition + shell surface | Reduce to registration and composition. |
| `terminal` | Terminal engine and renderer support | terminal module | Split engine from GPUI view when useful. |
| `theme` | Runtime theme and fonts | themes module | Add explicit color/icon registries. |
| `ui-kit` | Shared UI primitives | foundation | Enforce as the only source of shared controls. |
| `workspace` | Workspace, tabs, panes, docks, views, transfers, bridges | workspace plus tool modules | Reduce cross-feature ownership gradually. |

## Current structural violations

The current Cargo metadata shows several transitional edges that conflict with the new rules:

- `workspace` depends directly on AI, backend, command palette, hosts UI, notifications, settings, SFTP-related views, and feature views.
- `settings-ui` depends on backend, workspace, hosts UI, command palette, notifications, and panel contracts.
- `panel-explorer` depends on workspace, which prevents independent feature ownership.
- `hosts-ui` depends on settings and notifications, even though Hosts is not a Settings concern and connection management should emit through the app notification contract.
- `command-palette` depends on backend even though the palette should receive dynamic data through providers.
- `backend` exposes a broad `App`, global event bus, and unrelated modules under one public crate.
- `backend` still owns the filesystem watcher because it emits directly through the legacy app event bus; this is a deliberate temporary integration seam.
- `shell/src/commands.rs`, `shell/src/status_items.rs`, and workspace views still contain feature-specific behavior that belongs to owning modules.
- `workspace/src/toast_layer.rs` and `notifications` still encode the superseded toast model.

The dependency verifier allows four additional transitional edges while these
boundaries are extracted: command palette → settings, explorer → settings,
SCM → editor, and SCM → settings. They are deliberately visible in the
allow-list and must not be treated as target architecture.

These are migration findings, not reasons to perform a destructive rewrite. Each edge should be removed when the owning contract exists and its consumers have moved.

## Migration order

1. Introduce stable IDs, typed domain events, and narrow service traits.
2. Extract platform services from `backend` without changing user behavior. The pure filesystem service is now `labonair-filesystem`; watcher extraction is still open.
3. Split notification state from presentation and replace toast rendering.
4. Split command/keymap registries from the palette view.
5. Move transfers to their own module and statusbar owner.
6. Move hosts and SSH ownership out of Settings/workspace.
7. Move terminal/editor/SFTP views to their owning modules.
8. Remove compatibility edges and enforce the target graph.

## Evidence commands

The inventory was produced from:

```text
cargo metadata --no-deps --format-version 1
find crates/backend/src/modules -maxdepth 2 -type f
rg -n "pub struct|pub enum|pub trait|pub fn" crates/backend/src/modules crates/shell/src crates/workspace/src
```
