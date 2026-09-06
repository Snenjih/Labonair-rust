# Module and Crate Rules

**Status:** Normative

## 1. Ownership rule

Every capability has exactly one owning module. The owner is responsible for:

- domain models;
- state and lifecycle;
- backend operations;
- user interface;
- commands and keybindings;
- notifications it emits;
- persistence and migration;
- unit and integration tests.

No other module may duplicate this responsibility.

## 2. When to create a module

Create a module when a capability has its own user intent, lifecycle, state, or persistence. Examples are SSH, SFTP, transfers, themes, hosts, and notifications.

Do not create a module for a visual primitive or a one-line helper. Those belong to foundation crates.

## 3. When to create another crate

Split a module into multiple crates when at least one of these is true:

- the core logic can be tested without GPUI;
- UI and backend have different dependency requirements;
- persistence is an independent boundary;
- multiple consumers need a stable contract;
- compilation isolation materially improves iteration;
- a platform integration needs to be isolated.

Prefer a small number of intentional crates over empty facade crates.

## 4. Standard module layout

```text
crates/<module>-core/       # domain state and services, preferably UI-free
crates/<module>-ui/         # GPUI views and interaction
crates/<module>-storage/    # only when persistence is substantial
crates/<module>-integration/# only for external protocol/platform adapters
```

Small modules may combine these responsibilities in one crate. The module must still expose a narrow public API.

## 5. Public API rule

Public APIs contain domain values, commands, events, traits, and registration functions. They must not expose internal GPUI entities, storage schemas, or implementation-specific widgets unless the owning layer explicitly requires them.

## 6. Feature change workflow

Before implementation:

1. identify the owning module;
2. define the user flow and canonical entry point;
3. define the public contract and events;
4. check whether an existing registry or UI-kit component applies;
5. document new settings, commands, persistence, and notifications;
6. add a focused implementation task.

During implementation:

1. keep state and behavior in the owning module;
2. send user-visible messages through notifications;
3. use UI-kit components;
4. avoid adding shell conditionals;
5. test the contract and the user flow;
6. update the dependency allow-list if a new edge is intentional.

Before completion:

1. remove compatibility code made obsolete by the change;
2. verify all affected surfaces;
3. update the relevant normative document or ADR;
4. run the full verification gates.

## 7. Removal workflow

Removing a capability requires checking command IDs, keymap entries, settings, persistence, migrations, registries, status items, menus, documentation, and tests. Keep data migration only when it protects user data; do not keep dead UI compatibility indefinitely.
