# Bugs, fixes, and non-obvious constraints

## 2026-09-06 — Full AI test suite needs local listener permission

**Finding:** The first sandboxed `cargo test --workspace` run failed only in
the pre-existing AI end-to-end streaming test because its local HTTP fixture
could not bind a listener and returned `Operation not permitted`.

**Resolution:** Reran the unchanged workspace test suite with the approved
local socket permission. All workspace tests passed; this is an environment
constraint, not a product or SSH/SFTP failure.

## 2026-09-06 — SFTP view must close SSH when subsystem setup fails

**Finding:** The first injected SFTP connection path could leave the newly
authenticated SSH session registered if opening the SFTP subsystem failed.

**Resolution:** The view now disconnects the SSH session on SFTP-open failure,
while normal tab retirement closes the SFTP handle before disconnecting SSH.

## 2026-09-06 — SFTP must not own host authentication

**Finding:** The first draft of the SFTP service accepted `host_id`, password,
database, and secret-resolution concerns. The SSH/SFTP audit showed that this
would merely recreate the backend facade at a new crate boundary and would
make the SFTP contract responsible for authentication.

**Resolution:** Split the contract into `SftpSessionService` and
`SftpBrowserService`. SSH creates the authenticated session; SFTP receives an
opaque `SftpSessionHandle` and exposes only remote filesystem operations. The
backend adapter temporarily maps that handle to the existing session registry
while the remaining consumers migrate.

## 2026-09-06 — GPUI checks require the local macOS compiler cache

**Finding:** A sandboxed GPUI check failed while Metal shader compilation tried
to write Clang modules below the user's local cache directory.

**Resolution:** The affected GPUI checks were rerun with the approved local
compiler-cache access. Pure SSH/SFTP/backend checks compile without that
requirement.

## 2026-09-06 — Settings management categories must be removed as a UI boundary

**Finding:** Removing `Themes`, `Hosts`, and `Shortcuts` from `AREAS` alone
left dead custom panes, field-registry entries, deep-link tests, and a host
settings callback. That would make the old ownership model appear removed
while still retaining unreachable Settings behavior.

**Resolution:** Removed the three navigation entries and their Settings-only
panes, removed host/keymap management rows from the Settings field registry,
and kept the persisted `SettingsContent.hosts`/`keymap` values only as an
explicit migration-compatibility layer. `Open Hosts` now targets the command
palette Hosts page; host management surface wiring is left to the Hosts
capability instead of keeping a broken `settings://hosts` deep link.

## 2026-09-06 — Historical architecture documents must be visibly superseded

**Finding:** Several reports and early ADRs remained in active-looking paths
and still described Settings-host management, toasts, and the former task
tree. The current normative docs were correct, but readers could not reliably
distinguish historical evidence from project rules.

**Resolution:** Added explicit superseded markers, introduced the normative
capability matrix and Settings UI guidelines, documented the full transitional
edge families, and added a rework task template. The old reports remain
available for traceability but cannot override `docs/` contracts.

## 2026-09-06 — SCM and Project Diff need one source-control capability

**Finding:** `panel-scm` and the workspace Project Diff still imported the
backend facade directly for Git values and every local/remote operation. A
graph-only contract was not sufficient because staging, commits, branches,
tags, stashes, and synchronization were still coupled to backend internals.

**Resolution:** Expanded `labonair-git` with the UI-free Git value types and
`GitService` operation contract. `BackendGitService` now adapts the existing
executor, while both UI surfaces receive only `Arc<dyn GitService>` from
composition. The dependency verifier now rejects the old backend edges.

## 2026-09-06 — SSH snippet execution needs an injected capability contract

**Finding:** After local snippet execution moved into `labonair-snippets`, the
panel still called the backend's russh function directly and translated raw
`AppEvent` payloads back into UI events. That preserved the old backend/UI
coupling and made the local and SSH execution paths structurally different.

**Resolution:** Added shared `SnippetRunEvent` values and the asynchronous
`SshCommandExecutor` contract to `labonair-snippets`. The backend now exposes
`BackendSshExecutor` as an adapter that owns russh session/channel details.
`SnippetsView` receives the trait object during composition and consumes typed
events through its existing local channel. The raw event wrapper remains only
as a compatibility seam for older callers.

**Non-obvious constraint:** `SnippetRunState` must be shared by the injected
adapter and cancellation calls, so `AppInner::snippet_run` is an `Arc` rather
than an inline state value. This keeps cancellation independent from the UI
and avoids copying the in-flight channel registry.

## 2026-09-06 — Capability panels need cloneable infrastructure handles

**Finding:** `panel-snippets` had already stopped using backend execution
functions, but its Cargo dependency and every CRUD operation still pulled in
the whole `Backend` facade solely to reach the shared SQLite connection.

**Resolution:** `labonair-persistence::Database` now owns an `Arc<Mutex<_>>`
and implements `Clone`. The panel receives that narrow handle directly, while
the shell injects the concrete SSH executor separately. The dependency
allow-list now rejects a backend edge for the snippets panel.

## 2026-09-06 — Git graph UI should depend on a graph-specific contract

**Finding:** `panel-git-graph` used the broad backend facade for repository
detection, log loading, commit details, and branch actions. That made a pure
graph view know about both local and SSH transport state.

