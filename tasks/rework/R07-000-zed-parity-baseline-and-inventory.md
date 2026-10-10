# R07-000 — Implement clean-room Settings and shared UI parity

## Status

`🔄 In Progress`

## Owner

- Module: product architecture with all capability owners
- Capability-matrix row: full Zed feature and UI/UX parity
- Canonical contract: [`../../docs/zed-parity.md`](../../docs/zed-parity.md)
- User entry point: existing Settings command/window for the current Settings
  implementation slice; each subsequent surface retains its owning command or
  registered entry point

## Dependencies

- `R06-001-backend-adapter-eradication` (complete)

## Goal

Continue the pinned-reference parity work directly in Labonair runtime code.
The immediate runtime deliverable is a complete Settings UI built on shared
UI-kit primitives; the same primitives must then support the rest of the app.
This is progress toward the user's whole-app parity objective, not a claim
that Settings alone completes that objective.

## Scope

- In scope: reproducible reference acquisition, source-boundary protocol,
  capability and surface inventory, glossary, universal UI checklist,
  command/keymap and Settings crosswalk, current-state ownership map,
  architecture decisions, macOS visual baseline, and dependency-ordered
  implementation tasks.
- In scope: identify and compare the four reported attempts at Settings using
  their commits and visual artifacts; record verified reasons for each miss.
- First deliverable: a paired Settings baseline for the pinned Zed build and
  current Labonair build, with matching macOS window bounds, theme, scale,
  fonts, and capture conditions. Record navigation and one representative
  category down to labels, controls, dimensions, focus/keyboard behavior, and
  states.
- Initial history leads to verify against the four reported attempts:
  `c9075e9` (2026-10-07), `4cca14a`, `f197d32`, and `3aa2327` (2026-09-25).
  These are candidates from the Settings UI history, not yet confirmed as the
  exact four attempts. The only tracked image under `shots/` is
  `shots/labonair.png`, which shows the SFTP surface and cannot serve as
  Settings comparison evidence.
- Next deliverable: reconcile all four reported Settings attempts against
  their exact commits and available captures; label causes as verified or
  unknown, then produce the complete Settings field crosswalk and measurable
  view-state checklist.
- Broader deliverable: apply the same inventory record to every surface,
  action, capability, extension point, and workflow in the pinned build.
- In scope now: implement and unify the shared controls used by Settings,
  then rebuild Settings navigation, category pages, controls, search, scopes,
  popovers, keyboard behavior, and all applicable UI states against the pinned
  macOS reference.
- In scope for the continuing parity objective: apply the same shared UI
  architecture and evidence checklist across titlebar, menus, command palette,
  sidebars, docks, panels, tabs, splits, dialogs, Git, Explorer, Terminal,
  Editor, DiffView, themes, extensions, and remaining reference workflows.
- Out of scope: copying Zed code or assets, claiming visual parity without
  paired evidence, or treating the Settings implementation as completion of
  the whole-app objective.

## Reference and clean-room boundary

- Pin `zed-refrence/zed` to
  `3569541038dd51524b03998ba4d38d253cb54f80` (`nightly-9`).
- Use macOS as the first paired visual and interaction acceptance platform.
- Compare public documentation with the pinned runtime and record drift.
- Make the local gitlink reproducible on a clean checkout; the parent
  repository was missing its `.gitmodules` entry. The mapping is committed in
  `bf70e41`; a fresh parent clone at that commit successfully acquired the
  pinned submodule.
- Store source-level inspection only with reference analysis. Produce a
  separate implementation packet with independently worded observable
  requirements; do not give the implementation role the Zed checkout or
  source-level comparison reports.

## Required inventory entries

Every pinned-reference capability and every current Labonair capability must
be mapped to an owner, canonical user entry point, contract, relevant
commands/keybindings, settings, persistence, views, visual states, extension
points, external services, and acceptance evidence. Use the feature-record
template and status vocabulary in `docs/zed-parity.md`.

The inventory explicitly covers the universal Tab/Split model, shared
dropdown/menu and control system, Settings, Editor and language servers,
language/theme extensions, Explorer, Terminal, Git/Source Control, Git Graph,
universal DiffView, AI/agent, remote development, collaboration, platform
support, and hosted workflows present in the pinned build.

## Work order

1. Inspect the existing Labonair Settings owner and shared UI kit; compare
   observable behavior and geometry with the pinned reference.
2. Fix shared primitives at their owner, then compose the Settings surface
   from those primitives without adding local copies.
