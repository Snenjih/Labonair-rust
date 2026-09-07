# Rework progress — 2026-09-07

## Secrets ownership boundary

The backend Secrets compatibility wrapper is removed. Shell composition and
the SSH/SFTP/MCP adapters now use `labonair_secrets::SecretsState` plus its
canonical operations directly; no second storage API or backend re-export
remains. The storage format and secret lifecycle are unchanged.

## Unreferenced backend shell cleanup

The backend `shell` module was removed after repository-wide source search
found no active consumer beyond its own unit tests. It duplicated local
one-shot command, persistent agent-shell, and background-process execution
state without a live product owner. The running PTY/terminal and AI tool paths
remain unchanged; any future shared command execution must enter through an
explicit Terminal/AI capability contract.

## Updater capability boundary

The updater's manifest model, version comparison, network download,
minisign verification, macOS bundle installation, relaunch helper, and
auto-check cadence now live in the dedicated UI-free `labonair-updater` crate.
The shell keeps only `UpdaterView` and notification-driven presentation. The
backend updater module and its updater-only dependencies were removed; the
updater crate uses package version `1.0.0` so the current-version check matches
the application package rather than the backend's historical `0.1.0` version.

## Settings migration ownership boundary

The remaining pre-v2 Settings wire model and one-time `config.json` filename
and value migration now live in `labonair_settings::legacy_migrations`. The
backend Settings module and its transitional `labonair-settings-content`
dependency were removed. Runtime SettingsContent remains the canonical typed
value model; the migration module is intentionally compatibility-only. The
workspace test suite passed after this move.

## Workspace chrome migration boundary

The legacy `barItemPlacements` migration now lives in
`labonair_workspace::status_placements`. The Workspace owner performs the
id-remapping, backup, and idempotent conversion into
`statusBarItemPlacements`; the backend Settings compatibility module and its
dedicated migration file were removed. Workspace check, Clippy, full tests,
dependency validation, queue validation, and diff checks pass.

## MCP preferences ownership boundary

MCP bridge preferences now live in `labonair_mcp_core::preferences` as a
UI-free wire model with explicit `load_from`/`save_to` path boundaries. Shell
composition supplies the config directory; the backend Settings module no
longer owns MCP preference persistence. Focused MCP/backend tests, Workspace
check, and Clippy pass; the full workspace test gate remains part of the final
slice verification.

## Unreferenced backend module cleanup

The backend copies of `agents`, `directives`, and `model_prefs` had no active
consumers outside their own unit tests. They were removed rather than kept as
latent feature owners; a future implementation must enter through an AI-owned
contract and registry. This reduces backend surface without changing any
reachable product workflow.

## Legacy editor migration boundary

The old standalone editor persistence adapter and `Preferences::editor_prefs`
projection were unreferenced by the running app and are removed. The
historical `EditorPrefs` shape remains private inside `migrate_v2` solely to
read old files; its original defaults are preserved so default Vim search
values are not emitted as false user overrides.

## SSH connection event boundary

The SSH transport/authentication pipeline was narrowed after the PTY and
remote-file adapter split. `ClientHandler`, transport setup, jump-host
handshakes, authentication, and PTY reader disconnect reporting now receive
only `EventBus`; they no longer retain or pass the aggregate backend `App`.
The connection adapter still owns composition of database, secret, trust, and
session state, so those dependencies remain available only at that explicit
boundary while the next capability splits are prepared.

The former aggregate `BackendSshService` was then removed. Connection,
connection-test, SSH-config, and tunnel contracts now use separately named
adapters constructed in shell composition; each adapter stores only the state
required by its contract. Tunnel startup also receives `EventBus` directly,
so its asynchronous connection loop no longer carries the aggregate `App`.

The MCP HTTP server was narrowed next. `McpServerAccess` explicitly carries
only SSH state, local PTY state, database, secrets, and EventBus; server tools,
auth checks, activity events, and tab-operation requests use that bundle
instead of the aggregate backend App. `PtyState` is reference-counted in the
backend so the server can share local terminal sessions without widening its
state boundary.

The SFTP transfer worker was narrowed after that. Its queue loop and all
download/upload helpers now receive `EventBus` directly for progress, steps,
and connection-loss reports, alongside their existing SSH, conflict, and
settings state. `App::spawn_workers` only extracts and injects those concrete
capabilities; the worker no longer stores or passes the aggregate App.

The legacy SFTP connection orchestration now follows the same boundary. Its
health check, subsystem initialization, and session-established reporting use
an explicit `EventBus`; the SFTP connect path no longer stores or passes the
aggregate backend App merely for event emission.

The backend Credentials compatibility module was then removed completely. It
had no active consumers, while Hosts UI already calls the canonical
`labonair-credentials` crate directly; the backend dependency and module export
are gone.

The unused backend Themes compatibility module and duplicate bundled asset were
removed as well. The canonical `labonair-theme` crate already owns the static
theme and icon-theme registries, and the old network download/import surface
had no active consumers and conflicted with the current fixed-catalog product
direction.

The backend `terminal_exec` module was then removed after repository-wide source
search found no active callers. MCP already owns the live terminal execution
implementation, so retaining the duplicate module only kept an unnecessary
`App`-bound state and compatibility surface alive.

The remaining Git service constructors and snippets SSH runner were narrowed in
the same pass. Shell composition now injects Git's SSH/EventBus capabilities,
and snippet event delivery receives EventBus directly; neither adapter needs the
aggregate backend App.

