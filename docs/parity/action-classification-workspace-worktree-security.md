# Workspace Worktree Security Action Classification

**Status:** Static trust and Restricted Mode requirements; Labonair route,
native dispatch, persistence recovery, and paired visual acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Workspace (`labonair-workspace`) owns project/worktree trust
decisions, the canonical Restricted Mode surface, and durable trust policy.
Settings owns the global value only. Editor language services, project
settings, and MCP integrations consume a narrow typed trust decision.

## Evidence boundary

This packet independently states the user-visible security workflow and
current Labonair gap. Pinned source review establishes static action, settings,
modal, and persistence behavior; it does not prove native key dispatch or
rendered interaction. The same contributor session performed analysis and
implementation, so this is independent redevelopment, not separated-room
clean-room work, as recorded in the [parity README](README.md#provenance-and-source-boundary).
The Linux host has no native macOS runtime or display capture.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| Owner boundary | Workspace owns Restricted Mode state, the consent surface, and path-scoped trust. Settings stores the optional global auto-trust value. Editor language services, project settings, and MCP integrations check a typed Workspace trust result before project-controlled configuration or services take effect. Keep the decision separate from SSH host-key verification and MCP per-tab grants. |
| Toggle entry point | Pinned global binding: macOS `Ctrl+Cmd+S`, Linux `Ctrl+Alt+Shift+S`, Windows `Ctrl+Shift+Alt+S`. With restricted paths, it opens the decision. With no restricted paths, it does nothing. While the decision is open, invoking the same action chooses Stay in Restricted Mode and closes it. The titlebar Restricted Mode warning and Agent profile picker also open the decision. Labonair has no project-trust action, mouse entry, default binding, or consent surface. |
| Restricted Mode | Newly opened untrusted project/worktree paths remain restricted by default. The decision lists the affected paths and explains that Restricted Mode prevents project settings from applying, language servers from running, and MCP server integrations from installing. It asks users to review project-provided settings before granting trust. The titlebar shows a Restricted Mode warning control; project Settings shows a warning that some project settings may not apply. |
| Trust this project | Trust and Continue grants trust to the affected worktrees or single-file roots. It does not implicitly trust their parents unless the user selects the optional parent-scope control. For providers using this mechanism, trust applies to the matching local path or remote host identity and must not transfer between unrelated hosts. Provider coverage is not uniform: verify remote types that own a separate trust policy. |
| Optional parent scope | The checkbox starts unchecked. For one directory worktree, the dialog shows an editable folder path, prefilled with that project's parent and read-only until checked. The entered path must be nonempty, absolute for the host path style, and an ancestor of or equal to the project. A leading `~` expands against that host's home directory. Invalid input shows an inline error and leaves the project restricted. |
| Multiple paths and files | For several directory worktrees, the checkbox offers the available parent-folder scope without an editable field. For single-file roots, it offers “Trust all single files.” Verify mixed file/directory lists, missing parent paths, and remote paths rather than assuming the single-directory editor applies to every case. |
| Stay and dismissal | Stay in Restricted Mode leaves the listed paths restricted. Escape and clicking outside cannot dismiss the decision before a button choice. Invalid scope also keeps it open. Verify focus entry, tab order, scroll behavior, and the active action binding while the dialog has focus. |
| Agent and Git effects | Agent excludes untrusted worktree skills, hides tools not allowed in Restricted Mode, and rechecks those tools before execution if trust changes mid-thread. The unmodified built-in write/ask profiles may be downgraded to Minimal; custom or modified profiles are not rewritten. The Agent profile selector flags profiles with restricted tools. Git repository workers receive trust state, and a linked worktree may inherit trust only from a fully trusted source workspace. Verify each visible effect and refresh/revoke behavior. |
| Global auto-trust setting | The General Settings Security control and onboarding switch are labeled “Trust All Projects By Default.” Its declared default is false; enabling it skips individual Restricted Mode consent for new projects and permits project settings, language services, and MCP integrations. The Settings page declares `session.trust_all_projects`, while its backing value is named `trust_all_worktrees`; the effective stored path and live behavior require a saved-value runtime trace. |
| Remote coverage | Local, SSH, and WSL projects use the Restricted Mode trust flow. Docker and collaboration remotes have separate provider-managed trust behavior. Verify the user-visible safety gate for each remote type; do not silently treat SSH host verification, collaboration membership, or a container name as project trust. |
| Persistence and reset | Explicit trust decisions persist and are scoped to local paths or remote host identity. Verify trust after closing/reopening the app, changes to project paths, remote host changes, global auto-trust changes, and reset. The distinct unbound `ClearTrustedWorktrees` action removes saved trust: newly opened projects return to Restricted Mode, and already-open projects require restart for the clearing to take effect. |
| Events and registry contributions | Keep action dispatch and consent UI in Workspace. Expose only a typed trust query/event to consumers; do not duplicate trust maps in Editor, Agent, Git, MCP, Settings, or shell composition. The action contributes the default keybinding above; `ClearTrustedWorktrees` has no default keybinding. |
| Failure and recovery | Static evidence does not establish what users see after a failed trust write or whether a failed decision is retried. Before implementation, define and verify whether active-session consent remains effective, how persistence failure is reported through the notification center, how retry works, and that a failed write never appears durable after restart. |
| UI-kit | Use the shared modal, warning, path list, checkbox, editable input, buttons, focus, validation, and scrolling controls. Reuse the notification center for persistence failures; do not add a shell-local dialog or toast. |

## Action requirements and current route

| Requirement ID | Pinned bindings and contexts | Pinned outcome | Static Labonair route and gap |
|---|---|---|---|
| [`ACT-MACOS-workspace-ToggleWorktreeSecurity`](action-requirements.md) | macOS `Ctrl+Cmd+S`, Linux `Ctrl+Alt+Shift+S`, Windows `Ctrl+Shift+Alt+S`, global; titlebar and Agent profile-picker warning controls also open the decision. | Open the Restricted Mode consent decision for restricted paths; no-op when none exist. The same action while the modal is open chooses Stay in Restricted Mode. | No Workspace trust action, mouse route, dialog, default binding, durable trust store, or project-policy enforcement. Editor's untrusted-root policy type has no Workspace runtime caller. |
| [`ACT-REGISTRY-LINUX-workspace-ClearTrustedWorktrees`](registered-actions-unbound.md) | Registered in the pinned Linux action registry; absent from all three default keymaps. | Clear saved trusted-worktree records. Newly opened projects become restricted; already-open projects require restart. | Separate unbound action with no Labonair trust state or invocation route. Its pinned command-palette/menu reachability still needs runtime inspection. |

The global setting is already inventoried as
[`SET-general-session-trust-all-projects`](settings-requirements.md).
Its visible path/backing-field mismatch must be resolved together with the
trust action, not by adding a second setting row.

## Existing Labonair route and evidence

| Current evidence | Requirement relationship | Coverage limit |
|---|---|---|
| [Workspace capability record](../capabilities.md) and [security trust boundaries](../security/trust-boundaries.md) | Establish Workspace and security owner boundaries. | The normative trust boundary is recorded; Labonair still has no project/worktree Restricted Mode contract, durable trust policy, consumer wiring, or user decision flow. |
| `crates/editor/src/lsp_process.rs` | Defines `ProjectRootPolicy::Untrusted` and rejects process startup under that policy. | Repository search finds no Workspace runtime caller; it does not implement project trust, persistence, dialog, keybinding, or consumer wiring. |
| [SSH host verification](../security/remote-access.md) | Provides host-key trust for SSH connections. | SSH server identity is a separate decision and cannot grant trust to project files. |
| Agent and Git trust checks | Pinned Agent skills/tools/profile and Git repository flows react to worktree trust state. | Labonair has no connected Workspace trust state, per-agent restricted-tool policy, or Git worker trust route. |
| [MCP grants](../automation/grants.md) | Provides per-tab MCP access grants. | MCP client access does not decide whether project configuration and integrations are trusted. |
| [Trust boundary policy](../security/trust-boundaries.md) | Requires typed ownership and explicit security boundaries. | The target owner and typed consumers are now recorded; Labonair still needs the runtime policy and integrations. |

## Required user-flow and state acceptance

- Open projects containing one restricted directory, several directories, one
  single-file root, several files, and mixed roots. Confirm when the action
  opens, lists the affected paths, or is a no-op.
- Choose Stay and Trust. Verify language services, project settings, and MCP
  installation remain blocked or become available only according to the
  explicit decision.
- Open the decision from the titlebar warning button, the Agent profile
  selector, and each platform binding. Verify the warning text, active project,
  focus return, and status after trust changes.
- Test the unchecked parent option, the single-directory prefilled field,
  read-only/editable transitions, tilde expansion, host-specific path syntax,
  ancestor/equal paths, relative paths, empty input, and a path below the
  project. Confirm invalid paths leave the project restricted.
- Attempt Escape, outside click, Tab/Shift+Tab, the global binding, and
  Confirm while the modal and path field have focus. Verify action precedence,
  focus return, and that no implicit choice grants trust.
- Trust a local project and an SSH/WSL remote project. Restart, reopen the
  same path, open a different path on the same host, and open the same path
  through a different host identity. Inspect the distinct Docker and
  collaboration trust behavior without inheriting unrelated host approval.
- Enable and disable Trust All Projects By Default in Settings and onboarding.
  Verify defaults, immediate effects, restart behavior, saved JSON key, and
  behavior for projects already open when the value changes.
- In a restricted Agent thread, inspect the default and custom profile choices,
  visible tool list, project-local skills, and restricted-tool explanations.
  Change trust while a tool call is pending and verify execution is rechecked.
  Confirm Git worker behavior and whether a linked worktree inherits trust
  from trusted and restricted source workspaces.
- Invoke Clear Trusted Worktrees through every pinned discovery route found
  at runtime. Verify persistent records are cleared, newly opened projects
  are restricted, open projects require the documented restart, and
  `session.trust_all_projects` still has its independent effect.
- Inject trust-store write failure and restart. Verify user notification,
  retry/recovery, active-session state, durable state, and fail-closed
  behavior. Do not treat an in-memory choice as proof of persistence.
- Compare paired macOS captures for warning copy, path identity, all checkbox
  variants, validation, blocked dismissal, focus, scroll, narrow windows, and
  the Trust/Stay controls. Include the default and global-auto-trust flows.

## Acceptance evidence still required

Static source inspection establishes intent and the current owner gap only.
For each trace, record app and reference commits, platform, window bounds,
theme, scale, project roots and host identity, active bindings, modal state,
settings before and after, consumer behavior, persistence result, restart
result, notifications, and recovery. Native macOS interaction and paired
captures remain pending. The displayed Settings JSON path versus backing
field, unbound reset-action discoverability, and persistence-failure behavior
are explicit open findings; none may be marked complete from code inspection
alone.