3. Complete Settings flows and states, preserving the Settings value owner,
   canonical categories, persistence, and command entry point.
4. Apply the shared UI contracts and the same keyboard/state review across
   remaining app surfaces in dependency order; keep the full parity objective
   open until all catalog entries are implemented and accepted.

## Acceptance criteria

- [ ] The pinned reference and macOS capture environment are reproducible,
      or remaining blockers are named with evidence and an owner.
- [ ] The Settings pilot has paired captures with matching conditions, or
      capture blockers are documented and the text/interaction inventory is
      complete to the extent possible without those captures.
- [ ] All four previous Settings attempts are linked to exact commits and
      artifacts; verified causes and unknowns are stated separately.
- [ ] Every feature and workflow in the pinned baseline has a Labonair
      crosswalk; no current `defer` silently removes a reference capability.
- [ ] The independent glossary, universal checklist, and feature record
      template cover all owners and surface types.
- [ ] The universal Tab/Split and DiffView decisions have accepted typed
      contracts, including movement, focus, close, restore, and failure
      behavior.
- [ ] Shared popup/menu and UI control requirements have measurable geometry,
      input, and visual evidence criteria.
- [ ] Every Settings key and visible field in the pinned baseline has a
      mapped owner, type, default, scope, control, effect, and paired evidence.
- [ ] Implementation work packages are ordered by actual contract and crate
      dependencies; R09 Editor work is reconciled and remains unstarted until
      this task completes.
- [ ] License, asset, service, and distribution questions have named review
      items; this task makes no unsupported legal-clearance claim.

## Verification and evidence

- `python3 scripts/check_documentation.py`
- `python3 scripts/check_rework_queue.py`
- `git diff --check`
- Evidence: pinned commit and reproducible acquisition record, complete
  feature crosswalk, Settings pilot and prior-attempt review, paired macOS captures,
  glossary/checklist coverage, and ordered implementation task graph.

## Change contract

```text
Request type: Clean-room Settings/UI implementation
Owner module: Settings for its value controls and view; UI kit for reusable interaction primitives
Canonical capability crate: `labonair-settings-ui`; reusable components in `labonair-ui-kit`
Canonical user entry point: Settings command and its existing registered window
Public typed contract: Existing Settings and UI-kit APIs, extended only where a real shared interaction contract is needed
State/persistence owner: Existing typed Settings store and setting owners; no UI control owns persisted values
Commands and keymap entries: Preserve the existing Settings command and keyboard flows; add no duplicate entry point
Events and registry contributions: Preserve existing owner registrations; do not add shell-wide feature tables
Settings: Render the current and newly inventoried settings through their owning typed fields, defaults, and scopes
Notifications: Route operation errors through the existing notification owner; retain inline correction only for invalid active input
UI-kit components: Shared buttons, icon buttons, text/search inputs, selects, menus, popovers, tree/list rows, dividers, and state views
Allowed dependency changes: None unless a required GPUI capability is documented and the crate graph remains acyclic
Tests: Focused Settings/UI-kit coverage and repository Rust gates for implementation changes
Visual evidence: Paired native Settings captures are required; currently pending because native-window inspection/capture is unavailable in this session
Security impact: Inventory extension, remote, collaboration, agent, and hosted-service permission boundaries; no runtime policy change
Performance impact: Record measurable targets and fixtures in the parity catalog; no runtime performance claim
Removal condition for compatibility code: N/A; no compatibility path is introduced
```

## Progress record — 2026-10-09

- Verified the local Zed checkout at the exact pinned commit and clean tree.
- Built `target/debug/zed` successfully from that checkout with
  `mise exec cmake@4.4.3 -- cargo build --locked -p zed`. The linker emitted
  an `__eh_frame` size warning; the binary has not been opened for native
  visual inspection.
- Added the missing parent `.gitmodules` URL mapping. A fresh-parent-clone
  acquisition check passed after commit `bf70e41`: a fresh parent clone ran
  `git submodule update --init --depth 1 -- zed-refrence/zed` and checked out
  `3569541038dd51524b03998ba4d38d253cb54f80`.
- In that clean clone, the documentation check passed (43 normative documents,
  159 Markdown files) and the queue check passed (46 tasks; R07-000 active).
  The original shared worktree's pre-existing `reference-src/` deletions still
  make its documentation check fail on missing link targets.
