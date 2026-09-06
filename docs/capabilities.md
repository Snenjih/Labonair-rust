# Capability Matrix

**Status:** Normative ownership map
**Version:** 2
**Related:** [`architecture.md`](architecture.md), [`modules.md`](modules.md), [`registries.md`](registries.md)

This matrix is the authoritative index for product capabilities. A capability
has one owning module, one canonical capability crate, one canonical user
entry point, and one composition boundary. A module may have sibling crates
for real UI, storage, or integration boundaries, but they remain under the
same owner. The `Current implementation` column is deliberately separate from
the target columns so that documentation does not turn an unfinished
migration into an architectural claim.

The application-composition and foundation rows make non-product boundaries
visible for dependency auditing. The one-capability/one-capability-crate rule
applies to product rows; composition and foundation crates have one explicit
coordination or reusable-service responsibility instead.

| Capability | Owning module | Contract / domain crate | UI crate | Storage / integration | Canonical entry point | Current implementation state |
|---|---|---|---|---|---|---|
| Application composition | `application` | `labonair-shell` | `labonair-shell` | `labonair-backend` adapters during migration | App startup | Transitional composition root |
| Workspace orchestration | `workspace` | `labonair-workspace` | `labonair-workspace` | session/layout persistence | Workspace surface | Active, still broad |
| Terminal | `terminal` | `labonair-terminal` | `labonair-terminal` and workspace integration | PTY/process adapter | Terminal tab or standalone terminal | Active, extraction ongoing |
| Editor | `editor` | `labonair-editor` | `labonair-editor` and workspace integration | filesystem adapter | Editor tab or standalone editor | Active, extraction ongoing |
| Filesystem | `filesystem` | `labonair-filesystem` | Consuming feature UI | local filesystem and watcher | Explorer/editor consumers | Extracted |
| SSH | `ssh` | `labonair-ssh` | SSH connection surface | Backend adapter owns russh, auth, tunnels, jump hosts | Host action or standalone SSH | Contract and injected workspace/Hosts paths migrated; backend adapter remains transitional |
| SFTP | `sftp` | `labonair-sftp` | SFTP browser | Backend adapter owns russh-sftp; session comes from SSH | Host action or standalone SFTP | Authenticated session/browser contract and view migration complete; transfer queue remains separate |
| Hosts | `hosts` | `labonair-hosts` | `labonair-hosts-ui` | `labonair-persistence`, credentials, SSH adapters | Titlebar global menu → Hosts; picker in Command Palette | Domain/store extracted; management UI migration open |
| Credentials | `credentials` | `labonair-credentials` | Hosts UI consumer | `labonair-secrets`, keychain | Host management | Extracted |
| Transfers | `transfers` | `labonair-transfers` | `labonair-transfers-ui` | SFTP/SSH worker adapter | Statusbar Transfers badge | Typed registry and statusbar UI extracted; legacy backend event adapter remains transitional |
| Git / source control | `git` | `labonair-git` | `labonair-panel-scm`, `labonair-panel-git-graph`, Project Diff | backend Git executor adapter | Source Control panel / palette | Contracts extracted |
| Explorer | `explorer` | Filesystem contracts | `labonair-panel-explorer` | `labonair-filesystem` | Explorer dock panel | Backend edge removed; workspace shim remains |
| Snippets | `snippets` | `labonair-snippets` | `labonair-panel-snippets` | persistence and injected SSH executor | Snippets panel / palette | Contracts and panel isolated |
| Settings | `settings` | `labonair-settings-content`, `labonair-settings` | `labonair-settings-ui` | JSON/settings persistence | Settings window | Misplaced categories removed; value reduction and UI boundary migration open |
| Keymap | `keymap` | `labonair-keymap` | `labonair-command-palette` temporarily; dedicated UI only when a real boundary exists | keymap file and binding registry | Titlebar global menu → Keymap; quick access through palette | UI-free keymap crate extracted; editing and shell adapter migration ongoing |
| Command Palette | `command-palette` | `labonair-command-palette-core` | `labonair-command-palette` | typed providers from feature modules | Titlebar global menu / global shortcut | UI-free registry crate introduced; static entries and duplicate shell registry still being migrated |
| Notifications | `notifications` | `labonair-notifications-core` | `labonair-notifications` | none; retained in registry | Statusbar notification dropdown | Registry extracted; toast removal done |
| Themes (color and icon) | `themes` | `labonair-theme` | `labonair-theme` unless a real UI boundary requires a sibling crate | built-in definitions first | Titlebar global menu → Themes / Icon Themes → palette submenu | Registries exist; palette integration and preview remain open |
| AI | `ai` | `labonair-ai` | `labonair-ai` until a real UI boundary requires a sibling crate | provider/session persistence | AI panel / workspace context | Backend largely active; UI rework open |

## Rules for changing this matrix

1. Add a row before adding a new product capability.
2. Record exactly one owning module and one canonical capability crate per
   row. Sibling crates must be named in the same row and must not claim
   ownership.
3. A new crate must have one responsibility, a current consumer or independent
   test boundary, and a real dependency, lifecycle, storage, or platform
   reason. Visual primitives belong in `labonair-ui-kit` instead.
4. Record the canonical entry point before implementation. Alternate entry
   points may delegate to it but must not duplicate behavior.
5. Update the owning module's task, this matrix, the inventory, and the
   dependency verifier in the same change as a new internal edge or ownership
   move.
6. Do not delete a row because a feature is temporarily parked. Mark its
   disposition and migration state instead; remove it only when the product
   decision is explicitly `remove` and its data/command migration is complete.
