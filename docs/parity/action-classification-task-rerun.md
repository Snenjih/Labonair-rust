# Task Rerun Action Classification

**Status:** Pinned Workspace/Terminal rerun flow and Labonair owner gap classified; native task execution and context acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Tasks UI and Workspace own task lookup/scheduling; TerminalView routes its task identity through `task::Rerun`. Labonair has no canonical Tasks module or task-backed Terminal state, so task execution remains an explicit planned capability.

## Evidence boundary

Two bound actions participate in rerun. `task::Rerun` is in Workspace (macOS excludes Terminal explicitly); `terminal::RerunTask` is in Terminal. They share `cmd-alt-r` on macOS and selected `ctrl-shift-r`, `ctrl-alt-r`, or `alt-t` bindings on Linux/Windows, so context precedence is part of acceptance. The pinned JetBrains profiles add `cmd-f5` (`Workspace`) on macOS and `ctrl-f5` (`Workspace`) on Linux/Windows for `task::Rerun`; no Terminal RerunTask profile entry was found. Static contexts and layers do not prove which action receives a key at runtime.

## Pinned rerun contract

`task::Rerun` resolves the last scheduled task when no `task_id` is supplied, or a specific task when the internal `task_id` override is present. With the default `reevaluate_context=false`, it reruns the previous resolved task context; `true` rebuilds context from the current Workspace/editor state. Optional arguments override `allow_concurrent_runs` and `use_new_terminal`. If the task inventory has no rerunnable task, the handler opens the task picker.

`terminal::RerunTask` reads the active terminal's `TaskState`. If present, it dispatches `task::Rerun` with that task's ID, `allow_concurrent_runs=true`, `use_new_terminal=false`, and `reevaluate_context=false`, so the resolved task is rerun in its existing terminal. If the terminal has no TaskState, it dispatches a default `task::Rerun`, which falls back to the last scheduled task or task picker. The Terminal breadcrumb rerun button dispatches the same per-task override when the task enables `show_rerun`.

## Requirement crosswalk

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-task-Rerun](action-requirements.md) | Rerun the last/specified task, reusing its prior resolved context by default; optional context reevaluation and concurrency/new-terminal overrides; open the task picker when there is no prior task. | No local task inventory, Task command/action route, or task execution owner was found. The repository crosswalk still identifies Tasks as planned. |
| [ACT-MACOS-terminal-RerunTask](action-requirements.md) | Rerun the current terminal's recorded TaskId in place with concurrency allowed and prior resolved context. With no TaskState, fall back to default Task Rerun. The rerun button uses the same TaskId route when enabled by the task. | Local Terminal has no TaskState, rerun action/button, or connected Tasks owner. No rerun route exists for terminal sessions. |

## Ownership and shortcut resolution

The pinned Workspace action and Terminal action share chords with distinct contexts. macOS makes the Workspace action explicitly exclude Terminal. Linux and Windows store Workspace and Terminal bindings for some same chords; native key context ordering must confirm Terminal rerun wins inside Terminal without breaking Workspace rerun elsewhere. The two JetBrains profile entries rebind only Workspace `task::Rerun` to `cmd-f5`/`ctrl-f5`.

Labonair should keep Task definitions, inventory, scheduling, cancellation, status, and notifications in a Tasks owner when that dependency-ordered capability is reached. Terminal should contribute a typed current-task rerun request to that owner; Workspace/Shell should not become the task engine. This packet records the missing boundary and does not create a partial shell command.

## Acceptance evidence still required

- Exercise Workspace rerun with no previous task, a previous task, explicit TaskId, `reevaluate_context` on/off, and both concurrency/new-terminal overrides.
- Exercise Terminal rerun while its task is running, completed, failed, or cancelled; with and without `show_rerun`; verify the same TaskId and in-place replacement behavior.
- Change editor/workspace context after task completion and confirm default rerun reuses prior resolved context while reevaluation uses current context.
- Test no project, empty inventory, stale TaskId, spawn failure, local and remote tasks, guest/collaboration restrictions, cancellation, and terminal close/replacement recovery.
- Compare macOS, Linux, and Windows shared Workspace/Terminal chords and JetBrains overlays with runtime action dispatch; capture paired macOS task/terminal rerun states.
- Build a canonical Task owner and local end-to-end workflow before claiming parity. Source and existing tests were inspected, not run.

## Source records inspected

- Pinned action and scheduling: `zed-refrence/zed/crates/zed_actions/src/lib.rs`, `zed-refrence/zed/crates/tasks_ui/src/tasks_ui.rs`, and `zed-refrence/zed/crates/terminal_view/src/terminal_view.rs`.
- Pinned terminal task state/replacement: `zed-refrence/zed/crates/terminal/src/terminal.rs` and `zed-refrence/zed/crates/terminal_view/src/terminal_panel.rs`.
- Pinned keymaps: default macOS/Linux/Windows and macOS/Linux JetBrains profiles.
- Labonair search: `crates/` Task owner, action names, task state, rerun providers, and command registration; only terminal integration scripts and a terminal Clear command were found.
- Related registers: [action requirements](action-requirements.md), [parity packet index](README.md), and [Terminal capability crosswalk](feature-crosswalk.md).
