# Rework progress — 2026-09-07

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
