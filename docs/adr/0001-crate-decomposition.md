# ADR 0001 — Decompose the `ui` monolith into ~22 crates

* **Status:** Accepted (amended 2026-09-03 — see below)
* **Date:** 2026-09-03
* **Deciders:** project owner (Snenjih)
* **Related:** `docs/architecture.md`, `bericht-architektur-rework-roadmap.md`,
  `vergleichsbericht-zed-vs-rust.md`, roadmap phases 15–21 (T16-001 … T22-001)

> **Amendment (workflow rework, themes 1–3):** `labonair-panel-hosts` is
> removed from the ~22-crate target — the SSH host manager is neither a tab nor
> a dock panel. Its view code lives in **`labonair-hosts-ui`** (a plain view
> crate, no `impl Panel`), consumed by `labonair-settings-ui`. Connecting runs
> through the command palette; managing runs through **Settings › Hosts**.
> Details + the two other themes (optional tabs, settings design contract):
> `docs/architecture.md §8` and `bericht-workflow-rework.md`. The decomposition
> rationale below is unchanged.

## Context

The Rust port reached feature parity with a crate layout of
`crates/{app, ui, terminal, editor, backend, ai, theme}`. `crates/ui` is a
monolith:

* ~40 source files, roughly 48 000 lines total.
* `ui/settings.rs` — 5 957 lines.
* `ui/workspace.rs` — 4 076 lines.
* `ui/app_shell.rs` — 2 983 lines.
* `ui/hosts.rs`, `ui/ai_chat.rs`, `ui/git.rs` — each four-digit.

Concrete pain points:

* **God-object shell.** `AppShell` holds ~20 `Entity` fields and wires each with
  a manual `cx.observe(&x, …).detach()`. Adding a panel touches an
  `enum SidebarPanel` with 6 variants plus `label` / `slug` / `from_slug` /
  `render_panel_body` match arms. Adding a statusbar item extends a
  `render_bar_item` match cascade over `BarItemId`. Adding an action extends a
  large `.on_action(cx.listener(Self::act_*))` chain in `render`.
* **Frame-buffered events.** `render()` starts with
  `drain_pending_commands/bookmarks/ai` + `sync_live_bridge` — work done per
  frame that per event would suffice.
* **Latent cycle.** Panels need workspace types; the workspace needs a panel
  abstraction. Today both live in one crate, hiding the cycle.
* **Parallel settings model.** A `FIELDS` / `SECTION_GROUPS` table is maintained
  by hand next to the `Preferences` structs — two sources of truth.
* **Slow incremental builds.** Any change in `crates/ui` recompiles the whole
  48k-line crate and everything downstream.

The Zed codebase (`zed-refrence/zed/crates/`, ~300 crates) demonstrates the
target shape: one crate per panel, a `workspace` crate, a separate
`settings` / `settings_content` / `settings_ui` split, a `ui` design-system
crate, trait registries instead of central match cascades.

## Decision

Decompose `crates/ui` into ~22 focused crates as specified in
`docs/architecture.md` §2, and introduce trait registries
(`PanelRegistry`, `StatusItemRegistry`, `CommandRegistry`) so that panels,
statusbar items, commands and settings register themselves instead of being
enumerated by a central object.

Key structural commitments:

* A contracts-only crate `labonair-panel` (Panel / StatusItem traits +
  registries) that depends on **no** workspace-track crate — this breaks the
  Panel ↔ Workspace cycle before any panel moves.
* `labonair-shell` is the only crate that knows concrete panel types.
* `backend` / `ai` / `terminal` (engine) / `editor` depend on no UI crate.
* Naming: `labonair-<name>`, directory `crates/<name>/`, explicit
  `[lib] path = "crates/<name>/src/<name>.rs"`.

Phase 15 does this as **pure file moves + re-exports, zero behaviour change**,
with all four gates green after each extracted crate. The behavioural rework
(registries, new layout, settings model) follows in phases 16–19.

## Alternatives considered

1. **Leave the status quo.** Rejected: the god-object, per-frame buffers and
   the parallel `FIELDS` table keep compounding; every new feature is a
   multi-site edit and the whole `ui` crate recompiles.
2. **Extract only `settings`.** Rejected as insufficient: it addresses the
   largest file but not the `AppShell` god-object, the Panel ↔ Workspace cycle,
   or build granularity for panels and the workspace.
3. **Feature folders inside one crate instead of crates.** Rejected: module
   boundaries are not compiler-enforced, so the dependency rules in
   `docs/architecture.md` §3 could not be checked in CI, incremental builds
   would still recompile the whole crate, and the latent cycle would stay
   invisible.

## Consequences

**Positive**

* Compiler-enforced APIs and dependency direction; the acyclic graph is checked
  in CI (T16-010).
* Faster incremental builds: a change in one panel or in `labonair-shell`
  recompiles that crate, not 48k lines.
* Adding a panel / statusbar item / command / `bool` setting becomes a single
  registration line (rework success criteria 22).
* The Panel ↔ Workspace cycle is structurally impossible.

**Negative / costs**

* Many more `Cargo.toml` files and a larger workspace member list to maintain.
* One-time migration effort: ~40 files moved, all call sites re-pointed, gates
  re-verified per crate.
* Slightly more boilerplate per crate (crate root file, prelude imports).
* Contributors must learn the crate map (mitigated by `docs/architecture.md`).
