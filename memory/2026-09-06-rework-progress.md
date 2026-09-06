# Rework progress — 2026-09-06

## Workspace root ownership

Project-versus-standalone root precedence is a workspace concern, not shell
behavior. `labonair-workspace` exposes `Workspace::filesystem_root` for file
surfaces and `Workspace::git_root` for repository surfaces. The pure resolver
rules are tested beside `WorkspaceContext`; the shell only wires those
contracts into Explorer and Git.

This keeps composition roots responsible for construction and subscriptions,
while the owning capability retains the identity semantics that must remain
consistent across every consumer.

## Project identity persistence

`SessionSnapshot` includes the explicit `WorkspaceIdentity`. The field is
optional on input for backward compatibility, and old snapshots become
Standalone rather than inferring a project from a terminal directory. A
restored project identity is applied before tab replay so project-scoped
settings and surface-root observers start from the same context. Returning to
Standalone is an explicit command that leaves tabs, panes, and shell layout
intact.

## Native visual verification boundary

The exact Rust bundle passes the five-second LaunchServices smoke check and
the process is resolved by its absolute executable path. A later screenshot
attempt can still fail independently with macOS `could not create image from
window`; that result must remain an unaccepted visual check rather than being
confused with launching the legacy Tauri application.