- Recorded the local macOS version (`26.7.1`) and exact Labonair parent
  revision (`c42d33fae184b98c90516d747bfbb4844eb21992`). The Zed runtime
  binary, matching window bounds/theme/scale/fonts, and paired captures are
  not yet recorded.
- Confirmed the four Settings-history candidate commits and reviewed their
  code changes. No Settings capture exists for any candidate; visual miss
  causes remain unknown.
- Added a draft coverage crosswalk. It does not yet enumerate every pinned
  action, setting, extension, and workflow and must not be treated as complete.
- Added an independent Labonair glossary, reusable UI acceptance checklist,
  and proposed Tab/Split and DiffView lifecycle contracts. The proposed
  contracts still need owner review and typed API decisions.
- Recorded the pinned Settings navigation and all 70 visible Editor setting
  labels/keys from the research pass, plus representative control types and
  source-declared window dimensions. Full defaults/effects/control mapping and
  rendered interaction states remain pending.
- Named the legal, asset, extension-distribution, hosted-service, remote, and
  release review questions and their proposed review owners.
- Documented the source-blind checkout procedure, but that checkout has not
  been created or audited yet.
- The current CUA session reports no application surfaces (`apps: []`); its
  documented native entry points fail at runtime (`cua.getApp is not a
  function`; `cua.computer` is undefined).
  The prior native acceptance log also records macOS denying
  Screen Recording to the runner. Settings visual evidence remains pending;
  the workspace host operator owns restoring native-window access and the
  macOS Screen Recording grant for the runner.
- Started the updated Labonair app with `cargo run -p labonair`; the process
  stays live, but `cua.getState()` still returns `apps: []`. No native visual
  comparison is claimed for this implementation slice.

The user redirected the next step from specification-only work to direct
runtime implementation. This task now permits implementation, beginning with
the shared UI kit and Settings owner. Native visual evidence remains pending.

- Direct UI work migrated the Hosts search box from an end-appending string
  renderer to the shared native `InputState`, preserving host filtering and
  quick-connect activation while enabling selection, clipboard, IME, and undo.
- Migrated the Hosts detail form's active field to the shared native
  `InputState` and UI-kit field frame. Host values still autosave through the
  Hosts owner; password fields stay masked; Enter submits and Escape closes.
  The `hosts-ui` crate compiles and formatting is clean. Native visual review
  remains pending with the rest of the macOS acceptance evidence.
- Added one shared search-clear action and adopted it in Settings, Hosts,
  Explorer, and File Finder. Its keyboard activation uses the button's normal
  click path, avoiding duplicate keydown and click mutations.
- Added a shared compact icon-button primitive with enabled and inert states;
  Settings reset/back actions and the shared search-clear action now use it.
- Routed the Settings header's User/Project JSON action to the Workspace owner
  so it creates and opens the selected JSON file in Labonair's Editor instead
  of revealing the path in the system file manager.
- Added a shared inert button builder for disabled actions; Settings SFTP
  column controls and Search Overlay navigation/replacement actions now use it.
  Removed direct keydown activation from Settings reset/reorder actions to
  prevent double execution.
- Added a retry action and immediate loading repaint for failed Settings
  system-font discovery.
- Added a shared text-field frame with normal, focused, and invalid border
  states, then adopted it in Settings, Keymap, Explorer inline editing,
  Search Overlay replacement, and Editor rename.
- Project Settings values outside the explicit project whitelist now render
  inert controls across switches, numeric/text fields, selects, and SFTP column
  editing. Scope changes commit any active text or numeric edit to the previous
  target first. The shared UI kit now exposes disabled select and text-area
  surfaces; the Git commit composer uses the shared multiline frame, and its
  unavailable action uses the shared inert button.
- The updated Settings/UI-kit/SCM suite passed all 92 package unit tests;
  targeted Clippy and `cargo check --workspace --all-targets` passed.
- The expanded targeted suite passed all 272 tests across the affected
  Settings, UI-kit, Hosts, Keymap, Explorer, and Workspace packages.
- Fresh targeted verification after these changes passed: `cargo fmt --all
  --check`; `cargo check` for the affected Settings, UI-kit, Hosts, Keymap,
  Explorer, and Workspace crates; `cargo clippy --all-targets -- -D warnings`
  for those crates; and all 272 package unit tests. `git diff --check` and the
  rework queue check also passed.
- The documentation checker still fails only on missing `reference-src/`
  files already deleted in this shared worktree; those pre-existing deletions
  were preserved.
- Native paired captures remain pending because the current UI automation
  reports no app windows.

