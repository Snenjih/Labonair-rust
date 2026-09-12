# R09-001 — Editor E0: rope, anchors, and transactions

## Status

`⏳ Planned`

## Owner

- Module: editor
- Capability-matrix row: [Editor](../../docs/capabilities.md)
- Composition entry point: labonair-shell editor registration and Workspace Editor tab construction

## Dependencies

- R07-001-product-surface-acceptance must be complete.
- R03-002-keymap-runtime-and-editor must remain the canonical command/keymap boundary.
- ADR 0004 editor-internal splits is accepted and establishes the Editor-owned view boundary.

## Goal

Replace the line-oriented, full-document snapshot model with a scalable Editor
text model that supports stable positions during edits, grouped transactions,
and multiple consumers without moving ownership into Workspace.

## Scope

- In scope: the canonical labonair-editor buffer contract, rope-backed text
  storage, anchors, selections, transactions, edit deltas, revision numbers,
  undo/redo grouping, and focused contract tests.
- Out of scope: file I/O, LSP, Git, rendering, split layout, remote buffers,
  and a second editor crate.

## Contracts and ownership

- Public domain values: EditorBuffer, BufferSnapshot, Anchor, Selection,
  Transaction, Edit, Revision, and typed BufferEvent in labonair-editor.
- Service traits or typed events: immutable snapshots for readers and typed
  transaction/revision events for syntax, search, display, and Workspace
  adapters. No consumer receives mutable internal storage.
- Registry contributions: none. Editing commands use the existing command
  registry; this task must not create an editor-local registry.
- UI surface: no new surface. Existing Editor tab and standalone Editor remain
  the canonical entry points; the current GPUI view consumes snapshots.
- Shared UI-kit components: none in this core task. Any later selection,
  input, menu, or status control must use labonair-ui-kit.

## Commands and keymap

Reuse existing canonical Undo, Redo, Cut, Copy, Paste, Select All, Delete, and
Backspace command identities where present. Add an Editor-owned descriptor only
for a missing stable action, with Editor context and owner-provided default
binding. Do not edit a shell-wide table or introduce aliases that bypass
labonair-keymap.

## Settings

No new setting is required for the text model. Existing
editorTabSize, editorIndentWithTabs, and editorVimMode remain value settings
owned by Settings and consumed through the typed Editor adapter. Their defaults,
scope, validation, and migration are unchanged in this task.

## Persistence and migration

- Storage: the live buffer and undo history remain session-memory state.
- Migration: no user-data schema changes. Existing transient line snapshots
  are converted at load time to one rope and one primary selection, then are
  no longer authoritative.
- Compatibility: retain a conversion helper only while all existing
  Workspace and Editor tests use the new snapshot contract. Remove it when no
  caller consumes the old line-vector or full-document snapshot API.

## Notifications

None for ordinary edits. Contract or invariant failures return typed errors to
the owner; they must not become panic-based runtime handling or a new inline
error surface. User-visible save/reload failures remain the responsibility of
the file-lifecycle task and the notification registry.

## Dependency restrictions

- Keep ownership in labonair-editor; do not add a facade or general text crate.
- Prefer an already approved balanced-text implementation if one exists. If a
  new rope dependency is necessary, record its license, benchmark evidence,
  rationale, and dependency-verifier entry in the implementation diff before
  adding it.
- No dependency from labonair-editor to labonair-shell, Workspace, or a feature
  UI crate.

## Implementation plan

1. Define position, anchor affinity, selection, transaction, revision, and
   event semantics → verify focused contract tests for insertion, deletion,
   Unicode, EOF, and anchor movement.
2. Implement rope-backed storage and transaction-based history → verify
   randomized edit/property tests, coalescing tests, undo/redo round trips,
   and performance checks on large files.
3. Adapt the current Document, syntax, symbols, search, Vim, and Workspace
   consumers one boundary at a time → verify no old mutable buffer access
   remains and Editor integration tests pass.
4. Remove the old line-vector/full-snapshot path → verify source audit and
   dependency/documentation checks.

## Acceptance criteria

- [ ] One canonical buffer contract exists in labonair-editor.
- [ ] Anchors remain valid and deterministic across insert, delete, replace,
      undo, redo, and transaction boundaries.
- [ ] Transactions group edits without blocking the GPUI foreground thread.
- [ ] Unicode positions and line endings have focused regression coverage.
- [ ] Syntax, search, Vim, symbols, and Workspace consume snapshots/events
      rather than private mutable storage.
- [ ] No new shell dependency, feature god object, or duplicate registry exists.
- [ ] Focused Editor and integration tests pass.
- [ ] cargo fmt, cargo check, cargo clippy, cargo test, dependency checks,
      check_rework_queue.py, check_documentation.py, and git diff --check pass.

## Removal / exit condition

The task is complete only when every in-tree consumer uses the new typed model,
the old Vec<String> and full-document history are removed, compatibility
conversion has no remaining consumer, and the Editor contract can be tested
without GPUI or Workspace.

## Notes and follow-ups

E1 may build file identity and conflict state on BufferSnapshot and Revision;
it must not reintroduce a second text model. E2 may add multiple selections
and Editor-internal splits only after this contract is stable.

## Implementation evidence (current worker)

The first E0 migration slice is implemented and verified in the shared
worktree:

- `labonair-editor` now has a rope-backed `EditorBuffer`, immutable
  `BufferSnapshot`, `Revision`, UTF-8-safe `Anchor`/`AnchorRange`,
  `SelectionSet`, `Edit`, `Transaction`, `EditDelta`, and `BufferEvent`.
- `Document` now constructs and applies transactions for insert, backspace,
  forward delete, replace-all, undo, and redo. Undo/redo rebases the stored
  transactions to the live revision and restores the transaction selection;
  it does not replace the buffer with a snapshot or bypass the history.
- Reload establishes a fresh buffer baseline and clears transactions based on
  the previous revision. The compatibility `TextBuffer` alias and the
  untyped `EditorBuffer` insertion/deletion helpers have been removed.
- Focused and workspace checks passed: `cargo test -p labonair-editor` (75
  tests), `cargo check -p labonair-workspace --all-targets`,
  `cargo check --workspace --all-targets`, `cargo clippy --workspace
  --all-targets -- -D warnings`, and `cargo test --workspace`.
- Repository checks passed: `cargo fmt --all -- --check`, crate dependency
  verification, `check_rework_queue.py`, `check_documentation.py`, and
  `git diff --check`.

The initial implementation slice above was preparatory evidence. The final
consumer migration is recorded below; the formal queue status remains
`Planned` until the repository's active R07-001 prerequisite is complete.

### Consumer migration evidence (current worker)

The syntax, search, and symbol read-only boundaries have now been migrated in
the shared worktree:

- `SyntaxHighlighter::update` consumes `&BufferSnapshot` and uses its typed
  `Revision` for cache invalidation. `line_runs` also consumes a snapshot and
  line index, so the renderer no longer supplies an independent source string
  or byte offset. Tree-sitter still receives only a temporary contiguous view
  when highlighting requires one; the highlighter does not retain a second
  text store.
- Search consumes `&BufferSnapshot` exclusively. Matching preserves literal,
  case-sensitive/case-insensitive, whole-word, wrapping, Unicode scalar
  positions, and CRLF behavior. `replace_all` is read-only and returns ordered
  transaction-ready `Edit` values; `Document` applies them as one undoable
  transaction.
- Symbol extraction consumes `&BufferSnapshot` and returns stable definition
  and name `TextRange` values in the snapshot's byte space while preserving
  existing Tree-sitter tags semantics.
- Vim and Workspace now consume one `Document::snapshot()` per read/action or
  render step; no consumer accesses `Document.buffer`. `Document.buffer` is
  private, `Document::snapshot()` is the read-only boundary, and the
  `TextBuffer` alias is gone. Mutations remain routed through Document edit
  methods and typed transactions.
- The GPUI render path keeps one local snapshot for line layout, offsets,
  syntax, selection geometry, and text rows, avoiding per-line buffer clones.
- No new settings, registries, notifications, UI controls, or dependencies
  were introduced. Existing Vim edits, search, syntax, symbols, wrapping,
  rendering, save/reload, and undo/redo behavior remain covered.

Verification for this slice:

- `cargo fmt --all -- --check`
- `cargo test -p labonair-editor` — 78 tests passed, including Unicode/CRLF
  search, replacement ranges, visible syntax runs, symbol ranges, and empty
  snapshots
- `cargo clippy -p labonair-editor --all-targets -- -D warnings`
- `cargo check -p labonair-workspace --all-targets`
- `cargo check --workspace --all-targets`
- `scripts/check-crate-deps.sh`, `python3 scripts/check_documentation.py`, and
  `git diff --check`

The task remains `⏳ Planned` in the active queue because R07-001 is still open;
the queue deliberately keeps all R09 work planned until that prerequisite is
complete. The E0 implementation and consumer exit conditions are otherwise
complete in the shared worktree; R07-001 remains open and is not altered here.

### Final E0 consumer-boundary audit

- `rg` reports no `TextBuffer` symbol in Rust source and no Workspace/Vim
  `Document.buffer` access. The only remaining `TextBuffer` mentions are
  historical documentation/session records and this task's migration notes.
- `History` stores forward/inverse `Transaction` values only; no full-document
  snapshot history or Vec-backed editor buffer remains.
- `Document` exposes `snapshot()` and keeps its mutable `EditorBuffer` private.
- `cargo test -p labonair-editor` passes 79 tests, including the read-only
  Document snapshot boundary, transaction undo/redo, and Vim edits.
- `cargo check -p labonair-workspace --all-targets` passes, verifying the GPUI
  adapter's snapshot boundary at compile time.

Repository-wide verification after the final migration:

- `cargo fmt --all -- --check` — passed.
- `cargo check --workspace --all-targets` — passed.
- `cargo clippy --workspace --all-targets -- -D warnings` — passed.
- `cargo test --workspace` — passed with the privileged local test-server
  environment required by the existing AI HTTP test (all workspace tests and
  doc-tests passed).
- `scripts/check-crate-deps.sh` — passed: 57 crates, 227 internal edges,
  acyclic.
- `python3 scripts/check_rework_queue.py` — passed; R07-001 remains the active
  task and R09 remains planned as required by the queue.
- `python3 scripts/check_documentation.py` — passed.
- `git diff --check` — passed.

No R09-002 or later implementation is included in this task.
