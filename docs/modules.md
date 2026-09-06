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

Each product capability also has exactly one canonical capability crate. The
crate is the default home for the capability's public contract and core
behavior. If the capability grows, sibling `-ui`, `-storage`, or
`-integration` crates may be added inside the same module; they are an
implementation split, not a new owner. A crate must never mix unrelated
product capabilities.

## 2. When to create a module

Create a module when a capability has its own user intent, lifecycle, state, or persistence. Examples are SSH, SFTP, transfers, themes, hosts, and notifications.

Do not create a module for a visual primitive or a one-line helper. Those belong to foundation crates.

The product-module rule is intentionally explicit:

> One product capability has one owning module and one canonical capability
> crate. More crates are allowed only as named siblings within that module and
> only when a real boundary exists.

This prevents both mixed god-crates and speculative forests of empty facade
crates. A reusable button or list is a UI-kit component, not a product crate.
Buttons, lists, badges, and other reusable controls remain in the UI kit.

## 3. When to create another crate

Split a module into multiple crates when at least one of these is true:

- the core logic can be tested without GPUI;
- UI and backend have different dependency requirements;
- persistence is an independent boundary;
- multiple consumers need a stable contract;
- compilation isolation materially improves iteration;
- a platform integration needs to be isolated.

Do not split merely to mirror a theoretical layer. The new crate must have a
single responsibility, a current consumer or independent test boundary, and a
public API that reduces coupling. Do not create an empty facade crate.

## 4. Standard module layout

```text
crates/<capability>/             # canonical capability crate
crates/<capability>-ui/          # optional GPUI views and interaction
crates/<capability>-storage/     # optional substantial persistence boundary
crates/<capability>-integration/ # optional external protocol/platform adapter
```

The canonical crate may contain the UI and backend for a small capability. As
the module grows, move a responsibility into a sibling crate without moving
ownership to another module. Every sibling must expose a narrow public API.

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

The composition root may call registration functions and inject concrete
services, but it must not own feature state, render feature views, or contain
feature-specific dispatch tables. A feature registers its own commands,
keybindings, panels, status items, theme entries, and notification action
handlers through the appropriate typed registry.

Before completion:

1. remove compatibility code made obsolete by the change;
2. verify all affected surfaces;
3. update the relevant normative document or ADR;
4. run the full verification gates.

## 7. Removal workflow

Removing a capability requires checking command IDs, keymap entries, settings, persistence, migrations, registries, status items, menus, documentation, and tests. Keep data migration only when it protects user data; do not keep dead UI compatibility indefinitely.

For a migration, first write down the current owner and all consumers, then
establish the target contract and owner. Move one boundary at a time, keep any
temporary adapter behind a named removal condition, and update the capability
matrix, inventory, roadmap/task, and dependency verifier together. Delete the
old registration and compatibility path once consumers have moved; do not run
two implementations in parallel indefinitely.
