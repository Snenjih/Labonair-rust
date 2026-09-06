# ADR 0003 — Establish the modular product architecture

* **Status:** Accepted
* **Date:** 2026-09-06
* **Deciders:** project owner and implementation team
* **Related:** [`product.md`](../product.md), [`architecture.md`](../architecture.md), [`modules.md`](../modules.md)

## Context

The previous architecture optimized for porting the Tauri application and later decomposed a UI monolith. It left several responsibilities in the wrong places: a broad backend crate, settings management surfaces for hosts/themes/shortcuts, a toast-oriented notification implementation, static command-palette tables, and shell code that knew too much about feature behavior.

The product direction is now a modular Dev-Op workspace. Terminal, editor, SSH, SFTP, hosts, transfers, themes, keymap, notifications, command palette, settings, Git, and AI are independent capabilities that must be usable both inside and outside a project workspace.

## Decision

Labonair uses capability-owned modules with explicit composition at application startup.

1. Each capability has one owner responsible for its state, UI, backend behavior, persistence, commands, notifications, and tests.
2. A module may be split into core, UI, storage, and integration crates where that creates a meaningful dependency boundary.
3. Foundation crates provide reusable infrastructure and UI primitives but contain no product-specific behavior.
4. Cross-module interaction uses typed contracts, typed events, or registries.
5. The shell and app crates compose modules and render permanent surfaces; they do not implement feature behavior.
6. Settings contain values only. Hosts, themes, icon themes, keymap editing, transfers, notifications, and command registration have separate module ownership.
7. Notifications are persistent statusbar-dropdown entries. Toast rendering and feature-local inline error messages are not part of the product contract.
8. The command palette is a registry consumer. Dynamic submenus are contributed by providers rather than represented by a central feature table.

## Consequences

Positive consequences:

- feature changes compile and test within narrower boundaries;
- new commands, panels, status items, themes, and notifications do not require unrelated central-table edits;
- local and remote workflows can share workspace contracts;
- the UI kit can enforce a single visual language;
- obsolete features can be removed without preserving shell-wide compatibility code.

Costs:

- the current graph requires a deliberate migration;
- explicit contracts and registration code add some boilerplate;
- module ownership must be reviewed when a feature spans several capabilities;
- current historical tasks cannot be treated as the implementation queue.

## Rejected alternatives

### Full rewrite from zero

Rejected for now. Existing terminal, editor, SSH, SFTP, persistence, and GPUI work contain valuable behavior. Rewriting everything would increase risk without improving the product contract.

### Whole-project Zed fork

Rejected for now. Labonair has different product priorities, and a full fork would import substantial unrelated scope and licensing obligations. Zed remains a clean-room reference for interaction and architecture patterns.

### One crate per tiny UI element

Rejected. Buttons and lists are shared UI primitives, not independent product capabilities. They belong to the UI kit.
