# Labonair Architecture

**Status:** Normative target architecture
**Version:** 2
**Related:** [`product.md`](product.md), [`modules.md`](modules.md), [`registries.md`](registries.md)

## 1. Architecture objective

Labonair is a single native Rust application built with GPUI. It is composed from independently owned feature modules. The application composition layer connects modules, but it does not contain their business logic.

The architecture optimizes for three properties:

- **Isolation:** a module owns its state, behavior, UI, persistence, and tests.
- **Consistency:** shared interaction and visual behavior comes from foundation crates.
- **Replaceability:** a module can be rewritten or removed without rewriting unrelated modules.

## 2. Terms

- **Capability:** a user-facing ability such as SSH, SFTP, terminal, themes, or transfers.
- **Module:** the complete ownership boundary of one capability.
- **Crate:** a Rust compilation and dependency boundary inside a module or foundation layer.
- **Composition root:** the application startup code that constructs services and invokes registration functions.
- **Registry:** an extensible collection of metadata and providers owned by a foundation or host contract.
- **Workspace:** a runtime context containing tabs, panes, tools, and optional project/remote identity.

One module may contain multiple crates. A crate must never become a second owner for a capability.

## 3. Layer model

```text
Application composition
  labonair-app / labonair-shell
        ↓
Workspace orchestration
  workspace, tabs, panes, docks, sessions
        ↓
Feature modules
  terminal, editor, ssh, sftp, hosts, transfers, git, themes,
  keymap, notifications, command-palette, settings, ai
        ↓
Foundation and platform services
  ui-kit, gpui-ext, runtime, filesystem, secrets, persistence, process
```

Dependencies point downward. A feature may depend on a foundation contract, but a feature must not depend on the shell. Cross-feature behavior uses a typed contract, registry, or event; it does not reach into another feature's private state.

## 4. Target workspace crate map

### Foundation

| Crate | Responsibility |
|---|---|
| `labonair-gpui-ext` | GPUI helpers and small shared primitives. |
| `labonair-ui-kit` | Buttons, inputs, lists, dropdowns, dialogs, icons, badges, disclosure, tabs, and other reusable components. |
| `labonair-runtime` | Tokio/foreground runtime bridging and lifecycle helpers. |
| `labonair-filesystem` | Local filesystem abstractions and watchers. |
| `labonair-errors` | Structured, UI-free domain error contract and recovery metadata. |
| `labonair-process` | Process and PTY launching contracts. |
| `labonair-secrets` | Keychain and secret references. |
| `labonair-persistence` | Cloneable shared SQLite connection and schema lifecycle; feature modules own stores and queries. |
| `labonair-git` | UI-free Git value types plus graph and source-control capability contracts; implementations are injected adapters. |

### Cross-cutting modules

| Module | Initial crate split |
|---|---|
| Settings | `settings-content`, `settings-core`, `settings-json`, `settings-ui`, `settings-macros` |
| Keymap | `keymap-core`, `keymap-ui` |
| Command palette | `command-palette-core`, `command-palette-ui` |
| Notifications | `notifications-core`, `notifications-ui` |
| Themes | `theme-core`, `theme-ui` |
| Workspace | `workspace-core`, `workspace-ui` |

### Product modules

| Module | Responsibility |
|---|---|
| Terminal | PTY sessions, terminal engine, renderer, terminal commands. |
| Editor | Buffers, syntax, editing behavior, editor commands. |
| SSH | SSH transport, authentication, tunnels, jump-host execution. |
| SFTP | Remote filesystem browsing and SFTP operations. |
| Hosts | Saved host definitions, recent hosts, import/export, host management UI. The domain contract and store live in `labonair-hosts`; transport adapters remain capability-owned. |
| Credentials | Credential metadata, secret references, and generated SSH key material in `labonair-credentials`. |
| Transfers | Transfer queue, progress, cancellation, retry, history UI. |
| Git | Git service and source-control UI. |
| Explorer | Local file navigation UI. |
| Snippets | Snippet storage in `labonair-snippets`, execution, and UI. |
| AI | Providers, sessions, context, tools, and future UI. |

The current repository does not yet match this map. The migration is tracked in [`rework-roadmap.md`](rework-roadmap.md); this map is the target, not a claim about the current tree.

## 5. Composition root

Only `labonair-app` and `labonair-shell` may know all concrete feature modules. Their responsibilities are limited to:

- initialize platform services;
- create the application and workspace entities;
- call feature registration functions;
- connect typed events and callbacks;
- compose the permanent shell.

They must not contain SSH logic, transfer logic, theme definitions, settings field definitions, or feature-specific rendering.

## 6. Communication rules

Use the smallest communication mechanism that fits:

1. A direct trait for a synchronous capability request.
2. A typed event for an asynchronous state change.
3. A registry for discoverable, multi-provider contributions.
4. A shared identifier/value type only when it has stable domain meaning.

Stringly typed global events, arbitrary global mutable state, and cross-module entity mutation are prohibited for new code.

The application event bus is a transport boundary, not a business-logic layer. Each module translates external events into its own typed state changes. User-visible messages are forwarded to the notification center.

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

## 8. UI ownership

Feature crates own feature-specific views. `labonair-ui-kit` owns reusable visual and interaction primitives. A feature must use the kit for buttons, dropdowns, lists, menus, inputs, badges, popovers, dialogs, and standard rows.

Feature-specific components may compose kit components, but may not fork their styling locally. New shared behavior is added to the kit first, with a documented API and a component test.

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

## 10. Verification

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
