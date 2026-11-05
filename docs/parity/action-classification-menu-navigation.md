# Generic Menu Navigation Action Classification

**Status:** Static action, keymap, and route classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned `menu` defines generic action identifiers; focused components own list selection, submenu state, and the effect of secondary confirmation. Labonair `labonair-ui-kit` owns reusable popover/context menu controls, feature owners retain item-specific behavior, `labonair-shell` composes the native application menu, and `labonair-keymap` owns user bindings.

## Evidence boundary

This packet classifies the six default-macOS-bound `menu` actions whose semantic and local-route fields were pending: Restart, SecondaryConfirm, SelectChild, SelectFirst, SelectLast, and SelectParent. Their default platform bindings remain in [action-requirements.md](action-requirements.md): 17 macOS, 15 Linux, and 15 Windows binding records across the six IDs.

The profile registers contain 20 macOS overlay entries across four IDs: 18 Vim/Helix entries across four IDs and two Atom entries for SelectFirst/SelectLast. The shared Linux/Windows overlay adds two Atom entries for SelectFirst/SelectLast. These are static keymap records; no action is marked runtime-observed.

## Action semantics and routes

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-menu-Restart](action-requirements.md) | The action declaration says “restarts the menu from the beginning.” No `menu::Restart` handler or call site was found in the pinned Rust source, so its executable scope remains unresolved. | Shell defines application menu items in the `labonair::` action namespace. No generic Restart action or menu restart route was found. |
| [ACT-MACOS-menu-SecondaryConfirm](action-requirements.md) | Invoke the focused provider's secondary-confirm path. In Picker, it calls the delegate with `secondary = true`; in ContextMenu, it opens the selected submenu when that row is a submenu. The result depends on the focused provider. | UI Kit menu rows call their handler from a pointer click. No generic secondary-confirm action route was found; any alternate effect must stay with its owning picker or feature. |
| [ACT-MACOS-menu-SelectChild](action-requirements.md) | Ask the focused hierarchical provider to enter the selected child. Picker delegates the transition and propagates the action when it has no child transition. | Controlled UI Kit submenus open on Right, Enter, or Space. There is no shared typed SelectChild action; ordinary menu items and feature-owned trees have separate routes. |
| [ACT-MACOS-menu-SelectFirst](action-requirements.md) | Move the focused ordered list's selection to its first entry; handlers with an empty-result guard leave selection unchanged. Consumers include pickers, keymap and debugger lists, GitPanel, notebook blocks, and prediction review. | UI Kit's Up/Down handler requests next/previous focus movement, but does not bind a first-entry action or handle Home/Command+Up as this contract. |
| [ACT-MACOS-menu-SelectLast](action-requirements.md) | Move the focused ordered list's selection to its final entry; handlers with an empty-result guard leave selection unchanged. Consumers include pickers, keymap and debugger lists, GitPanel, notebook blocks, and prediction review. | UI Kit has no last-entry action or End/Command+Down route. Its row focus handler does not implement the pinned boundary-selection contract. |
| [ACT-MACOS-menu-SelectParent](action-requirements.md) | Ask the focused hierarchy provider to leave the current child or select its parent. Picker delegates the transition and propagates when no parent transition is available. | Controlled UI Kit submenus close on Left; there is no shared SelectParent action. Picker, thread sidebar, and call hierarchy need provider-owned hierarchy state. |

`SecondaryConfirm` does not specify one application-wide result: callers supply the alternate effect. It must not be treated as a synonym for Confirm or assigned a single feature owner.

## Labonair shared menu behavior

The `labonair-ui-kit` `context_menu` and `popover_menu` primitives build actionable rows with pointer handlers. Enabled action rows with handlers are focusable. Their local keydown handlers request next/previous focus movement on Up/Down, dismiss on Escape, and let a controlled submenu open on Right/Enter/Space or close on Left. Enter/Space on an ordinary row is intercepted without an explicit call to that row's click handler. The primitive has no generic first/last, secondary-confirm, restart, or typed parent/child action dispatch.

The shell's native menu is separate: `labonair-shell` defines app-menu actions in the `labonair::` namespace and binds native menu items to those actions. That does not establish generic `menu::` action registration or popover-key behavior.

## Keymap overlays

| Layer | Entries in scope | Actions and contexts |
|---|---:|---|
| Default platform maps | macOS 17; Linux 15; Windows 15 | The six actions' exact chords and contexts remain in [action-requirements.md](action-requirements.md). |
| macOS Atom | 2 | Home/End map to SelectFirst/SelectLast in ProjectPanel. |
| Vim/Helix | 18 | `g g`/Shift-G map to SelectFirst/SelectLast in ProjectPanel, OutlinePanel, GitGraph, GitPanel, FileHistoryView, NotebookEditor command mode, and ThreadsSidebar. `h`/`l` and `z c`/`z o` map to SelectParent/SelectChild in ThreadsSidebar. |
| Shared Linux/Windows Atom | 2 | Home/End map to SelectFirst/SelectLast in ProjectPanel. |

The profile registers record the bindings and contexts in [keymap-overlay-requirements.md](keymap-overlay-requirements.md) and [keymap-nonmacos-requirements.md](keymap-nonmacos-requirements.md). Runtime profile precedence and action dispatch remain unobserved.

## State, settings, and acceptance

- List selection, focus, and open-submenu state are transient and owned by the focused control or feature. No Settings effect or new persistence path was identified.
- Exercise first/last selection with empty, one-entry, disabled-entry, filtered, and changing lists; verify visible selection and focus at each boundary.
- Exercise child/parent on a regular menu item, a controlled submenu, a multi-level picker, ThreadsSidebar, and CallHierarchyPicker, including absent children/parents and action propagation.
- Exercise Confirm versus SecondaryConfirm on providers with alternate open/choose actions, and on selected submenu rows. Verify each provider's side effect independently.
- Exercise Restart using the pinned default binding and capture whether any focused control handles it; current static source inspection found no handler.
- Capture macOS action registration, native key dispatch, Atom and Vim/Helix layer precedence, keyboard focus, submenu opening/closing, and paired menu states. Static bindings and pointer routes do not satisfy these checks.

## Acceptance evidence still required

Static source review establishes the declared intent for five actions, the absence of a discovered `menu::Restart` handler, provider-specific secondary confirmation, current UI Kit keyboard routes, and the listed keymap/profile entries. macOS action registration and dispatch, exact focus movement in native controls, alternate-confirm outcomes, Restart behavior, profile precedence, and paired visual evidence remain open.

## Source records inspected

- Pinned action declarations: `zed-refrence/zed/crates/menu/src/menu.rs`.
- Pinned handlers: Picker, UI ContextMenu, CallHierarchyPicker, KeymapEditor, GitPanel, debugger lists, Notebook UI, prompt, and prediction modal.
- Pinned keymaps: default macOS/Linux/Windows maps, Atom overlays, and Vim/Helix overlay.
- Labonair controls and composition: `crates/ui-kit/src/context_menu.rs`, `crates/shell/src/menu.rs`, `crates/shell/src/commands.rs`, and `crates/shell/src/keymap_loader.rs`.
- Related registers: [action requirements](action-requirements.md), [macOS overlay requirements](keymap-overlay-requirements.md), and [non-macOS keymap requirements](keymap-nonmacos-requirements.md).
