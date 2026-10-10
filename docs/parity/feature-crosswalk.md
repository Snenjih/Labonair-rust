# Zed Parity Feature Crosswalk

**Status:** Draft coverage map for R07-000
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`
**Owner:** Product architecture with capability owners

This is an independent, user-facing inventory outline. It contains no Zed
source paths or implementation instructions. Rows name user workflows and
the current or proposed Labonair owner. `Partial` and `Missing` describe
current Labonair evidence; they are not accepted divergences. Runtime coverage,
actions, settings, failure states, and acceptance steps still need to be
expanded before R07-000 can exit.

| Area ID | User-visible capability and workflows to inventory | Labonair owner / entry point | Current parity status | Next specification work |
|---|---|---|---|---|
| `app.windows` | Launch, open a project, create windows, manage worktrees, restore a session, and recover from missing or moved folders | Application composition + Workspace / project picker and workspace | Partial | Record startup modes, window lifecycle, project/worktree identity, and restore behavior |
| `workspace.tabs` | Open, select, pin/preview, reorder, move, split, merge, close, and restore every view kind | Workspace / universal tab strip and registered views | Partial; universal Labonair tab model is an explicit design choice | Specify lifecycle, dirty-close, focus, movement, persistence, and failure behavior |
| `workspace.docks` | Open and move panels among left, right, and bottom docks; resize and restore dock state | Workspace + panel owners / dock controls | Partial | Map each panel, per-dock action, keyboard route, and narrow-window state |
| `workspace.shared-controls` | Use menus, pickers, dialogs, context menus, tooltips, buttons, inputs, and focus traversal consistently | UI kit + owning surface / local trigger | Partial | Measure geometry and input states from paired macOS evidence |
| `commands.keymap` | Discover actions, search, bind keys, resolve conflicts, use contexts, and navigate multi-stroke bindings | Command Palette + Keymap / global menu, palette, keymap tab | Partial | Crosswalk every action and binding; record context and availability rules |
| `settings.values` | Search, navigate, scope, edit, validate, reset, persist, and restore settings | Settings / Settings window | Partial | Finish the pinned field/control crosswalk and paired Settings pilot |
| `editor.text` | Open/edit/save files; multi-selection; undo/redo; search/replace; navigation; diagnostics; completions; formatting; and recovery | Editor / Editor tab | Partial | Reconcile all editor workflows with ordered R09 contracts after the inventory |
| `editor.multibuffer` | Review and edit excerpts returned by search, navigation, or diagnostics | Editor / search or navigation result | Missing or unverified | Assign the durable view owner and record selection/edit/save semantics |
| `language.services` | Detect languages, install/configure language support, run language servers, and handle diagnostics, symbols, formatting, and failures | Editor + planned Extensions owner / Editor and language setup | Partial | Inventory language and toolchain coverage, extension lifecycle, offline and permission behavior |
| `project.explorer` | Browse files, search, filter, preview, create/rename/move/delete, reveal the active file, and inspect project changes | Explorer + Filesystem / Explorer dock | Partial | Crosswalk all actions, row states, large-project behavior, and remote roots |
| `snippets` | Find, insert, edit, and execute reusable snippets with variables and language context | Snippets / Snippets dock and palette | Partial | Inventory snippet discovery, expansion, execution, storage, and error recovery |
| `terminal.tasks` | Create local/remote terminals, run tasks, resume sessions, resize, search output, and handle shell integration | Terminal + planned Tasks owner / Terminal tab and task picker | Partial | Separate terminal, task, and shell-integration workflows and record cancellation/retry |
| `debugger` | Configure debug targets; start/stop sessions; set breakpoints; inspect variables, stack, watches, and console output | Planned Debugger owner / debug action and debugger panel | Missing | Define owner, protocols, extension boundary, settings, lifecycle, and failure behavior |
| `repl` | Start language REPLs, send selections, inspect output, and recover from kernel/process failure | Planned REPL owner / language action or REPL view | Missing | Determine the pinned workflows and assign an owner and persistence policy |
| `git.source-control` | Inspect status/history, stage/unstage, commit, branch, stash, resolve conflicts, and open changes | Git / Source Control panel and palette | Partial | Map each action, confirmation, repository state, and error recovery |
| `git.graph` | Browse commit history and refs, select commits, and open related changes | Git / Git Graph tab | Partial | Record graph navigation, loading/empty/error behavior, and scalability |
| `diff.review` | Compare revisions, navigate hunks, review changes, and move between producers | Git + planned shared DiffView owner / Project Diff | Partial | Accept one typed diff contract and specify stable inputs, navigation, and tab lifecycle |
| `file.previews` | Inspect images, rendered documents, structured data, and other supported file formats | Planned Preview owner, with Editor integration / file open action | Missing or unverified | Inventory each preview kind, read-only/edit transition, and unsupported-file path |
| `appearance.themes` | Choose, preview, confirm, and restore color themes, icon themes, and typography | Themes / theme commands and global menu | Partial | Crosswalk built-in and extension-provided catalogs, persistence, and asset review |
| `extensions` | Discover, install, update, disable, remove, and troubleshoot language, debugger, theme, icon, snippet, and agent extensions | Planned Extensions owner / extension browser and owner setup flows | Missing | Define trust, permission, distribution, versioning, and offline contracts |
| `ai.agents` | Configure providers/models, start threads, add context, use tools, grant/deny actions, and recover from failures | AI + MCP / AI surface | Partial | Inventory all thread, profile, tool, sandbox, skill, external-agent, and privacy workflows |
| `collaboration` | Authenticate, join channels/calls, share a project, invite participants, and leave or recover sessions | Planned Collaboration owner / collaboration entry point | Missing | Establish account boundary, grants, participant lifecycle, and offline behavior |
| `remote.development` | Connect to remote projects, use environment variables and containers, run tools remotely, and recover transport failures | SSH + SFTP + planned Remote Development owner / host and project pickers | Partial | Separate SSH/SFTP from remote workspace/container lifecycle; specify secrets and cancellation |
| `ssh.transport` | Configure and use SSH authentication, host verification, tunnels, keep-alives, and jump hosts | SSH + Credentials / host action and connection flow | Partial | Map each authentication and trust decision, secret lifetime, timeout, and denial path |
| `sftp.browser` | Browse remote directories, filter and sort entries, edit/open files, and handle remote filesystem failures | SFTP / SFTP browser tab | Partial | Crosswalk navigation, hidden files, file operations, conflict handling, and remote-edit lifecycle |
| `hosts.connections` | Save, edit, group, select, authenticate to, and recover saved remote connections | Hosts + SSH / Hosts management and connection picker | Partial | Crosswalk host records, authentication choices, jump hosts, trust prompts, and picker actions |
| `transfers` | Queue, inspect, pause, resume, cancel, retry, and resolve file-transfer conflicts | Transfers / statusbar item and transfer history | Partial | Map lifecycle, progress, notifications, persistence, and safe retry behavior |
| `notifications` | Review, expand, mark read, dismiss, and recover operation messages | Notifications / statusbar dropdown | Partial | Crosswalk grouping, actions, retention, deduplication, and long-history behavior |
| `backgrounds` | Choose and display workspace backgrounds with opacity/tint and live preview | Background owner editor linked from Appearance | Partial | Compare visible controls, preview, persistence, and rendering impact |
| `accounts.hosted` | Sign in, manage plans, use hosted models, handle billing, and respect organization roles/policies | Planned Accounts/Hosted Services owner / account and provider entry points | Missing | Decide product support, service contracts, data use, account recovery, and legal review |
| `privacy.security` | Configure trust, telemetry, secret handling, tool grants, and organization security policies | Security-sensitive owning modules / corresponding setup surfaces | Partial | Inventory allow/deny behavior, retention, network boundaries, and auditability |
| `platform.integration` | Use platform-specific window, file, keyboard, input-method, update, and accessibility behavior | Platform adapters + owning capabilities / native application | Partial; macOS is first acceptance platform | Lock release matrix and test each platform after macOS baseline |
| `cli.updates` | Open paths/projects from the CLI, update the app, inspect release notes, and recover from update failure | Application CLI + Updater / CLI and update entry points | Partial | Crosswalk exact flags, handoff, signatures, rollback, and platform behavior |
| `application.updater` | Check, download, install, defer, and recover application updates | Updater / update command and update surface | Partial | Record channel selection, signature checks, restart, rollback, and offline behavior |

## Acceptance status

The table is a starting classification, not the exhaustive catalog required by
R07-000. No row is an approved divergence. Every row still needs exact pinned
runtime evidence, command/action references, setting IDs, persistence,
extension points, failure behavior, and testable acceptance steps. Current
public documentation and the pinned runtime must be cross-checked separately.

## Current Labonair capability reconciliation

The following current capability rows are also represented above. Foundation
and composition crates are not product capabilities and are excluded from
this user-workflow map.

| Capability matrix row | Crosswalk area |
|---|---|
| Application composition | `app.windows`, `cli.updates`, `platform.integration` |
| Workspace orchestration | `app.windows`, `workspace.tabs`, `workspace.docks` |
| Backgrounds | `backgrounds` |
| Terminal | `terminal.tasks` |
| Editor | `editor.text`, `editor.multibuffer`, `language.services`, `diff.review` |
| Filesystem | `project.explorer`, `file.previews` |
| SSH | `ssh.transport`, `hosts.connections`, `remote.development` |
| SFTP | `sftp.browser`, `remote.development` |
| Hosts | `hosts.connections` |
| Credentials | `ssh.transport`, `remote.development`, `privacy.security` |
| Transfers | `transfers` |
| Git / source control | `git.source-control`, `git.graph`, `diff.review` |
| Explorer | `project.explorer` |
| Snippets | `snippets` |
| Settings | `settings.values` |
| Keymap | `commands.keymap` |
| Command Palette | `commands.keymap` |
| Notifications | `notifications` |
| Themes (color and icon) | `appearance.themes`, `extensions` |
| Updates | `application.updater`, `cli.updates`, `platform.integration` |
| AI | `ai.agents`, `extensions`, `privacy.security` |
