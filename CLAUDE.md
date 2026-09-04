# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with this repository.

# Labonair-rust — CLAUDE.md
**AI Developer Guidelines & Project Reference**

This is a **hard fork** of Labonair — a Tauri v2 + React 19 web app being rewritten as a **pure native Rust app using GPUI**. It is fully standalone: there is **no** link, symlink, submodule, or dependency back to the original Labonair repo. The original web-app source was copied once into **`reference-src/`** inside this repo and that copy is the *only* reference — a read-only design/behavior spec, never a build target. Work exclusively from what's in `reference-src/`. The web-app artifacts there (React, WebView, xterm.js, CodeMirror, Zustand, Tailwind) are being **replaced** by native Rust, not extended.

**Goal is full feature parity:** everything Labonair does today must work in the pure-Rust version at the end — no feature is out of scope. The only unavoidable deviation is the in-app URL/web preview tab (GPUI cannot embed a WebView); that is replaced by native markdown rendering + "open in system browser".

## Philosophie (ab Architektur-Rework)

**„Der effizienteste Weg, seine Arbeit in Labonair fertig zu bekommen — mit maximaler Performance und Modularität für Personalisierung."** Ab den Roadmap-Phasen 15–21 ist das die normative Leitlinie für alle Tasks: Feature-Parität mit der Referenz-App bleibt Pflicht, ist aber ab hier das *Minimum*, nicht das Ziel. Maßgebliche Ziel-Architektur, Layout-Vertrag und Crate-Graph: [`docs/architecture.md`](./docs/architecture.md).

## Commands

| Task | Command |
|---|---|
| Check (type-check Rust) | `cargo check` |
| Build (debug) | `cargo build` |
| Run | `cargo run` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Format check | `cargo fmt --check` |
| Format | `cargo fmt` |
| Rust tests | `cargo test` |
| TreeSitter grammars | `cargo build --release --features build-grammars` |

There is **no** `pnpm`/`npm`/Node toolchain in this project (nothing to build on the frontend — there is no frontend). Do not run `pnpm` commands. Test runner is `cargo test` (unit tests colocated as `#[cfg(test)]` modules / `tests/`).

## Architecture

