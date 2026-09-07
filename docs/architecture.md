# Labonair Architecture

**Status:** Normative target architecture
**Version:** 3
**Related:** [`product.md`](product.md), [`capabilities.md`](capabilities.md), [`modules.md`](modules.md), [`registries.md`](registries.md), [`repository-layout.md`](repository-layout.md), [`feature-lifecycle.md`](feature-lifecycle.md)

## 1. Architecture objective

Labonair is a single native Rust application built with GPUI. It is composed from independently owned feature modules. The application composition layer connects modules, but it does not contain their business logic.

The architecture optimizes for three properties:

- **Isolation:** a module owns its state, behavior, UI, persistence, and tests.
- **Consistency:** shared interaction and visual behavior comes from foundation crates.
- **Replaceability:** a module can be rewritten or removed without rewriting unrelated modules.

## 2. Terms

- **Capability:** a user-facing ability such as SSH, SFTP, terminal, themes, or transfers.
- **Module:** the complete ownership boundary of one capability.
- **Capability crate:** the canonical Rust crate for one product capability. It
  is the default starting point for the module and must not contain unrelated
  product capabilities.
- **Sibling crate:** an additional crate inside the same owning module, used
  only for a real core, UI, storage, or integration boundary.
- **Composition root:** the application startup code that constructs services and invokes registration functions.
- **Registry:** an extensible collection of metadata and providers owned by a foundation or host contract.
- **Workspace:** a runtime context containing tabs, panes, tools, and optional project/remote identity.

Every product capability has exactly one owning module and one canonical
capability crate. A module may contain sibling crates, but splitting a module
does not split ownership. Foundation crates are not product capabilities and
must have one explicit, reusable foundation responsibility.

## 3. Layer model

```text
Application composition
  labonair / labonair-shell
        ↓
Workspace orchestration
  workspace, tabs, panes, docks, sessions
        ↓
Feature modules
  terminal, editor, ssh, sftp, hosts, transfers, git, themes,
  keymap, notifications, command-palette, settings, ai
        ↓
Foundation and platform services
  ui-kit, gpui-ext, interaction-contracts, filesystem, secrets, persistence
```

Dependencies point downward. A feature may depend on a foundation contract, but a feature must not depend on the shell. Cross-feature behavior uses a typed contract, registry, or event; it does not reach into another feature's private state.

`labonair-interaction-contracts` is a lower-level UI-free identity foundation.
It owns stable `ShortcutId` values and has no feature dependencies.
`labonair-command-palette-core` is a contract-level registry crate even though
it belongs to the Command Palette module. Capability crates may depend on this
UI-free contract to contribute command metadata; they must not depend on the
palette UI crate. `labonair-keymap` may depend on the command contract to
publish keymap-owned commands, while the command contract depends only on the
identity foundation. This one-way direction keeps the graph acyclic.

Persisted keymap action names cross into the runtime through one canonical
resolver (`keymap::runtime::command_for_action`). Platform adapters may map the
resulting `CommandId` to GPUI actions, but they must not introduce a second
string alias table. Rich GPUI context predicates remain an adapter concern;
portable keymap resolution uses typed command contexts.

Command owners also publish their default bindings on the command descriptor.
The keymap module converts those typed defaults into the default resolution
layer, then applies the user keymap as the override layer. The composition root
registers providers but does not maintain a feature-wide default-shortcut table.
The module-owned `keymap::adapter::load` boundary returns the resulting
immutable snapshot, including diagnostics; the shell only installs it into
GPUI and connects file-watch events.
The editor contract keeps raw JSONC source separate from its parsed view, so
comments, unknown commands, and malformed edits remain available for recovery
and are never lost through a deserialize/serialize round trip.

## 4. Target workspace crate map

### Foundation

