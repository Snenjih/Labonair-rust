# Capability Matrix

**Status:** Normative ownership map
**Version:** 1
**Related:** [`architecture.md`](architecture.md), [`modules.md`](modules.md), [`registries.md`](registries.md)

This matrix is the authoritative index for product capabilities. A capability
has one owning module, one canonical user entry point, and one composition
boundary. The `Current implementation` column is deliberately separate from
the target columns so that documentation does not turn an unfinished
migration into an architectural claim.

| Capability | Owning module | Contract / domain crate | UI crate | Storage / integration | Canonical entry point | Current implementation state |
|---|---|---|---|---|---|---|
| Application composition | `application` | `labonair-shell` | `labonair-shell` | `labonair-backend` adapters during migration | App startup | Transitional composition root |
| Workspace orchestration | `workspace` | `labonair-panel`, workspace types | `labonair-workspace` | session/layout persistence | Workspace surface | Active, still broad |
| Terminal | `terminal` | terminal/session contracts | `labonair-terminal` and workspace view | PTY/process adapter | Terminal tab or standalone terminal | Active, extraction ongoing |
| Editor | `editor` | editor document contracts | `labonair-editor` and workspace view | filesystem adapter | Editor tab or standalone editor | Active, extraction ongoing |
| Filesystem | `filesystem` | `labonair-filesystem` | Consuming feature UI | local filesystem and watcher | Explorer/editor consumers | Extracted |
| SSH | `ssh` | Target: `labonair-ssh` | SSH connection surface | russh, auth, tunnels, jump hosts | Host action or standalone SSH | Backend-owned adapter |
| SFTP | `sftp` | Target: `labonair-sftp` | SFTP browser | russh-sftp | Host action or standalone SFTP | Workspace/backend-owned |
| Hosts | `hosts` | `labonair-hosts` | `labonair-hosts-ui` | `labonair-persistence`, credentials, SSH adapters | Command Palette → Hosts | Domain/store extracted; UI migration open |
| Credentials | `credentials` | `labonair-credentials` | Hosts UI consumer | `labonair-secrets`, keychain | Host management | Extracted |
| Transfers | `transfers` | Target: `labonair-transfers-core` | Target: `labonair-transfers-ui` | SFTP/SSH worker adapter | Statusbar Transfers badge | Workspace/backend-owned; extraction next |
| Git / source control | `git` | `labonair-git` | `labonair-panel-scm`, `labonair-panel-git-graph`, Project Diff | backend Git executor adapter | Source Control panel / palette | Contracts extracted |
| Explorer | `explorer` | Filesystem contracts | `labonair-panel-explorer` | `labonair-filesystem` | Explorer dock panel | Backend edge removed; workspace shim remains |
| Snippets | `snippets` | `labonair-snippets` | `labonair-panel-snippets` | persistence and injected SSH executor | Snippets panel / palette | Contracts and panel isolated |
| Settings | `settings` | `labonair-settings-content`, `labonair-settings` | `labonair-settings-ui` | JSON/settings persistence | Settings window | Rework and category removal open |
| Keymap | `keymap` | Target: `labonair-keymap-core` | Target: `keymap-ui` | keymap file and binding registry | Settings → Keymap / palette | Partly in command-palette and shell |
| Command Palette | `command-palette` | Target: `labonair-command-palette-core` | `labonair-command-palette` | providers from feature modules | Titlebar menu / global shortcut | Registry migration ongoing |
| Notifications | `notifications` | `labonair-notifications-core` | `labonair-notifications` | none; retained in registry | Statusbar notification dropdown | Registry extracted; toast removal done |
| Color themes | `themes` | `labonair-theme` | Target: `themes-ui` | built-in definitions first | Palette → Themes | Registry exists; palette integration open |
| Icon themes | `themes` | `labonair-theme` | Target: `themes-ui` | built-in definitions first | Palette → Icon Themes | Registry exists; palette integration open |
| AI | `ai` | `labonair-ai` | Target: `ai-ui` | provider/session persistence | AI panel / workspace context | Backend largely active; UI rework open |

## Rules for changing this matrix

1. Add a row before adding a new product capability.
2. Never assign two owning modules to one capability.
3. A new contract or UI crate must be justified by a real dependency,
   lifecycle, storage, or platform boundary; visual primitives belong in
   `labonair-ui-kit` instead.
4. Update the owning module's task and the dependency verifier in the same
   change as a new internal edge.
5. Do not delete a row because a feature is temporarily parked. Mark its
   canonical entry point and migration state instead.