The final production use of `App::emit` was removed from the SSH logging macro,
and the app-state smoke test now exercises `app.events` directly. The aggregate
facade no longer exposes an event-emission shortcut.

The aggregate backend `App`/`AppState` was then moved out of
`labonair-backend`. `labonair-shell::BackendComposition` now owns only
application construction, capability extraction, and worker startup; the
backend package exposes concrete platform adapters without a broad application
state facade. Backend tests now construct only the capabilities under test.

The binary bootstrap was narrowed one step further: shell composition now owns
settings migration, development event diagnostics, and updater type exposure.
The `labonair` package no longer declares direct backend, filesystem, or feature
engine dependencies, leaving it as a thin native process entrypoint.

## R06-001 backend facade inventory and error boundary

Recorded the complete backend module/export and direct-consumer map in
`docs/audits/backend-facade-inventory.md`. The structured error contract was
already owned by `labonair-errors`, so the backend root re-exports, the unused
`AppResult` alias, and `backend::modules::errors` were removed. Backend SSH,
SFTP, and MCP implementation code now imports `labonair-errors` directly. This
is the migration pattern for the remaining facade entries: consume the
canonical contract first, then remove the compatibility edge once source
search and focused tests prove it has no external consumer.

The AI crate's stale direct dependency on `labonair-backend` was removed after
source search confirmed that `NativeHost` already uses `labonair-filesystem`
directly. Its documentation now names the actual filesystem capability.

System-font discovery moved from the unused backend `fonts` module into the
Theme owner, which already owns bundled fonts and typography. The backend
custom-font compatibility module and its `fontdb` dependency were removed
because no active UI or crate consumer exists; Settings now receives the
Theme-owned discovery service.

The full workspace test suite passed after the Theme migration (including the
172 backend, 70 Theme, and 22 Settings UI tests); formatting, Clippy, check,
dependency, queue, and diff gates are also green. Current HEAD is `5406c1f`.

The legacy `barItemPlacements` data remains readable only inside the one-time
settings migration. Its unused backend read/write API and `App` lock were
removed, along with the obsolete unit test; current status-bar placement APIs
remain unchanged for the live workspace surface.

The affected backend/workspace/shell tests and all repository gates passed
after the bar-placement removal. Current HEAD is `fc68c23`; the worktree is
clean and R06-001 remains active.

Workspace Git ownership was tightened: `Workspace::new` now receives the
canonical `GitService` and `GitGraphService` contracts, and the shell builds
the concrete backend adapters once for both Workspace and the Git panels.
Workspace no longer imports or constructs `BackendGitService` implementations.

Live status-bar placement and panel-toggle visibility persistence was moved
from backend Settings into `labonair-workspace::status_placements`. A single
workspace-owner async write lock serializes both blobs because they share the
same config file; backend Settings retains only JSON helpers needed by legacy
migrations and value-settings adapters. The affected tests and full compile,
Clippy, and focused test gates passed.

Terminal scrollback persistence was then moved from the backend module into
`labonair-terminal::scrollback`. The terminal capability now owns compression,
atomic writes, size limits, restore, deletion, orphan cleanup, and retention;
Workspace and shell only call its public API. The old backend module and its
direct consumers are gone. To preserve the target dependency graph, the data
directory is passed as an explicit path context by Workspace and shell rather
than making the terminal crate depend on Filesystem. Focused and full workspace
gates passed; the slice is committed as `534ccd5`.

The updater's broad backend-root re-export was removed as well. Shell and app
tests now import `AvailableUpdate` and the updater contracts from
`backend::modules::updater`, leaving the backend root for composition/events
only. The affected checks passed and this cleanup is committed as `8cdf216`.

The MCP boundary now has a real UI-free contract crate,
`labonair-mcp-core`, for `SessionKind`, `TabOpResult`,
`SessionGrantRequest`, `McpSessionAccessService`,
`McpTabOperationService`, `McpEvent`, and `McpEventSource`. The backend
implements those contracts through explicit adapters, while shell composition
injects them into `AgentAccessStore` and Workspace. Workspace grant lifecycle,
tab-operation responses, and MCP event handling no longer call backend MCP
functions or subscribe to the aggregate event bus directly; grant failures are
returned as failed tab-operation results.

The SSH boundary now follows the same pattern: `labonair-ssh` owns typed
`SshConnectionEvent`, `SshEventReceiver`, and `SshEventSource` contracts, the
backend translates its legacy events in `BackendSshEventSource`, and Workspace
receives only typed connection events through `SshEventBridge`. The Workspace
crate no longer declares `labonair-backend`; the remaining global event bus is
an internal source of the shell-composed backend adapters. Full workspace
check, Clippy, tests, dependency, queue, format, and diff gates passed for
this boundary.

The unconsumed backend filesystem watcher and `App::watcher` state were removed.
Backend modules now import `labonair-filesystem::paths` directly, and the
backend filesystem compatibility module was deleted. The unproduced
`fs:dir-changed` typed event and the broad global `AppEvent` decoder were
removed as well. SSH and MCP adapters now decode only their own raw event names
directly into their canonical contracts; MCP grant revocation keeps its
existing legacy wire event through `App::emit` until the remaining backend
adapter is extracted.

SSH, MCP, and Transfer event-source adapters now receive `EventBus` directly
instead of retaining the aggregate backend `App`. This keeps raw event
transport available at the composition boundary without widening the adapter's
state dependency.