| Crate | Responsibility |
|---|---|
| `labonair-gpui-ext` | GPUI helpers and small shared primitives. |
| `labonair-interaction-contracts` | Stable UI-free identities shared by interactive feature modules. |
| `labonair-ui-kit` | Buttons, inputs, lists, dropdowns, dialogs, icons, badges, disclosure, tabs, and other reusable components. |
| `labonair-filesystem` | Local filesystem abstractions and watchers. |
| `labonair-background` | Background-image storage, import/delete operations, decoded image cache, and GPUI background layers. |
| `labonair-errors` | Structured, UI-free domain error contract and recovery metadata. |
| `labonair-events` | UI-free in-process transport primitives for adapter-level events; it owns no product event vocabulary or application state. |
| `labonair-secrets` | Keychain and secret references. |
| `labonair-persistence` | Cloneable shared SQLite connection and schema lifecycle; feature modules own stores and queries. |
| `labonair-panel` | UI-free panel, dock, and status-item contracts used by workspace-owned surfaces. |
| `labonair-mcp-core` | UI-free MCP session/grant, tab-operation, and persisted bridge-preference contracts shared by the agent-access host and its injected bridge adapter. |
| `labonair-terminal-integration` | UI-free OSC 7/133 shell-integration payloads shared by local PTY and remote SSH adapters. |

### Capability modules and crates

The names below are the current canonical crates. They are not a request to
pre-create empty `-core` or `-ui` crates. A sibling crate is added only when
the module has a demonstrated dependency, lifecycle, storage, or platform
boundary.

| Module | Canonical capability crate | Existing sibling crates | Ownership boundary |
|---|---|---|---|
| Settings | `labonair-settings` | `settings-content`, `settings-json`, `settings-macros`, `settings-ui` | Typed values, layered persistence, and value-only settings UI. |
| Keymap | `labonair-keymap` | `keymap-ui` | Binding descriptors, file data, resolution, conflicts, and a presentation adapter; no feature behavior. |
| Command palette | `labonair-command-palette-core` | `labonair-command-palette` | UI-free command registry contract; the sibling owns GPUI search/navigation and submenu presentation. |
| Notifications | `labonair-notifications-core` | `notifications` | Notification registry/state and its GPUI statusbar presentation. |
| Themes | `labonair-theme` | none yet | Built-in color and icon-theme registries, preview, and selection. |
| Workspace | `labonair-workspace` | none; panel crates are separate capabilities | Workspace identity, tabs, panes, focus, layout, and session orchestration. |
| Backgrounds | `labonair-background` | none yet | Background-image persistence and rendering; settings values are consumed through the settings store, but Settings UI does not own the capability. |

### Product modules

| Module | Canonical capability crate | Responsibility |
|---|---|---|
| Terminal | `labonair-terminal` | PTY sessions, terminal engine, renderer, and terminal commands. Sibling `labonair-terminal-integration` owns shared shell protocol payloads. |
| Editor | `labonair-editor` | Buffers, syntax, editing behavior, and editor commands. |
| SSH | `labonair-ssh` | SSH contracts plus the `labonair-ssh-transport` integration sibling for russh transport, authentication, tunnels, and jump-host execution. |
| SFTP | `labonair-sftp` | Remote filesystem browsing and SFTP operations. |
| Hosts | `labonair-hosts` | Saved host definitions, recent hosts, import/export, and host management. Transport adapters remain capability-owned. |
| Credentials | `labonair-credentials` | Credential metadata, secret references, and generated SSH key material. |
| Transfers | `labonair-transfers` | Transfer queue, progress, cancellation, conflict resolution, and lifecycle history. Retry is a follow-up contract when supported by the worker. |
| Git | `labonair-git` | Git contracts and source-control behavior; sibling panel crates provide Git views. |
| Explorer | `labonair-panel-explorer` | Local file navigation UI over filesystem contracts. |
| Snippets | `labonair-snippets` | Snippet storage, execution contracts, and snippet behavior; sibling panel crate provides the view. |
| AI | `labonair-ai` | Providers, sessions, context, tools, and future UI. MCP host contracts live in `labonair-mcp-core`. |

