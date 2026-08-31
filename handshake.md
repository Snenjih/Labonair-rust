# Handshake — Session State (Labonair-rust Port)

Authored by: GPUI-native port of Labonair (formerly Tauri v2 + React 19 → now pure Rust/GPUI).

> This file is the authoritative continuity doc for the **port** project. This is a **hard fork** — fully standalone, no link/symlink/submodule to any external Labonair repo. The old web-app source is a frozen read-only copy at `reference-src/` inside this repo and is the only reference. Do not mistake the old git history/tech for the current target.

## Last Session: 2026-08-31 (T01-003 — verify reference-src + project docs)

### What Was Done
- **T01-003 ✅ Done.** Verified all reference paths under `reference-src/` exist and are readable: `src/styles/globals.css`, `src/modules/` (23 feature modules), `src-tauri/src/modules/` (all listed backend modules incl. `secrets.rs`, `dock_menu.rs`, `menu_sync.rs`, `errors.rs`), `src-tauri/Cargo.toml`, `src-tauri/src/modules/pty/scripts/` (zshrc/bashrc + z* init scripts). Nothing missing.
- **README.md** already existed (from T01-001) and already describes the hard-fork / read-only `reference-src/` character. Added a "Goal" (full feature parity, web-preview → native markdown) and "Status" (links to ROADMAP + handshake) section.
- **.gitignore** confirmed: `reference-src/` is NOT ignored (tracked); no external symlink entry.
- Reworded 3 historical lines in `handshake.md` that still contained the literal old `../Labonair` path so `git grep -n "\.\./Labonair"` is now 0 hits.
- **Verified:** `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` (117 tests) — all green.

### Current State
- Branch `master`, 3 unpushed commits + this one. No code changes — docs only.
- `reference-src/` untouched.

### What's Next
- **T01-004** `tasks/phase-00-setup/T01-004-event-system.md` — typed event routing/logging on top of `crate::events`. Deps (T01-001, T01-002) satisfied.
- Then T01-005 (CI).

### Blockers
- None.

---

## Session: 2026-08-31 (T01-002 — full backend port from reference-src, Tauri stripped)

