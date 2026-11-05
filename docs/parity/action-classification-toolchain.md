# Toolchain Selection and Add Action Classification

**Status:** Pinned selection/add workflows and keymaps classified; local toolchain owner, native action dispatch, persistence failure behavior, and paired macOS acceptance remain open

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Pinned owners:** ToolchainSelector owns the picker/add UI; Project owns language-specific detection, user toolchain scopes, and activation; Workspace contributes the active Editor status item and persists selected/custom toolchains. Labonair has no Toolchains capability owner yet. The Editor is the closest candidate for active-file selection; the owner and persistence boundary must be resolved before implementation.

## Evidence boundary

This packet covers the registered-unbound `toolchain::Select` action and the related default-bound `toolchain::AddToolchain` action. `Select` has no binding in the default macOS/Linux/Windows maps or the inspected profile layers. Its action handler is registered on Workspace. When the active Editor has a file-backed, language-bearing buffer and a current toolchain resolves, its status item opens the same selector by pointer; the status item is hidden when no active toolchain resolves. Command Palette reachability remains unverified.

`AddToolchain` has two default bindings on each platform. The Workspace context uses macOS `cmd-k cmd-m`, Linux `ctrl-k ctrl-m`, and Windows `ctrl-m ctrl-m`. The `ToolchainSelector` context uses macOS `cmd-shift-a`, Linux `ctrl-shift-a`, and Windows `ctrl-shift-a`. Each ToolchainSelector context block sets `use_key_equivalents=true`. No additional profile binding was found. These static entries do not prove native dispatch.

## Pinned workflows

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-toolchain-Select](registered-actions-unbound.md) | `toolchain::Select` toggles the selector for the current Workspace only when its active item is an Editor with a language-bearing, file-backed buffer and a parent directory. It asynchronously loads the current selection and candidates for that language and directory. The pointer entry is the active Editor's Toolchain status item. | Labonair has Editor-owned local language services, but no toolchain data type, detector, selector, per-file choice, or active-language status item. The `language.services` crosswalk records Editor plus planned Extensions ownership; a single canonical toolchain owner remains to be set. |
| [ACT-MACOS-toolchain-AddToolchain](action-requirements.md) | With the selector active, `AddToolchain` opens a project path picker. The Workspace binding attempts to open the selector when no selector is active, provided there is a file-backed, language-bearing active Editor buffer; it does not immediately enter path selection. The ToolchainSelector binding and footer button enter the add flow. A chosen path is resolved asynchronously for the active language. Resolution errors remain in the path picker and preserve the selected path. A single-line name field is prefilled from the detected toolchain, and a scope chooser offers Subproject, Project, and Global. Confirming a non-empty name adds the custom toolchain and returns to the search list; an empty name is a no-op. | No Labonair path resolver, scoped custom toolchain store, add flow, or corresponding settings/persistence route exists. |

### Selection list and state transitions

The picker combines language-detected candidates with user toolchains scoped to the current subproject, project, or machine. It filters by toolchain name and path, displays a path relative to the worktree when possible, and selects the current toolchain when it appears in the list. User-created candidates have a remove control; removal updates the Project toolchain store and refreshes the Workspace snapshot.

Confirming a candidate stores the selected toolchain in WorkspaceDb for the Workspace, worktree root, relative directory, and language, then activates it through Project. The picker dismisses immediately after scheduling that asynchronous operation. If the Workspace has no database ID, it still dismisses without persistence or activation. A persistence error is logged and activation is still attempted; the inspected handler does not show a user-facing error route. Escape/dismiss closes without applying another candidate.

Separately, `ActiveToolchain` tracks the active Editor buffer and language. If there is no explicit selection, it chooses the first non-global user toolchain, falling back to the detected default, then stores and activates that choice. This automatic default route is not caused by `toolchain::Select`.

The Add flow stores custom toolchain definitions in the Project toolchain store by scope. A `CustomToolchainsModified` event causes Workspace to serialize the user toolchains. WorkspaceDb saves custom definitions separately from the selected per-directory toolchain. These are persistence records, not Settings fields.

## Labonair owner and current route

`crates/editor/src/language_services.rs` provides syntax-derived local services and an opt-in LSP runtime, but it does not discover or select executable toolchains. No matching toolchain API or Settings value was found in Editor, Workspace, Shell, or Settings. The capability crosswalk currently lists Editor and a planned Extensions owner for language services/tooling.

For a future implementation, active-file selection is an Editor-owned behavior candidate because the chosen runtime depends on the open file's language and project directory. Workspace may compose its status item and provide project identity through a typed boundary; toolchain discovery or installation may come from the planned Extensions owner. R07-001 must settle one canonical owner, persistence scope, provider contract, and permission model before implementation. Do not put toolchain behavior in Shell or create a second Settings copy of runtime state.

## Acceptance still required

- Verify macOS action registration and dispatch for `toolchain::Select` and both `AddToolchain` contexts; test command-palette discovery and status-item pointer access separately.
- Exercise no active Editor, pathless/unsaved buffers, missing language, unsupported language, no candidates, and a current toolchain missing from the candidate list.
- Verify candidate filtering by name/path, worktree-relative path display, current selection, detected defaults, and each custom scope.
- Confirm selection changes only the active language/worktree directory; verify WorkspaceDb persistence, activation ordering, failure reporting, switch/restart restoration, and remote project behavior.
- Exercise Add path selection, cancel, invalid/unreadable path, async resolution, empty name, each suggested/manual scope, duplicate toolchain, custom removal, and Workspace persistence.
- Compare focus, keyboard navigation, footer buttons, narrow layout, modal dismissal, statusbar visibility, and error states with paired native macOS captures.
- Before local implementation, assign the canonical owner and define how project trust/permissions gate invoking language-specific executables.

## Source records inspected

- Pinned actions, selector, add state machine, and status item: `zed-refrence/zed/crates/toolchain_selector/src/toolchain_selector.rs` and `active_toolchain.rs`.
- Pinned toolchain scopes and candidates: `zed-refrence/zed/crates/language_core/src/toolchain.rs` and `zed-refrence/zed/crates/project/src/toolchain_store.rs`.
- Pinned selection/custom persistence and Workspace event handling: `zed-refrence/zed/crates/workspace/src/persistence.rs` and `workspace.rs`.
- Platform defaults: `zed-refrence/zed/assets/keymaps/default-macos.json`, `default-linux.json`, and `default-windows.json`.
- Labonair language owner and crosswalk: `crates/editor/src/language_services.rs`, [`docs/capabilities.md`](../capabilities.md), and [feature crosswalk](feature-crosswalk.md).
