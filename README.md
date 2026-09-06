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
- [`docs/rework-roadmap.md`](docs/rework-roadmap.md) — implementation sequence

The old task tree remains for historical traceability. New work must follow the rework roadmap and not the historical queue.

## Build commands

```text
cargo check --workspace --all-targets
cargo build
cargo run
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Platform priority is macOS first, with Linux later. The application contains no WebView, JavaScript frontend, or IPC layer.