The first transport split is intentionally contract-first:
`labonair-ssh` owns UI-free SSH session, PTY, trust, remote-command, tunnel,
tester, and SSH-config contracts. `labonair-sftp` owns the authenticated SFTP
session handle and remote-browser contracts. `labonair-ssh-transport` owns the
concrete russh implementation and its contract adapters; workspace and feature
views consume injected traits. `labonair-backend` no longer owns the SSH
transport module.
Transfers remain a separate capability and are not part of the SFTP browser
contract.

`labonair-events` is a foundation transport only. It replaces the former
backend-local event primitive, while SSH, MCP, and Transfers translate raw
adapter events into their own typed contracts at the boundary. Product code
must not add feature semantics or a second global event registry to this
crate.

The transfer capability is now split into `labonair-transfers` and
`labonair-transfers-ui`. The former owns the UI-free job values, typed worker
contracts, event translation boundary, and retained lifecycle registry. The
latter owns the queue dropdown and resolution dialogs. The backend exposes a
temporary worker adapter, while SFTP only submits typed transfer requests.
The statusbar owns the trigger/anchor, but not transfer state.

SSH connection lifecycle and MCP bridge events follow the same contract-first
rule. `labonair-ssh` and `labonair-mcp-core` expose typed event sources and
receivers; shell composition supplies the backend translation adapters, and
Workspace owns only the GPUI bridges and feature reaction. No Workspace code
subscribes to a transport adapter directly; the shared raw transport is owned
by `labonair-events` and remains hidden behind the composition boundary.

The current repository does not yet match every ownership boundary in this
map. The migration is tracked in [`rework-roadmap.md`](rework-roadmap.md), and
the observed state is recorded in [`audits/architecture-inventory.md`](audits/architecture-inventory.md).

Workspace chrome state is persisted by `labonair-workspace` in the dedicated
`workspace-layout.json` file. Dock membership, open/closed state, active panel,
size, zoom, and the primary dock edge are runtime layout state; they are not
`SettingsContent` values and must not be project settings. The workspace owner
imports legacy `workspace.sidebar*` and `workspace.dockLayout` values once,
then removes those keys from `config.json`. Session/tab contents remain owned
by the workspace session snapshot and are not folded into this layout file.

The titlebar is a shell surface, not a feature owner. Its single global-menu
button publishes `TitlebarEvent` intent. The composition root connects that
intent to Settings, the keymap file surface, the Hosts management surface, or a Command Palette page. The
menu uses the shared `popover_menu` primitive and anchors it in window
coordinates directly below the clicked button; it must not implement a second
menu or feature-specific behavior.

`labonair-workspace::context` owns the UI-free distinction between workspace
identity (`Standalone` or `Project`) and activity (`Empty` or `Active`). Tool
tabs do not choose a second layout model for standalone use. Cross-surface
requests such as opening Hosts or presenting the project picker are emitted as
typed `WorkspaceEvent` values; the shell subscribes and composes the
platform-specific destination. Identity mutation uses the single typed
`WorkspaceTransition` contract (`OpenProject` or `ReturnToStandalone`); a
terminal current-working-directory event is never a workspace transition. The
pure `WorkspaceContext` applies that transition; GPUI workspace state only
coordinates the resulting settings and invalidation side effects.

## 5. Composition root

Only the `labonair` application package and `labonair-shell` may know all concrete feature modules. Their responsibilities are limited to:

- initialize platform services;
- create the application and workspace entities;
- call feature registration functions;
- connect typed events and callbacks;
- compose the permanent shell.

They must not contain SSH logic, transfer logic, theme definitions, settings field definitions, or feature-specific rendering.

The composition root may construct concrete implementations and pass them to
owners, but it may not reinterpret their domain events or duplicate their
registries. Registration calls belong to the owning module; the root only
assembles registrations and connects typed interfaces.

## 6. Communication rules

Use the smallest communication mechanism that fits:

1. A direct trait for a synchronous capability request.
2. A typed event for an asynchronous state change.
3. A registry for discoverable, multi-provider contributions.
4. A shared identifier/value type only when it has stable domain meaning.