## Progress record — 2026-10-10

- The shared Settings select now renders option rows through GPUI's
  virtualized list, retaining the existing 320-pixel popup limit for large
  system-font catalogs.
- Open select menus support arrow navigation, Home/End, Page Up/Down,
  activation, Escape dismissal, and scrolling the highlighted option into
  view. Focused behavioral coverage was added for the index transitions.
- The latest focused Settings/UI-kit run passed all 76 unit tests, and Clippy
  passed for both crates with warnings denied.
- Shared rich-content popovers and menu cards invoke their dismissal callback
  on Escape while popup content has keyboard focus; Settings retains select
  dismissal in its owner because focus remains with the Settings view.
- Adjusted the Settings rail's explicit search margin from 12 to 10 spacing
  units because the parent already adds a 2-unit flex gap; the rendered total
  now matches the documented 12-unit search-to-navigation spacing.
- The visual verifier passes its catalog/index checks, but reports 0 captures
  and 63 pending states; native UI automation still reports no app windows.
  Paired visual acceptance remains pending for this behavior and the Settings
  surface as a whole.
- Consolidated pointer menus, trigger selects, and rich-content popovers on
  the UI kit's measured popup positioner. It flips horizontally and vertically
  toward the side with more room, clamps to an 8-pixel viewport inset, and
  limits the panel to the available viewport size. Five geometry tests cover
  corner flips, independent-axis flips, edge clamping, and short viewports.
- The focused Settings/UI-kit run passed 81 package tests, and targeted
  Clippy with warnings denied, formatting, and `git diff --check` passed. The
  rework queue check passed with R07-000 still active. The documentation check
  remains blocked by missing link targets under the already-deleted
  `reference-src/` tree in this shared worktree; those deletions were not
  changed here.
- Native visual acceptance for the popup changes remains pending: CUA reports
  no app windows in this session, so there are no screenshots to compare.
- Inspected the pinned Settings UI and button components at
  `3569541038dd51524b03998ba4d38d253cb54f80`. The window's 900×750 logical
  baseline, 226-pixel rail, and 400-pixel content minimum match Labonair's
  current geometry. The shared button did not: it still used the former web
  reference's 36-pixel default height and pill radius.
- Replaced the UI-kit button surface with the pinned reference's compact
  16/18/22/28/32-pixel size scale, font/density scaling, small-radius corners,
  subtle default appearance, explicit outlined/filled/error treatments, and
  shared hover, pressed, focused, and disabled styling. Migrated app callsites
  to the new appearance names, marked primary actions as filled, and updated
  the Settings JSON action to medium size. The component gallery now exercises
  every button appearance and size through the shared primitives.
- GPUI 0.2.2 has no modality-specific `focus-visible` style. Buttons retain a
  visible focus treatment for keyboard users; it also appears after pointer
  focus. The framework limitation is recorded in the design-system contract.
- Added an independent Settings page-to-owner crosswalk. It records all 15
  target page names, maps the current seven persisted value groups to their
  likely pages, and names the missing owner dependencies. It explicitly keeps
  navigation categories separate from the serialized SettingsContent layout;
  no empty page or inert setting is treated as implemented.
- The Settings crosswalk is an implementation order, not field-level
  acceptance. The category model is now implemented in the Settings UI; the
  remaining Settings work is the per-field record of label, type, default,
  effect, scope, reset behavior, owner, and visual states, followed by page and
  control parity against paired macOS evidence.
- Expanded the shared IconButton from a square-only helper into a UI-kit
  builder for wide/square shape, selected style and glyph, disabled state,
  semantic icon tint, independent font-scaled glyph size, opacity, bordered
  status indicator, and tooltip. Settings reset and back actions plus search
  clear now use the builder; the component gallery shows wide, selected,
  indicator, and compact icon sizes. The glossary and UI checklist record the
  new control states.
- The UI-kit, Settings UI, and Shell package tests passed (98 unit tests);
  `cargo check` and Clippy with warnings denied passed for those packages, as
  did formatting and `git diff --check`. Paired visual comparison remains
  pending because this session still has no native window capture access.

## Progress record — 2026-10-10 — Settings page model

- Decoupled visible Settings categories from the seven persisted
  `SettingsContent` groups. The runtime navigation now has nine owner-backed
  pages: General, Appearance, Editor, Languages & Tools, Search & Files,
  Window & Layout, Terminal, Version Control, and Network. The other six
  target pages remain crosswalk work and are not represented with placeholders.