Single native Rust binary using **GPUI** (the Zed editor's UI framework) + **gpui-component**. No webview, no JS, no IPC — UI and backend are direct in-process function calls.

```
Labonair-rust (single GPUI binary)
├── UI (GPUI + gpui-component)
│   ├── Terminal renderer (alacritty cells)
│   ├── TreeSitter editor
│   ├── Tabs/Split-layout, Explorer, SFTP, Git, AI-chat, Settings
│   └── Theme layer (oklch tokens from the original globals.css)
└── Backend (directly embedded modules, NO IPC)
    ├── alacritty_terminal     → terminal engine
    ├── portable-pty           → local PTY sessions
    ├── russh + russh-sftp     → SSH + SFTP
    ├── rusqlite               → SQLite (hosts/creds/snippets)
    ├── keyring                → OS keychain (macOS), secrets never in SQLite
    ├── git CLI                → source control (local + remote-over-SSH)
    └── tokio                  → transfer queue, AI streaming
```

Platform: **macOS first, Linux later, no Windows.** The GPUI renderer differs per platform (macOS Metal / Linux Vulkan); keep platform-specific integration isolated.

The authoritative **target architecture** for the ongoing crate-split rework (roadmap phases 15–21) is [`docs/architecture.md`](./docs/architecture.md) — the target crate graph, the binding dependency rules, the layout contract, and the Zed pattern catalog. Its rationale is recorded in [`docs/adr/0001-crate-decomposition.md`](./docs/adr/0001-crate-decomposition.md). When a rework task is unclear, consult that document before deciding.

## Critical Rules (NEVER Violate)

1. **Reference, don't edit the source** — `reference-src/` is a read-only design/behavioral reference (a frozen copy). Never modify files there. Never add a link/symlink/submodule to any external Labonair repo — this fork is standalone.
2. **No web tech in the result** — no WebView, no xterm.js, no CodeMirror, no Zustand, no Tailwind, no npm deps. GPUI-native rendering only.
3. **UI values come from the reference** — colors/spacing/radii/shadows are taken 1:1 from `reference-src/src/styles/globals.css` (converted from oklch to GPUI theme). No hardcoded arbitrary values that diverge from the reference.
4. **No incomplete/wrong GPUI usage** — GPUI is largely **undocumented**: check `cargo doc`, the `gpui`/`gpui-component` source, or the Zed codebase (both on GitHub) before guessing an API. Never invent an API that doesn't exist.
5. **No blocking on the main thread** — all I/O is `async`/`tokio::spawn` / `spawn_blocking`. Never `std::thread::sleep` on the main thread.
6. **No `unwrap()` on predictable errors** — return `Result<T, String>` (or GPUI error type) with descriptive messages.
7. **Verify before completing** — a task is only `completed` after `cargo check` + `cargo clippy -- -D warnings` + `cargo test` pass.
8. **Layout-Vertrag einhalten** — Titlebar nur Tabs + der eine Menü-Button; Statusbar = Panel-Steuerung links / Info-Dropdowns rechts; Overlays nur über `ModalLayer`/`ToastLayer`. Abweichungen zuerst in `docs/architecture.md` begründen.
9. **Settings-Design-Kontrakt einhalten** — jede Einstellung ist ein typisiertes `SettingsContent`-Feld mit generierter UI; ein Navigations-Modell (Kategorie → Abschnitt → optionale Unter-Seite); Custom-Panes nur für echte Nicht-Feld-UIs und immer im Standard-Chrome. Details + Abweichungs-Prozess: `docs/settings-guidelines.md`.

## Next Task Protocol (THE core workflow)

The source of truth for what to build is the **roadmap in [`tasks/ROADMAP.md`](./tasks/ROADMAP.md)** with exhaustive task files in [`tasks/phase-*`](./tasks/). Work proceeds **strictly task-by-task in order**. When asked to "work on the next task" — or when starting any implementation session — do the following:

1. **Find the next task.** Read `tasks/ROADMAP.md` and scan the `tasks/phase-*/` task files. Each task file has a `## Status` header followed by a status value on the next line ( `✅ Done`). The next task is the earliest-phase, lowest-numbered `T*\-\*` file. Respect the `## Abhängigkeiten` (dependencies) listed in that task — only start it once its dependencies are marked Done.
2. **Read the full task file.** Understand its `## Ziel` (goal), `## Kontext`, `## Anweisungen` (instructions), `## Akzeptanzkriterien` (acceptance criteria), `## Notizen`, `## Warnungen` (warnings) before writing any code.
3. **Set its status.** Change the value line under the task file's `## Status` header to .
4. **Implement.** Write the code following the task's instructions and the Critical Rules above. Use the `cargo` commands to compile/test frequently. When you need to check the reference app's exact behavior/values, read the corresponding files under `reference-src` (e.g. `src/styles/globals.css` for theme, `src-tauri/src/modules/*/` for backend logic to port, `src/modules/*/` for UI behavior to replicate).
5. **Verify.** Satisfy the task's `## Akzeptanzkriterien`:
   - Run `cargo check`, `cargo clippy -- -D warnings`, and `cargo test`. All must pass (add/adjust tests as the task requires).
   - Confirm each acceptance criterion is genuinely met.
6. **Mark it done.** Change the value line under the task file's `## Status` header to `✅ Done`.
7. **Update `handshake.md`** with a new entry at the top: what was done (concrete, referencing the task id + files), the current state (branch/commit), what's next (the next task id), and any blockers. Also record any non-obvious lessons, API quirks, or bugs encountered in `~/.claude/projects/.../memory/bugs_and_fixes.md` (Bug & Error Memory Rule below).
8. **Commit.** Commit all changes (task file status, code, handshake, memory summary) with a descriptive conventional commit message referencing the task id, e.g. `feat(terminal): T03-001 integrate alacritty engine`. Push only if asked.

> If the user says something like "work on the next task" or "continue", you run the exact workflow above. Do NOT skip ahead, work out of order, or tackle multiple tasks at once — one task at a time, in order, each ending verified + committed.

## Session START Protocol

At the start of every session:
1. Read `handshake.md` (repo root) to see where the last session left off — the authoritative continuity doc (last task completed, current branch/commit, next task id, blockers).
2. If there is an uncommitted `🔄 In Progress` task, finish or resolve it first.
3. Check `git status` / `git log` for in-flight work not yet reflected in `handshake.md`.
4. Briefly summarize to the user: last completed task, next task id you're about to work, and any blockers.

## Session End Protocol

At the end of every session:
1. Update `handshake.md` with what was done, current state, what's next, blockers.
2. Write memory entries to `~/.claude/projects/.../memory/` for progress and any bugs/decisions.
3. Commit all changes with a descriptive conventional commit message.
4. Record any error/debug you encountered in `~/.claude/projects/.../memory/bugs_and_fixes.md` (what failed, why, how fixed).

## Bug & Error Memory Rule

If you hit a build error, unexpected behavior, or had to debug something non-obvious:
→ Write a memory entry in `~/.claude/projects/.../memory/bugs_and_fixes.md`
→ Include: what failed, why it failed, how it was fixed. In particular, note any GPUI API you had to reverse-engineer (crate version, function signature, or where in the Zed source you confirmed it) — this knowledge is hard-won and should not be re-derived.

## Reference Usage

The frozen reference copy lives at `reference-src/` in this repo. It is the single source of truth for:
- **Visual design**: `reference-src/src/styles/globals.css` (oklch token values), component layout, spacing, fonts.
- **Behavior**: how each feature behaves (terminal, explorer, editor, SSH, SFTP, git, AI, settings, backgrounds, updater, native menus, MCP bridge).
- **Backend logic to port**: `reference-src/src-tauri/src/modules/*/` — port these to native Rust modules under `crates/`, stripping the Tauri/IPC wrappers and calling them in-process.

Read from `reference-src/` only; never edit it, never reach outside the repo for it.

## Language Protocol

- **Code, Comments, Commits & Documentation:** ALWAYS in English.

## Repo / Remote

- GitHub: `https://github.com/Snenjih/Labonair-rust`
- Default branch: `master` (push/PR workflow is feature/PR-driven — only create branches/PRs when the user asks, following the Conventional Commits style).
