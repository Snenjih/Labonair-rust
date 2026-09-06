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