`BackendTransferService` was narrowed in the same way: it now receives only
`TransferWorkerState`, while `BackendTransferEventSource` continues to receive
only `EventBus`. Shell composition remains the sole place that reads those
fields from the aggregate during migration.

The SFTP service boundary was narrowed next. `BackendSftpService` now receives
only the shared `SshState` and `EventBus`; its remote operation helpers use the
same event bus for network-loss reporting. Authentication and host lookup stay
on the separate SSH connection service path. Focused Backend and Shell tests,
formatting, dependency, queue, and diff checks passed.

The MCP grant/session boundary was narrowed next. `BackendMcpSessionAccess`
now receives only `McpState` and the shared `Database`; host-block revocation
uses explicit MCP state plus `EventBus`, and the auto-revoke sweeper emits
through `EventBus` directly. Token storage, listener startup, and MCP terminal
tool execution remain the separate App-bound server boundary.

The Git boundary was narrowed next: `GitExecutor::Remote` now stores only
`SshState` and `EventBus`; all public Git operation functions receive an
explicit `EventBus` as well. `BackendGitService` and
`BackendGitGraphService` retain only those two capability states, so the
aggregate App is no longer part of the Git operation surface.

The SSH adapter was split by capability next. PTY write/resize now use
`BackendSshPtyService` with only `SshState`; remote command/file operations
use `BackendSshRemoteService` with `SshState + EventBus`. Connection,
trust, config, tester, and tunnel operations remain in the broader adapter
until their database/secrets/trust/tunnel dependencies are extracted.

The Secrets compatibility surface was narrowed alongside this work:
`SecretsState` is now the only state passed to secret reads/writes,
encryption access, and service-name migration. SSH, SFTP, and MCP no longer
pass the aggregate App merely to read or store a secret, and jump-host
resolution now takes only database and secrets inputs.

## Native visual verification

The exact Rust bundle was opened through its absolute `.app` path. Core
Graphics confirmed a visible layer-0 window for the exact bundled Rust PID;
the macOS screenshot API then denied capture because Screen Recording access
is unavailable in the runner. The legacy Tauri app was not used.

`scripts/screenshot.sh` now validates caller-provided PIDs against the exact
Rust executable before window lookup and distinguishes an invalid process from
macOS Screen Recording denial. This keeps visual evidence fail-closed.
## R02-003 completion

The user confirmed the native Rust shell visual state is acceptable, so the
visual acceptance criterion for `R02-003` is closed. Project and standalone
workspace identity now use the typed `WorkspaceTransition` contract with
session persistence and no cwd-based inference. `R03-001` is the next active
task.

## R03-001 provider boundary

Workspace, terminal, editor, hosts, themes, and settings now expose
owner-local `CommandProvider` implementations. The shell composition root
assembles them into the single command registry and its transitional execution
adapters assert descriptor equality against the owner snapshots. A direct
keymap provider was deliberately not added: `command-palette-core` currently
depends on keymap's `ShortcutId`, so `keymap → command-palette-core` would be
a Cargo cycle. The future fix is to extract/invert the stable command/shortcut
identity contract.

The migrated provider rows were removed from the palette-only shell table.
Remaining shell descriptors are transitional execution adapters and are
validated against owner metadata, so they cannot silently become a second
palette discovery source.

The provider migration now includes workspace tabs/panes, Git, and Snippets.
The palette-only shell table is now gone. `Toggle Full Screen` is a shell-owned
provider because it targets the native window directly. Keymap provider
registration remains blocked by the current command-core-to-keymap dependency
cycle.

The command-palette core now owns the global palette command metadata, and
Settings owns toggle-command metadata. The static palette table has been
removed entirely.

The legacy `settings::OpenShortcuts` action now resolves centrally to the
canonical `zed::OpenKeymap` action and is accepted by keymap validation without
being exposed as a palette command.

The dependency verifier was updated for the intentional owner-to-command
contract edges and passes with an acyclic graph. The command-core-to-keymap
edge remains one-way by design until shortcut identity extraction.

## R03-001 dynamic submenu registry

`labonair-command-palette-core` now provides a typed `SubmenuRegistry`,
`SubmenuSnapshot`, and `SubmenuAction` contract. The palette consumes registry
snapshots for all dynamic pages instead of maintaining separate `PaletteData`
arrays or reading workspace/panel entities while rendering. Snapshot builders
now live in the workspace, hosts, editor, theme, snippets, and Git provider
modules. The shell supplies live values and registers the snapshots. Hidden
status-bar state and labels are now owned by the workspace status registry as
well.

## R03-001 shortcut identity boundary

The stable `ShortcutId` enum now lives in the UI-free
`labonair-interaction-contracts` foundation crate. `labonair-keymap` re-exports
that type, owns its defaults/resolution, and publishes a `KeymapCommandProvider`
for `Open Keymap (JSON)`. `labonair-command-palette-core` depends only on the
identity foundation, so keymap can depend on the command contract without a
cycle. The shell adapter and provider metadata are equality-checked.

## R03-001 completion

Zoom and Color Mode are now owner snapshots too, so no palette submenu page
constructs its own rows. Hidden status-bar state and labels are composed by the
workspace owner. `R03-001-command-palette-provider-registry` is complete after
full workspace tests, Clippy, formatting, dependency verification, and the
user's visual confirmation. `R03-002-keymap-runtime-and-editor` is next.

## R03-002 resolver slice

