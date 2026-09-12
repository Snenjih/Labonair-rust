# R09-005 — Editor E4: local language services and LSP

## Status

⏳ Planned

## Owner

- Module: editor
- Capability-matrix row: [Editor](../../docs/capabilities.md)
- Composition entry point: labonair-shell constructs local language-service processes and injects the Editor service boundary

## Dependencies

- R09-001-editor-e0-text-model-contract
- R09-002-editor-e1-file-lifecycle
- R09-004-editor-e3-display-search-navigation
- R07-001-product-surface-acceptance must be complete.

## Goal

Add a local-first language-service layer with diagnostics, completion,
navigation, formatting, code actions, semantic tokens, symbols, folding
ranges, and hover while keeping protocol/process lifecycle out of Workspace.

## Scope

- In scope: local language-server process lifecycle, capability negotiation,
  document synchronization, versioned positions, diagnostics, completion and
  signature help, hover, definition/declaration/references/implementation,
  rename, code actions, semantic tokens, document symbols, folding ranges,
  formatting, cancellation, stale-result rejection, and supported-language
  configuration.
- Out of scope: remote LSP, collaboration, marketplace/extensions, bundled
  servers, and provider-specific UI surfaces.

## Contracts and ownership

- Public domain values: LanguageId, LanguageServerId, DocumentVersion,
  LspPosition, Diagnostic, CompletionItem, Symbol, CodeAction, HoverContent,
  SemanticToken, FoldingRange, and typed LanguageServiceEvent in
  labonair-editor.
- Service traits or typed events: `LocalLanguageService` remains the
  synchronous syntax-provider contract; `LocalLanguageServiceProcess` and
  `LanguageServiceEvent` provide the asynchronous process boundary. The adapter owns
  JSON-RPC/process I/O; Editor owns document mapping, stale-result handling,
  presentation state, and commands.
- Registry contributions: the Editor language-service registry is allowed
  because multiple local providers/languages need discovery. It registers
  stable language/server IDs, command capabilities, executable policy, and
  lifecycle status; it must not become a second settings or process owner.
- UI surface: Editor tab/standalone Editor, completion/signature/hover
  popovers, inline diagnostics/inlays, outline, and notification dropdown.
  Workspace only hosts the view and injects the service.
- Shared UI-kit components: completion lists, popovers, menus, dialogs,
  badges, list rows, tooltips, keyboard hints, and error/loading states.
  Inline diagnostic and token rendering use Editor display contracts and theme
  tokens.

## Commands and keymap

Register stable Editor commands for Trigger Completion, Go To Definition,
Go To Declaration, Go To References, Go To Implementation, Peek Definition,
Rename Symbol, Code Action, Format Document, Format Selection, Organize
Imports, Restart Language Server, Stop Language Server, Show Diagnostics,
Next/Previous Diagnostic, and Go To Symbol. Commands are no-ops only when the
server does not advertise the capability; the UI explains unavailable
capabilities locally and does not publish misleading success messages.

## Settings

These are proposed value settings and require complete inventory entries
before implementation:

| Key | Type / default | Scope | Migration and test |
|---|---|---|---|
| editorLspEnabled | bool / true | Global + safe Project | New key; validate bool, reset true, test service lifecycle |
| editorDiagnostics | bool / true | Global + safe Project | New key; validate bool, reset true, test decoration visibility |
| editorCompletion | bool / true | Global + safe Project | New key; validate bool, reset true, test trigger/accept/cancel |
| editorSemanticTokens | bool / true | Global + safe Project | New key; validate bool, reset true, test stale-result handling |
| editorInlayHints | bool / false | Global + safe Project | New key; validate bool, reset false, test layout and toggle |
| editorCodeLens | bool / false | Global + safe Project | New key; validate bool, reset false, test empty/unsupported state |

Server executable paths, environment variables, and project trust decisions
are not arbitrary Settings values. They require a separate owner contract and
must not be stored with secrets. Format-on-save remains an E6 value and is
false by default.

## Persistence and migration

- Storage: server processes and live capabilities are runtime state. Editor
  persists language identity and enabled presentation preferences only.
- Migration: existing path-based language detection maps to LanguageId; an
  unknown language opens with syntax-only/plain-text behavior and no server.
- Compatibility: an adapter may translate the current Tree-sitter symbol
  output into the new Symbol contract until LSP and parser consumers converge.
  Remove it once document outline and Go To Symbol use the typed range-rich
  contract.
- No server logs, command lines containing secrets, or project credentials
  enter session persistence or notifications.

## Notifications

Server start/stop/crash, protocol, timeout, unsupported-capability, and
workspace-root failures publish structured notifications with source, details,
deduplication, and actionable restart/stop commands where appropriate.
Diagnostics are editor data rendered inline and in the editor diagnostics
surface; they are not duplicate operational-error notifications.

## Dependency restrictions

