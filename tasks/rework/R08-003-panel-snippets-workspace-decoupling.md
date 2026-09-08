# R08-003 — Remove Workspace entity coupling from Snippets UI

## Status

`✅ Done`

## Owner

- Module: `snippets`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell::bootstrap`

## Goal

`labonair-panel-snippets` no longer depends on `labonair-workspace`. Snippet
execution crosses a narrow owner-neutral contract injected from composition.

## Scope

- In scope: new leaf crate `crates/snippets-host`
  (`labonair-snippets-host::SnippetExecutionHost`),
  `crates/panel-snippets/src/panel_snippets.rs`,
  `crates/panel-snippets/Cargo.toml`, `crates/shell/{Cargo.toml,src/bootstrap.rs}`,
  `Cargo.toml`, `scripts/check_crate_deps.py`, architecture docs.
- Out of scope: snippet state, persistence (`labonair_snippets::store`),
  execution-mode semantics, the silent-run runner (`labonair_snippets::exec`),
  and the SSH executor adapter — all already outside Workspace.

## Contracts and ownership

- Public domain values: none new — the contract carries `&str` / `String` /
  `Option<String>` intents only.
- Service traits or typed events: `SnippetExecutionHost` — four `Rc<dyn Fn>`
  callbacks (`inject_into_active_terminal`, `run_snippet_local`,
  `run_snippet_ssh_terminal`, `ssh_session_for_host`), plus `disconnected()`
  for headless/tests. Mirrors the `labonair-explorer-host` pattern.
- Registry contributions: unchanged (`snippets` palette action still owned by
  the panel).
- UI surface: `crates/panel-snippets`.
- Shared UI-kit components: unchanged.

## Dependencies

- Existing edges removed: `labonair-panel-snippets → labonair-workspace`.
- New edges: `labonair-panel-snippets → labonair-snippets-host`,
  `labonair-shell → labonair-snippets-host` (leaf, `gpui` only).
- Dependency verifier change: add the `labonair-snippets-host` allow-list
  entry (empty set), swap `labonair-workspace` for `labonair-snippets-host` in
  `labonair-panel-snippets`, add it to `labonair-shell`.

## Persistence and migration

- Settings: None.
- Storage: None.
- Compatibility: None — the panel keeps the same `Database` + SSH executor
  inputs; only the Workspace entity is replaced.

## User-visible behavior

- Canonical entry point: Snippets panel + palette (unchanged).
- Notifications: unchanged (missing-host / no-session errors still local
  `notify_error`).
- Inline errors/toasts: none new.

## Implementation plan

1. Add the `labonair-snippets-host` leaf crate with the contract + tests →
   verify: `cargo test -p labonair-snippets-host`.
2. Replace the `Entity<Workspace>` field/param and the four call sites in
   `panel_snippets.rs`; drop the `workspace` module shim and the manifest
   dependency → verify: `cargo check -p labonair-panel-snippets`.
3. Wire `SnippetExecutionHost` from the active Workspace in `bootstrap`;
   update the verifier and architecture docs → verify:
   `bash scripts/check-crate-deps.sh`.

## Acceptance criteria

- [x] `panel-snippets` has no `labonair-workspace` dependency.
- [x] `SnippetsView` contains no `Entity<Workspace>`.
- [x] Local execution, remote execution, terminal injection, and
      active-session lookup retain their behavior (same Workspace methods, now
      reached through the host).
- [x] Focused contract tests exercise execution without constructing Workspace
      (`snippets-host::tests`).
- [x] `cargo fmt --check`, `cargo check`, `cargo clippy -D warnings`,
      `cargo test --workspace`, `scripts/check-crate-deps.sh`, and
      `git diff --check` pass.

## Notes and follow-ups

`inject_into_active_terminal` is `&App`-only on Workspace but the contract
takes `&mut App` so all four intents share one callback shape. Next task:
R08-004 (P2.1, popover trigger-bounds anchoring).