`labonair-keymap::runtime` now provides a GPUI-/file-format-free resolver for
typed `CommandId` bindings. Global and active-context precedence, equal-level
last-write-wins replacement, normalized keystroke identity, and stable first
appearance ordering are covered by tests. The existing settings parser remains
the lossless file adapter, while the GPUI menu adapter now consumes the
keymap-owned canonical action resolver before selecting concrete actions.

## R03-002 adapter integration

Persisted action names now cross into the GPUI menu adapter through
`keymap::runtime::command_for_action`, producing a typed `CommandId` before the
adapter selects a concrete GPUI action. This removes the shell's duplicate
string alias table while preserving the legacy `settings::OpenShortcuts` alias
and rich GPUI context predicates. Commit: `360bece`.

## R03-002 keymap file ownership

Moved the complete JSONC keymap file contract from `labonair-settings::keymap`
to `labonair-keymap::file`: parsing, layered merge, validation, default
platform assets, user path, and scaffold creation. Settings now remains a
values-only module and no longer depends on Keymap; Shell and Workspace use
Keymap directly. The dependency verifier is still acyclic. The existing
Settings rename-watcher test passes outside the sandbox. This extraction is
the basis for moving reload/install orchestration behind the Keymap boundary.

## R03-002 keymap load ownership

Moved last-good user-file recovery, validation issue retention, default/user
layer composition, and shipped-default parsing into `labonair-keymap::file`.
The shell loader now only supplies the current command vocabulary, connects the
existing watcher, installs GPUI bindings, and publishes display hints. This
keeps file semantics and recovery in Keymap while leaving platform input in a
thin adapter. Commit: `c2bffcf`.

## R03-002 owner default bindings

`CommandDescriptor` now owns optional typed default bindings. The keymap
runtime converts provider descriptors into `KeymapBinding` values and the file
layer materializes them before the user override layer. This keeps shortcut
defaults with the feature that owns the command; the shell only supplies the
registry and platform adapter. The first migrated default is `Open Keymap
(JSON)`, while the shipped JSONC defaults remain compatible during migration.

## R03-002 keymap loading adapter

Added `labonair-keymap::adapter::load`, which accepts the command registry and
returns one immutable snapshot containing effective bindings and diagnostics.
Known-action vocabulary, owner-default materialization, user-file recovery,
and layer composition no longer live in the shell loader. The shell now keeps
only GPUI installation, display-hint publication, and filesystem-watch wiring.

## R03-002 owner default migration

Migrated the existing default bindings into their owning command providers for
palette opening, editor search, workspace tabs/panes/sidebar/zoom, settings Zen
mode, and shell debug actions. The shell execution adapter derives matching
descriptor metadata only during the migration, while provider equality checks
prevent the two snapshots from drifting. Adding the editor provider's stable
shortcut identity required the allowed foundation edge from `labonair-editor`
to `labonair-interaction-contracts`.

## R03-002 command-keyed display path

Changed the palette, panel tooltips, and tab context-menu key-hint lookups to
use the effective keymap by `CommandId`, with descriptor defaults as the
test/bootstrap fallback. The old `ShortcutId` table is no longer an active UI
source and remains exported only for migration compatibility.

## R03-002 lossless editor document

Added `keymap::file::KeymapDocument`, which keeps raw JSONC source authoritative
and exposes parsing/validation as derived state. Comments, unknown actions, and
malformed edits are retained for the management/editor surface instead of
being discarded by a parse/serialize round trip. `save_user_keymap_document`
persists the raw source through the Keymap owner.

## R03-002 management surface

Added the real `labonair-keymap-ui` sibling crate. It consumes a
`KeymapManagementSnapshot` from the UI-free keymap module, loads file state on
the GPUI background executor, renders searchable command rows and validation
diagnostics with `ui-kit`, and delegates raw JSONC editing back to the shell
through one injected callback. Titlebar and palette Keymap entrypoints now
open the dedicated native window. Per-row rebinding and conflict actions are
now implemented with GPUI keystroke validation, same-context conflict
detection, explicit unbind, and lossless append-only JSONC overrides. Keymap
diagnostics and load/save failures publish through the retained notifications
registry; the view no longer renders a duplicate passive error banner. The
user confirmed the native visual state.

The keymap runtime canonicalizer now handles multi-step chords by normalizing
each keystroke independently while preserving chord order. This is important
because the file layer already treats a chord as a space-separated sequence;
single-keystroke normalization would otherwise fail to detect equivalent
chords.

## R03-002 completion

Full workspace check, Clippy, tests, formatting, dependency verification,
queue validation, and diff checks pass. R03-002 is complete; R04-001 is the
next active task.

## R04-001 static catalog slice

The runtime theme path now exposes only the embedded deterministic color and
icon catalogs. Settings no longer scans or edits theme files, shell startup no
longer watches theme directories, and the palette receives built-in choices
directly from `labonair-theme` while keeping preview/commit/cancel behavior.
Theme file loaders remain isolated extension adapters for a later product
decision and are not invoked by the supported workflow.

The first post-change full workspace test pass had one unrelated concurrent
backend Git test fail with SQLite `database is locked`; rerunning that exact
test with one test thread passed. Treat this as test-environment contention,
not a Theme change regression, and keep the isolated rerun in the verification
record.

R04-001 is complete. The user confirmed the native visual state. The next
active queue task is R04-002, which establishes the single Hosts owner and
SSH/SFTP picker flow. The combined implementation/documentation commit is
`20bff4b`.

## R04-002 Hosts owner slice

