# Task Definition Entry Action Classification

**Status:** Static file-target and owner requirements; Task/Debugger ownership,
native dispatch, and paired acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** The planned Tasks capability owns task definitions; the planned
Debugger capability owns debug scenarios. Workspace hosts their documents as
Editor items. These owners and canonical crates are not present in Labonair.

## Evidence boundary

This packet independently states the global, project, and worktree setup
configuration workflows and current Labonair routes. Static action metadata,
keymaps, menu entries, and source establish the file target and scope; they do
not establish native dispatch, runtime task execution, debugger launch, or
visual acceptance. Work in this session is independent redevelopment, not
separated-room clean-room implementation, as recorded in the [parity
README](README.md#provenance-and-source-boundary). Paired macOS Run-menu and
Worktree Picker evidence remains pending.

## Owner and workflow contract

| Contract field | Requirement and current Labonair record |
|---|---|
| User-global task definitions | `OpenTasks` opens the user's `tasks.json` in a Workspace Editor item and initializes it when absent. The planned Tasks owner owns task definitions; Workspace owns document presentation. Labonair has no Task owner or matching user-file route. |
| User-global debug scenarios | `OpenDebugTasks` opens the user's `debug.json` in a Workspace Editor item and initializes it when absent. The planned Debugger owner owns debug scenarios; Workspace hosts the document. Labonair has no Debugger owner or matching user-file route. |
| Project task definitions | Run > Edit tasks.json opens or creates `.zed/tasks.json` in the first visible project worktree with a directory root. Labonair has no Task owner, project task document route, or matching Run menu. |
| Project debug scenarios | Run > Edit debug.json opens or creates `.zed/debug.json` in the first visible project worktree with a directory root. The planned Debugger owner owns its schema and behavior. Labonair has no Debugger owner or matching project route. |
| Worktree setup task | In WorktreePicker contexts, `OpenWorktreeSetupTasks` opens the project `.zed/tasks.json` and inserts a harmless create-worktree example only if the text contains no existing `create_worktree` hook (including its accepted alias). It reports when no project folders are open. Labonair has no equivalent Worktree Picker or hook editor route. |
| Owner and crate boundary | Task and Debugger are planned capability owners in the feature crosswalk; neither has a Labonair capability crate. Workspace is only the file host. Define each capability contract and canonical crate before implementation; do not place task execution in Settings or add a generic configuration facade. |
| Events and registries | Opening these documents needs no new event or registry. Task discovery/execution and debugger launch require separate owner and consumer-count analysis. |
| State and persistence | User-global definitions persist in user config files. Project task and debug definitions persist in their respective `.zed` files. Worktree setup may seed a missing hook example in project task JSON. Runtime task and debugger session state are outside this entry packet. |
| Settings | No Settings value is added. Global task/debug files and project task/debug files are configuration documents, not values in the Settings schema. |
| Notifications and recovery | Project-scoped opens without a visible directory worktree show an explanatory notification. File creation/open errors use Workspace handling; malformed task/debug data remains for the owning capability to validate. |
| UI kit | Task/debug JSON files use the Workspace Editor. The Run menu and Worktree Picker use their owning surfaces and shared controls. |
| Current Labonair route | No Task or Debugger action, user/project task-config opener, project `.zed` task/debug file route, or worktree setup hook was found. Terminal creation and generic Workspace JSON editing do not provide these workflows. |

## Action and related configuration requirements

| Requirement ID | Pinned entry and target | Current Labonair route and gap |
|---|---|---|
| [`ACT-MACOS-zed-OpenWorktreeSetupTasks`](action-requirements.md) | `cmd-shift-c` in `WorktreePicker` or `WorktreePicker > Picker > Editor` on macOS; `ctrl-shift-c` on Linux; `ctrl-shift-,` on Windows. Open project task JSON and conditionally seed the create-worktree example. | No Worktree Picker, Task owner, hook editor route, or matching default binding. |
| [`ACT-REGISTRY-LINUX-zed-OpenTasks`](registered-actions-unbound.md) | Registered `zed: open tasks` action. Open or initialize the user's task definition JSON. | No user-global task file action or Task owner. |
| [`ACT-REGISTRY-LINUX-zed-OpenDebugTasks`](registered-actions-unbound.md) | Registered `zed: open debug tasks` action. Open or initialize the user's debug scenario JSON. | No user-global debug file action or Debugger owner. |
| [`ACT-REGISTRY-LINUX-zed-OpenProjectTasks`](registered-actions-unbound.md) | Run > Edit tasks.json. Open or create the project `.zed/tasks.json` file. | No project task file route or matching Run menu. |
| [`ACT-REGISTRY-LINUX-debugger-OpenProjectDebugTasks`](registered-actions-unbound.md) | Run > Edit debug.json. Open or create the project `.zed/debug.json` file. | No project debug file route or matching Run menu. |

## Required user-flow and state acceptance

- Open user-global tasks and debug JSON from their registered actions. Verify
  absent files receive their initial contents, existing edits remain intact,
  and each document opens in the Workspace Editor.
- With a project open, open task and debug definitions from the Run menu.
  Verify the first visible directory worktree is selected, missing `.zed`
  paths/files are created, and existing files are preserved.
- From each WorktreePicker context, open Worktree Setup Tasks. Verify a single
  safe example is inserted when no hook exists, an existing `create_worktree`
  or accepted alias suppresses duplicate insertion, and the selected project's
  task document remains the target.
- Invoke project actions with no open folder. Verify the explanatory
  notification and recovery path; verify file and parse errors retain their
  Task/Debugger or Workspace owner.
- Capture paired macOS Run-menu and Worktree Picker interactions, global and
  project JSON opening, seeded and existing-hook states, and the no-project
  notice. Record OS, active project/worktree, target path, and focus. Verify
  non-macOS Worktree Picker bindings separately.

## Acceptance evidence still required

Static metadata and source review establish target paths, project scope,
initial-file behavior, and the worktree hook guard. They do not establish key
dispatch, menu availability by project state, native Picker behavior, task
execution, debug launch, error visibility, or paired visual acceptance. The
Task/Debugger owner contracts and all runtime routes remain open.