- Page rows use full JSON paths. Explicit cross-page placement takes
  precedence over a persisted-group fallback, and the same resolver powers
  search grouping, search activation, deep links, and the trailing fallback
  section. Persisted keys and migrations are unchanged.
- Added checks that every current field resolves to a page, explicit rows
  reference real fields without duplicates, cross-page assignments do not
  appear in fallback sections, and search/deep-link locations match the page
  descriptors.
- `cargo test -p labonair-ui-kit -p labonair-settings-ui -p labonair-shell`
  passed (95 unit tests); targeted Clippy with warnings denied and formatting
  passed. Workspace `cargo check --workspace --all-targets`, workspace Clippy
  with warnings denied, and `cargo test --workspace` also passed. Documentation
  and queue checks were run: the Settings scope, generated Settings catalog,
  visual catalog, and queue checks pass; the
  documentation checker still reports 21 existing missing links into the
  pre-deleted `reference-src/` tree.
- A native app launch succeeds, but the computer-use surface reports no apps or
  windows even while the process is running. The visual catalog has 63 states
  and 0 captures; paired visual acceptance remains pending and no
  visual-parity claim is made.

## Progress record — 2026-10-10 — shared tab item

- Added the UI-kit `tab_item` for the common horizontal and vertical tab-row
  treatment. It owns shared geometry, selected/disabled styling, dirty/busy/
  peek indicators, optional icon and rename content, close affordance, and
  keyboard activation/navigation. Workspace still owns tab lifecycle,
  persistence, drag/drop, ordering, rename, and close confirmation.
- Migrated the existing mixed Workspace tab rows to the shared component and
  added horizontal/vertical examples to the component gallery. Arrow-key
  navigation stays within the source tab's Space; disabled tabs ignore their
  activation callback.
- Added the tab item to the glossary and universal visual checklist.
- `cargo test -p labonair-ui-kit -p labonair-workspace -p labonair-shell`
  passed (56 UI-kit, 161 Workspace, 16 Shell unit tests plus the Shell icon
  integration test). Targeted Clippy with warnings denied, formatting, and
  `git diff --check` passed.
- Paired visual acceptance remains pending: the current visual catalog has 63
  states and 0 captures, and this session's native UI surface reports no app
  windows. The documentation checker still reports 21 links into the already
  removed `reference-src/` tree.
- Next implementation slice: finish Settings against a paired macOS capture,
  starting with the window frame, navigation rail, and Appearance page, then
  proceed through the remaining pages using the same shared controls.

## Progress record — 2026-10-10 — Settings icon-action unification

- Replaced the SFTP column-order controls' locally styled icon buttons with
  the shared UI-kit icon-button builder. Enabled actions now inherit shared
  size, focus, tooltip, and icon treatment; unavailable moves use the shared
  disabled state. The Settings owner still supplies each move action.
- `cargo test -p labonair-settings-ui` passed (26 tests), as did targeted
  Clippy with warnings denied and formatting. A native app launch succeeded,
  but the computer-use inventory still reports no app windows, so this control
  and the full Settings surface have no paired visual capture yet.

## Progress record — 2026-10-10 — Appearance theme-owner actions

- Added app-theme and icon-theme action rows to the Appearance page. Each row
  invokes the existing searchable picker through a typed Settings service;
  the Theme owner retains catalog, preview, activation, and persistence.
- Kept the page virtualized and used the shared outlined button treatment with
  pointer and Enter/Space activation. Added coverage for action placement and
  field section routing, and updated the Settings contract, crosswalk, glossary,
  and universal checklist.
- `cargo test -p labonair-settings-ui -p labonair-shell` passed (27 Settings
  tests, 16 Shell unit tests, and the Shell icon integration test). Paired
  visual acceptance remains pending because the native UI capture API is
  unavailable in this session.
- Targeted Clippy with warnings denied, `cargo build -p labonair`, the Settings
  and visual verifier scopes, the rework-queue check, formatting, and
  `git diff --check` passed. The full documentation checker still reports the
  21 pre-existing links into the removed `reference-src/` tree.
- The Appearance audit also confirmed seven Background-owned persisted values
  have no current Settings editor. R05-001 explicitly keeps those values out
  of `SettingsContent`; the Background owner editor and Appearance entry point
  are now implemented below without restoring a duplicate Settings value
  store.

## Progress record — 2026-10-10 — Background owner editor

- Added the shared token-styled `Slider` adapter and a component-gallery row;
  the Background view pairs each continuous slider with the keyboard-operable
  numeric stepper.
