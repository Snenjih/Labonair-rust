# Labonair Product Contract

**Status:** Normative
**Version:** 2
**Scope:** Product identity and user-facing behavior

## Product identity

Labonair is a fast, keyboard-first Dev-Op workspace for local and remote work. It combines terminal, SSH, SFTP, editor, Git, and AI capabilities without forcing the product into the shape of a terminal emulator or a traditional IDE.

The primary unit is a workspace. A workspace may represent a project, a remote environment, or a temporary collection of tools. Every tool must also support standalone use when no project context exists.

## Product goals

1. Make local and remote development tasks fast to start and easy to finish.
2. Give terminal and remote workflows first-class status, not secondary treatment.
3. Keep the default interface quiet and intentional while making power features discoverable through keyboard navigation.
4. Allow features to evolve independently without inconsistent UI or cross-module coupling.
5. Make the application feel coherent: one interaction model, one visual language, one location for each responsibility.

## Non-goals

- Labonair is not a Zed fork and does not copy Zed source code.
- Labonair is not required to reproduce every Zed feature.
- Labonair does not initially include a marketplace, remote theme downloads, or extension hosting.
- Settings are not a general management surface for hosts, themes, keymaps, transfers, or notifications.
- The application must not preserve a feature merely because it existed in the Tauri predecessor.

## Core principles

### Simple by default

Only the active task should occupy permanent UI. Secondary actions use progressive disclosure through the command palette, contextual menus, or focused dialogs.

### Keyboard-first, mouse-complete

Every meaningful action has a command identity and can receive a keybinding. Mouse interaction remains complete, but no important workflow may depend on mouse-only navigation.

Every visible action that can be invoked by a user must expose a stable command
ID. Frequent navigation and workflow actions should ship with a default
binding; rare or destructive actions must remain reachable through the command
palette even when they have no default binding.

### One responsibility, one home

Every feature has one canonical owner and one primary entry point. Alternate entry points are allowed only when they improve discoverability without duplicating business logic.

### Local and remote parity

Local and remote workflows should use the same workspace concepts wherever their capabilities are equivalent. Differences belong in the owning module, not in shell-wide special cases.

### Progressive replacement

Existing implementation may be reused only when it satisfies this contract. Compatibility code is temporary and must have an explicit removal plan.

## Product surfaces

The permanent application chrome has four zones and one overlay layer:

1. **Titlebar** — tabs and one global menu button.
2. **Workspace** — active tab content and split panes.
3. **Docks** — registered panels attached to left, right, or bottom docks.
4. **Statusbar** — panel controls on the left; global status and information items on the right.
5. **Overlay layer** — command palette, dialogs, focused pickers, and other explicitly modal surfaces.

No feature may add a new permanent global strip, badge row, or shell toolbar without an ADR explaining why the existing surfaces cannot host it.

## Product decisions

- Themes and icon themes are selected through command-palette submenus with live preview.
- Hosts are selected through a command-palette submenu; `Enter` opens SSH and `Shift+Enter` opens SFTP.
- Host management is a dedicated management surface, not a settings category.
- Notifications are persistent entries in the statusbar notification dropdown. There is no toast system.
- Transfers are represented by a statusbar item with progress and history.
- Jump hosts are part of connection configuration and connection execution, not a standalone primary menu.
- AI frontend work is paused until the new workspace and service contracts are stable.
