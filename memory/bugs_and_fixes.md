# Bugs, fixes, and non-obvious constraints

## 2026-09-06 — Host domain models must be separated before host persistence

**Finding:** `backend::modules::hosts` combined serializable host models,
SQLite persistence, secret access, and App/MCP behavior. Moving the whole
module would have created another capability-owned crate with the same global
coupling.

**Resolution:** Extracted only `Host`, `Group`, and `ReorderItem` into the
UI-/backend-free `labonair-hosts` contract crate. `HostsDb` and App-bound
operations remain explicitly transitional in `labonair-backend` until a
narrow host store/service API and typed MCP-revocation callback are defined.
Feature crates now import the domain models directly.

## 2026-09-06 — Structured errors extracted as a shared contract

**Finding:** `LabonairError` was implemented in `backend`, so any future
capability crate that needed consistent classification or recovery metadata
would have to depend on the backend facade.

**Resolution:** Created `labonair-errors` and moved the catalog, conversions,
classification rules, serialization, and tests there. `backend::modules::errors`
now only re-exports it, preserving existing internal call sites while making
the contract available to standalone capability services.

## 2026-09-06 — Secret storage extracted without retaining App ownership

**Finding:** The secret store used `backend::App` only to resolve its data
directory, which made encryption/cache logic appear to belong to the backend
and forced unrelated callers through that facade.

**Resolution:** Created `labonair-secrets` with `SecretsState::new(data_dir)`;
the crate now owns path resolution from its state, plain/encrypted migration,
cache, and secret operations. `backend::modules::secrets` is only a
compatibility adapter until SSH, Hosts, and MCP consume the service directly.
Added isolated plain-store and encryption-toggle round-trip tests.

## 2026-09-06 — Interrupted Cargo rebuild can leave stale artifact locks

**Finding:** After `cargo clean` during a full-disk condition, an interrupted
parallel build left missing intermediate proc-macro artifacts and stale Cargo
processes holding the artifact lock. This produced misleading `syn`/`serde`
compile errors and prevented a second clean.

**Resolution:** Confirmed the affected processes and stopped only those build
PIDs, recreated the regenerable target directories, and reran the changed
crate tests serially. The focused `labonair-filesystem` and
`labonair-backend` suites passed; the full workspace had already passed before
the final watcher-only contract change, while workspace check and Clippy also
passed after it.

## 2026-09-06 — First platform boundary extracted from backend

**Finding:** Local file access, traversal, mutation, path resolution, and
search were implemented under `labonair-backend`, making feature crates reach
through the backend facade for a shared platform capability.

**Resolution:** Created `labonair-filesystem` as a UI-free workspace crate,
moved the filesystem services, watcher implementation, and tests there,
centralized home-path expansion, and kept only a small application-event
adapter in the backend as an explicit transitional seam. Feature crates now
consume the filesystem crate directly; the dependency verifier tracks the new
platform edge.

## 2026-09-06 — Workspace AI integration test needs loopback permission

**Finding:** The first sandboxed `cargo test --workspace` run failed only at
`client::tests::end_to_end_streams_openai_sse_over_http` because the test binds
a local TCP listener and the sandbox returned `Operation not permitted`.

**Resolution:** Re-ran the unchanged workspace test with approved loopback
network permission; all workspace tests passed. This is an environment
constraint, not a product or documentation regression.

## 2026-09-06 — Architecture reset supersedes the port-era planning contract

**Finding:** The former `AGENTS.md`, architecture document, settings contract,
roadmap, and reports still treated feature parity, a toast layer, Hosts in
Settings, and a broad backend as active decisions. They conflicted with the
new Dev-Op workspace direction.

**Resolution:** Added the v2 product, architecture, module, registry,
design-system, workspace, settings, and rework-roadmap contracts under
`docs/`; reduced `AGENTS.md` and `CLAUDE.md` to current rules; archived the
former architecture and idea documents; moved reports under `docs/reports/`;
and marked superseded tasks/ADRs as historical. The target is capability-owned
modules with explicit composition, typed cross-module contracts, and shared
UI-kit primitives.

## 2026-09-06 — Dependency gate must distinguish target and migration edges

**Finding:** `scripts/check_crate_deps.py` still encoded the v1 crate graph and
failed on four already-known transitional edges, while its success message
claimed that the old architecture was fully satisfied.

**Resolution:** Updated the checker to use the v2 migration inventory, keep the
four transitional edges explicit, reject every untracked edge/cycle, and report
that a green result means no untracked violation rather than migration
completion. The remaining edges are now removable one by one as contracts move.

## 2026-09-05 — GPUI panic: "hover style already set" when `.hover()` applied twice

**Context:** `ui-kit::ListItem` lets call sites override a row's background via
`.extra(..)`, and the host/SCM rows wanted a custom hover fill (accent-border /
border) distinct from `ListItem`'s default `selected_fill` bg-tint hover.

**Finding:** A `Stateful` element may only receive `.hover(..)` once. Calling it
inside `.extra(..)` after `ListItem` had already applied its own default hover
(`row.hover(|s| s.bg(selected_fill))` when `!selected`) panics at render time
with "hover style already set".

**Fix:** Added `ListItem::hover_style(f)` (crates/ui-kit/src/list.rs) — a builder
slot that replaces the default hover fill and is applied in `IntoElement` before
`.extra(..)`, so call sites never call `.hover(..)` twice. Migrated
`hosts-ui` (accent-border hover) and `panel-scm` file rows (border hover, accent
selected) onto it.

**Reference:** confirmed via the panic message at runtime; the constraint is
visible in the Zed `gpui` element state (one `hover` slot per element).

## 2026-09-05 — Zed UI source cannot be copied into the Apache-2.0 project

**Context:** A source-level comparison was made for Zed's dock/status bar,
Project Panel, Git Panel, and shared UI primitives.

**Finding:** The inspected Zed crates `workspace`, `project_panel`, `git_ui`,
and `ui` each declare `GPL-3.0-or-later`. Labonair's root license is
Apache-2.0.

**Resolution:** Treat Zed as a behavioral and architectural reference. Specify
observable interaction outcomes and independently implement them over
Labonair's existing panel, dock, workspace, and UI-kit APIs. Do not copy or
closely translate Zed function bodies, type layouts, comments, or algorithms
unless the project first makes an explicit, reviewed licensing decision.

**Reference:** `docs/ui-comparison-zed-sidebar-status-bar.md` section 2.
