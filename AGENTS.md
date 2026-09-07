# Labonair-rust — Agent Instructions

Labonair-rust is a standalone native Rust/GPUI application. `reference-src/` is a frozen behavioral and visual reference. Never edit it, link to an external Labonair repository, or reintroduce web technologies into the result.

The documents in `docs/` define the target architecture. The inventory and
capability matrix explicitly describe unfinished migrations; do not mistake a
target rule for a claim that the current tree already satisfies it.

The current product and engineering contracts are in [`docs/README.md`](docs/README.md). Read the relevant normative documents before changing architecture, product behavior, module boundaries, settings, registries, or UI. The canonical repository placement is [`docs/repository-layout.md`](docs/repository-layout.md), and the required boundary-first change workflow is [`docs/feature-lifecycle.md`](docs/feature-lifecycle.md).

## Product direction

Labonair is a fast, keyboard-first Dev-Op workspace for local and remote work. Terminal, editor, SSH, SFTP, Git, transfers, themes, keymap, notifications, and AI are independent feature modules. The product supports both project workspaces and standalone tool use.

The application is not a terminal-only app, not an IDE-only app, and not a Zed fork. Zed is used as a behavioral and architectural reference only; its source is not copied.

## Architecture rules

1. Every product capability has exactly one owning module and one canonical
   owning capability crate. A module may contain sibling crates for real core,
   UI, storage, or integration boundaries, but those crates remain under the
   same owner and may not create a second owner.
2. The owning module owns the capability's state, behavior, UI, persistence,
   commands, notifications, and tests. Splitting those concerns across crates
   does not split ownership.
3. Each product crate has one capability or one explicitly named foundation
   concern. Do not combine unrelated capabilities in a general-purpose crate.
4. Foundation crates, especially `labonair-ui-kit`, must not depend on product modules.
5. Feature crates must not depend on `labonair-shell` or reach into another feature's private state.
6. Cross-module behavior uses the smallest suitable typed contract, typed
   event, or registry. A registry is required when multiple providers or
   consumers need discovery; it is not a default abstraction for one caller.
7. `labonair` and `labonair-shell` are composition roots only; they must not become feature god objects.
8. Every reusable control uses `labonair-ui-kit`. Feature crates compose shared components instead of creating local button, list, menu, input, badge, or dropdown styles.
9. Settings contain values only. Hosts, themes, icon themes, keymap editing, transfers, notifications, and command registration belong to their own modules.
10. New permanent shell chrome requires an ADR and must not duplicate an existing product surface.
11. New dependencies require a documented reason and must preserve an acyclic, CI-checked graph.
12. The command palette, keymap, hosts, themes, transfers, and notification
    surfaces have one canonical owner and entry point as defined in `docs/`.
    Do not add parallel shell tables, settings categories, toasts, or feature
    local status surfaces.

## Product surfaces

The shell has four permanent zones and one overlay layer:

- titlebar: tabs and one global menu button;
- workspace: active tab content and split panes;
- docks: registered panels;
- statusbar: panel controls on the left and global information items on the right;
- overlay layer: command palette and dialogs.

Notifications are shown in the statusbar notification dropdown. There is no toast system.

## Development rules

- Keep code, comments, commits, and normative documentation in English.
- Keep changes surgical and scoped to the requested module or contract.
- Do not block the GPUI foreground thread. Use async work, Tokio tasks, or `spawn_blocking` for I/O.
- Do not use `unwrap()` for predictable runtime errors; return descriptive results.
- Do not add speculative abstractions or compatibility paths without an explicit removal reason.
- Consult GPUI and the local Zed reference source before relying on undocumented APIs.
- Never expose secrets in source, logs, SQLite, tests, or commits.
- Run the native application with `cargo run -p labonair` or an explicit
  `target/**/labonair` path. Never use `open -a Labonair` for development or
  visual verification because the legacy Tauri app may share that display
  name.

## Feature workflow

Before implementation, identify the owning module, canonical user entry point, public contract, events, registry contributions, settings, persistence, and UI-kit components. Then create or update a task from [`docs/rework-roadmap.md`](docs/rework-roadmap.md).

During implementation, keep feature logic inside its owner, route user messages through the notification center, and avoid shell-wide conditionals.

The only active architecture-rework queue is `tasks/rework/`. Follow its
README in dependency order and work only on the earliest incomplete task;
everything under `tasks/archive/` is historical and is not an alternate
implementation queue.

For every new feature or migration, follow [`docs/feature-lifecycle.md`](docs/feature-lifecycle.md) and record the owner, canonical capability
crate, public contract, user entry point, registry contributions, settings,
persistence, notifications, and UI-kit components before coding. Add the
capability to `docs/capabilities.md` and the work to
`docs/rework-roadmap.md` or its implementation task. Move the contract first,
then adapters and consumers, and remove the old path once no consumer needs
it. Compatibility code is allowed only with a named removal condition.

Prefer removal or deferral over a new framework when a feature has no current
user workflow. A passing build does not justify keeping duplicate UI,
stringly-typed dispatch, a backend facade, or an unused setting.

Before completion, update the relevant normative document or ADR, remove obsolete compatibility code, and run:

```text
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

UI and layout changes also require a visual check. Dependency changes require the crate dependency verifier.
Documentation or repository-control changes require
`python3 scripts/check_documentation.py`. Changes to the active architecture
queue also require `python3 scripts/check_rework_queue.py`.

## Repository continuity

`handshake.md` records session continuity only. It is not an architecture authority. Record non-obvious bugs and API discoveries in `memory/bugs_and_fixes.md`.