### What Was Done
- **T01-002 ✅ Done.** Ported the entire Rust backend from `reference-src/src-tauri/src/modules/` into `crates/backend/src/modules/` (18 modules: ssh, sftp, git, fs, pty, hosts, credentials, snippets, secrets, shell, themes, backgrounds, fonts, scrollback, terminal_exec, settings, mcp, errors). ~16k LOC, ~150 `#[tauri::command]` fns.
- **Approach: verbatim copy + mechanical Tauri strip** (perl passes in `/tmp/detauri.pl`) rather than hand-retyping — preserves logic 1:1. Transformations: drop `#[tauri::command]`; `tauri::State<'_,T>` → `&T`; `tauri::AppHandle` → `crate::App`; `tauri::ipc::Channel<T>` → `crate::events::EventChannel<T>`; `use tauri::*` removed; `.inner().clone()` → `.clone()`; `tauri::async_runtime::spawn` → `tokio::spawn`; `app.state::<T>()` → `&app.<field>` (5 sites, hand-fixed); `app.emit(...)` → `crate::App::emit`.
- **New infra:** `crates/backend/src/events.rs` (`EventBus` broadcast + `EventChannel<T>` point-to-point sink — forward-compatible with T01-004), `crates/backend/src/app.rs` (`App`/`AppState` = `Arc<AppInner>` holding every sub-state + `EventBus`; `App::new(&Path)` opens SQLite + builds all state; `App::spawn_workers()` starts the SFTP transfer worker + MCP auto-revoke sweeper). `lib.rs` re-exports `App`, `AppState`, `AppError` (= `LabonairError`, `From` impls extended with `serde_json::Error` + `String`), `AppResult`.
- **Hand-rewritten:** `settings/mod.rs` (`tauri_plugin_store` → plain atomic JSON read/merge/write against `config_dir()/labonair-settings.json`). `modules/mod.rs` (dropped `dock_menu`/`menu_sync` → deferred to T04-005).
- **Deps added to `crates/backend/Cargo.toml`:** russh(+rsa), russh-sftp, rusqlite, reqwest(rustls), portable-pty, tokio-util, thiserror, log, aes-gcm, rand, md5, openssl(vendored), base64, ignore, grep-regex, grep-searcher, globset, notify, notify-debouncer-mini, flate2, vte 0.15, rmcp 2.2, axum 0.8, schemars, fontdb, socket2, libc, dirs, uuid. (No git2 — git via CLI. No keyring — secrets use aes-gcm local store.)
- **One pre-existing broken test fixed:** `git::tests::branches_parses_current_and_upstream_tracking` used `|` separators but `parse_branches` splits on `\0` (matches `BRANCH_FORMAT` `%00`) — test data corrected, no logic change.
- **Verified:** `cargo check`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` — all green. 116 backend unit tests + `tests/app_state.rs` (AppState init acceptance test) pass.

### Current State
- Branch `master`, 2 unpushed commits ahead + this one. `crates/backend` fully ported and green. `crates/app` still just the empty GPUI window (does not wire `AppState` yet — that's later phases).
- `reference-src/` untouched.

### What's Next
- **T01-003** `tasks/phase-00-setup/T01-003-reference-symlink.md` — verify `reference-src/` is intact + write project README/.gitignore notes. Deps (T01-001) satisfied.
- Then T01-004 (event system — build typed routing on top of `crate::events`), T01-005 (CI).

### Blockers
- None. Note for later: SSH/SFTP/git/MCP got a *mechanical* Tauri strip — runtime behaviour needs the user's testing once a UI wires them (per the project's KI-builds / user-tests workflow).

---

## Session: 2026-08-31 (T01-001 — Cargo workspace scaffolded, empty GPUI window runs)

### What Was Done
- **T01-001 ✅ Done.** Scaffolded the cargo workspace: root `Cargo.toml` (resolver 2, `[workspace.dependencies]`) + 7 crates under `crates/`: `app` (bin `labonair`), `ui`, `theme`, `terminal`, `editor`, `backend`, `ai`. Placeholder `lib.rs` per crate (doc comment only — **not** the `pub mod ...;` stubs the task listed, since those module files don't exist yet and would break `cargo build`; later phases add them).
- **`crates/app/src/main.rs`** — GPUI entry: `Application::new().run(...)`, `Bounds::centered`, `cx.open_window(WindowOptions { window_bounds: Some(WindowBounds::Windowed(bounds)), .. }, |_, cx| cx.new(|_| Root))`, `cx.activate(true)`. `Root` is a `Render` view: full-size `div` bg `0x1a1b26`, child text "Labonair-rust — ready for development" in `0xc0caf5`. `tracing_subscriber::fmt::init()`.
- **Deps corrected vs task file**: `gpui = "0.2.2"` (real crates.io crate; `gpui-component` is a **separate** crate `0.5.1`, NOT a gpui feature — deferred to T04+). Dropped `alacritty_config_derive`, `fontdb`, `rsa` feature on russh (not needed yet). russh resolved to 0.62.7, russh-sftp 2.4.0.
- **Metal Toolchain installed** — `cargo build` of gpui failed with "missing Metal Toolchain"; fixed via `xcodebuild -downloadComponent MetalToolchain` (688 MB, one-time, machine-level).
- Root `README.md` added (workspace layout + commands + Metal Toolchain note).
- Verified: `cargo check`, `cargo build`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, `cargo test` all green. `cargo run` opens the window — **user visually confirmed** "ready for development" text renders.

### Current State
- Branch `master` (still 1 unpushed planning commit ahead + this one). `Cargo.lock` committed.
- Working GPUI window. `reference-src/` untouched.

### What's Next
- **T01-002** `tasks/phase-00-setup/T01-002-extract-backend-logic.md` — port backend modules from `reference-src/src-tauri/src/modules/` into `crates/backend/`, stripping Tauri/IPC. Deps (T01-001) satisfied.

### Blockers
- None.

---

## Session: 2026-08-31 (Planning correction: hard-fork restructure, reference-src, roadmap gaps closed)

### What Was Done
- **Hard fork clarified & enforced.** User: Labonair-rust must be fully decoupled — no symlink/submodule/path-dependency to the original repo. Memory written (`memory/hard-fork-reference-src.md`).
- **Moved the copied web app into `reference-src/`** (`git mv` of `src/`, `src-tauri/`, `docs/`, `scripts/`, all web configs, `CHANGELOG/CONTRIBUTING/SECURITY/README`, and the old `.github/workflows/`). Repo root is now clean: `CLAUDE.md`, `handshake.md`, `tasks/`, `reference-src/`, `LICENSE`, `.github/` (templates only), `.gitignore`.
- **Swept old external relative paths → `reference-src`** across CLAUDE.md, handshake.md, ROADMAP.md, and all task files (0 remaining hits for the old parent-dir path).
- **`.gitignore` rewritten** for Rust (`/target`, `.claude/`, `session-*.md`); `reference-src/` is tracked.
- **T01-001 deps corrected** to match `reference-src/src-tauri/Cargo.toml`: `russh 0.62.2` (ring), `russh-sftp 2.3.0`, `rusqlite 0.40`, `portable-pty 0.9`; **removed `git2`** — git runs via the `git` CLI (local + remote-over-SSH), same as the original. Same fix applied in T01-002 and T09-001.
- **T01-003 rewritten** — no longer a symlink-to-parent-repo task; now "verify reference-src + write README + .gitignore". Standalone.
- **Roadmap gaps closed** (user wants full feature parity, nothing out of scope). Added tasks:
  - `T01-005` CI pipeline (cargo check/clippy/test/fmt on macOS)
  - `T02-005` font handling/bundling · `T02-006` terminal background images
  - `T04-003` app-shell & window chrome (header/statusbar/sidebar/root coordinator) · `T04-004` notifications/toasts · `T04-005` native macOS menus (app menu + dock menu)
  - `T11-005` MCP bridge server · `T11-006` MCP bridge UI/grants
  - `T15-005` auto-updater (Sparkle) · `T15-006` feature-parity acceptance checklist
  - ROADMAP: added "Feature-Parität" section + success criteria 16–21. Only accepted deviation: web-preview tab → native markdown + open-in-system-browser (GPUI has no WebView).

### Current State
- Branch `master`. This planning-correction commit is **local, not yet pushed** (push when user asks).
- `reference-src/` holds the full frozen original. No cargo workspace scaffolded yet.
- Roadmap now has **55 task files** across 15 phases + expanded ROADMAP.md.

### What's Next
1. Execute **`tasks/phase-00-setup/T01-001-setup-cargo-workspace.md`** per the Next Task Protocol: scaffold the cargo workspace (7 crates), empty GPUI window, `cargo check`/`clippy`/`run` green → mark `✅ Done` → update handshake → commit.
2. Then T01-002 (extract backend from `reference-src/`) → T01-003 (verify reference-src + README) → T01-004 (events/logging) → T01-005 (CI).

### Blockers
- None. T01-001 has no dependencies.

---

## Session: 2026-08-31 (Roadmap complete + repo created + CLAUDE.md Next Task Protocol)

### What Was Done
- **Roadmap built out fully.** `tasks/ROADMAP.md` defines the 15-phase port (Setup → Theme → Terminal → Tabs → Explorer → Editor → SSH → SFTP → Git → Git-Graph → AI → Snippets/Palette → Settings → Session → Testing/Polish). 43 task files exist under `tasks/phase-*/` (each is instruction-only: context, goal, instructions, acceptance criteria, notes, warnings — no code). Naming: `T{NN}-{OOO}.md` where NN = phase 01–15, OOO = task number (files currently use the sequential scheme, e.g. `T01-001-setup-cargo-workspace.md`, `T13-001-settings-preferences.md`, `T15-004-packaging-release.md`). Next Task Protocol in CLAUDE.md consumes these in order.
- **Initial commit** `ead881a` committed the full repo (including the copied original source, kept as reference).
- **GitHub repo created & linked**: `gh repo create Labonair-rust --public --source=. --remote=origin --push` → remote `origin` set, `master` tracked to `origin/master`, URL https://github.com/Snenjih/Labonair-rust
- **CLAUDE.md rewritten** for the port (was stale Tauri/web content). Now documents: Rust/GPUI commands (`cargo check/build/run/clippy/test/fmt`), the single-binary GPUI architecture, Critical Rules (no web tech, reference-only original, GPU API must be source-verified), **Next Task Protocol** (the core task-by-task workflow), session start/end protocols, bug-memory rule, reference usage, language protocol.
- **handshake.md reset** for the port project (this file).

### Current State
- Branch `master`, repo pushed to `origin/master`.
- **First implementation task** (`T01-001-setup-cargo-workspace.md`) is the next to work — nothing implemented yet (only roadmap + config docs). `cargo` project has NOT yet been scaffolded.
- The original source files (src/, src-tauri/, docs/, package.json, etc. from the clone) still sit in the repo as an untracked-from-the-web-app reference. **These are reference artifacts for the port to read, NOT the app to extend.** Future phases will replace the web-tech portions.

### What's Next
1. Execute **T01-001-setup-cargo-workspace.md** per the Next Task Protocol in CLAUDE.md: scaffold the cargo workspace, set it up, verify (`cargo check`), mark `✅ Done`, update this handshake, commit.
2. Then T01-002 → T01-003 (reference symlink → `reference/`) → T01-004, and continue through the phases in order.

### Blockers
- None. Next task is dependency-clean (T01-001 has no dependencies).

---

## Previous Session: (none — port infrastructure only)

Project start. Created repo clone, initialized git, wrote the full task roadmap + task files. No application code written yet.