The Hosts capability now exposes typed `HostOpenRequest`/`HostOpenMode` values
and canonical immutable picker rows. `HostManagerView` is composed once by the
shell and opened through the Hosts-owned native window; the titlebar, Open Hosts
command, and SSH error recovery all use that surface. Palette host rows are
owner snapshots with SSH as the primary action and SFTP as the Shift+Enter
secondary action. The obsolete SQLite-hosts-to-Settings projection was removed
from startup and migration code, along with the Settings host model and
credential-ref compatibility helper. Focused Hosts/UI/Workspace/Shell tests
pass. The task remains active for backend adapter cleanup and visual state
review.

## R04-002 backend host-adapter removal

Removed `backend::modules::hosts` entirely. The backend now initializes the
generic `labonair-persistence::Database` directly, while SSH/SFTP transport
adapters use that foundation type and Host CRUD remains in
`labonair-hosts::store`. The MCP grant-revocation callback was moved to the MCP
module as the one explicitly tracked host integration. Focused compilation,
full tests, Clippy, dependency, queue, formatting, and diff checks remain
green. The task remained active only until the user's visual confirmation.

## R04-002 completion

The user confirmed the Hosts management and picker visual states, closing the
empty, filtered, invalid, and connection-pending review. The canonical
picker-ordering test passes, and the full workspace verification gates remain
green. R04-002 is complete; `R05-001-settings-audit-and-value-normalization`
is next.

## R05-001 settings inventory

R05-001 is active. Added `docs/settings-inventory.md` as the normative
field-to-consumer and scope inventory for all 138 typed Settings fields. It
classifies current values to keep, duplicated state to move to Background,
Workspace, SFTP/Transfers, or capability owners, and legacy/unsupported values
to remove after explicit migration handling. The next step is consumer proof
and lossless migration design.

The shipped default also contained an untyped `general.notifyOnErrors` key
with no consumer. It was removed, and `SettingsContent` now tests the raw
default asset's object shape against the typed default serialization so future
untyped default drift fails immediately.

The first ownership move is complete: `backgroundImage`, `backgroundOpacity`,
`backgroundBlur`, `backgroundTintColor`, and `backgroundTintOpacity` no longer
exist in `AppearanceContent` or the Settings UI. The migration moves those
legacy appearance values to the top-level keys already consumed by
`labonair-background`, including files stamped `sparsified: true`; existing
values are preserved and covered by two migration tests.

The Background move is committed in `af7167f`. Full workspace check, Clippy,
serial workspace tests, dependency verification, queue validation, formatting,
and diff checks pass. Next R05 slice: prove the remaining indirect consumers
and move workspace layout/runtime state out of Settings with a lossless
migration.

## R05-001 Workspace layout ownership

Dock/sidebar runtime state now has an explicit `labonair-workspace::layout`
owner and is persisted as versioned `workspace-layout.json`. Workspace startup
loads that snapshot directly; dock persistence no longer calls back into the
shell or writes `SettingsStore`. Settings UI and project whitelisting no longer
expose layout state. The app runs an idempotent migration that imports legacy
v2 `workspace.sidebar*`/`dockLayout` values, writes the owner file atomically,
and removes the migrated keys from `config.json`. The old typed fields remain
temporarily as migration compatibility input and are the next cleanup target.

The layout module has round-trip and legacy-removal tests. Formatting, workspace
check, Clippy, dependency verification, queue validation, and diff checks pass.
The full serial workspace suite passed except for the known AI local HTTP test
being blocked by sandbox socket permissions; that one test passed when rerun
outside the sandbox. The active task remains R05-001.

The follow-up removed the eight dock/sidebar fields from `WorkspaceContent`,
its shipped defaults, and the Settings model. `labonair-workspace::layout`
now accepts both legacy v1 `preferences` and v2 `workspace` objects, so the
app runs that migration before `migrate_settings_v1_to_v2`; the backend's
legacy `Preferences` remains only as an input wire shape and records these
fields in its exhaustive migration-accounting test. JSONC parsing is used for
the legacy config read, while the owner file is written atomically as JSON.
Focused layout, settings-content, and backend migration tests pass.
This removal is committed in `a474d03`; the active task remains R05-001.

## R05-001 Terminal settings cleanup

Removed the unsupported terminal settings from the typed model, shipped
defaults, generated Settings UI, and v1-to-v2 Settings mapping:
`terminalFontWeight`, `terminalLetterSpacing`, `terminalCursorBlinkInterval`,
`terminalWordSeparator`, `terminalScrollSensitivity`,
`terminalFastScrollModifier`, `terminalUseWebgl`, all command-composer flags,
and all block-terminal flags. The native terminal has no consumers for these;
theme token typography remains theme-owned rather than pretending to be a
Settings value. The legacy backend `Preferences` wire shape stays readable and
the exhaustive migration test accounts for the removed keys. Focused tests,
full workspace check/Clippy/tests, dependency and queue checks are green.
Committed as `3bcde42`; R05-001 remains active for the next consumer audit.

## R05-001 General settings cleanup

Removed `autostart`, `credentialEncryption`, and `confirmQuitWithSsh` from
`GeneralContent`, shipped defaults, generated UI, and the Settings migration.
No native startup, credential, or quit-flow consumer exists for these values;
the legacy `Preferences` wire shape remains readable and the exhaustive
migration test records them as removed compatibility input. Updated the
initial project template and affected parser/whitelist tests. Full workspace
tests, check, Clippy, dependency, queue, formatting, and diff checks pass.
Committed as `bca66ed`. `startupTerminalCount` is intentionally still under
review rather than removed speculatively.

