# Labonair-rust

A **hard fork** of Labonair (Tauri v2 + React 19) rewritten as a pure native
Rust application on **GPUI** (the Zed editor's UI framework). Single binary, no
WebView, no JS, no IPC — UI and backend are direct in-process calls.

This repo is fully standalone. The original web-app source is a frozen,
read-only reference copy at [`reference-src/`](./reference-src/) and is the only
reference — never a build target.

## Goal

Full feature parity — everything Labonair does today must work in the pure-Rust
version. Only unavoidable deviation: the in-app web-preview tab becomes native
markdown rendering + "open in system browser" (GPUI cannot embed a WebView).

## Status

See [tasks/ROADMAP.md](./tasks/ROADMAP.md) and [handshake.md](./handshake.md).

## Workspace layout

| Crate | Purpose |
|---|---|
| `crates/app` | Main binary — GPUI application entry (`labonair`) |
| `crates/ui` | UI components & theme provider |
| `crates/theme` | Theme system & design tokens (from `reference-src` `globals.css`) |
| `crates/terminal` | Terminal engine (`alacritty_terminal`) + GPUI renderer |
| `crates/editor` | TreeSitter-based code editor |
| `crates/backend` | SSH, SFTP, Git, filesystem, PTY, hosts, credentials, secrets |
| `crates/ai` | AI provider integration, agent/tool system, chat sessions |

## Commands

| Task | Command |
|---|---|
| Type-check | `cargo check` |
| Build | `cargo build` |
| Run | `cargo run` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format | `cargo fmt` |
| Test | `cargo test` |

Platform: macOS first (Metal), Linux later, no Windows. Building GPUI on macOS
requires the Metal Toolchain (`xcodebuild -downloadComponent MetalToolchain`).

The roadmap and task-by-task plan live in [`tasks/`](./tasks/).
