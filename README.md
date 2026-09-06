# Labonair-rust

Labonair is a standalone native Rust application built with GPUI. It is a fast, keyboard-first Dev-Op workspace for local and remote work, combining terminal, editor, SSH, SFTP, Git, transfers, themes, keymap, notifications, and AI as independently owned capabilities.

The product supports both project workspaces and standalone use. It is not a terminal-only app, an IDE-only app, or a Zed fork. Zed is used as a clean-room reference for interaction patterns and architecture ideas. The frozen predecessor source is [`reference-src/`](reference-src/).

## Current direction

The active architecture and rework sequence are documented in [`docs/`](docs/):

- [`docs/product.md`](docs/product.md) — product contract
- [`docs/architecture.md`](docs/architecture.md) — target architecture
- [`docs/modules.md`](docs/modules.md) — module and crate rules
- [`docs/registries.md`](docs/registries.md) — registry contracts
- [`docs/design-system.md`](docs/design-system.md) — UI consistency rules
- [`docs/repository-layout.md`](docs/repository-layout.md) — canonical repository and crate placement
- [`docs/feature-lifecycle.md`](docs/feature-lifecycle.md) — required feature change workflow
- [`docs/rework-roadmap.md`](docs/rework-roadmap.md) — implementation sequence
- [`docs/capabilities.md`](docs/capabilities.md) — ownership and migration matrix
- [`docs/settings.md`](docs/settings.md) — value-only settings boundary
- [`docs/workspace-model.md`](docs/workspace-model.md) — project and standalone workflows

The old task tree remains for historical traceability. New work must follow the rework roadmap and not the historical queue.

The architectural invariant is simple: one product capability, one owning
module, and one canonical capability crate. A module may split into sibling
core, UI, storage, or integration crates only when a real boundary justifies
it. The shell composes registered capabilities; it does not implement them.

Before adding a feature, update the capability matrix, choose its canonical
surface, define its typed contract, and check whether an existing registry or
UI-kit component already provides the needed extension point. User-visible
messages go to the notification dropdown, and settings contain values only.

## Build commands

```text
cargo check --workspace --all-targets
cargo build
cargo run
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
python3 scripts/check_rework_queue.py
```

Platform priority is macOS first, with Linux later. The application contains no WebView, JavaScript frontend, or IPC layer.