The follow-up removed `startupTerminalCount` as well: the native workspace
opens one terminal from the startup-tab decision, and no current runtime
consumer uses a configurable count. The field was removed from the typed
model, defaults, generated UI, project whitelist, templates, and migration
mapping; legacy Preferences remains readable as compatibility input. Affected
parser, project-layer, and store tests were redirected to the retained
`restoreWindowState` value. Full workspace tests and all repository gates pass.
Committed as `61abf4c`; R05-001 remains active for the next inventory group.

## R05-001 Appearance radius normalization

Removed the obsolete `appCornerRadius` field from `AppearanceContent`, shipped
defaults, generated Settings UI, and the Settings runtime accessor. Legacy v1
and already-sparsified v2 values are converted from historical pixel units to
the current `cornerRadiusScale` value during migration; an explicit modern
scale wins when both keys are present. The legacy Preferences field remains
deserializable as migration-only input, and migration accounting plus v1/v2
fixtures cover the conversion. The inventory now records the Background owner
as active rather than pending removal from Settings. Full workspace tests,
check, Clippy, dependency, queue, formatting, and diff checks pass.
Committed as `c6e9890`; R05-001 remains active for the next indirect Appearance
field audit.

## R05-001 Appearance layout cleanup

Removed the five Appearance values with no native runtime consumer:
`sidebarTabInfoLine`, `sidebarGroupByFolder`, `sidebarGroupSingleTabs`,
`badgesAlwaysVisible`, and `titlebarsIconsPosition`. They are gone from the
typed model, defaults, generated Settings UI, and v1 migration target while
the legacy Preferences wire shape remains readable for old files. The
inventory now records them as remove-only compatibility input. Full workspace
tests, check, Clippy, dependency, queue, formatting, and diff checks pass.
Committed as `d596df2`; R05-001 remains active for indirect consumer proof.

## R05-001 Editor audit cleanup

Removed the four Editor values with no current native editor consumer:
`editorAutoSave`, `editorAutoSaveDelay`, `editorAutocompleteDebounceMs`, and
`editorMaxFileSizeMb`. They are gone from the typed model, defaults, generated
Settings UI, and v1 migration target while the legacy Preferences wire shape
remains readable for old files. The inventory now records them as remove-only
compatibility input. Full workspace tests, check, Clippy, dependency, queue,
formatting, and diff checks pass. Committed as `145966b`; R05-001 remains
active for the File Manager and Connection audits.

## R05-001 Remote settings ownership cleanup

Removed all unconsumed SFTP/browser and connection timing values from the
Settings model, shipped defaults, generated UI, and migration targets. The
File Manager Settings area now contains only Explorer/SCM values with proven
native consumers. The empty Connections Settings module/category and source
file were removed; legacy Preferences fields remain migration-only input and
are covered by exhaustive accounting. Transfer runtime policy remains in the
existing transfer worker instead of being duplicated in Settings. Full
workspace tests, check, Clippy, dependency, queue, formatting, and diff checks
pass. Committed as `af6db5f`; R05-001 remains active for the remaining Review
values and consumer tests.
## R05-001 Command Palette settings cleanup

Removed `commandPaletteBlur` and `commandPaletteAnimation` from the typed
Workspace settings, defaults, generated UI, and v1 migration target because
the native palette has no consumers for them. Legacy Preferences remains
deserializable and exhaustive migration accounting covers both keys. Focused
Settings/content/UI/backend tests and the workspace check passed. Committed as
`a945ef5`; R05-001 remains active for the final retained-value consumer audit.

## R05-001 SCM settings cleanup

Removed `gitStatusPollIntervalMs` from the Workspace settings model, defaults,
generated UI, project configuration surface, and v1 migration target because
no native SCM code consumes it. Legacy Preferences remains readable and the
exhaustive migration accounting covers the compatibility key. Check, Clippy,
and focused Settings/content/UI/backend tests pass. Committed as `8804d60`;
R05-001 remains active for final retained-setting consumer tests.

## R05-001 Terminal consumer cleanup

Removed six Terminal settings with no native consumer:
`terminalDefaultPath`, `newTabInheritsCwd`, `confirmCloseTerminalTab`,
`terminalLineHeight`, `terminalShowPaneHeader`, and
`terminalShowPaneFooter`. They are gone from the typed model, defaults,
generated UI, and migration targets. The two non-functional pane toggle
commands were removed from the command-palette core, Settings provider, shell
registration, and toggle execution path; the theme-owned terminal line-height
token remains intact. Full workspace tests, check, Clippy, dependency, queue,
formatting, and diff checks pass. Committed as `701b1b3`; R05-001 remains
active for final retained-setting consumer proof.

## R05-001 Editor consumer audit

The consumer audit found eight additional Editor values that existed only in
the typed model, Settings UI, and migration: `editorLineHeight`,
`editorTrimTrailingWhitespace`, `editorInsertFinalNewline`,
`editorBracketMatching`, `editorShowCursorPosition`,
`editorShowSelectionStats`, `editorShowOutline`, and
`editorIndentationGuides`. They were removed from the current model and UI;
the legacy Preferences wire shape and exhaustive migration accounting remain
for compatibility. The Settings search test was updated to assert a valid
multi-area query after removing the stale cursor-position expectation. Full
workspace tests, check, Clippy, dependency, queue, formatting, and diff checks
pass. Committed as `3bc4930`; R05-001 remains active for the final retained
value proof.

## R05-001 Format-on-save cleanup