- Added a Background-owned settings view for image selection/import/removal,
  opacity, blur, tint color and strength, image scaling, and render target.
  Import, deletion, settings reads/writes, and image decoding run on the
  background executor; range updates are debounced and pending values flush
  when the modal closes. Import/removal expose loading states, removal requires
  confirmation, and tint input validates six-digit hex.
- Added a Background owner action in Appearance. The shell opens the owner
  view in the single modal layer and routes operation failures through the
  notification center. Background state and persistence remain outside
  `SettingsContent`.
- Updated the Settings, Background capability, UI-kit, glossary, and universal
  checklist records to reflect the owner surface and shared slider.
- Focused verification passed: `cargo fmt --all --check`, `git diff --check`,
  and `cargo test -p labonair-background -p labonair-ui-kit
  -p labonair-settings-ui -p labonair-shell` (108 unit tests total, plus the
  Shell icon integration test), targeted Clippy with warnings denied, and
  `cargo build -p labonair`. The architecture/dependency, capability, Settings,
  visual-catalog, and rework-queue scopes pass. The documentation checker still
  reports the 21 pre-existing links into the removed `reference-src/` tree.
  Native paired captures remain pending because the UI automation still
  reports no application windows; no visual-parity claim is made.

## Progress record — 2026-10-10 — Native Settings capture preflight

- The user supplied a Settings screenshot identified as Labonair. It shows
  seven navigation entries, while the current Rust Settings page model has
  nine. The image is useful for discussion but has no verifiable executable
  identity, so it is not accepted as a native capture or parity evidence.
- Fixed `scripts/screenshot.sh` to canonicalize relative executable paths
  before matching the exact Rust binary. This makes the documented
  `cargo run -p labonair` process path pass identity validation.
- Retried capture with the exact Rust binary. The helper passed executable
  validation but could not find a layer-0 window in this session. The paired
  macOS capture gate remains open; next step is to capture the Rust Settings
  window and the pinned Zed Settings window under matching conditions.

## Progress record — 2026-10-10 — Keymap Settings entry point

- Added the Keymap page to the Settings navigation and routed its owner action
  to the existing Workspace Keymap tab. Settings search indexes owner
  actions, so keyboard shortcuts and theme queries can find and activate their
  canonical owner surfaces. The keymap module retains binding editing,
  conflict state, and persistence; Settings adds no second editor.
- Updated the Settings contract, capability decision, and page crosswalk to
  record the pinned Keymap navigation entry and its owner boundary.
- `cargo test -p labonair-settings-ui -p labonair-shell` passed (29 Settings
  tests, 16 Shell unit tests, and the Shell icon integration test); formatting,
  Settings-scope verification, queue verification, and `git diff --check`
  passed. Targeted Clippy with warnings denied and `cargo build -p labonair`
  also passed. The documentation checker still reports the 21 existing links
  into the removed `reference-src/` tree. The visual catalog and path checks
  pass, while rendered evidence remains at 0 captures and 63 pending states.
  Paired macOS visual acceptance remains open under the capture preflight
  blocker above.

## Progress record — 2026-10-10 — Registered Settings owner surfaces

- Replaced the Settings renderer's closed owner-surface enum and action match
  with a startup registry of stable IDs, searchable metadata, page placement,
  and owner callbacks. Navigation and search are composed from the same
  registry snapshot. Duplicate IDs, incomplete metadata, and incompatible
  page metadata fail registration.
- Moved the registry contract into `labonair-settings`. Theme, Background, and
  Keymap UI modules now contribute their own metadata and callbacks; the shell
  invokes those owner registration functions and injects only live window and
  workspace actions. Adding a row no longer edits the Settings renderer,
  search index, or a shell-owned metadata list.
- Recorded the Background and Keymap UI dependencies on the Settings contract
  in the architecture graph. The contract exposes no Settings store or
  feature-private state.
- Updated the Settings contract, crosswalk, registry contract, glossary,
  universal checklist, architecture policy, and dependency contract.
- Focused tests passed for Settings, Background, Theme UI, Keymap UI, and Shell;
  owner tests assert that each module registers its own metadata. `cargo fmt
  --check`, `cargo check --workspace --all-targets`, workspace Clippy with
  warnings denied, and `cargo test --workspace` passed. Architecture, agents,
  Settings, visual-catalog, queue, and diff checks also passed. The
  documentation/knowledge checks still report the 21 existing links into the
  deleted `reference-src/` tree. The native app launched, but the screenshot
  helper found no layer-0 Labonair window; visual evidence remains at 0
  captures and 63 pending states, so rendered parity is unverified.