- Keep language-service lifecycle and Editor-facing behavior within the Editor
  capability. A protocol sibling is permitted only as a real integration
  boundary under the same owner.
- Workspace may inject process services but may not own language-server state or
  dispatch requests.
- Local process I/O must be asynchronous or blocking work on a background
  executor; never block the GPUI foreground thread.
- Do not add remote transport or a general provider facade in this task.
- Any LSP/JSON-RPC dependency needs license, version, security, and
  dependency-verifier documentation plus focused protocol tests.

## Implementation plan

1. Define local service lifecycle, document sync, versioning, cancellation,
   and capability contracts → verify mock-server protocol tests.
2. Implement process adapter and supported-language registration → verify
   start/stop/restart/crash, root detection, timeout, and malformed-message
   recovery tests.
3. Integrate diagnostics, completion, navigation, hover, code actions,
   semantic tokens, symbols, folds, and formatting into E3 display contracts
   → verify stale-result, range mapping, cancellation, and UI-state tests.
4. Register commands/settings and publish operational notifications → verify
   command/keymap, settings migration, notification deduplication, and native
   visual checks.

## Acceptance criteria

- [ ] Local language servers start, stop, restart, and recover without blocking
      the foreground thread.
- [ ] All displayed results are versioned and stale responses are ignored.
- [ ] Diagnostics, completion, navigation, hover, code actions, symbols,
      semantic tokens, folds, and formatting work when advertised.
- [ ] Unsupported capabilities fail predictably and remain user-visible only
      through appropriate local state or notifications.
- [ ] Settings have complete owner, type, default, scope, validation, reset,
      migration, UI, and consumer-test records.
- [ ] Focused protocol, mapping, command, notification, and visual tests pass.
- [ ] Full repository, dependency, documentation, queue, and diff checks pass.

## Removal / exit condition

The task is complete when local language-service behavior crosses only the
typed Editor boundary, Workspace owns no LSP state, the legacy symbol-only
path is removed or has a named remaining consumer, and no remote process path
is implied by the local contract.

## Notes and follow-ups

Remote language services are intentionally deferred. E5 consumes diagnostics
and document ranges for Git/editor decorations; E6 owns format-on-save policy
and persistence polish.

## Implementation evidence (E4 core)

- Added the Editor-owned typed language-service boundary in
  `crates/editor/src/language_services.rs` with `LanguageId`, server and
  document-version identities, UTF-16/LSP range mapping, capabilities,
  requests/responses, cancellation, background execution, and stale
  version/generation rejection.
- Added the Editor-owned asynchronous stdio adapter in
  `crates/editor/src/lsp_process.rs`. `ProcessServerConfig` validates an
  argv-based command, arguments, child-only environment, absolute cwd,
  explicit trusted-project policy, bounded frame size, and initialize/request/
  shutdown timeouts. No shell interpolation, secret-bearing command logging,
  or environment logging is used.
- The adapter owns Tokio process and pipe tasks, Content-Length framing with
  malformed-frame recovery, initialize/initialized and shutdown/exit, request
  ID correlation, pending-request cancellation and timeouts, typed EOF/crash/
  stop/restart events, and replay of opened documents after restart. The
  Editor-facing document contract sends full-text didOpen/didChange/didClose
  notifications and rejects stale document versions/generations both before
  dispatch and after a response returns.
- The protocol boundary converts common LSP completion/signature-help/hover,
  locations, diagnostics, workspace edits/text edits, code actions, semantic
  token deltas, document symbols (including nested symbols), folding ranges,
  and formatting responses into the existing Editor types. Server-advertised
  capabilities are typed; unadvertised requests return
  `UnsupportedCapability`.
- The existing syntax-derived provider for Rust, Python, and JSON remains
  available through `LocalLanguageService` for compatibility. Its typed
  diagnostics, completion, hover, navigation, rename, code actions, semantic
  tokens, symbols, folding, and formatting behavior is unchanged.
- Added `DisplaySnapshot::diagnostic_decorations` and
  `DisplaySnapshot::semantic_token_decorations`, so the existing Editor-owned
  display map consumes service ranges without Workspace-owned LSP state.
- Added the explicit `labonair-editor-lsp-fixture` test command and
  `crates/editor/tests/lsp_process.rs`. The deterministic process tests cover
  handshake, document synchronization, diagnostics notifications, response
  correlation, cancellation, request timeout, malformed-message recovery,
  stale input, unsupported capabilities, crash detection, restart, and
  shutdown without installed tools or shell-specific behavior. Unit tests
  cover split framing, header recovery, UTF-16/range mapping, and capability
  negotiation.
- No new third-party LSP/JSON-RPC dependency was added: the adapter reuses
  the workspace-pinned Tokio and `serde_json` dependencies. The security
  boundary is direct argv spawning, child-only environment inheritance, an
  explicit trusted absolute project root, bounded frames, and redacted typed
  lifecycle events.