Removed `editorFormatOnSave` from the typed Editor settings, defaults, project
whitelist, generated UI, and migration target because the native editor does
not implement formatting on save. Removed its Settings/command-palette toggle
path from the command registry and shell action handling as well. Updated the
project template and Settings inventory. Full workspace tests, check, Clippy,
dependency, queue, formatting, and diff checks pass. Committed as `7ad02f1`;
R05-001 remains active for the retained-setting consumer audit.

## R05-001 completion

The Settings audit is complete. Every remaining typed field has a current
consumer, default, type, scope, and generated UI path; all unsupported,
duplicated, and misplaced values were removed or migrated. Hosts, themes,
keymaps, notifications, transfers, and workspace layout are not Settings
categories or duplicate state owners. The complete workspace gates pass,
including serial tests, check, Clippy, dependency verification, queue
validation, formatting, and diff checks. R05-001 is closed; R06-001 is next.

## R06-001 local terminal capability boundary

The old backend `PtyState` was a second local PTY implementation. It was not
the session registry used by Workspace, and local MCP grants were therefore
stored with a stale numeric `local_pty_id` address while Workspace always
created sessions in `labonair-terminal::TerminalRegistry`.

The MCP core now exposes `LocalTerminalAccess` and a raw-output receiver
contract. Shell composition adapts the real terminal registry into that
contract; MCP writes and output capture use the same local session id as the
visible terminal. The duplicate backend PTY module and obsolete grant field
were deleted. Shared OSC 7/133 shell scripts were moved to the UI-free
`labonair-terminal-integration` crate and are consumed by both local and SSH
adapters. Workspace compile, Clippy, tests, dependency, queue, format, and
diff gates pass.

## R06-001 transfer/SFTP ownership cleanup

The SFTP module contained queue operations and worker state even though the
product model treats transfers as a separate capability. Queue state and
command dispatch now live in the canonical `labonair-transfers` crate and are
consumed by the backend transfer adapter, which implements
`labonair-transfers::TransferService`. The unreferenced SFTP
settings and reconnect wrappers were removed; the transport worker remains
temporarily under the backend SFTP adapter because it still depends on the
concrete SSH/SFTP session implementation.

## R06-001 adapter event transport extraction

The raw `EventBus`, `EventChannel`, and `RawEvent` primitives were moved from
the backend into the UI-free `labonair-events` foundation crate. This keeps
the backend focused on concrete transport adapters and makes the shared
transport available without importing a backend facade. SSH, MCP, and
Transfers still decode their own raw event names at their adapter boundaries;
the foundation crate intentionally contains no product event vocabulary.

## R06-001 SSH transport extraction

The concrete SSH module was moved from `labonair-backend` into the new
`labonair-ssh-transport` integration sibling. It now owns the russh session
registry, authentication/connection flow, PTY and remote operations, tunnel
state, config adapters, and SSH contract implementations. Backend Git, SFTP,
MCP, and snippet adapters consume the transport through explicit public
types/functions, while `labonair-ssh` remains the UI-free contract crate.

## R06-001 transfer worker extraction

The concrete SFTP transfer worker moved from `labonair-backend` into
`labonair-transfers-ssh`, a Transfers-owned integration sibling. It owns
chunked upload/download, checksums, conflict handling, cancellation, and
reconnect requeue behavior; `labonair-transfers` remains the UI-free
lifecycle/queue contract while the backend retains only service/event
translation.

## R06-001 SFTP adapter extraction

The concrete SFTP session setup and `labonair-sftp` contract adapters moved
from `labonair-backend` into `labonair-sftp-ssh`. The new integration sibling
receives SSH state and the raw EventBus explicitly and owns the SFTP session
lifecycle boundary; the UI-free SFTP crate remains contracts-only.

## R06-001 Git adapter extraction

The concrete local/remote Git CLI executor and Git contract adapters moved
from `labonair-backend` into `labonair-git-transport`. The integration sibling
receives SSH state and EventBus explicitly; `labonair-git` remains the
implementation-free capability contract.

## R06-001 MCP server extraction

The concrete MCP HTTP server, grant state, PTY bridge, host revocation, and
MCP service/event adapters moved from `labonair-backend` into
`labonair-mcp-server`. The shell now composes this integration sibling with
explicit SSH, local-terminal, database, secrets, and EventBus capabilities;
`labonair-mcp-core` remains contracts-only.

## R06-001 — Snippet/transfer adapters and backend package removal

The remaining concrete SSH snippet executor moved into the new
`labonair-snippets-ssh` integration sibling. The transfer service and event
source adapters moved into `labonair-transfers-ssh` beside the concrete SFTP
worker. Both integrations receive explicit capability state and EventBus
values; neither exposes or accepts aggregate application state.

The obsolete `labonair-backend` package, root exports, module tree, workspace
member, and shell dependency were then deleted. The shell composition bundle is
now named `AppComposition` and lives in `labonair-shell::composition`, making
its role explicit without creating a replacement feature facade. The active
crate graph contains 50 workspace crates, is acyclic, and has no backend
dependency or broad-facade allow-list exception. Current source and docs use
the named integration siblings as the ownership boundary.

## R07-001 audit — Panel contribution ownership

The four built-in panel crates now each build their own typed
`PanelRegistration`. Shell composition only collects those contributions and
inserts them into the shared Workspace registry; it no longer contains a
generic helper that names every concrete panel type. The Git Graph crate stays
acyclic because it returns a contribution without importing Workspace. The
remaining shell command-execution and status-item lists are tracked separately
by R07-002.