## Progress record — 2026-10-10 — Settings navigation and owner-action states

- Kept Settings page selection independent from section disclosure. The
  initial navigation is collapsed, selecting a category does not open its
  section anchors, and deep links/search reveal the page they target. This
  matches the pinned navigation behavior and keeps the rail compact.
- Owner-surface failures now keep their diagnostic log and also enter the
  canonical notification center, for both pointer and keyboard activation.
- Updated the Settings navigation contract. Focused Settings verification and
  queue checks pass; `cargo fmt --all --check`, `cargo test
  -p labonair-settings-ui` (29 tests), targeted Clippy, the Settings scope,
  the visual-catalog scope, and `git diff --check` pass. The visual catalog
  still has 0 captures and 63 pending states, so rendered parity remains
  unverified. The documentation checker reports the same 21 links into the
  removed `reference-src/` tree; the supplied repository rules forbid
  restoring that tree. The user screenshot cannot identify the exact
  executable and is not native acceptance evidence.

## Progress record — 2026-10-10 — Modified setting row actions

- Matched the pinned Settings row treatment for user/project overrides: the
  reset icon and “Modified in …” source label now sit beside the setting title;
  the value control remains in the trailing column. Reset supports pointer,
  Enter, and Space activation through the shared icon-button primitive.
- Kept source-of-value resolution in the Settings store and value clearing in
  the existing sparse-scope write path. Added a focused test for default,
  user, and project source labels, and updated the Settings guideline,
  glossary, and universal capture checklist.
- Verification passed: `cargo fmt --all --check`,
  `cargo test -p labonair-settings-ui` (30 tests),
  `cargo clippy -p labonair-settings-ui --all-targets -- -D warnings`,
  `python3 scripts/verify.py --scope settings`,
  `python3 scripts/verify.py --scope visual`, and `git diff --check`.
  Visual verification is still pending: the catalog has 0 captures and 63
  pending states, and the native capture helper could not find a layer-0 app
  window. The documentation checker still reports 21 links into the deleted
  `reference-src/` tree.

## Progress record — 2026-10-10 — Copyable Settings field links

- Added the pinned Settings row's hover copy-link affordance using the shared
  icon-button component. It copies a Labonair-owned
  `labonair://settings/<json-path>` URL and shows a copied check state.
- Added incoming URL routing at the app boundary and a thin shell forwarder to
  the Settings owner. The owner accepts the registered scheme only for known
  JSON field paths, opens or focuses its existing Settings window, selects the
  owning page, scrolls to the field, and highlights it. The macOS bundle now
  declares the `labonair` scheme in `Info.plist`.
- Updated the Settings contract, Settings guidelines, glossary, universal
  checklist, and packaging contract. Verification passed: `cargo fmt --all
  --check`, `cargo test -p labonair-settings-ui -p labonair-shell` (32 Settings
  tests, 16 Shell tests, and the Shell icon integration test), `cargo clippy
  -p labonair-settings-ui -p labonair-shell --all-targets -- -D warnings`,
  `cargo check -p labonair`, Settings/visual catalog/queue checks,
  `plutil -lint packaging/macos/Info.plist`, and `git diff --check`. Native
  visual acceptance remains pending at 0 captures / 63 states because the
  helper cannot find a layer-0 Labonair window.

## Progress record — 2026-10-10 — Shared dialog frame

- Added `modal_overlay` and `dialog_surface` to `labonair-ui-kit`. The shared
  primitives own the fixed centered scrim and token-bound card frame; each
  feature retains its dialog state, focus handling, dismissal, and actions.
- Replaced duplicated modal/card styling in SFTP permissions and properties,
  transfer resolution, Explorer delete confirmation, and Snippets' host picker
  with the shared primitives. Updated architecture, design-system, glossary,
  and dialog acceptance criteria.
- Focused verification passed: UI-kit (57 tests), Workspace (161), Transfers
  UI (7), Explorer (22), Snippets (15), targeted Clippy with warnings denied,
  `cargo check -p labonair`, formatting, and `git diff --check`. Paired native
  visual review remains pending; the capture catalog still has no captures.

## Progress record — 2026-10-10 — Compact text-field frame

- Added shared compact and standard text-field frame sizes to the UI kit.
  Source Control and Snippets now use the shared compact height, padding,
  border, surface, radius, and focus treatment instead of local field chrome.
