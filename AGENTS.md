# Labonair-rust — Agent Instructions

Labonair-rust is a standalone native Rust/GPUI application. `reference-src/` is a frozen behavioral and visual reference. Never edit it, link to an external Labonair repository, or reintroduce web technologies into the result.

The current product and engineering contracts are in [`docs/README.md`](docs/README.md). Read the relevant normative documents before changing architecture, product behavior, module boundaries, settings, registries, or UI.

## Product direction

Labonair is a fast, keyboard-first Dev-Op workspace for local and remote work. Terminal, editor, SSH, SFTP, Git, transfers, themes, keymap, notifications, and AI are independent feature modules. The product supports both project workspaces and standalone tool use.

The application is not a terminal-only app, not an IDE-only app, and not a Zed fork. Zed is used as a behavioral and architectural reference only; its source is not copied.

## Architecture rules

1. Every capability has exactly one owning module.
2. A module owns its state, behavior, UI, persistence, commands, notifications, and tests.
3. A module may contain separate core, UI, storage, and integration crates when that creates a real boundary.
4. Foundation crates, especially `labonair-ui-kit`, must not depend on product modules.
5. Feature crates must not depend on `labonair-shell` or reach into another feature's private state.
6. Cross-module behavior uses typed traits, typed events, or registries.
7. `labonair-app` and `labonair-shell` are composition roots only; they must not become feature god objects.
8. Every reusable control uses `labonair-ui-kit`. Feature crates compose shared components instead of creating local button, list, menu, input, badge, or dropdown styles.
9. Settings contain values only. Hosts, themes, icon themes, keymap editing, transfers, notifications, and command registration belong to their own modules.
10. New permanent shell chrome requires an ADR and must not duplicate an existing product surface.
11. New dependencies require a documented reason and must preserve an acyclic, CI-checked graph.

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

## Feature workflow

Before implementation, identify the owning module, canonical user entry point, public contract, events, registry contributions, settings, persistence, and UI-kit components. Then create or update a task from [`docs/rework-roadmap.md`](docs/rework-roadmap.md).

During implementation, keep feature logic inside its owner, route user messages through the notification center, and avoid shell-wide conditionals.

Before completion, update the relevant normative document or ADR, remove obsolete compatibility code, and run:

```text
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

UI and layout changes also require a visual check. Dependency changes require the crate dependency verifier.

## Repository continuity

`handshake.md` records session continuity only. It is not an architecture authority. Record non-obvious bugs and API discoveries in `memory/bugs_and_fixes.md`.
