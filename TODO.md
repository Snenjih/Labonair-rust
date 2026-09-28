# Labonair TODO

Product and UX follow-up work. Each item must become an owned, testable task
before implementation and must follow the feature lifecycle and active rework
queue.

## Extensions

- [ ] Define the extension/plugin capability and its owner.
  - Specify the public API, lifecycle, version compatibility, permissions, and
    failure/isolation behavior.
  - Decide whether local extensions, downloads, or a marketplace are in scope.
  - Add a registry only where multiple extension providers or consumers need
    discovery.
  - Keep this deferred until the core workflows are stable, as documented in
    the product roadmap.

## Universal split panes

- [ ] Replace feature-specific split behavior with one universal workspace pane
  system.
  - A pane must be able to host any tab type, including terminals, editors,
    SFTP, and future tab types.
  - Support up to eight visible panes at the same time.
  - Support horizontal and vertical splits, pane focus/navigation, resizing,
    closing, and moving or opening tabs in a selected pane.
  - Support drag-and-drop for existing tabs between panes, including a clear
    drop-area preview that shows where the tab will be placed.
  - Allow tabs to be dragged out of the titlebar, moved into another pane, and
    merged into an existing pane or tab group.
  - Add a pane list to each tab's context menu so a tab can be moved directly
    to a selected pane without using drag-and-drop.
  - Allow tabs or sub-tabs from a pane to be dragged back into the titlebar to
    turn them into separate top-level tabs again.
  - Define the canonical layout tree, pane identifiers, commands, keybindings,
    persistence, and behavior when the eight-pane limit is reached.
  - Reconcile the design with ADR 0004 and the planned Editor-internal split
    work so that two competing layout systems are not introduced.
  - Keep split state in the workspace/layout owner; feature modules provide tab
    content but do not own a second split implementation.

## Appearance

- [ ] Add an app-wide transparency setting.
  - Expose the value from Appearance settings with a clear default and safe
    bounds.
  - Apply it consistently to the Labonair window/background while preserving
    readable contrast and platform-specific window behavior.
  - Keep the setting value in Settings and the background/window presentation
    behavior in its owning capability.

- [ ] Add a global, subtle fade-in animation setting for supported UI elements.
  - Use shared duration/easing tokens instead of per-view animation values.
  - Apply the animation consistently to newly shown elements without delaying
    focus or keyboard interaction.
  - Integrate with the existing reduced-motion behavior.

- [ ] Add a global corner-roundness setting matching the reference design.
  - Provide the options `Square`, `Subtle`, `Rounded`, and `Round` in a
    dropdown.
  - Map the selected option to shared UI-kit radius tokens for surfaces and
    controls.
  - Ensure components do not silently bypass the global value with local
    radius styles.

- [ ] Add a custom app-wide background workflow.
  - Allow selecting a user-provided background, previewing it, replacing it,
    and resetting to the default.
  - Persist the selection safely and keep image ownership, decoding, and
    rendering inside the Backgrounds capability.
  - Define how the custom background interacts with transparency, opacity,
    tint, blur, and readability.

## UI and typography

- [ ] Perform an overall UI overhaul and polish pass.
  - Audit the titlebar, tabs, workspace, docks, statusbar, overlays, dialogs,
    settings, empty states, and error states for hierarchy, spacing, contrast,
    focus states, and consistency.
  - Reuse `labonair-ui-kit` components and shared design tokens throughout.
  - Include visual verification at normal, narrow, focused, empty, loading, and
    error states.

- [ ] Make all font-editing settings fully functional.
  - Ensure app, terminal, editor, and other supported font settings validate,
    persist, and apply live to the correct consumers.
  - Provide a searchable font dropdown: typing filters the list, keyboard
    navigation works, the current value is visible, and selection can be
    confirmed or cancelled without losing focus.
  - Define fallback behavior for unavailable or invalid fonts and provide a
    useful preview where appropriate.

## Universal text-input behavior

- [ ] Standardize text fields and search inputs across the app through the
  shared UI-kit input behavior.
  - Support typing, cursor movement, arrow-key navigation, selection, select
    all, deletion, copy/cut/paste, undo/redo, and the platform-standard
    keyboard shortcuts.
  - Make focus, selection, placeholder, disabled, and validation states
    consistent.
  - Apply the behavior to settings, searchable dropdowns, command/search
    surfaces, dialogs, terminal/editor-related inputs, and future text fields.
  - Add focused interaction tests for keyboard navigation and selection so
    regressions are caught centrally.

## Completion gates

- [ ] Record the owning module, canonical crate, entry point, settings,
  persistence, commands, notifications, UI-kit components, and dependency
  changes for each implementation task.
- [ ] Update the capability matrix, roadmap, or ADRs when a topic changes the
  documented product or architecture contract.
- [ ] Run the required Rust, documentation, dependency, and visual checks for
  each completed task.