Stringly typed global events, arbitrary global mutable state, and cross-module entity mutation are prohibited for new code.

The application event bus is a transport boundary, not a business-logic layer. Each module translates external events into its own typed state changes. User-visible messages are forwarded to the notification center.

Persistent state follows the same ownership rule as runtime behavior: the
owner writes its own file or store through a narrow persistence boundary. The
composition root may order migrations and provide services, but it must not
serialize a feature's private state into Settings as a shortcut.

## 7. Dependency rules

- Foundation crates do not depend on product modules.
- `labonair-ui-kit` contains no product-specific behavior.
- Feature modules do not depend on `labonair-shell`.
- Feature modules do not import private types from another feature.
- Core crates do not depend on GPUI unless their state genuinely requires it; pure domain logic stays UI-free.
- Persistence and secrets are accessed through narrow service contracts.
- The shell is the only all-feature composition layer.
- Every new dependency edge requires a reason in the module manifest or an ADR.
- A CI allow-list must verify the dependency graph and reject cycles or forbidden edges.
- A feature crate may depend on another module's public contract crate when its
  user flow requires it, but not on that module's private state or feature UI.
- No new capability code may be added to `labonair-backend`; during migration it
  may contain only explicitly named adapters whose removal condition is tracked
  in the inventory.

## 8. UI ownership

Feature crates own feature-specific views. `labonair-ui-kit` owns reusable visual and interaction primitives. A feature must use the kit for buttons, dropdowns, lists, menus, inputs, badges, popovers, dialogs, and standard rows.

Feature-specific components may compose kit components, but may not fork their styling locally. New shared behavior is added to the kit first, with a documented API and a component test.

The UI kit is a component boundary, not a product module. It may provide
generic buttons, inputs, lists, menus, dropdowns, dialogs, badges, tabs,
disclosure, scrolling, and focus behavior, but it must not know about hosts,
themes, transfers, or any other product capability.

## 9. Runtime lifecycle

Initialization is explicit and ordered:

1. platform/runtime services;
2. persistence, secrets, settings, and keymap stores;
3. theme registry and active theme;
4. notification center and transfer service;
5. command, panel, status-item, and theme registrations;
6. workspace and shell;
7. optional feature workers.

I/O is asynchronous. The GPUI foreground thread only performs bounded state updates and rendering. No startup or render path may perform blocking filesystem, network, process, or database work.

## 10. Adding and migrating a capability

Every feature change follows the same boundary-first sequence:

1. Record the user flow, disposition (`keep`, `redesign`, `defer`, or
   `remove`), owner, canonical entry point, and canonical capability crate in
   the capability matrix and roadmap.
2. Define the owning module's domain state, public typed contract, events,
   persistence, commands/keybindings, notifications, and required UI-kit
   components. Choose a direct trait, typed event, or registry based on the
   number of providers and consumers; do not introduce a registry by default.
3. Implement state, behavior, UI, and tests inside the owner. Register
   commands, panels, status items, themes, or notifications through their
   typed registration APIs; do not add a second central table.
4. Wire concrete implementations only in the composition root. Consumers use
   public contracts or immutable snapshots and never mutate another module's
   state directly.
5. During migration, move the contract first, then adapters and consumers.
   Mark each compatibility path with a removal condition, update the
   inventory and dependency verifier, and remove the old path before declaring
   the migration complete.

Do not introduce a new abstraction, permanent surface, or sibling crate
without a current consumer and a documented boundary. If a capability has no
current workflow, remove it or park it outside the permanent product surface.

## 11. Verification

Architecture changes require:

- `cargo fmt --check`
- `cargo check --workspace --all-targets`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- dependency graph verification
- a visual check for any UI or layout change

The dependency verifier currently permits only the explicit transitional edges
listed in [`audits/architecture-inventory.md`](audits/architecture-inventory.md).
It must be kept strict while those edges are removed. A green dependency check
therefore means "no untracked violation", not that the migration is complete.
