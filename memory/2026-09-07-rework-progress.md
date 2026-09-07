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