- The two panels still route key events into plain strings. Text selection,
  cursor movement, clipboard editing, IME composition, and undo/redo therefore
  remain missing in those fields; this change only unifies presentation and
  leaves native `InputState` migration as a required keyboard-parity item.
- Focused verification passed: UI-kit (57 tests), SCM (18), Snippets (15),
  targeted Clippy with warnings denied, formatting, the visual-catalog scope,
  queue validation, and `git diff --check`. The visual catalog still has 0
  captures and 63 pending states. The documentation check remains blocked by
  the 21 existing links into the deleted `reference-src/` tree.

## Progress record — 2026-10-10 — Settings text-field geometry

- Matched the shared standard text field to the pinned Settings control's
  declared contract: 32-pixel base height, 8/4-pixel inner padding, medium
  radius, and 256-pixel minimum width. Settings no longer forces text editors
  to 220 pixels. Standard and compact input frames now scale their geometry
  with the active UI font and density; input text tracks the UI font scale.
- Updated the universal control checklist to require recording field bounds,
  padding, radius, scaling, clipboard, IME, and undo/redo behavior. The source
  review provides structural evidence only; rendered parity remains pending.
- Focused verification passed: Settings/UI-kit tests (89), targeted Clippy
  with warnings denied, `cargo check -p labonair`, formatting, queue validation,
  and the visual-catalog scope. The catalog has 0 captures and 63 pending
  states; the user-provided Labonair screenshot is not a paired capture with a
  pinned Zed build under recorded matching conditions. The documentation
  checker still reports 21 links into the deleted `reference-src/` tree.

## Progress record — 2026-10-10 — Settings section row geometry

- Matched the shared Settings value and owner-action row layout to the pinned
  section structure: 16 logical pixels above each row, 16 below rows within a
  section, 40 below the final row, and a divider only between rows in that
  section. Removed the extra vertical padding from section headings. Search
  results use the same row decoration as generated pages.
- Added a unit check for inner-row dividers and section-end spacing. Updated
  the design-system contract and universal capture checklist; paired visual
  confirmation remains pending.
- Verification passed: 90 focused Settings/UI-kit tests, `cargo test --workspace`,
  workspace `cargo check --all-targets`, workspace Clippy with warnings denied,
  formatting, visual-catalog validation, queue validation, and `git diff
  --check`. `cargo run -p labonair` compiled and started, but CUA exposed no
  native apps and the screenshot helper could not inspect the process list
  (`sysmond service not found`), so there is no window ID or capture. The
  documentation checker still reports 21 existing links into the deleted
  `reference-src/` tree.

## Progress record — 2026-10-10 — Snippets form text editors

- Replaced Snippets create/edit form's simulated text rows with shared UI-kit
  text fields backed by native `InputState`. Name, description, and working
  directory retain compact geometry; Command uses the growing multiline
  editor. The Snippets owner still validates and persists the form values.
- Focused verification passed: Snippets (15 tests), package Clippy with
  warnings denied, `cargo check -p labonair`, formatting, and `git diff
  --check`. The native app launched and the exact-process screenshot helper
  captured `/private/tmp/labonair-snippets-form.png` (PID 30354); it shows the
  initial workspace, not the Snippets form, so this change's rendered and
  editing states remain visually unverified. CUA still exposes no apps.

## Progress record — 2026-10-10 — Git Graph branch prompt input

- Replaced the simulated character buffer in Git Graph's “Create Branch Here”
  prompt with a native UI-kit `InputState` editor and the shared text-field
  frame. The Git Graph owner still validates the submitted name and performs
  the branch operation; Enter submits through the owner key handler and Escape
  dismisses the prompt.
- Focused verification passed: Git Graph (10 tests), package Clippy with
  warnings denied, `cargo check -p labonair`, formatting, and `git diff
  --check`. The native UI surface reports no available apps, so the changed
  prompt could not be captured; its rendered parity remains pending. The
  documentation checker still reports the 21 existing links into the removed
  `reference-src/` tree; the active rework queue check passes.

## Exit condition

This task is complete only when the full feature inventory has no unclassified
areas, all prior Settings attempts have an evidence-based disposition, the
source-blind implementation packet is ready, and the next implementation task
can begin without inventing a missing product or ownership decision. If native
capture access remains blocked, the blocker and owner must be explicit; visual
parity itself cannot be claimed from incomplete evidence.