## R07-001 — Statusbar contribution ownership

The dedicated Jump Hosts badge was removed because it duplicated the canonical
host-selection surface without exposing useful status. Agent Access now lives
in `labonair-workspace::status_items`, and the transfer badge plus its queue
refresh subscription live in `labonair-transfers-ui::status_item`. Both owners
return typed `StatusItemRegistration` values; shell composition only inserts
them into the shared registry. The transfers UI consequently depends on the
Workspace contract for post-transfer refresh, while the dependency graph stays
acyclic.

The pure CWD breadcrumb helpers were moved from `labonair-shell` to
`labonair-workspace::cwd_breadcrumb`. The interactive statusbar view still
needs to move, but its path/provider model is now owned by the workspace
module. Workspace check, dependency validation, formatting, and diff checks
pass after the move.

## R07-001 — Command handler boundary

Added the neutral `labonair-command-palette-runtime` crate for GPUI-facing
owner callbacks. Workspace now registers tab, pane, project-lifecycle, and
focus handlers against its own `Entity<Workspace>` without importing shell or
palette UI. Shell composition installs those handlers before its transitional
legacy adapters; an owner handler suppresses the duplicate legacy behavior at
runtime. The old shell table is still present and must be deleted incrementally
after the remaining owners (terminal, editor, hosts, themes, settings, and
shell-native actions) receive equivalent contributions.

The Workspace command migration then removed the shell registrations for tab
creation/selection, pane splitting/closing, focus navigation, save/close/
duplicate tab operations, and project lifecycle. Their metadata remains in the
Workspace provider, while executable callbacks come from the Workspace handler
contribution. The shell command table now retains only unmigrated feature
adapters and native-window wiring.

Cursor Position and Preview URL statusbar views were moved into
`labonair-workspace::status_items` and now expose typed registrations. Shell
composition no longer constructs these views directly; their rendering still
uses the existing Workspace active-tab query contract. Targeted Clippy, tests,
dependency validation, formatting, and diff checks pass.

Dock panel buttons were moved into `labonair-workspace::dock_status_item`.
That owner now contains the panel icon/title mapping, dock move/hide menu,
statusbar rendering, and typed contribution constructor. Shell composition
only requests the three edge registrations. The Workspace/Shell checks and
tests, dependency validation, formatting, and diff checks pass.

The interactive CWD breadcrumb status item was then moved into
`labonair-workspace::cwd_status_item`. It owns directory loading, breadcrumb
rendering, subdirectory actions, and its status-menu contribution; shell
composition now requests only its typed registration. Workspace/Shell Clippy
and tests, dependency validation, formatting, and diff checks pass.

The GPUI updater view was moved from `labonair-shell` into the new
`labonair-updater-ui` sibling. The UI-free `labonair-updater` capability stays
independent; the sibling owns the dialog, progress rendering, notification
integration, and updater state view. Shell keeps only a compatibility module
re-export for existing composition consumers, with no updater UI source left
in the shell.

The updater statusbar badge was then moved into the same `labonair-updater-ui`
owner. It now owns the badge rendering, updater-state observation, click action,
and typed `StatusItemRegistration`; shell composition only inserts the owner
registration. The panel dependency is explicit in the updater UI crate and in
the dependency-boundary validator.

The Terminal `ClearTerminal` command was then moved out of the shell execution
table. `labonair-terminal::command_provider` now owns its executable handler
and accepts a narrow `TerminalCommandTarget`; Workspace provides the adapter
for its active-pane operation. This keeps terminal command ownership local
without introducing a terminal-to-workspace dependency or a second dispatch
table.

Settings toggle commands were then moved out of the shell table as well.
`labonair-settings::command_provider` now owns the Zen Mode, chrome, editor,
terminal-cursor, and Vim preference handlers. The shell only registers this
owner contribution; Settings remains the single writer for these values.

The updater `CheckForUpdates` command was then moved into
`labonair-updater-ui::command_provider`. Its metadata and executable handler
are owned by the updater UI sibling and receive the existing `UpdaterView`
entity from bootstrap. The shell command table no longer contains updater
behavior; the updater entity is created before keymap/command composition so
owner handlers are available from the first registry snapshot.

The Settings window entrypoint was then moved into
`labonair-settings-ui::command_provider`. The sibling owns the executable
`OpenSettings` handler, while the metadata remains in the Settings command
provider; shell composition only connects the two owner contributions.

The Hosts management entrypoint was then moved into
`labonair-hosts-ui::command_provider`. Its handler receives the existing
`HostManagerView` entity from bootstrap and opens the canonical Hosts window;
the shell no longer carries that command behavior.

Workspace surface commands were then moved into
`labonair-workspace::command_provider`: sidebar toggling, dock cycling/zoom,
Snippets and Source Control panel focus, and Git Graph tab opening. Their
handlers operate on the Workspace entity directly, so the shell no longer
contains layout/panel orchestration closures or helper methods for them.

The Keymap management entrypoint was then moved into
`labonair-keymap-ui::command_provider`. The owner receives a stable descriptor
snapshot and an injected raw-file callback, owns the Keymap window action, and
does not depend on Workspace or Shell. The shell no longer contains the
`OpenKeymapJson` execution adapter.

The two settings-file editor commands (`OpenProjectSettings` and
`OpenSettingsJson`) were then moved to the Workspace handler contribution as
well. Settings continues to own their command metadata, while Workspace owns
the tab/file lifecycle that executes them; the shell no longer contains these
cross-surface closures.
