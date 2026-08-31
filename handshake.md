# Handshake — Session State (Labonair-rust Port)

Authored by: GPUI-native port of Labonair (formerly Tauri v2 + React 19 → now pure Rust/GPUI).

> This file is the authoritative continuity doc for the **port** project. This is a **hard fork** — fully standalone, no link/symlink/submodule to any external Labonair repo. The old web-app source is a frozen read-only copy at `reference-src/` inside this repo and is the only reference. Do not mistake the old git history/tech for the current target.

## Last Session: 2026-08-31 (Planning correction: hard-fork restructure, reference-src, roadmap gaps closed)

### What Was Done
- **Hard fork clarified & enforced.** User: Labonair-rust must be fully decoupled — no symlink/submodule/path-dependency to the original repo. Memory written (`memory/hard-fork-reference-src.md`).
- **Moved the copied web app into `reference-src/`** (`git mv` of `src/`, `src-tauri/`, `docs/`, `scripts/`, all web configs, `CHANGELOG/CONTRIBUTING/SECURITY/README`, and the old `.github/workflows/`). Repo root is now clean: `CLAUDE.md`, `handshake.md`, `tasks/`, `reference-src/`, `LICENSE`, `.github/` (templates only), `.gitignore`.
- **Swept `../Labonair` → `reference-src`** across CLAUDE.md, handshake.md, ROADMAP.md, and all task files. `git grep "\.\./Labonair"` = 0.
- **`.gitignore` rewritten** for Rust (`/target`, `.claude/`, `session-*.md`); `reference-src/` is tracked.
- **T01-001 deps corrected** to match `reference-src/src-tauri/Cargo.toml`: `russh 0.62.2` (ring), `russh-sftp 2.3.0`, `rusqlite 0.40`, `portable-pty 0.9`; **removed `git2`** — git runs via the `git` CLI (local + remote-over-SSH), same as the original. Same fix applied in T01-002 and T09-001.
- **T01-003 rewritten** — no longer "create symlink to ../Labonair"; now "verify `reference-src/` + write README + .gitignore". Standalone.
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
