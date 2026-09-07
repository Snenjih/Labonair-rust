# Feature Lifecycle

**Status:** Normative
**Version:** 1

This is the required workflow for adding, redesigning, deferring, or removing
a Labonair capability. It turns a product idea into one owned, testable,
removable implementation without creating parallel systems.

## 1. Classify the request

Before touching code, classify the request as one of:

- **Keep:** the existing capability and workflow fit the product contract;
- **Redesign:** the user flow remains, but ownership, behavior, or UI changes;
- **Defer:** the idea is valid but has no current implementation priority;
- **Remove:** it has no supported workflow, duplicates another surface, or
  conflicts with the product contract.

The classification and its reason belong in the capability matrix or the
active task. A historical implementation is not evidence that a feature
should remain.

## 2. Define the boundary before implementation

Every task must record:

1. the user flow and canonical entry point;
2. the owning module and canonical capability crate;
3. domain state, identifiers, lifecycle transitions, and public snapshots;
4. typed traits/events or a registry, chosen for the actual number of
   consumers and providers;
5. commands, default keybindings, contexts, and palette contributions;
6. notifications, details, actions, and deduplication behavior;
7. settings values and scope, if any;
8. persistence, migration, and secret-handling impact;
9. UI-kit components and the feature-owned view surface;
10. allowed dependency changes and the removal condition for compatibility
    code.

If these answers cannot be written clearly, the task is not ready for code.

## 3. Implement in boundary order

Use this sequence for a new capability or a migration:

1. **Contract:** add typed values, traits, events, or registry entries in the
   owner; add focused contract tests.
2. **Owner:** move state, behavior, UI, persistence, commands, and
   notifications into the owner; use shared UI-kit components.
3. **Adapter:** inject platform/storage implementations at composition time;
   translate external events once at the boundary.
4. **Consumers:** replace direct private-state access with the public contract
   and remove duplicate dispatch or rendering paths.
5. **Removal:** delete the old implementation, obsolete settings/events,
   registrations, imports, and dependency allow-list exception before marking
   the task complete.

The composition root may assemble these pieces, but it may not reinterpret
feature state or maintain a second feature registry.

## 4. User-facing behavior rules

- Every meaningful action has a stable command identity.
- Multi-provider or multi-consumer discovery uses the owning registry;
  single-use behavior uses a direct typed contract.
- Every reusable button, input, list, dropdown, menu, badge, dialog, tab, and
  disclosure control comes from `labonair-ui-kit`.
- Passive user messages go to the notification registry and its statusbar
  dropdown. New toasts and duplicate inline error surfaces are prohibited.
- A local dialog or field validation may remain only when the user must make a
  decision or correct input to complete the current operation.
- Long collections define focused, selected, empty, loading, error, and
  scroll/virtualization behavior.

## 5. Verification and evidence

The task must include focused tests for state and contracts, then run the
repository gates:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo metadata --format-version 1 --no-deps | python3 scripts/check_crate_deps.py
python3 scripts/check_rework_queue.py
python3 scripts/check_documentation.py
git diff --check
```

UI or layout work also requires a manual visual check at normal, narrow,
focused, empty, loading, and error states. If the environment prevents that
check, the task remains in progress and the limitation is recorded in the
handshake; a passing compile is not a substitute for visual evidence.

## 6. Removal checklist

For a removed or replaced capability, inspect and resolve all of:

- command IDs, keymap entries, palette pages, and menu actions;
- settings fields, defaults, migrations, and project-scope handling;
- persistence tables and serialized data;
- panels, status items, badges, dialogs, overlays, and notifications;
- backend adapters, event names, imports, and dependency allow-list entries;
- documentation, tasks, tests, and user-facing labels.

Retain migration code only when it protects existing user data. Retain a
compatibility adapter only with a named consumer and an explicit removal
condition.