**Resolution:** Added the UI-free `labonair-git` crate with `CommitInfo`,
`GitFuture`, and `GitGraphService`. `BackendGitGraphService` adapts the
existing backend Git executor, while the graph view receives an
`Arc<dyn GitGraphService>` from composition. The panel no longer depends on
`labonair-backend`; its existing UI and generation-guard behavior is unchanged.

## 2026-09-06 — Local snippet execution belongs in the snippets capability

**Finding:** Local snippet execution was implemented in
`backend::modules::snippets::exec`, even though it only needed a shell,
working directory, process cancellation, and output events. Passing the whole
backend `App` into that code coupled a local feature to unrelated SSH, MCP,
and persistence state.

**Resolution:** Added `labonair_snippets::exec` with `LocalRunRegistry`,
typed `LocalRunEvent` values, and a caller-provided event sink. The snippet
panel now runs local silent snippets directly through that API and cancels
them through its own registry. The backend retains only the SSH execution
adapter until a narrow SSH session contract is available.

**Compatibility note:** SSH output still uses the existing app event bus
while its transport contract is extracted. Local output no longer crosses
that stringly-typed bus.

## 2026-09-06 — Notification lifecycle must be UI-free

**Finding:** `labonair-notifications` combined notification data, GPUI entity
state, toast rendering, severity-specific timers, callback actions, and an
error preference gate. That made the notification system a presentation
implementation instead of a reusable cross-module registry.

**Resolution:** Added `labonair-notifications-core` with typed notification
kinds, stable registry IDs, retained records, read state, details, action
metadata, bounded history, and explicit deduplication. The GPUI crate now only
adapts the registry and keeps temporary callback compatibility. Notifications
are retained for the statusbar dropdown; no timer or toast layer remains.

**Verification detail:** The statusbar uses `overflow_y_scroll` for the full
history and toggles record details on row activation. The unread badge is
driven by registry read state rather than total retained records.

## 2026-09-06 — Remove dead error-notification setting with the toast path

**Finding:** `notify_on_errors` was exposed by Settings but no longer had a
valid product behavior once all messages were required to reach the central
notification dropdown.

**Resolution:** Removed the field from the typed settings content, legacy
preferences projection, migration mapping, and settings UI. Existing JSON
remains forward-compatible because unknown removed fields are ignored during
deserialization.

## 2026-09-06 — Shared database lifecycle is infrastructure, not host logic

**Finding:** The existing `HostsDb` wrapper initialized one SQLite database
containing hosts, credentials, and snippets. Treating that wrapper as the
host module's store would keep unrelated persistence concerns coupled.

**Resolution:** Created `labonair-persistence` for the shared SQLite
connection, schema creation, and idempotent migrations. The backend keeps only
the compatibility alias `HostsDb` and the old initialization path for now;
feature-specific query ownership remains the next migration step.

## 2026-09-06 — Host read and group operations can move without App state

**Finding:** Host listing, ordering, and group CRUD only need the shared
database and do not need secrets, MCP grants, or GPUI state.

**Resolution:** Added `labonair_hosts::store` and migrated host manager and
snippet consumers to use it directly. Secret-bearing create/update/delete and
duplicate operations remain in the backend adapter until their secret and MCP
event contracts are explicit.

## 2026-09-06 — Host write ownership moved behind typed requests

**Finding:** Keeping the old host write implementation in `backend` would
leave the host capability dependent on the application facade even after its
domain and read paths were extracted.

**Resolution:** Moved create, update, duplicate, delete, and sudo-password
storage into `labonair_hosts::store` using `HostCreateRequest` and
`HostUpdateRequest`. `HostEvent::AgentAccessBlocked` is the only callback
contract; the backend adapter remains responsible for inspecting MCP grants
and emitting `AppEvent::McpGrantExpired`. Added a direct store test proving
secrets do not enter the `Host` read model.

## 2026-09-06 — Credential capability has the same backend facade pattern

**Finding:** Credential metadata, secret storage, host references, and SSH
keypair generation were all implemented in `backend::modules::credentials`.
The module only needed database, secrets, and an explicit data directory; it
did not need `App` or UI state.

**Resolution:** Extracted the implementation into
`labonair-credentials`, including the russh-compatible keypair tests. The
backend module now only adapts the old App-based signatures and resolves the
data directory at the composition boundary.

## 2026-09-06 — Snippet persistence is independent from process execution

**Finding:** Snippet models and CRUD queries lived beside local/SSH execution
and therefore forced the entire backend module into every snippet UI caller.

**Resolution:** Created `labonair-snippets` with domain models and the SQLite
store, migrated `panel-snippets` to consume it directly, and retained only
the execution functions in the backend until process/session contracts are
extracted.

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
## 2026-09-06 — Transfer capability extraction and wire compatibility

**Decision:** Transfer lifecycle state and the queue UI now belong to the
`transfers` module. `labonair-transfers` is UI-free and receives legacy worker
events only through the backend adapter; `labonair-transfers-ui` owns the
statusbar queue. Workspace owns only the typed enqueue request and the
tab-local SFTP refresh callback. Transfer history remains in memory until a
separate persistence decision; retry remains a separate typed worker feature.

**Bug found:** The first typed `TransferStatus` enum omitted the existing
`serde(rename_all = "snake_case")` contract. The decoder therefore rejected
worker payloads such as `"status": "running"` even though compilation passed.

**Fix:** Restored the wire-format attribute in `crates/transfers/src/lib.rs`
and added a decoder regression test. The focused transfer tests and full
workspace gates now pass.
