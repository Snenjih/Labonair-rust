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