- Verification on 2026-09-12: `cargo fmt --all -- --check`,
  `cargo test -p labonair-editor --all-targets` (135 unit tests plus 10
  integration/fixture tests), and
  `cargo clippy -p labonair-editor --all-targets -- -D warnings`,
  `cargo check --workspace --all-targets`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test -p labonair-workspace --all-targets` (123 passed),
  `bash scripts/check-crate-deps.sh`, `python3 scripts/check_rework_queue.py`,
  `python3 scripts/check_documentation.py`, and `git diff --check` pass.
- The task remains formally `Planned` because the active queue dependency
  `R07-001-product-surface-acceptance` is still open. Native visual evidence
  remains bounded follow-up work; remote LSP and bundled servers remain out of
  scope.

## Implementation evidence (Editor vertical integration)

- `crates/editor/src/runtime.rs` composes the syntax registry with an
  explicitly injected `LocalLanguageServiceProcess`. `EditorView` uses this
  Editor-owned runtime on open, change, reload, and close; requests run on a
  background thread/Tokio handle and carry document version plus generation
  metadata. Process failures fall back to the syntax provider and are exposed
  as explicit Disabled, Starting, Running, Crashed, Fallback, or Unsupported
  states.
- `crates/workspace/src/views/editor.rs` consumes the runtime projection for
  revision-safe inline diagnostics through `DisplaySnapshot`; the existing
  syntax highlighter remains the default rendering path. Workspace injects the
  runtime handle and does not store protocol maps, process state, or document
  language-service state.
- `Workspace::set_editor_language_services` is the narrow composition adapter.
  The application/shell can create a runtime from validated process
  configuration, attach the Tokio handle, and inject it into existing and new
  editor views. No default factory searches `PATH`, launches a process, or
  claims that an external server is installed.
- No new persisted LSP settings were added. The existing canonical
  `EditorSettings` values `editorDiagnostics`, `editorSemanticTokens`,
  `editorCompletion`, and `editorHover` own the presentation/request toggles
  (typed bool values, global/project merge scope, safe defaults, schema
  validation, reset-through-defaults, migration, Settings UI, and consumer
  tests). The runtime receives them through `set_presentation_policy` and owns
  only in-memory process opt-in. Executable/argv/server configuration remains
  a validated `ProcessServerConfig` supplied by the composition factory; it is
  not stored as arbitrary settings or with secrets.
- The canonical Editor command provider contributes metadata and default
  keymap entries for completion, navigation, rename, code actions, formatting,
  diagnostics, and local-server lifecycle commands. The existing command
  palette remains the sole discovery surface. Runtime state uses the existing
  UI-kit banner for process lifecycle/unsupported states, while synchronization
  failures use the existing notification-center path; no toast or permanent
  shell chrome was added.
- Verification on 2026-09-12: `cargo fmt --all -- --check`,
  `cargo test -p labonair-editor --all-targets` (145 unit tests plus 10
  LSP/runtime integration/fixture tests), `cargo test -p labonair-workspace --all-targets`,
  `cargo test -p labonair-settings-ui --all-targets`, workspace check/clippy,
  dependency, documentation, queue, and diff checks pass. Runtime coverage
  includes enabled/disabled selection, syntax fallback, safe policy defaults,
  document synchronization, stale-result rejection, process lifecycle events,
  capability negotiation, and fixture protocol behavior. The settings suite
  has one pre-existing sandbox-flaky watcher rename test
  (`watcher never observed the rename`); it was not changed by this work.
- The canonical Editor command provider and Workspace active-tab
  bridge now dispatch the available typed requests. The current UI slice now
  renders completion and hover results, exposes multi-action code-action
  choices, and provides an editor-owned rename input whose result is sent
  through the typed Rename request. Range formatting and organize-imports use
  typed local edit requests, with JSON-RPC capability mapping for configured
  local servers and deterministic syntax fallback. Native visual acceptance remains gated by
  R07-001. Remote LSP, bundled servers, and server-configuration UI remain out
  of scope. The task stays formally `Planned` until the R07 dependency is
  closed.
- The Editor language-intelligence overlay now consumes the typed
  `SignatureHelp` projection and renders cursor-near loading, ready, empty,
  unavailable, and error states. Completion selection is Editor-owned and
  supports keyboard movement/wrapping, Enter/Tab acceptance, Escape cancel,
  click selection, item kind/detail/documentation, and bounded overflow. Both
  response paths reject stale document revision/generation, request serial,
  and cursor-position results before presentation or acceptance. Focused
  coverage adds one Editor state test and three Workspace interaction/state
  tests. The task remains formally `Planned` because R07-001 is still open.
- The full workspace test command completed all relevant Editor/Workspace
  tests but reports two unrelated pre-existing sandbox failures: the AI HTTP
  stream test cannot bind its local listener (`Operation not permitted`), and
  the Settings watcher rename test does not observe the rename. Neither test
  touches the local LSP/runtime path.
