# Rework progress — 2026-09-07

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
