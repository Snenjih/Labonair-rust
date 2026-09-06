# Workspace Model

**Status:** Normative

## Workspace types

Labonair supports two equally valid entry modes:

1. **Project workspace** — tied to a local folder, remote folder, repository, or other durable work context.
2. **Standalone workspace** — a temporary context for a terminal, editor, SSH, SFTP, or other single action.

Both use the same tabs, panes, commands, notifications, and statusbar. The difference is identity and persistence, not a separate UI architecture.

The current implementation represents this contract in
`labonair-workspace::context` as `WorkspaceIdentity` plus `WorkspaceState`.
`Workspace` owns the state and exposes one snapshot for shell surfaces. It
starts as `Standalone`; a future project-opening flow changes the identity
explicitly through the workspace API rather than inferring a project from a
terminal's current working directory.

A workspace is an application context, not a requirement that every action be
project-based. The context carries optional identity and shared navigation
state; the tool module still owns the terminal, editor, SSH, SFTP, or transfer
state inside it.

## Workspace contents

A workspace contains:

- optional project or remote identity;
- tabs;
- recursive split panes;
- active tool instances;
- layout state;
- scoped settings overrides where supported;
- session persistence metadata.

No tool may require a project workspace unless its operation genuinely needs a project root.

## Entry and lifetime rules

- Opening a project or remote folder creates a project workspace with a stable
  identity and eligible session persistence.
- Opening a terminal, editor, SSH connection, SFTP browser, or one-off
  transfer without a project creates a standalone workspace containing only
  the requested tool context.
- Standalone and project workspaces use the same command, tab, pane, dock,
  statusbar, and notification contracts. The shell does not maintain a second
  standalone implementation.
- A standalone workspace is temporary by default. It is not silently turned
  into a project or written into project settings; persistence is explicit and
  belongs to the workspace module.

Host selection and host management remain owned by Hosts. A selected host may
create a standalone remote workspace or attach to an existing workspace
through the SSH/SFTP contracts; Workspace only places and focuses the
  resulting tool instance.

Workspace-to-shell navigation uses `WorkspaceEvent` for requests such as
opening Hosts. Workspace does not store a shell callback or decide whether the
destination is a palette page, panel, or standalone window.

## Tool instances

Tabs and panels reference tool instances through stable IDs and typed capabilities. The workspace orchestrates placement and focus; the owning tool module controls its state and operations.

Workspace must not inspect feature-private state to decide how a tool works.
It receives typed capabilities and lifecycle events, and delegates commands to
the owning module. Layout and focus are the only cross-tool responsibilities
that belong here.

## Empty workspace

An empty workspace is valid. The shell must not create a fallback terminal merely to fill space. The empty state offers only a small set of discoverable actions, such as opening a terminal or invoking the command palette.

## Remote contexts

An SSH connection may provide a workspace root and terminal sessions. An SFTP connection may provide a remote filesystem context. Shared concepts should have common identifiers, while transport-specific behavior remains owned by SSH or SFTP.

Local and remote tools should follow the same workspace flow when their
capabilities are equivalent. Transport differences are expressed by typed
contracts, not by shell-wide conditionals or duplicated workspace views.
