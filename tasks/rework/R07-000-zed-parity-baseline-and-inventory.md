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

## Progress record — 2026-10-10 — Titlebar Editor-tab parity

- Inspected the user-supplied Zed tab-row image and the pinned `3569541` source.
  The image shows a flat dark strip, separate back/forward controls, content-
  sized file tabs, muted inactive labels, and an italic active preview. Native
  reference launch is unavailable in this session because the computer-control
  runtime has no `getApp` function and reports no app windows.
- Added the source-side comparison at
  [`../../docs/reports/zed-titlebar-tabbar-analysis-2026-10-10.md`](../../docs/reports/zed-titlebar-tabbar-analysis-2026-10-10.md)
  and the independent behavior/acceptance packet at
  [`../../docs/parity/titlebar-tab-strip.md`](../../docs/parity/titlebar-tab-strip.md).
  This is recorded as an independent rewrite by one contributor, not a
  separated-room implementation.
- Change contract: Workspace (`labonair-workspace`) owns tab order, active
  Space, focus, close policy, and context actions; Shell (`labonair-shell`)
  owns permanent titlebar placement, global menu, and window gestures;
  `labonair-ui-kit` owns reusable horizontal tab/button/menu chrome. Existing
  `TabStore`, `TabKind`, `TabData`, `ActiveTabChanged`, and `TitlebarEvent`
  contracts remain canonical. No dependency, settings, persistence, registry,
  or notification changes are planned. Reuse existing Previous/Next Tab
  commands, close lifecycle, tab-menu owner actions, and theme tokens.
- UI-kit/theme work: use toolbar/background/border/foreground/muted/accent roles
  from `Palette`; leave vertical tab rows unchanged. Workspace work: retain
  Spaces, new-tab menu, global menu, preview/dirty state, drag reorder, and
  existing keyboard routes; double-click promotes previews and enters inline
  rename. Add navigation affordances and complete the current-Space
  close-left/right/clean menu workflow with sequential dirty-tab confirmations.
  No Editor internals or new keymap commands are required.
- Evidence plan: UI-kit tab and Workspace tab-state coverage, Rust format/check/
  clippy/test gates, catalog/queue/documentation checks, and exact native visual
  captures for hover, preview, dirty, rename, context menu, navigation, narrow,
  Zen, and sidebar states. Visual evidence remains Pending until a native
  Labonair window can be captured.

## Progress record — 2026-10-10 — Stable macOS action requirement IDs

- Added [`../../docs/parity/action-requirements.md`](../../docs/parity/action-requirements.md)
  with stable IDs for all 585 distinct action names bound in the pinned default
  macOS profile. The register preserves each key sequence and active context
  and defines the per-action workflow, state, settings/persistence, owner, and
  evidence fields required before acceptance.
- The IDs close only the inventory-identity gap. All 585 action effects,
  exact Labonair owners/routes, context availability, settings effects, and
  runtime acceptance remain Pending. Unbound actions and Windows/Linux
  profiles remain outside this register and are still required by the user
  objective.
- Updated the independent parity index and command/keymap crosswalk. Analysis
  and implementation occurred in the same contributor session; the work is
  recorded as independent redevelopment and does not claim separated-room
  clean-room provenance. No source-blind checkout was used.
- This session has no general native UI interaction/capture tool available.
  The prior acceptance record documents the host's missing app-window access
  and denied Screen Recording grant; paired pinned Zed/Labonair evidence stays
  Pending with the host operator as owner.
- Replaced dead hyperlinks into the absent `reference-src/` checkout with
  non-clickable historical path text; no files in that frozen tree were
  restored or edited. `python3 scripts/check_documentation.py` and
  `python3 scripts/verify.py --scope docs` pass (43 normative documents, 163
  Markdown files); `python3 scripts/check_rework_queue.py` passes (46 tasks,
  R07-000 active), and `git diff --check` passes.
- R07-000 remains In Progress. No R09 work has started; R07-001 remains next
  after this inventory task.

## Progress record — 2026-10-10 — Default platform keymap inventory

- Initialized the parent gitlink at the exact pinned Zed commit for authorized
  static reference analysis. The Zed checkout remains an analysis input; the
  same-session independent-redevelopment provenance limitation above still
  applies.
- Expanded the stable action register to all three default operating-system
  keymaps. The pinned files contain 930 macOS, 931 Linux, and 870 Windows
  active bindings, referring to 585, 561, and 557 distinct actions. Their
  union has 595 action names, with 546 common to all three. The register now
  tracks all 2,731 active binding records, including contexts and arguments,
  plus four explicit null-key entries.
- The 585 macOS action IDs are unchanged; ten additional Linux/Windows action
  names receive stable IDs. Static keymap inventory is now complete for these
  three defaults. Semantics, runtime reachability, local owner/route, alternate
  profiles, unbound registered actions, and all user overrides remain open.
- The current execution host is Linux and has no macOS screen-capture tool, so
  it cannot provide the required macOS native interaction or paired-capture
  evidence. The previously recorded host Screen Recording/app-window blocker
  remains assigned to the macOS workspace operator.
- R07-000 remains In Progress; no R09 task has started.



## Progress record — 2026-10-10 — Pinned Linux action registry

- Built the exact pinned Zed debug executable and ran `--dump-all-actions` on
  Linux x86_64. The host had the versioned `libX11-xcb.so.1` runtime but no
  development symlink; a temporary linker symlink under `/tmp` allowed the
  final link without installing packages or changing the pinned checkout. The
  command was run from `zed-refrence/zed/` with `LIBRARY_PATH=/tmp/labonair-native-libs`
  and `CARGO_TARGET_DIR=/tmp/labonair-pinned-zed-target`.
- Preserved the successful 447,231-byte runtime metadata output and schemas at
  [`../../docs/parity/evidence/zed-actions-3569541-linux-x86_64.json`](../../docs/parity/evidence/zed-actions-3569541-linux-x86_64.json)
  (SHA-256 `fda3ca179a9e6a27508646fde530a5ebf956252020d5505f45e2cfea60620db5`).
  The runtime registered 1,404 actions; its metadata contains documentation for
  1,338 and argument schemas for 187.
- Reconciled all three default-map name sets with runtime-declared deprecated
  aliases: 590 names match directly, five aliases resolve to three registered
  actions, and 591 distinct registered actions are referenced by the defaults.
  Added stable IDs for the remaining 813 Linux-registered actions in
  [`../../docs/parity/registered-actions-unbound.md`](../../docs/parity/registered-actions-unbound.md).
- This catalog proves Linux action registration and preserves metadata. It
  does not prove action effects, keymap dispatch, user reachability, macOS
  registration, Labonair ownership/routes, or end-to-end behavior; those remain
  Pending. Alternate profiles and user overrides also remain open.
- Updated the parity index, feature crosswalk, and normative parity inventory.
  R07-000 remains In Progress; no R09 task has started.

## Progress record — 2026-10-10 — Settings requirement IDs

- Added [`../../docs/parity/settings-requirements.md`](../../docs/parity/settings-requirements.md)
  with 486 stable requirement IDs: 382 static field/control rows, 43 typed
  per-language options, 44 shipped language profiles, 10 dynamic setup pages,
  and 7 shared Settings flows.
- Preserved the two Collaboration Panel controls that share the declared
  `collaboration_panel.dock` path as separate requirements. The second row's
  default references `default_width`; the effective path is an open runtime
  question.
- The inventory records defaults and declared scopes where available. Exact
  field owners, value consumers, validation, reset/inheritance, persistence,
  migrations, runtime effects, and paired macOS captures remain Pending.
- Updated the Settings index and feature crosswalk. R07-000 remains In
  Progress; no R09 task has started.

## Pre-change contract — expose the existing Editor format-on-save setting

```text
Request: Expose the existing consumer-backed format-on-save value in Settings.
Request type: Owner-backed Settings field and category placement.
In scope: One Editor-page switch for editor.editorFormatOnSave.
Out of scope: Adding formatter behavior or language-specific policy; enabling
  auto-save, trailing-whitespace removal, or final-newline settings without
  verified consumers; reproducing Zed's multi-mode/per-language formatter model.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Existing Editor save path, SettingsContent value, and SettingsStore.
Owner module: Editor.
Canonical capability crate: labonair-editor.
Sibling crates: labonair-settings-content, labonair-settings,
  labonair-settings-ui, labonair-workspace (hosting adapter).
Canonical user entry point: Settings → Editor → Formatting & Save.
Alternate entry points: User or Project settings file for editorFormatOnSave.
Public typed contract: SettingsContent.editor.editor_format_on_save and
  EditorSettings::format_on_save().
Events: Existing SettingsStore observer refreshes Editor preferences; add none.
Registry contributions: None.
Allowed dependency changes: None.
Removal condition for temporary code: Not applicable.
State owner: Editor.
Persistence owner: Settings and settings-content; preserve existing JSON key,
  default, project whitelist, and migration behavior.
Settings and scope: Boolean, default false, User and Project scopes supported.
Migration impact: None; existing compatibility/migration code remains in place.
Commands: None.
Default keymap: None.
Notifications: Preserve the existing notification when formatting is unavailable
  or fails and the current text is saved.
UI-kit components: Existing Settings field renderer and shared Switch.
User workflow: Open Settings → Editor, toggle Format on Save, then save a file;
  Editor requests formatting asynchronously and saves after applying edits.
Failure/cancellation/retry: On a formatter error or unsupported capability,
  notify the user and save the current text once without retrying the formatter.
Security boundary: No new external access or secret handling.
Performance budget: Keep formatter work asynchronous; do not block GPUI.
Implementation order: Remove the field from the hidden-field filter, place it
  in the Editor Formatting & Save group, and link its requirement record.
Obsolete path removed: None.
Focused tests: Existing Settings field registration and Editor save-policy tests.
Contract/integration tests: Existing SettingsStore-to-Editor live-update path.
Persistence/migration tests: Existing schema/migration behavior; no changes planned.
Visual states: Off/on, focus, User/Project scope, narrow layout, formatter absent,
  and formatter error; paired macOS capture remains required.
Security/negative tests: Not applicable beyond existing formatter failure path.
Performance measurement: No new benchmark; preserve background service execution.
Workspace gates: Docs and queue scopes; Rust gates at R07-000 acceptance.
Documentation updates: Settings requirement register, Settings crosswalk, this log.
Current status: Pre-change; current field is filtered from the UI.
Target status: Visible Editor setting backed by the existing live save consumer.
Known limitations: Local value is Boolean and global; pinned behavior includes
  richer modes and per-language overrides, which remain separate requirements.
Failed/unavailable gates: Native paired macOS visual capture is unavailable here.
Next safe task: Continue R07-000 Settings field/runtime acceptance.
```

## Progress record — 2026-10-10 — Editor format-on-save Settings route

- Exposed the existing `editor.editorFormatOnSave` switch in the shared
  Settings UI under Settings → Editor → Formatting & Save. Its schema, default,
  persistence path, project scope, live SettingsStore observer, and asynchronous
  formatter/save behavior already existed; this change adds no setting,
  behavior, dependency, event, or registry.
- Added a Settings page-location regression assertion for the Editor group.
  Formatter unavailability or failure continues to use the existing notification
  and saves the current text without another formatting attempt.
- Recorded this as Labonair-only ID `LAB-SET-EDITOR-FORMAT-ON-SAVE`, not as an
  additional pinned-baseline requirement. It is only a partial counterpart to
  `SET-LANGUAGE-FIELD-format-on-save`; per-language modes and overrides remain
  open.
- Passed for this slice: `cargo fmt --check`, `cargo test
  -p labonair-settings-ui` (33 passed), `cargo clippy -p
  labonair-settings-ui --all-targets -- -D warnings`, `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py`, and `git diff --check`.
  Workspace-wide check, clippy, and test gates remain part of R07-000's finish
  gates. Native macOS visual and interaction evidence is unavailable on this
  host and remains an open acceptance gate.
- R07-000 remains In Progress; no R09 task has started.

## Pre-change contract — inventory the macOS keymap layers

```text
Request: Record every pinned macOS alternate-profile and conditional keymap binding.
Request type: Independent reference requirement inventory.
In scope: Seven macOS base-profile overlays, the conditional Vim/Helix layer,
  and macOS-specific overrides; stable IDs for every key/context/action/argument
  or explicit null binding; links to existing action requirement IDs through
  direct registration or declared alias metadata; record platform registration
  gaps without inventing a binding action ID.
Out of scope: Keymap runtime implementation, claims about action effects from
  binding presence alone, macOS dispatch or visual acceptance, Linux/Windows
  alternate-profile layers, and user-authored keymap contents.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Pinned platform maps and runtime action catalog; Keymap owner contract.
Owner module: Keymap.
Canonical capability crate: labonair-keymap.
Sibling crate: labonair-keymap-ui.
Canonical user entry point: Titlebar global menu → Keymap; Settings → Keymap;
  Command Palette quick access.
Public typed contract: Existing KeymapDocument, KeymapManagementSnapshot,
  command/action registry, and Workspace Keymap tab composition; profile-layer
  acceptance remains pending.
Events: Existing file-watch/reload flow only; add none.
Registry contributions: No changes.
Allowed dependency changes: None.
State owner: Keymap.
Persistence owner: Keymap owns user binding persistence and runtime composition.
  The profile/mode value and persistence boundary remains open under the
  Settings and Keymap owner contracts; do not assume a local profile field.
Settings and scope: Profile selection and Vim/Helix mode are inventoried as
  behavior conditions; exact local profile parity remains open.
Migration impact: None.
Commands: No new commands; every inventoried action links to its action ID.
Default keymap: Preserve the pinned layer order and null bindings as requirements.
Notifications: No changes.
UI-kit components: No changes.
Security boundary: User keymap files are untrusted input; no parser or runtime
  behavior is changed in this inventory step.
Performance budget: No runtime work added.
Implementation order: Count the pinned macOS layers, add profile/layer and
  per-binding stable IDs, link actions, and record direct/alias registration
  evidence plus unresolved macOS runtime differences.
Focused tests: Inventory count, ID uniqueness, action-ID resolution, and docs checks.
Workspace gates: Documentation and queue scopes; full Rust gates at R07-000 exit.
Documentation updates: Action register, overlay binding register, parity index,
  unbound-action limitation, and this task log.
Current status: Profile overlays are named but their binding records are absent.
Target status: Exact static macOS layer/binding inventory with explicit open
  runtime dispatch, semantic, and native-capture acceptance.
Current Labonair gap: Keymap has the Workspace editor and default platform maps,
  but no base-profile loader/setting; Editor has `editor.vimMode`, with no
  Helix mode or combined Vim/Helix layer.
Known limitations: Analysis and implementation are in the same session; this is
  independently written requirements work, not a separated clean-room claim.
Failed/unavailable gates: Native macOS interaction/capture is unavailable here.
Next safe task: Continue R07-000 action semantics, runtime evidence, and full
  platform-profile inventory.
```

## Progress record — 2026-10-10 — macOS keymap overlay inventory

- Added [`../../docs/parity/keymap-overlay-requirements.md`](../../docs/parity/keymap-overlay-requirements.md)
  with 1,402 stable binding IDs across seven macOS base-profile overlays, the
  conditional Vim/Helix layer, and macOS-specific overrides. The register
  preserves 1,377 action bindings, 25 explicit nulls, action arguments, exact
  contexts, and each overlay block's key-equivalent setting.
- Added 133 stable IDs for the default macOS map's context blocks and their
  key-equivalent flags: 109 explicitly enabled and 24 not set. The default map
  has 585 action identifiers; its specific override layer adds 12 distinct
  identifiers, yielding 597 in the built-in default composition before
  optional profiles or modes.
- Reconciled 578 raw overlay action identifiers against the pinned Linux
  runtime: 576 direct registrations and two declared deprecated aliases. Of
  the overlay identifiers, 312 are registered actions absent from all three
  default-map files; their binding rows now link to the existing Linux
  registry IDs. macOS registration and dispatch remain unverified.
- Recorded `BaseKeymap::None` as a required profile state: it is in the pinned
  typed setting but omitted from the macOS selector options. Its Settings
  reachability and runtime behavior need acceptance evidence.
- Updated the default action register, unbound-action scope note, parity index,
  and feature crosswalk. Linux/Windows alternate-profile maps, per-action
  semantics/owners/routes, user-keymap behavior, and native macOS interaction
  evidence remain open. R07-000 remains In Progress; no R09 task has started.
- Passed `python3 scripts/verify.py --scope docs` (43 normative docs and 166
  Markdown files), `python3 scripts/check_rework_queue.py` (46 tasks; active
  R07-000), `git diff --check`, and a static audit confirming 1,402 unique
  binding IDs, 133 unique default-map block IDs, and 1,377 resolved action-ID
  references. Rust workspace gates are still required at R07-000 exit. Native
  macOS interaction and paired capture remain unavailable on this host.

## Pre-change contract — inventory Linux and Windows keymap layers

```text
Request: Complete the static alternate-keymap inventory for non-macOS platforms.
Request type: Independent platform requirements inventory.
In scope: Six non-macOS base-profile overlays shared by Linux and Windows, their
  shared platform override layer, default Linux/Windows context-block key-equivalent
  metadata, stable per-binding and per-block IDs, and action-ID cross-references.
  Reuse the existing Vim/Helix layer IDs rather than duplicating its shared map.
Out of scope: Changing keymap or settings code, claiming Linux/Windows action
  effects from registration, native Windows runtime/visual acceptance, user map
  contents, and per-action behavior/owner implementation.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Existing default action register, macOS overlay register, pinned
  Linux runtime action catalog, and Keymap capability contract.
Owner module: Keymap.
Canonical capability crate: labonair-keymap.
Sibling crate: labonair-keymap-ui.
Canonical user entry point: Keymap owner editor, reached from the titlebar
  menu, Settings link, and Command Palette.
Public typed contract: Existing KeymapDocument, KeymapManagementSnapshot,
  command/action registry, and Workspace Keymap tab; profile composition is
  still absent locally.
Events: No additions; record the existing user-file reload path only.
Registry contributions: No changes.
Allowed dependency changes: None.
State owner: Keymap; base-profile persistence and selection contract remain open.
Persistence owner: Keymap owns user binding persistence; do not assume the
  profile value is currently stored in Settings.
Settings and scope: The pinned base_keymap choices and None/TextMate behavior
  are requirements; local equivalents are unimplemented or unverified.
Migration impact: None.
Commands: No additions.
Default keymap: Preserve default Linux/Windows maps, overlay contexts,
  arguments, nulls, and exact key-equivalent flags.
Notifications: No additions.
UI-kit components: No changes.
Security boundary: No parser or runtime behavior changes in this inventory step.
Performance budget: No runtime work added.
Implementation order: Record counts and options, assign stable IDs to all
  non-macOS overlay rows and default-map block flags, resolve action IDs, and
  document platform-specific runtime gates.
Focused tests: Inventory counts, ID uniqueness, action-ID resolution, and docs checks.
Workspace gates: Documentation and queue scopes; full Rust gates at R07-000 exit.
Documentation updates: New non-macOS keymap register, parity index, crosswalk,
  and this task log.
Current status: Default maps are inventoried, but Linux profile overlays,
  shared non-macOS overrides, and non-macOS block flags are not separately
  listed; TextMate availability differs by platform.
Target status: Exact non-macOS static maps with explicit runtime and platform
  option gaps.
Known limitations: Same-session analysis and implementation; no separated
  clean-room provenance claim.
Failed/unavailable gates: Windows native runtime/capture and paired visual
  evidence are unavailable on this host.
Next safe task: Continue R07-000 action semantics, remaining platform research,
  and runtime acceptance.
```

## Progress record — 2026-10-10 — Linux and Windows keymap inventory

- Added [`../../docs/parity/keymap-nonmacos-requirements.md`](../../docs/parity/keymap-nonmacos-requirements.md)
  with 446 stable binding IDs for six shared Linux/Windows base-profile
  overlays and their shared override layer: 429 action bindings and 17 nulls.
  It links the 894 already inventoried shared Vim/Helix rows instead of
  duplicating their IDs; all conditional non-macOS layers total 1,340 entries.
- Added 259 default-map context-block IDs for key-equivalent behavior: 130
  Linux blocks (35 explicitly enabled, 95 not set) and 129 Windows blocks
  (112 enabled, 17 not set). Default map actions, contexts, and arguments
  remain linked from the existing default action register.
- Reconciled 217 raw non-macOS overlay action identifiers against Linux
  registry evidence: 215 direct registrations and two deprecated aliases.
  Thirty-two spellings are absent from all three default maps; these sets
  overlap with macOS profile-only IDs and are not additive action counts.
- Recorded the non-macOS TextMate mismatch (typed enum value with no selector
  option or overlay asset), the pinned `None` state, and current Labonair gaps:
  no base-profile/overlay loader or Helix mode; existing `editor.vimMode` is
  only a partial Editor-owned counterpart.
- Updated the action inventory, unbound-action scope note, parity index,
  feature crosswalk, and this task log. Linux and Windows dispatch, per-action
  semantics/owners/routes, profile selection and persistence, and Windows
  native evidence remain open. R07-000 remains In Progress; no R09 task has
  started.
- Passed `python3 scripts/verify.py --scope docs` (43 normative docs and 167
  Markdown files), `python3 scripts/check_rework_queue.py` (46 tasks; active
  R07-000), `git diff --check`, and a static audit confirming 1,848 unique
  binding IDs, 392 unique default-map block IDs, and 1,806 resolved action-ID
  references across both overlay registers. Workspace Rust gates and all
  pinned runtime/visual acceptance remain open.

## Pre-change contract — classify the default-macOS Search action family

```text
Request: Independently state each Search-family action's user goal and map its
  current Labonair owner, route, implementation evidence, and open runtime gate.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: All 23 distinct macOS-default-bound buffer_search, project_search,
  and search actions; retain their existing action IDs and exact binding rows;
  distinguish catalog-described intent, Labonair source coverage, and runtime
  acceptance; identify unsupported interactions rather than treating names as
  proof of behavior.
Out of scope: Implementing action handlers, changing keymaps, starting R09,
  claiming pinned macOS dispatch, or claiming native visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, pinned Linux action metadata, Editor and
  Workspace owner contracts, current SearchOverlay and FileFinder source.
Owner module: Context-specific feature owner. Editor owns buffer/project search;
  Terminal owns terminal-search state; Workspace composes the cross-pane Find
  entry; other generic search-field actions remain with their host feature.
Canonical capability crate: labonair-editor for Editor search state; existing
  terminal, workspace, AI, Settings, and Keymap owners for their own contexts.
Canonical user entry point: Existing Find command / SearchOverlay for Editor
  and Terminal; no parallel entry point is introduced.
Public typed contract: Existing SearchQuery, SearchOptions, ProjectSearchQuery,
  ProjectSearchSession, and Editor view operations.
Events: None added.
Registry contributions: None added. The one-to-one pinned action/key route gap
  is recorded instead of adding speculative registrations.
Allowed dependency changes: None.
State owner: The capability named by the active action context; Editor retains
  Editor buffer/project query, match, and replacement state.
Persistence owner: No durable query-history store exists. Workspace retains one
  process-local last query only.
Settings: SearchOptions are transient; their defaults and missing durable
  history/setting behavior are recorded in the requirements packet.
Migration impact: None.
Commands: Preserve CommandId::Find; classify every exact pinned action ID and
  default macOS key/context through the existing action register.
Notifications: Preserve invalid-pattern and project-search notification paths.
UI-kit components: Existing SearchOverlay controls; no new components.
User workflow: Open contextual search, edit query, navigate or replace results,
  adjust matching options, dismiss, and recover from no-match/error/loading.
Failure/cancellation/retry behavior: Record current model/source support and
  unverified pinned/native behavior separately.
Security boundary: Project-result paths must remain rooted in the Workspace;
  no new file or external-service access is added.
Performance budget: Preserve async project grep, generation checks, and result
  caps; no new measurement claim.
Removal condition for compatibility code: N/A.
Focused tests: Existing Editor find/replace/invalid-regex and project-session /
  filesystem-adapter tests, linked as evidence only; none claims UI acceptance.
Visual evidence: Pending paired native macOS interaction/capture for both apps.
Workspace gates: Documentation, queue, and diff checks for this inventory slice.
Documentation updates: New action classification packet, action register,
  parity crosswalk/index, and this progress record.
Current status: Static bindings are enumerated, but Search semantics, context
  ownership, local correspondences, and remaining gaps are not mapped.
Target status: Every in-scope action has an independent user-goal statement,
  explicit owner/route disposition, code/test trace, and separate runtime gate.
Known limitations: Runtime and native visual sessions are unavailable here;
  source inspection is not runtime observation. Same-session work is independent
  redevelopment, without a separated-room provenance claim.
Failed/unavailable gates: Pinned macOS dispatch and paired native visuals remain
  pending; no action is accepted by this inventory alone.
Next safe task: Continue with another default-macOS action family and the
  registered-but-unbound action catalog while preserving R07 queue order.
```

## Progress record — 2026-10-10 — default-macOS Search action classification

- Added [`../../docs/parity/action-classification-search.md`](../../docs/parity/action-classification-search.md)
  with independently stated intent, context-specific Labonair ownership, and
  source-checked coverage/gap records for all 23 distinct default-macOS
  `buffer_search::*`, `project_search::*`, and `search::*` action IDs.
- The crosswalk traces `CommandId::Find`, Editor-owned query/match/replace and
  project-result state, Workspace's overlay and filesystem adapter, process-
  local last-query memory, current query defaults, and the relevant regression
  tests. It identifies missing per-action routes, query history, result picker/
  tabs/filter controls, selection-only and multi-match selection, and project
  replacement without claiming that key names establish runtime behavior.
- Added an explicit test-to-requirement map: six actions have related
  model-level regression evidence; the other 17 have no action-specific test,
  and all 23 lack native interaction and paired visual evidence.
- Linked the slice from the action register, feature crosswalk, parity index,
  and normative parity scope. Existing action IDs and binding/context rows are
  preserved. Remaining action semantics, owner routes, and all pinned runtime
  and visual acceptance are still open.
- Passed for this documentation-only slice: `python3
  scripts/check_documentation.py` (43 normative docs and 168 Markdown files),
  `python3 scripts/verify.py --scope docs`, `python3
  scripts/check_rework_queue.py` (46 tasks; R07-000 active), `git diff --check`,
  and a custom action-ID coverage audit (23 register rows match 23 unique
  crosswalk IDs) plus the test-coverage audit (six IDs map to related model
  tests; 17 explicitly have no action-specific regression test).
- Not run for this documentation-only slice: Cargo checks, clippy, and tests;
  R07-000's workspace Rust gates remain required before task exit. Unavailable:
  pinned macOS dispatch and paired native visuals/interaction. The current host
  is Linux with neither `DISPLAY` nor `WAYLAND_DISPLAY`. No action is
  runtime-observed. R07-000 remains In Progress; R07-001 and all R09 tasks
  remain unstarted.

## Pre-change contract — classify default-macOS Pane actions

Task: document the independently stated goals, current owner routes, source
coverage, and acceptance gaps for the 22 default-macOS `pane::*` actions.
This is an R07-000 requirements-inventory slice; it does not implement action
handlers or claim runtime parity.

Dependencies: `docs/parity/action-requirements.md`, pinned action metadata,
`docs/workspace-model.md`, ADR 0004, and the Workspace, Editor, Explorer,
Search, and Terminal owner contracts.
Owner module: Context-specific. Workspace (`labonair-workspace`) owns the
universal tabs and outer panes; Editor (`labonair-editor`) owns editor-internal
groups and search state; Explorer owns project-tree reveal; the active view
owner keeps its navigation history. Workspace hosts the existing Search
overlay and terminal pane layout.
Canonical user entry point: Existing universal tab strip/context menu,
registered Workspace commands, Editor split commands, Explorer active-file
following, and Workspace `Find`. No parallel entry point is introduced.
Public typed contract: Existing `TabStore`, `WorkspaceLayout`, Workspace
command descriptors, Editor `EditorSplitTree`/`EditorCommand`, Explorer
settings/view, and Search contracts.
Events: None added.
Registry contributions: None added. Record the lack of one-to-one action
routes without adding speculative aliases or registrations.
Allowed dependency changes: None.
State and persistence owners: Workspace owns universal tab and outer Terminal
pane session state; Editor owns internal split state and search state; Explorer
owns project-tree state; each active view owns its own navigation state. No
general pin or closed-item history is added.
Settings: No values or defaults changed. Record the existing Explorer
auto-reveal default only as related behavior.
Migration impact: None.
Notifications: Preserve existing dirty-close and Search error routes; add none.
UI-kit components: Existing shared tab, menu, pane, focus, and Search controls.
Failure/cancellation/retry: Specify dirty-close cancellation, missing history,
unavailable Explorer targets, search errors, and pane creation failure as
acceptance states; keep unobserved behavior marked pending.
Security and performance: No new filesystem access or performance path; retain
existing project-root safety and asynchronous Search behavior.
Removal condition for compatibility code: N/A.
Focused tests: Trace existing TabStore, Workspace descriptor, and pane geometry
tests as related evidence only; do not claim action-dispatch coverage.
Visual evidence: Pending paired native macOS interaction and captures for all
relevant states.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, action-ID audit, and test-trace audit.
Documentation updates: Pane classification packet, action register, parity
index/crosswalk, normative parity scope, and this progress record.
Current status: 22 IDs have static semantic and owner/route mapping; none has
pinned macOS dispatch or native interaction evidence in this environment.
Target status: Every ID has a traceable goal, owner route, source/test
correspondence, and separately recorded runtime/visual acceptance.
Known limitations: Analysis and implementation remain same-session independent
redevelopment, not separated-room clean-room work. Host is Linux without a
display server; no paired native capture is available.
Next safe task: Continue with the next default-macOS action family or the
registered-but-unbound action catalog while R07-000 remains active.

## Progress record — 2026-10-10 — default-macOS Pane action classification

- Added [`../../docs/parity/action-classification-pane.md`](../../docs/parity/action-classification-pane.md)
  with independently stated user goals, per-action Labonair owner/crate,
  existing route and source correspondence, related tests, and explicit
  unverified interaction states for all 22 default-macOS `pane::*` IDs.
- Kept the accepted universal tab strip, Workspace outer-pane tree, and
  Editor-owned groups distinct. The packet also maps Explorer reveal, the
  Workspace-hosted Search overlay, and active-view navigation ownership.
- Updated all 22 action-register rows to link the static crosswalk while
  preserving their action IDs and platform binding/context/argument records.
  Every row remains `Pending (macOS interaction not observed)`.
- Linked the packet from the parity index, feature crosswalk, and normative
  parity scope. The per-action audit found exact agreement between 22 unique
  register IDs and 22 unique crosswalk IDs; all 22 have an explicit owner.
- Test traceability maps 14 action IDs to related model or descriptor tests.
  None has an action-specific regression or pinned-key dispatch test, and all
  22 lack native interaction and paired visual evidence.
- Passed: `python3 scripts/check_documentation.py` (43 normative documents,
  169 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 remains active),
  `git diff --check`, and the custom ID/owner/test/runtime-status audit.
- Not run for this documentation-only slice: Cargo formatting, workspace
  check, Clippy, and tests. They remain required before R07-000 exit.
  Unavailable: pinned macOS dispatch and paired native interaction/captures;
  this host is Linux without `DISPLAY` or `WAYLAND_DISPLAY`. No action is
  runtime-observed. Same-session work is independent redevelopment and does
  not claim separated-room clean-room provenance. R07-001 and R09 remain
  unstarted.

## Pre-change contract — classify Editor Go to Line

Request: Independently state the pinned `go_to_line::Toggle` user goal and
map its current Editor owner, route, source correspondence, and acceptance gap.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-go_to_line-Toggle`, its default binding/context, arbitrary
line/column and relative navigation workflow, current Labonair caret operations,
related route/test evidence, and pending runtime/visual acceptance.
Out of scope: Implementing a prompt or action handler, changing keymaps,
starting R07-001/R09, or claiming pinned dispatch or native visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, pinned action metadata, Editor and
Workspace contracts, and the current Outline-to-line palette route.
Owner module: Editor owns document caret/navigation state; Workspace composes
the current Editor view and typed command-palette action.
Canonical capability crate: labonair-editor; current view integration is in
labonair-workspace.
Canonical user entry point: No direct Go to Line prompt or toggle is
registered. Outline symbol selection jumps to a known line only.
Public typed contract: Editor Position/goto_line/goto_position operations and
Palette SubmenuAction::GoToLine(usize), which currently carries a line only.
Events: None added.
Registry contributions: None added.
Allowed dependency changes: None.
State and persistence: Editor owns the caret. The query and preview are
transient; no setting or persistence schema is added.
Notifications and failure: Preserve existing caret operation behavior; record
the missing invalid-query, cancel, and focus behavior as acceptance work.
UI-kit: No direct Go to Line input exists; any future prompt must use the
shared input controls and remain in the Editor owner.
Workspace and shell: Workspace composes the active Editor view and applies
typed Outline actions; no shell-wide action conditional is added.
Security and performance: No new filesystem or external-service access.
Focused tests: Inspect existing route and operation tests; map none as direct
action-dispatch coverage when absent.
Visual evidence: Pending paired native macOS interaction and captures.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, action-ID audit, and test-trace audit.
Documentation updates: Go to Line packet, action register, parity
index/crosswalk, normative parity scope, and this progress record.
Known limitations: Same-session analysis and implementation are independent
redevelopment, not separated-room clean-room work. The Linux host has no
display server or paired native capture.
Next safe task: Continue with another default-macOS action family while
R07-000 remains active.

## Progress record — 2026-10-10 — default-macOS Go to Line action classification

- Added [`../../docs/parity/action-classification-go-to-line.md`](../../docs/parity/action-classification-go-to-line.md)
  with the independently stated line/column/relative-navigation goal, Editor
  owner, current route, related local code, and required runtime states for
  `ACT-MACOS-go_to_line-Toggle`.
- The action register preserves its `Ctrl+G` full-Editor context and now links
  the packet. The crosswalk distinguishes Editor-owned caret state and the
  current Workspace composition from the missing direct prompt/Toggle route.
- Existing Workspace `goto_line`/`goto_position` operations and Outline's
  typed line jump are recorded as related behavior only. No direct
  action-specific local test or pinned-key dispatch test was found.
- Linked the packet from the parity index, Editor workflow and command/keymap
  crosswalks, and normative parity scope. A custom one-ID audit passed for
  register/payload linkage, both owner crates, route gap, and pending runtime
  status.
- Passed: `python3 scripts/check_documentation.py` (43 normative documents,
  170 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace check,
  Clippy, and tests. They remain R07-000 exit gates. Pinned `Ctrl+G` dispatch,
  native Labonair interaction, and paired visuals remain unobserved on this
  Linux host without `DISPLAY` or `WAYLAND_DISPLAY`. This work remains
  same-session independent redevelopment. R07-001 and R09 remain unstarted.

## Pre-change contract — classify the default-macOS File Finder action

Request: Independently state `file_finder::Toggle`'s user goal and map current
Editor ownership, Workspace/Shell composition, settings, route, and evidence.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-file_finder-Toggle`, its exact default binding/context,
query/result workflow, repeat invocation, selection/open behavior, related
settings, local route/tests, and pending runtime/visual acceptance.
Out of scope: Implementing actions or settings, changing keymaps, starting
R07-001/R09, or claiming native/macOS acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md, pinned action
metadata, Editor and Workspace contracts, and the existing Open File route.
Owner module: Editor owns matching/ranking/session state; Workspace owns the
current UI adapter and file listing/open validation; Shell supplies modal
composition only.
Canonical capability crate: labonair-editor; current UI adapter is in
labonair-workspace.
Canonical user entry point: Current `CommandId::OpenFile` / File > Open File;
the pinned Cmd+P direct action and repeat-selection behavior are not mapped.
Public typed contract: FileFinderQuery, FileFinderCandidate, FileFinderResult,
FileFinderSession/Snapshot/Status, and Workspace's validated candidate route.
Events: None added.
Registry contributions: None added; preserve the existing OpenFile command and
record the missing one-to-one pinned action/key route.
Allowed dependency changes: None.
State and persistence: Query/results are transient; inspect history and
preview behavior without adding persistence.
Settings: Cross-reference the four existing target requirement IDs for ignored
files, icons, active search focus, and preview tabs; add no values.
Notifications and failure: Record current loading/empty/error, cancellation,
stale-result, missing-root, and stale/deleted-file behavior.
UI-kit: Existing shared search input, buttons, and list rows.
Security and performance: Preserve workspace-root path validation, bounded
listing/ranking, async I/O, and stale-generation rejection.
Focused tests: Trace Editor model and Workspace adapter tests; distinguish
them from action dispatch or modal interaction coverage.
Visual evidence: Pending paired native macOS interaction and captures.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, action-ID/settings/test-trace audit.
Documentation updates: File Finder action packet, action register, settings
cross-reference, parity index/crosswalk, normative scope, and this progress.
Known limitations: Same-session analysis/implementation is independent
redevelopment, not separated-room clean-room work. Linux host has no display
server or native capture.
Next safe task: Continue with the next default-macOS action family while
R07-000 remains active.

## Progress record — 2026-10-10 — default-macOS File Finder action classification

- Added [`../../docs/parity/action-classification-file-finder.md`](../../docs/parity/action-classification-file-finder.md)
  with the independent file-picker goal, Editor/Workspace/Shell boundaries,
  current route, related target Settings, local test evidence, and native
  acceptance states for `ACT-MACOS-file_finder-Toggle`.
- Preserved the pinned `Cmd+P` / Workspace binding in the action register and
  mapped local `CommandId::OpenFile` / `Cmd+Shift+O` as a related route. The
  packet identifies the direct key route, repeat selection behavior, recent
  navigation history, setting wiring, and preview configuration gaps.
- Cross-referenced the four target `USER`-scoped Settings IDs for ignored
  files, file icons, active-item focus, and preview tabs. Their effects and
  reset/reload behavior remain pending acceptance.
- Test traceability records six Editor model tests, two Workspace adapter
  tests, and one descriptor-ownership test as related evidence. None exercises
  pinned `Cmd+P` dispatch or the complete modal workflow; no native
  interaction or paired visual artifact exists for the action.
- Linked the packet from the parity index, Editor workflow and command/keymap
  crosswalks, and normative parity scope. The exact action-row, owner/route,
  Settings-ID, and test-evidence audit passed.
- Passed: `python3 scripts/check_documentation.py` (43 normative documents,
  171 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace check,
  Clippy, and tests. They remain R07-000 exit gates. Pinned macOS dispatch,
  native interaction, and paired visuals remain unavailable on this Linux
  host without `DISPLAY` or `WAYLAND_DISPLAY`. Same-session work is independent
  redevelopment; R07-001 and R09 remain unstarted.

## Pre-change contract — classify Workspace file-open actions

Request: Distinguish the pinned Workspace native path-prompt actions from
File Finder and map current Labonair owners, routes, settings, and evidence.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-Open` and `ACT-PLATFORM-workspace-OpenFiles`,
their default key/context, file/directory/multi-selection behavior, window
placement, settings, and current Labonair file/project-open routes.
Out of scope: Implementing a native picker, changing menu/keymap bindings,
starting R07-001/R09, or claiming platform runtime acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md, product,
Workspace, command/registry, and Editor owner contracts.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns path
selection and Workspace/window transitions; Editor owns opened document state.
Canonical user entry points: Pinned macOS Workspace Open (`Cmd+O`) and the
Linux/Windows Workspace OpenFiles (`Ctrl+O`) menu action. Labonair currently
routes Open File to Editor-owned FileFinder and Open Project to a typed
Workspace transition; neither is a multi-path OS-picker route.
Public typed contract: Existing Workspace identity/transition and file-open
operations; no Labonair multi-path prompt contract currently exists.
Events: No new events; distinguish the existing OpenProject transition.
Registry contributions: No new command or action registrations in this
inventory slice. Record the missing native picker and exact action routes.
Allowed dependency changes: None.
Settings: Cross-reference target `default_open_behavior` and
`use_system_path_prompts`; add no values and leave their
load/save/reset/reload effects pending the Settings owner.
State and persistence: The Workspace owns project/window identity and session
restoration; selected paths become open Workspace items or project roots.
Notifications and failure: Record cancellation, invalid/missing paths,
unavailable directories, and failed window/workspace creation as acceptance.
Security: Native selection may include paths outside the current root; verify
the target scope and permission behavior without weakening FileFinder's
root-safe candidate validation.
UI-kit: Reuse shared selection/picker surfaces where the native path dialog
does not provide the platform UI; add no local replacement control here.
Focused tests: Trace Workspace identity/transition and command-owner evidence;
do not call them native picker or multi-file acceptance.
Visual evidence: Pending paired macOS and Linux/Windows runtime interaction.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, exact action/settings route audit.
Documentation updates: Workspace-open action packet, action register,
FileFinder cross-reference, parity index/crosswalk, normative scope, discovery
log, and progress.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native app interaction is unavailable here.
Next safe task: Continue R07-000 with action inventory and later implementation
work only from the independent requirements packets.

## Progress record — 2026-10-10 — Workspace Open action classification

- Added [`../../docs/parity/action-classification-workspace-open.md`](../../docs/parity/action-classification-workspace-open.md)
  for `ACT-MACOS-workspace-Open` and
  `ACT-PLATFORM-workspace-OpenFiles`. It separates general file/folder Open
  from the Linux/Windows multi-file Open Files flow and records Workspace and
  Editor ownership, menu/key routes, path-prompt states, error recovery, and
  required native acceptance.
- Recorded target Settings
  `SET-general-default-open-behavior` (`existing_window`, `USER`) and
  `SET-general-use-system-path-prompts` (`true`, `USER`), with Labonair wiring
  and settings lifecycle checks still open.
- Traced current Labonair Open Project to a single-directory, in-place project
  picker and current Open File to Editor File Finder. The packet records that
  the local `editor::OpenFile` route does not implement pinned Open Files,
  despite the pinned Linux action catalog's deprecated alias metadata.
- Logged the non-obvious action-name collision in
  [`../../memory/bugs_and_fixes.md`](../../memory/bugs_and_fixes.md) so future
  work keeps the two path-selection workflows distinct.
- Updated the action register, File Finder packet, parity index, feature
  crosswalk, and normative parity scope. The exact ID/default/scope/owner-route
  audit passed. No local test covers pinned action dispatch, multi-path
  selection, or the required window behavior.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  172 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused Workspace Open inventory audit.
- Not run for this documentation-only slice: Rust formatting, workspace check,
  Clippy, and tests. They remain R07-000 exit gates. Pinned macOS and Windows
  runtime interaction and paired captures remain unavailable; Linux evidence
  is static only. Same-session work is independent redevelopment. R07-001 and
  R09 remain unstarted.

## Pre-change contract — classify Workspace New Window

Request: Record the pinned `workspace::NewWindow` user flow, its new-window
content setting, current Labonair route, and runtime acceptance gaps.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-NewWindow`, its default key/menu route, window
activation and initial content, and `SET-general-on-new-window`.
Out of scope: Implementing a new-window command, changing window behavior,
starting R07-001/R09, or claiming runtime acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md, product,
Workspace, and application-composition owner contracts.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
workspace action and resulting workspace state; application composition
supplies the platform window host and registers the owner contribution.
Canonical user entry point: Pinned File > New Window and `Cmd+Shift+N` on
macOS; Linux and Windows bindings remain in the action register. Labonair has
no user-facing Workspace New Window command or keybinding.
Public typed contract: No Labonair create-new-workspace-window contract is
currently registered. Specify a Workspace-owned typed request carrying the
`on_new_window` content choice to the native window host. Do not promote the
shell's debug Gallery window to the product route.
Events and registry contributions: None added in this inventory slice; record
the missing Workspace-owned action/command route.
Allowed dependency changes: None.
Settings: Cross-reference `on_new_window` and its target default/options;
add no value and leave settings lifecycle and runtime effects pending the
Settings and Workspace owners.
State and persistence: A successful action creates and activates one new
application window. Its initial view follows the target setting. Preserve the
requesting window and existing workspace state. Verify empty-tab and
Launchpad persistence separately.
Notifications and failure: Record create/activate failure and retry behavior;
use the notification center for recoverable user-facing errors.
Security: No new path is selected. Preserve the existing session/project trust
state and avoid implicitly cloning sensitive project content into the new
window.
UI-kit: Reuse Workspace's existing content surfaces; add no local window or
launchpad controls here.
Focused tests: Trace current application startup and Workspace ownership
contracts. Do not treat startup-window or debug Gallery creation as this
action's runtime acceptance.
Visual evidence: Pending pinned macOS and Labonair interaction with paired
window/state captures.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, exact action/settings route audit.
Documentation updates: NewWindow action packet, action register, capability
crosswalk, parity index/normative scope, and R07-000 progress.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace New Window action classification

- Added [`../../docs/parity/action-classification-workspace-new-window.md`](../../docs/parity/action-classification-workspace-new-window.md)
  for `ACT-MACOS-workspace-NewWindow`, including its key/menu route, Workspace
  owner, typed window-host request, existing-window preservation, and native
  interaction requirements.
- Cross-referenced `SET-general-on-new-window` (`launchpad`, user scope) and
  its `launchpad` / `empty_tab` initial-content choices. Labonair has no
  matching setting wiring or user-facing Workspace New Window route.
- Distinguished app startup and the debug-only Component Gallery window
  from the required Workspace action. Pinned source inspection records new
  window activation and initial content; no pinned runtime observation or
  Labonair end-to-end test exists.
- Updated the action register, parity index, app-window and command/keymap
  crosswalks, and normative parity scope. The exact action/Settings/route
  audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  173 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused Workspace New Window inventory audit.
- Not run for this documentation-only slice: Rust formatting, workspace check,
  Clippy, and tests. They remain R07-000 exit gates. Pinned macOS and Windows
  interaction and paired captures remain open; Linux evidence is static only.
  Same-session analysis is independent redevelopment. R07-001 and R09 remain
  unstarted.

## Pre-change contract — classify Workspace New File

Request: Record the pinned `workspace::NewFile` untitled-buffer flow, local
New Editor Tab route, related unsaved-buffer Settings, and runtime gaps.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-NewFile`, all default binding contexts,
pathless Editor creation, first input, save-as continuation, and restoration.
Out of scope: Implementing or rebinding New File, changing file-save behavior,
starting R07-001/R09, or claiming end-to-end acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md, product,
Editor, Workspace, and command/registry owner contracts.
Owner module and canonical crate: Editor (`labonair-editor`) owns untitled
document creation and file lifecycle. Workspace (`labonair-workspace`) hosts
the document in its accepted universal Editor tab surface.
Canonical user entry point: Pinned File > New, `Cmd+N` in Workspace and
Welcome contexts. Labonair currently exposes File > New Editor Tab and
`Cmd+E`; the universal-tab route is a related workflow with different label
and binding.
Public typed contract: Editor contributes the New Untitled Document intent
through its canonical command surface; Workspace hosts and focuses one new
Editor tab through its typed view contract. Do not add a shell-owned duplicate
command path or require a second OS window in Labonair.
Events and registry contributions: Record owner contribution and the
Workspace host contract; add no route, event, registry, or binding in this
inventory slice.
Allowed dependency changes: None.
Settings: Cross-reference `session.restore_unsaved_buffers` and
`restore_on_startup`; add no Settings values and leave reload/reset/migration
effects to the Settings owner.
State and persistence: Start with one empty pathless document and focus its
Editor. Labonair creates one universal Editor tab rather than a new application
window, per the accepted Workspace architecture. Verify typing, Save As,
unsaved close, app restart, and session restoration.
Notifications and failure: Record buffer-creation failure and save/recovery
errors; route user messages through the notification center.
Security: No path or project is selected during creation. Save As must still
enforce normal path and permission rules.
UI-kit: Reuse the shared Editor tab, titlebar, and dialog surfaces; add no
feature-local tab or save control.
Focused tests: Trace NewEditorTab descriptor ownership and Editor buffer
creation code. Do not call descriptor/model evidence action acceptance.
Visual evidence: Pending pinned macOS and Labonair interaction with paired
first-input, dirty, and restored-session states.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, exact action/settings/route audit.
Documentation updates: New File action packet, action register, File Finder
and New Window distinctions, parity index/crosswalk, normative scope, and
R07-000 progress.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace New File action classification

- Added [`../../docs/parity/action-classification-workspace-new-file.md`](../../docs/parity/action-classification-workspace-new-file.md)
  for `ACT-MACOS-workspace-NewFile`, its Workspace/Welcome contexts, untitled
  document result, focus and save lifecycle, and runtime acceptance states.
- Recorded `SET-general-session-restore-unsaved-buffers` (`true`, `USER`) and
  `SET-general-restore-on-startup` (`last_session`, `USER`). Neither setting
  has corresponding Labonair wiring for this pathless Editor flow.
- Distinguished `workspace::NewFile` from Project Panel filesystem creation
  and New Window. Labonair's `Cmd+E` New Editor Tab provides a related blank
  pathless view; its accepted universal-tab adaptation does not yet establish
  key, save, dirty-close, or session-restore acceptance.
- Updated the action register, parity index, app-window/tab, command/keymap,
  and Editor workflow crosswalks, plus the normative parity scope. The exact
  action/Settings/context/route audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  174 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused Workspace New File inventory audit.
- Not run for this documentation-only slice: Rust formatting, workspace check,
  Clippy, and tests. They remain R07-000 exit gates. Pinned macOS interaction,
  save/restore verification, and paired captures remain open; Linux evidence
  is static only. Same-session analysis is independent redevelopment.
  R07-001 and R09 remain unstarted.

## Pre-change contract — classify Workspace New Terminal

Request: Independently state the pinned `workspace::NewTerminal` user goal,
Terminal-context key routes, focus-dependent placement, local/remote session
selection, shell/cwd settings, and the existing Labonair terminal-tab route.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-NewTerminal`, default platform bindings,
Terminal versus Workspace focus behavior, the optional `local` action value,
Terminal ownership, launch defaults, failures, and interaction evidence.
Out of scope: Rebinding `NewTerminalTab`, adding the pinned center-terminal or
remote-shell workflow, changing Terminal settings/session creation, starting
R07-001/R09, or claiming action acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md,
feature-crosswalk.md, capabilities.md, Terminal, Workspace, and command/keymap
owner contracts.
Owner module and canonical crate: Terminal (`labonair-terminal`) owns shell
configuration, process/session state, and terminal rendering. Workspace
(`labonair-workspace`) owns tab/pane hosting and the current `NewTerminalTab`
entry route. A Terminal action may request a typed session and placement from
Workspace; the shell remains a composition root.
Canonical user entry point: Pinned `Ctrl+~` in Workspace and `Cmd+N` in
Terminal on macOS; `Ctrl+~` in Workspace and `Ctrl+N` in Terminal on Linux;
`Ctrl+Shift+Backtick` in Workspace and `Ctrl+N` in Terminal on Windows. The
Terminal context menu also exposes New Terminal. Labonair has File > New
Terminal Tab, Command Palette `New Terminal Tab`, and `Cmd+T` on macOS / `Ctrl+T`
on Linux and Windows; it opens a new local workspace terminal tab.
Public typed contract: Reuse `labonair-terminal::SessionOptions` and
`TerminalRegistry::create` for configured local session creation. The
transport-backed terminal entry accepts remote input/output adapters supplied
by the owning SSH/remote contract. Workspace owns new terminal tab, panel, and
center placement; any future owner-crossing request must be typed and narrow.
Keep the `local` versus project/remote execution choice explicit; do not add a
second string-dispatched action table or an unnecessary registry.
Events and registry contributions: The existing local menu and command
provider call the Workspace handler, which calls `TerminalRegistry`. No
terminal-launch event or new registry is required for this inventory slice.
Settings: Cross-reference the pinned shell and working-directory settings,
plus local shell, shell-argument, and environment settings. Record the local
absence of a configurable working-directory policy; add no Settings values or
change defaults.
State and persistence: Create one new shell session and preserve all existing
sessions. Pinned default `local=false` uses the active project execution
environment (remote when the project is remote); `local=true` explicitly
forces a local shell. Labonair currently creates a local process, using the
active terminal cwd, then project root, and otherwise passes no explicit cwd;
the inherited process cwd needs runtime verification. Verify remote, standalone,
and project behavior as separate cases.
Notifications and failure: A failed process/session spawn must preserve
existing tabs and expose a recoverable notification with retry. Record the
current local `tracing::error!` plus silent return as incomplete user-facing
failure handling; change no notification path in this inventory slice.
Security: Do not leak project/remote credentials into local process
environment. A forced-local action in a remote project must use local cwd and
environment only. Record shell path, arguments, inherited variables, and
remote/local target in acceptance traces.
UI-kit: Reuse Terminal, Workspace tab, panel, and pane surfaces. Add no
feature-local tab/menu/button controls.
Focused inspection: Trace action parameters and handler, platform keymaps,
current TerminalSettings consumption, `NewTerminalTab` descriptor/handler, and
the TerminalRegistry options contract. Descriptor presence is not action
dispatch acceptance.
Visual evidence: Pending pinned macOS and Labonair interaction/capture with
Workspace focus, Terminal focus, panel placement, center placement, and remote
project state.
Workspace gates: Documentation checker, docs verification scope, queue
check, diff check, exact action/settings/route audit.
Documentation updates: New Terminal action packet, action register, parity
index, Terminal feature crosswalk, normative parity scope if needed, and
R07-000 progress.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace New Terminal action classification

- Added [`../../docs/parity/action-classification-workspace-new-terminal.md`](../../docs/parity/action-classification-workspace-new-terminal.md)
  for `ACT-MACOS-workspace-NewTerminal`, its Workspace and Terminal key
  contexts, focus-dependent dock/center placement, and the explicit local
  override for remote projects.
- Recorded Terminal's typed local session contract separately from
  transport-backed session I/O. Labonair's current New Terminal Tab always
  starts a local process in a new universal workspace tab; it does not supply
  the pinned action's local/remote choice or focus-dependent placement.
- Cross-referenced the pinned shell and working-directory Settings and local
  shell, argument, and environment fields. The local working-directory policy
  setting is absent. Spawn errors currently reach tracing and silently return
  without a notification.
- Updated the action register, parity README, Terminal and command/keymap
  crosswalks, normative parity scope, and this task progress record. The
  focused action/settings/route audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  175 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Pinned dispatch,
  macOS interaction, local/remote shell behavior, spawn-failure notification,
  and paired captures remain open. Same-session analysis is independent
  redevelopment. R07-001 and R09 remain unstarted.

## Pre-change contract — classify Workspace Open With System

Request: Independently state the pinned `workspace::OpenWithSystem` behavior,
its ProjectPanel and InvalidBuffer contexts, default-platform bindings and
Windows legacy spelling, and current local Explorer/Editor paths.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-OpenWithSystem`, selected local files and
folders, failed local file loading, OS default-app handoff, platform alias
records, missing local action paths, and error/security evidence.
Out of scope: Adding or rebinding an action, changing preview launch behavior,
opening remote paths in local applications, starting R07-001/R09, or claiming
native-app acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, feature-crosswalk.md, capabilities.md,
Workspace, Explorer, Editor, and platform external-open contracts.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
generic local-path default-app action and its platform handoff. Explorer
(`labonair-panel-explorer`) owns selected-path UI state; Editor supplies the
invalid-file context. The shell remains a composition root.
Canonical user entry point: Pinned `Ctrl+Shift+Enter` in ProjectPanel and
InvalidBuffer on macOS and Linux. Windows' default ProjectPanel map contains
`project_panel::OpenWithSystem` with the same key; the inspected pinned Linux
registry declares that spelling as a deprecated alias for the canonical
Workspace action. Windows InvalidBuffer has no recorded binding. Labonair's
Explorer context menu has Open, Open Preview, and Reveal in Finder; Preview
offers an external-open button for external content. There is no action or key
route matching Open With System for Explorer selection or an invalid Editor
file.
Public typed contract: Pinned dispatch uses the typed
`workspace::OpenWithSystem` action, resolves the selected ProjectPanel entry
or the invalid local item's absolute path, then calls the native
`open_with_system(Path)` service. Labonair has a Workspace preview helper that
spawns the system opener for preview paths/URLs, but no shared Explorer/Editor
contract. If that action is implemented, keep a local `PathBuf` handoff typed
and Workspace-owned; no provider-discovery registry is needed.
Events and registry contributions: This inventory adds no event, registry,
action, or menu contribution. Keep external launch separate from Explorer's
`Reveal in Finder` operation and the in-app Editor `Open` action.
Settings: None. Use the OS default file association; add no values or profile
selection controls.
State and persistence: Do not mutate Editor or Explorer state. Opening a
selected local path externally should leave the current selection, tabs,
unsaved buffers, and project identity unchanged. The OS application owns the
external document state.
Notifications and failure: A failed OS handoff should report a recoverable
notification. Labonair's existing Preview helper only logs a warning when
process spawn fails; change no notification path in this inventory slice.
Security: Only expose local paths through this action. The pinned ProjectPanel
does not show its Default App item for remote worktrees, and InvalidBuffer only
shows the button for local paths. Avoid shell interpolation; do not pass
credentials or remote locations to a local default app.
UI-kit: Reuse Explorer's shared context menu and Editor's shared button/menu
surface. Add no local open-with button style.
Focused inspection: Trace action bindings/alias metadata, selected-entry
resolution, invalid-item action, local Explorer context menu, preview opener,
and native platform open operation. Static route evidence does not establish
Windows alias dispatch or OS association behavior.
Visual evidence: Pending pinned macOS and Labonair pointer/keyboard interaction
and paired captures for a local file, directory, invalid local file, and
remote-path absence.
Workspace gates: Documentation checker, docs verification scope, queue
check, diff check, exact action/alias/route audit.
Documentation updates: Open With System packet, action register, parity
index, Explorer/editor crosswalk, normative parity scope if needed, and
R07-000 progress.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace Open With System action classification

- Added [`../../docs/parity/action-classification-workspace-open-with-system.md`](../../docs/parity/action-classification-workspace-open-with-system.md)
  for `ACT-MACOS-workspace-OpenWithSystem`, its ProjectPanel and InvalidBuffer
  contexts, local file/folder handoff, and remote-path restriction.
- Recorded the Windows default-map binding as the deprecated
  `project_panel::OpenWithSystem` spelling in ProjectPanel only. The pinned
  Linux registry metadata maps it to the canonical Workspace action; Windows
  runtime alias dispatch remains unverified.
- Distinguished default-app launch from Labonair's Explorer `Open`, Preview
  external opener, and `Reveal in Finder`. No matching local Explorer or
  invalid-file action/key route was found.
- Updated the action register, parity README, Explorer/Editor and command/keymap
  crosswalks, normative parity scope, and this task progress record. The
  focused action/alias/route audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  176 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Pinned macOS
  dispatch, Windows alias behavior, native OS handoff, and paired visual
  evidence remain open. Same-session analysis is independent redevelopment.
  R07-001 and R09 remain unstarted.

## Pre-change contract — classify Workspace dock sizing actions

Request: Independently classify the six pinned Workspace active/open dock
increase, decrease, and reset actions, their pixel arguments, platform routes,
related Settings, and Labonair's persisted dock-resize controls.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `DecreaseActiveDockSize`, `DecreaseOpenDocksSize`,
`IncreaseActiveDockSize`, `IncreaseOpenDocksSize`, `ResetActiveDockSize`, and
`ResetOpenDocksSize` under their stable action requirement IDs.
Out of scope: Adding keyboard actions, changing dock layout or min/max rules,
changing Settings values, starting R07-001/R09, or claiming visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md,
feature-crosswalk.md, Workspace, Dock, and panel owner contracts.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns dock
visibility, focus, edge geometry, size bounds, reset behavior, and layout
persistence. Registered panel owners provide their default/minimum sizes.
Canonical user entry point: Pinned Workspace keybindings. macOS/Linux use
`Ctrl+Alt` plus `-`, `=`, `0`, `_`, `+`, or `)` for active/all-open actions.
Windows has active-dock `Shift+Alt+-`/`Shift+Alt+=`, active reset `Alt+R`, and
open-dock reset `Shift+Alt+0`; open-dock increment/decrement have no binding.
Labonair currently resizes the pointer-selected dock by dragging its handle
and resets one dock by double-clicking its handle.
Public typed contract: Pinned Workspace actions carry `px` for incremental
changes. `px=0` resolves to the UI font size. Workspace applies one signed
delta to the focused open dock or every open dock, or resets the eligible
active panels to their registered default size. No new event, registry, or
action contract is required for this inventory slice.
Events and registry contributions: Existing Workspace action handlers mutate
Workspace-owned dock state. Panel providers contribute their view/default
size and constraints; no new provider registration is needed.
Settings: Cross-reference `ui_font_size` (16, user scope) for zero-argument
increments. Record `workspace.resize_all_panels_in_dock` (default `["left"]`)
as a pinned JSON setting currently missing from the Settings register; leave
its supported scope pending unless the source proves it. Labonair has no
matching setting, and its dock stores one size per edge for all panels.
State and persistence: Pinned resize updates the active panel, or panels using
the same sizing mode in configured edges. Reset-active targets the focused
open dock; Reset-open targets every dock with a visible panel. Preserve open
state and active panel, respect min/max/workspace bounds, and persist through
the Workspace layout. Labonair persists one size for each edge dock; keyboard
adjust/reset-all operations are absent.
Notifications and failure: A missing focused dock is a no-op for active-dock
actions; no visible dock is a no-op for all-open actions. Bounds and empty
docks must not disturb other layout state. No notification is expected for a
no-op; do not add error messages in this inventory slice.
Security: No external process, path, or secret access. Do not change project
trust or unrelated workspace state.
UI-kit: Reuse existing dock resize handles and shared panel surfaces; add no
local controls or resize handle styling.
Focused inspection: Trace all six action parameters, handlers, platform
bindings, Settings dependencies, reset-all behavior, local drag/double-click
route, bounds, and layout persistence. Static source is not key dispatch or
visual acceptance.
Visual evidence: Pending matched macOS comparison for left/right/bottom docks,
active/all-open resize and reset, panel focus, bounds, and persisted reload.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, exact action/settings/route audit.
Documentation updates: Six action register rows, dock-sizing packet,
`ui_font_size` and resize-all Settings crosswalk, Workspace/Settings feature
crosswalk, normative parity scope if needed, and R07-000 progress.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace dock sizing action classification

- Added [`../../docs/parity/action-classification-workspace-dock-sizing.md`](../../docs/parity/action-classification-workspace-dock-sizing.md)
  covering six active/all-open dock resize and reset actions, their platform
  bindings, `px=0` UI-font-size fallback, focus and bounds behavior, and
  `workspace.resize_all_panels_in_dock`.
- Added `SET-settings-json-only-workspace-resize-all-panels-in-dock` to the
  Settings requirements inventory. Its pinned default is `["left"]`; the
  Settings scope remains pending. Labonair stores one persisted dimension per
  edge dock and currently exposes pointer drag and per-dock double-click reset,
  with no matching keyboard actions.
- Updated all six action rows, Settings and feature crosswalks, parity index,
  and normative parity scope. The focused audit passed for all six action IDs,
  the new Settings ID, current inventory counts, packet references, and
  crosswalk links.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  177 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Pinned keyboard
  dispatch, platform interactions, persisted reload behavior, and paired
  native visual evidence remain open. Same-session analysis is independent
  redevelopment, not separated-room clean-room work. R07-001 and R09 remain
  unstarted.
- Next safe task: continue R07-000 with the next unclassified action family.

## Pre-change contract — classify Workspace dock visibility actions

Request: Independently classify the Workspace close-active-dock, toggle-all-
docks, and per-edge dock visibility actions, including focus, restoration,
keybindings, and current Labonair routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `CloseActiveDock`, `ToggleAllDocks`, `ToggleBottomDock`,
`ToggleLeftDock`, and `ToggleRightDock` under their stable action IDs.
Out of scope: The unbound `CloseAllDocks` registry action, new keybindings or
controls, changing panel-location settings, implementation, R07-001/R09, and
claims of native interaction or visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, feature-crosswalk.md, capabilities.md,
Workspace, Dock, statusbar panel controls, and current tab-close command.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns dock
edge visibility, active-panel focus, remembered open positions, and layout
persistence. Panel owners provide edge placement and enabled/active state.
Canonical user entry point: Pinned Workspace-context keybindings. `CloseActiveDock`
uses macOS `Cmd+W` and Linux/Windows `Ctrl+F4` or `Ctrl+W`; `ToggleAllDocks`
uses macOS `Alt+Cmd+Y`, Linux `Ctrl+Alt+Y`, and Windows `Ctrl+Shift+Y`;
left/right/bottom toggles use macOS `Cmd+B`, `Cmd+Alt+B`/`Cmd+R`, `Cmd+J`,
and Linux/Windows `Ctrl+B`, `Ctrl+Alt+B`, `Ctrl+J` respectively. Labonair has
per-panel statusbar buttons and `Toggle Sidebar`, which toggles the primary
dock; its `Cmd+W` palette command closes the current tab.
Public typed contract: Workspace closes the focused open dock, toggles a
specific edge, or closes all open docks and restores the last remembered open
edge set when all are closed. Closing an active dock without dock focus
propagates the action. No new event, registry, or typed contract is required
for this inventory slice.
Events and registry contributions: Existing Workspace action handlers own
visibility changes. Statusbar panel buttons and existing Workspace commands
remain their current owner contributions; add no parallel shell action table.
Settings: No new setting. Existing panel-edge settings determine placement;
the per-edge toggle actions are independent of panel identity. Labonair's
`Toggle Sidebar` currently targets its configurable primary dock.
State and persistence: Preserve active panel, edge, size, and unrelated docks
when toggling one edge. Toggling all remembers which edges were open before
closing them; confirm whether this restoration memory survives reload rather
than implying persistence. Labonair stores per-dock open and active-panel state
in Workspace layout snapshots.
Notifications and failure: Closing when no dock is focused propagates rather
than reporting an error. Toggling all with no open or remembered docks is a
no-op. Empty/disabled edges, zoom, focus transfer, and panel visibility need
acceptance coverage; no notification is expected for a normal toggle.
Security: No external path, process, or secret access.
UI-kit: Reuse registered panel statusbar controls and shared menus; create no
local dock toggle buttons or close-button style.
Focused inspection: Trace the five action declarations, handlers, platform
bindings, remembered-position flow, panel activation/focus behavior, current
statusbar/menu/palette routes, `Cmd+W` local command, and layout persistence.
Visual evidence: Pending paired macOS checks for each edge, all-dock
close/restore, empty and zoomed docks, focus transfer, and narrow windows.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action/route/persistence audit.
Documentation updates: Five action register rows, dock visibility packet,
Workspace and keymap crosswalks, parity index, normative parity scope if
needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace dock visibility action classification

- Added [`../../docs/parity/action-classification-workspace-dock-visibility.md`](../../docs/parity/action-classification-workspace-dock-visibility.md)
  for five Workspace actions: closing the focused dock, toggling each edge,
  and hiding/restoring all open edges with the remembered set.
- Recorded the focus propagation for Close Active Dock, per-edge activation
  and focus behavior, all-dock restore semantics, and the unresolved reload
  behavior of the pinned in-memory remembered set. Kept unbound `CloseAllDocks`
  outside this default-map action family.
- Cross-checked Labonair's panel statusbar buttons, the primary-dock Toggle
  Sidebar command, persisted per-edge open/active state, and local `Cmd+W` tab-close
  command. No fixed-edge, all-docks, or close-active keyboard route exists in
  the current Workspace command map.
- Updated five action rows, Workspace and keymap crosswalks, the parity index,
  normative parity scope, and this task's pre-change/progress records. The
  focused action/route/persistence audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  178 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Pinned keyboard
  dispatch, active-dock focus behavior, all-dock restoration after reload,
  platform interaction, and paired native visual evidence remain open.
  Same-session analysis is independent redevelopment, not separated-room
  clean-room work. R07-001 and R09 remain unstarted.
- Next safe task: continue R07-000 with the next unclassified action family.

## Pre-change contract — classify Workspace major-region focus actions

Request: Independently classify forward/backward keyboard navigation among
the major visible Workspace regions, its wrapping and closed-dock behavior,
and Labonair's current focus routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `FocusNextPart` and `FocusPreviousPart` under their stable action
requirement IDs.
Out of scope: Split-pane cycling, tab order changes, new focus controls,
accessibility API migrations, implementation, R07-001/R09, and claims of
native screen-reader or visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, feature-crosswalk.md, workspace-model.md,
design-system.md, UI-kit focus controls, titlebar, docks, and statusbar.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns
cross-region focus order and selects content focus targets. Shell provides
titlebar/statusbar surfaces; dock panel owners provide their focus handles.
Canonical user entry point: Pinned Workspace-context keys: forward `F6` and
`Cmd+F6` on macOS, `F6` and `Ctrl+F6` on Linux/Windows; backward `Shift+F6`
on all three platforms. Labonair has `FocusNextPane` for cycling split leaves
and arrow traversal inside the statusbar, but no major-region actions.
Public typed contract: Move focus forward or backward through visible
titlebar, left dock, center editor, right dock, bottom dock, and statusbar
parts, skipping closed docks and wrapping. When current focus is outside the
part list, focus its first part. This inventory adds no event or registry.
Events and registry contributions: Workspace actions remain owned by
Workspace; surface owners expose existing focus handles. Add no shell-wide
focus table or cross-feature callback.
Settings and persistence: No setting, stored state, or persistence change.
Accessibility and UI-kit: Toolbar parts land on the first focusable control;
dock/editor parts target usable content for sighted keyboard use and named
landmarks when assistive technology is active. Record the current GPUI
accessibility-role limitation; do not claim screen-reader support without
native verification. Reuse shared controls and focus styles.
Failure behavior: A toolbar with no focusable control keeps focus on its
container. With a nonempty part list, focus outside all known parts resolves
to the first part. Hidden/closed regions are excluded.
Security/performance: No sensitive data or I/O; no new dependency or measurable
performance path is expected.
Focused inspection: Trace part order, current-focus detection, wrap behavior,
closed docks, titlebar/statusbar first-control handling, active dock panel,
center pane, screen-reader mode, and Labonair's pane/statusbar-only routes.
Visual evidence: Pending paired macOS focus-ring and native accessibility
checks across sparse and fully open region layouts.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action/order/route audit.
Documentation updates: Two action register rows, major-region focus packet,
Workspace/command crosswalks, parity index, normative parity scope if needed,
and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS and screen-reader interaction are
unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace major-region focus action classification

- Added [`../../docs/parity/action-classification-workspace-major-region-focus.md`](../../docs/parity/action-classification-workspace-major-region-focus.md)
  for `FocusNextPart` and `FocusPreviousPart`, recording visible-region order,
  wrapping, closed-dock filtering, toolbar/landmark targets, and the outside-
  focus fallback.
- Distinguished major-region navigation from Labonair's split-leaf
  `FocusNextPane` route and statusbar arrow handling. The pinned default map
  also assigns F6 to `debugger::Pause` while `debugger_session` is active;
  exact context precedence remains a runtime acceptance check.
- Updated two action rows, the parity index, Workspace/focus and command-keymap
  crosswalks, normative parity scope, and this task's pre-change/progress
  records. The focused ID, F6-overlap, route, and index audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  179 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native F6
  dispatch, function-key settings, screen-reader announcements, focus-ring
  states, and paired macOS captures remain open. Same-session analysis is
  independent redevelopment, not separated-room clean-room work. R07-001 and
  R09 remain unstarted.
- Next safe task: continue R07-000 with Workspace directional pane activation
  and swap actions, then classify the remaining default-map actions.

## Pre-change contract — classify Workspace pane activation and swap actions

Request: Independently classify indexed pane activation, directional pane/dock
focus, and directional center-pane swapping, including key arguments,
boundaries, and current Labonair command routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ActivatePane`, `ActivatePaneDown`, `ActivatePaneLeft`,
`ActivatePaneRight`, `ActivatePaneUp`, and `SwapPaneDown`, `SwapPaneLeft`,
`SwapPaneRight`, `SwapPaneUp` under their stable action IDs.
Out of scope: `ActivateNextPane` and other Pane actions already in the Pane
packet, `MovePane*`, item movement, Editor-internal groups, adding commands or
bindings, implementation, R07-001/R09, and visual acceptance claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, action-classification-pane.md,
workspace-model.md, Workspace, SplitTree, pane focus, dock focus, and TabStore.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
outer pane topology, pane order, active pane, directional focus, and swap
behavior. View owners provide each pane's clone/split capability and focus
target. Editor-internal group state remains Editor-owned.
Canonical user entry point: macOS `Cmd+1` through `Cmd+9` selects indices 0–8;
Linux/Windows `Alt+1` through `Alt+9` does the same. Directional focus uses
`Cmd+K` then `Cmd+Arrow` on macOS and `Ctrl+K` then `Ctrl+Arrow` on Linux/
Windows. Swaps use `Cmd+K` then `Shift+Arrow` on macOS and `Ctrl+K` then
`Shift+Arrow` on Linux/Windows. Labonair currently maps `Cmd+1` through `Cmd+9`
to tab selection, provides a split-leaf `FocusNextPane`, and splits right/down.
Public typed contract: Numeric `ActivatePane(usize)` focuses an existing
zero-based center-pane index; when the index is absent it attempts one split to
the right of the active pane with a clone of the active item, and no-ops if
there is no cloneable active item. Directional activation focuses a neighboring
pane or an eligible adjacent dock/sidebar; unavailable directions no-op.
`SwapPane*` exchanges
the Workspace active center pane with an adjacent center pane in the requested
direction, regardless of whether a dock currently has focus; absent neighbors
no-op. No new event, registry, or typed contract is required for this
inventory slice.
Events and registry contributions: Workspace owns action handling and layout
changes. Pane and dock owners provide focus/split capabilities through current
contracts; add no second pane owner or shell action table.
Settings: No new value. User keymap overrides and existing pane/layout
preferences remain separately inventoried.
State and persistence: Focus actions preserve view contents and split
geometry. Swaps preserve pane identities and contents while exchanging their
positions; out-of-range numeric activation may add a split. Verify Workspace
layout restore for every topology change. Labonair's numeric shortcuts select
tabs instead of panes and has no directional activation or swap command.
Notifications and failure: Missing directional neighbors do not mutate the
layout. Non-cloneable or empty active items make numeric out-of-range fallback
a no-op. No notification is expected for normal focus/swap actions.
Security: No path, process, credential, or secret access.
UI-kit: Reuse shared pane, tab, dock, and focus surfaces; add no local split or
directional controls in this inventory slice.
Focused inspection: Trace numeric indices and fallback, spatial neighbor
selection, dock/sidebar fallback, swap geometry, no-neighbor behavior,
platform bindings, local Cmd+1..9 collision with SelectTab1..9, and persistence.
Visual evidence: Pending paired macOS checks for 1–9 indices, nested and
unequal splits, all directions, dock boundaries, clone failure, and restore.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action/argument/route audit.
Documentation updates: Nine action rows, pane activation/swap packet, Workspace
and keymap crosswalks, parity index, normative parity scope if needed, and
this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS interaction is unavailable here.
Next safe task: Continue R07-000 with the next unclassified action family.

## Progress record — 2026-10-10 — Workspace pane activation and swap action classification

- Added [`../../docs/parity/action-classification-workspace-pane-activation-swap.md`](../../docs/parity/action-classification-workspace-pane-activation-swap.md)
  for nine indexed, directional-focus, and adjacent-swap actions.
- Recorded zero-based pane indices and their one-through-nine key arguments,
  the one-right-split clone fallback for a missing index, directional focus
  into eligible docks/sidebar, and center-only swaps using Workspace's active
  pane even when focus is in a dock.
- Distinguished Labonair's `Cmd+1..9` tab selection from the pinned numeric
  pane actions, and its split-leaf `FocusNextPane`/right-down split routes from
  missing directional focus and swap actions. Cross-referenced the existing
  Workspace/Editor pane ownership and current Terminal-only outer split scope.
- Updated nine action rows, the parity index, Workspace tabs and command/keymap
  crosswalks, normative parity scope, and this task's pre-change/progress
  records. The focused action/argument/conflict/route audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  180 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native geometric
  focus, clone fallback, swap behavior, key dispatch, pane persistence, and
  paired macOS evidence remain open. Same-session analysis is independent
  redevelopment, not separated-room clean-room work. R07-001 and R09 remain
  unstarted.
- Next safe task: continue R07-000 with the next unclassified Workspace action
  family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace bulk item and pane closing

Request: Independently classify the close-all and close-inactive item/pane
actions, including pinned-item preservation, save intent, partial cancellation,
and Labonair's current Space-scoped tab-close routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `CloseAllItemsAndPanes` and `CloseInactiveTabsAndPanes` under their
stable action requirement IDs.
Out of scope: Pane-local close actions already covered by the Pane packet,
window/tab close and quit behavior, adding commands or bindings, implementation,
R07-001/R09, and runtime or visual acceptance claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, action-classification-pane.md,
workspace-model.md, Workspace pane lifecycle, item save/close contracts, and
Labonair's universal tabs and Space model.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns
cross-pane close selection and pane removal. Each view owner retains its
content state and save/close contract; Editor owns buffer dirty state.
Canonical user entry point: Pinned Pane-context actions use macOS
`Cmd+K Cmd+W` for closing all items and panes and `Ctrl+Alt+Cmd+W` for closing
inactive tabs and panes. Linux uses `Ctrl+K Ctrl+W` and
`Alt+Ctrl+Shift+W`; Windows uses `Ctrl+K Ctrl+W` and
`Ctrl+Shift+Alt+W`, respectively. Labonair currently offers Space-scoped tab
context-menu close routes and a `Close Other Tabs` palette command.
Public typed contract: `CloseAllItemsAndPanes` closes unpinned items in every
center pane; `CloseInactiveTabsAndPanes` retains the active item in the active
center pane while closing other unpinned items in all center panes. Both
preserve pinned items. Their optional `save_intent` defaults to `Close`. The
inactive action applies that argument to other panes, while its active-pane
retained-item path delegates with the default close intent. Empty panes are
removed where possible, while the final center pane remains. No new event,
registry, or API is required for this inventory slice.
Events and registry contributions: Workspace remains the cross-pane action
owner; do not add a shell close table or duplicate view ownership.
Settings and persistence: No setting or schema change. Item removal and
empty-pane cleanup change Workspace layout; determine save/reload and restore
behavior through acceptance rather than assuming an automatic reopen history.
Notifications and failure: Dirty items follow each view's save/close flow.
Record group and per-item cancellation, save errors, partial completion across
panes, and focus/layout outcomes; the operation is not assumed atomic. No
notification is expected for ordinary cancellation or close.
UI-kit: Reuse the existing shared tab, pane, dialog, and menu components; add
no local bulk-close controls in this inventory slice.
Focused inspection: Trace each action's candidate items, pinned filtering,
active item and pane retention, default and explicit save intent, dirty-group
prompts, per-item errors/cancellation, pane removal, focus, and layout restore.
Compare Labonair's current Space-scoped tab routes, preview tabs, dirty Editor
confirmation, and pending-close cancellation.
Visual evidence: Pending paired macOS keyboard/mouse checks for clean and dirty
items, pinned items, multiple panes, save/discard/cancel/error outcomes, and
the final-pane boundary.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact ID/argument/pinned-state/current-route audit.
Documentation updates: Two action rows, an independently worded bulk-close
packet, Workspace tabs/command crosswalks, parity index, normative parity
scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS dispatch and paired visual
acceptance are unavailable here.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace bulk close action classification

- Added [`../../docs/parity/action-classification-workspace-bulk-close.md`](../../docs/parity/action-classification-workspace-bulk-close.md)
  for `CloseAllItemsAndPanes` and `CloseInactiveTabsAndPanes`, including
  cross-pane selection, pinned-item retention, default and explicit save
  intent, partial cancellation, and the final-center-pane boundary.
- Distinguished these Workspace actions from Pane-local close actions and
  Labonair's Space-scoped universal-tab commands. Recorded that Labonair's
  preview-tab state does not implement pinned pane items and that its pending
  close queue has different cancellation behavior.
- Updated the two stable action rows, Workspace tabs and command/keymap
  crosswalks, parity index, normative parity scope, and this task's
  pre-change/progress records. The focused ID, route, save-intent, and
  cancellation audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  181 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native key dispatch,
  dirty-item prompt/cancellation outcomes, focus, layout restore, and paired
  macOS captures remain open. Same-session analysis is independent
  redevelopment, not separated-room clean-room work. R07-001 and R09 remain
  unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace Close Window

Request: Independently classify closing the current Workspace window, its
context-specific secondary-window bindings, dirty-item/call confirmation,
session handling, and Labonair's native-window close route.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-CloseWindow` across its global,
`SettingsWindow`, `SkillCreator`, and `Terminal` binding contexts.
Out of scope: `CloseProject`, app Quit, closing tabs or panes, adding commands
or bindings, implementation, R07-001/R09, and runtime or visual acceptance
claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, feature-crosswalk.md, Workspace window
lifecycle, dirty-item save/close contracts, application close policy, and
Labonair's universal tab/session model.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
Workspace close lifecycle; application composition owns process-level quit
policy; secondary window owners close their own Settings or Skill Creator
surfaces.
Canonical user entry point: Pinned File > Close Window and macOS
`Cmd+Shift+W` globally; Linux `Ctrl+Shift+W`; Windows `Ctrl+Shift+W`, with
additional SettingsWindow/SkillCreator context bindings and Windows Terminal
`Alt+F4`. Labonair has the native OS window-close affordance, `Cmd+W` for
Close Current Tab, and `Cmd+Shift+W` for Close Active Pane while the Terminal
context is active; it has no Workspace Close Window command or menu route.
Public typed contract: Close the current window only after its Workspace
passes the close preparation flow. Dirty items may be serialized to hot-exit
storage when the Workspace remains reachable; otherwise users must be able to
save, discard, or cancel; cancelling keeps the window and dirty content open,
while an explicit discard accepts losing those unsaved edits. Failed saves may
offer a close-without-saving choice. The pinned flow may also ask for
confirmation before leaving an active call when this is the sole Workspace
window. App Quit and Close Project remain distinct. No new event, registry, or
API is required for this inventory slice.
State and persistence: The close flow may serialize dirty items, remove a
Workspace from session state, flush window serialization, and remove the
window. Whether the final-window action quits the process depends on platform
and application policy. Verify session restore, last-window policy, and
reopening behavior; do not equate a window close with tab close.
Settings: Map existing last-window/session-restore and active-session
confirmation behavior where it affects the user flow; add no setting in this
slice.
Notifications and failure: Dirty-item save/discard/cancel and active-call
confirmation can block closure. A failed or cancelled close keeps the window
available and resets its removal state. Labonair's native close callback uses
the notification center for a running-shell confirmation; its behavior is
not equivalent to the Workspace dirty-item preparation flow.
Events and registry contributions: Keep the Workspace owner and platform
window composition boundary; add no second shell close table or global
feature-specific conditional.
UI-kit: Reuse shared dialog and notification surfaces; do not add a local
window-close control in this inventory slice.
Focused inspection: Trace every action context, File menu route, active-call
prompt, dirty-buffer hot-exit and save/discard/cancel behavior, save failure,
last-window policy, session removal/restore, local OS close callback, current
tab/pane key conflicts, and secondary Settings/Skill Creator windows.
Visual evidence: Pending paired macOS checks for menu/key closure, dirty
buffers, active call, cancellation, another surviving window, and final-window
behavior.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact context/key/route/persistence audit.
Documentation updates: One action row, a Workspace Close Window packet,
app.windows and command/keymap crosswalks, parity index, normative parity
scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS dispatch and paired captures are
unavailable here.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace Close Window action classification

- Added [`../../docs/parity/action-classification-workspace-close-window.md`](../../docs/parity/action-classification-workspace-close-window.md)
  for the global, SettingsWindow, SkillCreator, and Windows Terminal contexts
  of `CloseWindow`.
- Recorded the current-window close-preparation flow, dirty-item
  save/discard/cancel choices, active-call confirmation, session/hot-exit
  handling, final-window policy, and separation from app Quit, Close Project,
  tab close, and Pane close.
- Distinguished Labonair's native running-shell/session-restore callback from
  the missing Workspace close action. Recorded its `Cmd+W` tab close and
  Terminal-context `Cmd+Shift+W` pane close collision with the pinned global
  Close Window shortcut. Updated the action row, app.windows and command/keymap
  crosswalks, parity index, normative parity scope, and this task's records.
- Passed the focused action-context/route audit, `python3
  scripts/check_documentation.py` (43 normative documents, 182 Markdown
  files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native key/menu
  dispatch, dirty-item and call prompts, cancellation, session restore,
  final-window process policy, and paired macOS captures remain open.
  Same-session analysis is independent redevelopment, not separated-room
  clean-room work. R07-001 and R09 remain unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace path-copy actions

Request: Independently classify copying absolute and project-relative paths
from the pinned OutlinePanel, ProjectPanel, and Git ChangesList contexts,
including selection cardinality, path formatting, and current Labonair routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `CopyPath` and `CopyRelativePath` under their stable action IDs.
Out of scope: Copying file contents or names, OS reveal/open actions, SFTP's
separate path-copy menu, path editing, implementation, R07-001/R09, and
runtime or visual acceptance claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, Workspace action ownership, Explorer and
Git selected-entry contracts, project/worktree path styles, and clipboard
behavior.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
shared path-copy action intent and key routes. Explorer and SCM/Git owners
provide the selected eligible path or paths; no new registry is required.
Canonical user entry point: macOS uses `Cmd+Alt+C` and `Cmd+Alt+Shift+C` in
OutlinePanel when not editing, ProjectPanel, and GitPanel ChangesList outside
branch/repository selectors. Linux binds Copy Path to `Alt+Insert` or
`Ctrl+Alt+C` in GitPanel ChangesList; Copy Relative Path uses `Alt+Shift+Insert`
or `Alt+Ctrl+Shift+C` in OutlinePanel, ProjectPanel, and GitPanel ChangesList.
Windows binds Copy Path to `Shift+Alt+C` in GitPanel ChangesList; Copy
Relative Path uses `Ctrl+Shift+Alt+C` in OutlinePanel and `Ctrl+K`
`Ctrl+Shift+C` in ProjectPanel and GitPanel ChangesList. Each panel also offers
the corresponding context-menu item. Labonair exposes these actions from the
Explorer row context menu, without the same keyboard action IDs or Git/Outline
routes.
Public typed contract: Copy the selected path to the system clipboard without
mutating the filesystem. Absolute copy returns the selected entry's absolute
path; relative copy returns its path relative to the containing project or
worktree, rendered with the project path style. A ProjectPanel multi-selection
copies one path per line; OutlinePanel and Git ChangesList use their eligible
selected entry. An absent or ineligible path leaves the clipboard unchanged or
propagates to the next handler. Verify selection order and path formatting for
all owners and platforms.
Events and registry contributions: Keep the shared Workspace action names and
panel-owned selection behavior; add no clipboard registry or duplicate
global action table.
Settings and persistence: No settings or persisted state. The result replaces
the system clipboard contents only after an eligible selection is resolved.
Notifications and failure: No notification is expected. Missing selection,
unsupported row type, or unavailable path must not write a stale or partial
clipboard value.
Security: This is a user-initiated disclosure of a path to the operating
system clipboard. Do not log copied paths or add automatic copying.
UI-kit: Reuse shared context-menu components; add no local menu styling.
Focused inspection: Trace single and multiple selections, directories,
worktree roots, platform path separators, multi-root projects, remote roots,
OutlinePanel editing and non-filesystem rows, Git selectors/changes rows, and
the clipboard result after an ineligible selection.
Visual evidence: Pending paired macOS checks for Explorer, OutlinePanel, and
Git context menus, keyboard contexts, multi-selection, and clipboard readback.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact binding-context/selection/path/route audit.
Documentation updates: Two action rows, a Workspace path-copy packet,
app.windows or project-explorer/Git crosswalks as applicable, parity index,
normative parity scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS dispatch and paired captures are
unavailable here.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace path-copy action classification

- Added [`../../docs/parity/action-classification-workspace-path-copy.md`](../../docs/parity/action-classification-workspace-path-copy.md)
  for `CopyPath` and `CopyRelativePath`, recording absolute versus
  worktree-relative output, project path style, panel-specific selection
  cardinality, and empty/unsupported selection behavior.
- Recorded the exact platform binding contexts, including Outline's
  not-editing condition and Git branch/repository selector exclusions. Mapped
  Labonair's Explorer row menu and distinguished its separate SFTP path menu;
  Git ChangesList and Outline routes remain missing.
- Updated two action rows, the Explorer and Git feature crosswalks, command/
  keymap counts, parity index, normative parity scope, and this task's
  pre-change/progress records. The focused ID/context/clipboard audit passed.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  183 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active), and
  `git diff --check`.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native keyboard
  dispatch, clipboard text/order/path style, failure behavior, and paired
  macOS captures remain open. Same-session analysis is independent
  redevelopment, not separated-room clean-room work. R07-001 and R09 remain
  unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace save actions

Request: Independently classify active-item Save, workspace-wide Save All,
Save As, and Save Without Format, including typed save intents, formatting
settings, conflicts, and current Labonair Editor routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `Save`, `SaveAll`, `SaveAs`, and `SaveWithoutFormat` under their
stable action requirement IDs. Include the optional `save_intent` arguments
accepted by Save and SaveAll.
Out of scope: Auto Save timers, close/quit save flows, standalone formatter
actions, adding keybindings or commands, implementation, R07-001/R09, and
runtime or visual acceptance claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, Editor file lifecycle and dirty-state
contracts, Workspace active-pane/item model, format-on-save settings, and
save conflict/recovery behavior.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
four user actions and their scope. Editor (`labonair-editor`) owns document
content, dirty state, formatting, path, and disk writes for Editor items.
Canonical user entry point: macOS `Cmd+S`, `Cmd+Alt+S`, `Cmd+Shift+S`, and
`Cmd+K S`; Linux `Save`/`Ctrl+S`, `Alt+Save`/`Ctrl+Alt+S`,
`Shift+Save`/`Ctrl+Shift+S`, and `Ctrl+K S`; Windows `Ctrl+S`, `Ctrl+K S`,
`Ctrl+Shift+S`, and `Ctrl+K Ctrl+Shift+S`, respectively. Labonair exposes
active Editor Save in its File menu and command palette; its descriptor has no
published default key binding, and no default Save All, Save As, or Save
Without Format route is registered.
Public typed contract: Save targets the active item in the active center pane
and defaults to `Save`, which writes even if unchanged. Save All targets dirty
items across Workspace center panes and defaults to `SaveAll`, which writes
items with local changes. Save As always requests a new destination for the
active saveable item. Save Without Format saves the active item while
bypassing automatic formatting for this invocation. Save/SaveAll's optional
`save_intent` supports Save, FormatAndSave, SaveWithoutFormat, SaveAll, SaveAs,
Close, Overwrite, and Skip; record behavior of each used intent. Preserve the
active item, dirty-state, and file identity correctly after success, conflict,
failure, and cancellation.
Events and registry contributions: Keep action ownership in Workspace and
file writes in Editor; add no parallel save registry or generic shell save
table.
Settings and persistence: No new setting or schema. Map the pinned effective
per-language `format_on_save` setting (`SET-LANGUAGE-FIELD-format-on-save`)
and Labonair's global `editor.editorFormatOnSave` counterpart. Pinned
SaveWithoutFormat bypasses automatic format-on-save; FormatAndSave forces it.
No settings are persisted by the save action itself beyond normal document
file writes and Workspace session state.
Notifications and failure: Save, Save As, and Save Without Format route
errors through the save error surface. Save All is asynchronous and its error
handler is log-only in the static route; determine which item failures are
surfaced and whether earlier saves remain committed. External-change and
deleted-file prompts must preserve the user's edits on Cancel.
UI-kit: Reuse the shared command/menu surfaces, file picker, prompts, and
notification center; add no local save button or dialog style in this slice.
Focused inspection: Trace active item/pane scope, readonly/no-item behavior,
pathless documents, SaveAs path selection/overwrite, dirty and clean writes,
per-language formatter modes, forced/bypassed formatting, file conflicts,
deleted files, save errors, multiple dirty items, partial Save All, and local
menu/keymap registration.
Visual evidence: Pending paired macOS checks for the File menu, Save As
picker, conflict/deleted-file prompts, formatter availability, and dirty
indicators before/after save.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact ID/intent/key/context/current-route audit.
Documentation updates: Four action rows, a Workspace save packet,
Editor/settings and command-keymap crosswalks, parity index, normative parity
scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native macOS dispatch, file picker interaction,
and paired captures are unavailable here.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace save action classification

- Added [`../../docs/parity/action-classification-workspace-save.md`](../../docs/parity/action-classification-workspace-save.md)
  for `Save`, `SaveAll`, `SaveAs`, and `SaveWithoutFormat`, recording active
  item versus cross-pane dirty-file scope, typed save intents, path-picker and
  conflict behavior, partial batch failures, and format-on-save semantics.
- Updated the four stable action rows with pinned outcomes, platform bindings,
  Workspace/Editor ownership, and current Labonair route gaps. Linked the
  packet from the parity index, Editor and command/keymap crosswalks, Settings
  crosswalk, and normative parity scope. The Editor Settings record already
  contains the pinned language field and local global Boolean.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  184 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused save ID/context/owner/intent/failure
  audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native key
  dispatch, Save As picker and conflict interaction, formatter behavior,
  partial Save All failure visibility, and paired macOS captures remain open.
  Same-session analysis is independent redevelopment, not separated-room
  clean-room work. R07-001 and R09 remain unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace Reopen Last Picker

Request: Independently classify `ReopenLastPicker`, including eligible picker
modals, per-window stash behavior, state and focus restoration, active-modal
timing, and the current Labonair modal route.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-ReopenLastPicker` in Workspace context on macOS,
Linux, and Windows. Record the pinned modal-layer and generic Picker opt-in
contract, including behavior when no eligible modal is stashed.
Out of scope: Implementing a modal stash or picker registry, changing picker
owners, enumerating every dynamically contributed picker workflow, unrelated
picker presentation and selection semantics, R07-001/R09, and runtime or
visual acceptance claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, Workspace modal hosting and focus
contracts, and the owning picker views' dismiss/state behavior.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
action and modal lifecycle. Each eligible picker owner retains its query,
selection, and other view state while its existing modal entity is stashed.
Canonical user entry point: macOS `Cmd+K Cmd+P`; Linux and Windows
`Ctrl+K Ctrl+P`, in Workspace context. No Labonair command descriptor or
default action binding currently exposes this action.
Public typed contract: Reopen the most recently dismissed reopenable picker
modal in the current Workspace window, revealing the same view with its query,
selection, and other transient state intact. Keep only one stashed view; a
newly dismissed eligible picker replaces the previous stash. Dismissing a
non-reopenable modal leaves the stash intact. With no eligible stash, do
nothing. If another modal is active when the action is dispatched, reveal the
stash once that modal closes; command-palette invocation must tolerate its
asynchronous dismissal. Focus the restored picker and preserve normal modal
focus restoration when it is later dismissed.
Events and registry contributions: Generic pinned Picker views are eligible
by default; the command palette explicitly opts out. Workspace owns the
single per-window eligible-modal stash and deferred reveal. Labonair currently
has no corresponding reopenability contract or stashed-modal state; do not
add either in this inventory slice.
Settings and persistence: No settings, disk persistence, or session restore.
The one-entry stash exists only for the lifetime of its Workspace modal layer.
Notifications and failure: An empty stash is a silent no-op. A pending active
modal delays the reveal until dismissal; verify dismissal-blocked or
never-closing modal cases and ensure an explicit new modal request does not
reveal stale content unexpectedly.
UI-kit: Reuse the owning picker view and shared modal/focus controls. This
action must reveal the existing view, not construct a new local picker or
duplicate its inputs/list styling.
Focused inspection: Trace picker eligibility, default opt-in and opt-out,
replacement and retention, same-view state preservation, empty
stash, active-modal ordering, command-palette dismissal, focus transfer,
per-window scope, and Labonair's current modal teardown.
Visual evidence: Pending paired macOS checks for reopening an Open File or
other eligible picker with a non-empty query, selected row, keyboard focus,
and a different modal open before the action.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action-ID/key/context/owner/stash-route audit.
Documentation updates: One action row, a Workspace reopen-picker packet,
shared-controls and command/keymap crosswalks, parity index, normative parity
scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native key dispatch, focus behavior, and paired
macOS captures are unavailable here.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace Reopen Last Picker classification

- Added [`../../docs/parity/action-classification-workspace-reopen-last-picker.md`](../../docs/parity/action-classification-workspace-reopen-last-picker.md)
  for `ACT-MACOS-workspace-ReopenLastPicker`, recording eligible-picker
  behavior, one-entry per-window retention, same-view state preservation,
  active-modal ordering, and focus restoration without prescribing reference
  implementation mechanics.
- Updated the stable action row with macOS/Linux/Windows bindings, Workspace
  ownership, and the missing local route. Linked the packet from the parity
  index, shared-controls and command/keymap crosswalks, and normative parity
  scope. Static inspection confirms Labonair's modal host drops dismissed
  views and File Finder creates a fresh view on open.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  185 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused action ID/bindings/context/owner/state/
  focus/evidence audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native dispatch,
  picker-owner coverage, modal ordering and focus interaction, and paired
  macOS captures remain open. Same-session analysis is independent
  redevelopment, not separated-room clean-room work. R07-001 and R09 remain
  unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace Add Folder to Project

Request: Independently classify adding one or more selected directories to
the current project, including multi-root behavior, project-panel visibility,
collaboration restrictions, persistence, and current Labonair routes.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-AddFolderToProject` and its File menu and
project-root context-menu entry points. Record its `RecentProjects` and nested
picker key contexts on all three desktop platforms.
Out of scope: General Open/OpenFiles, opening a folder as a new project or
Space, creating/renaming/removing folders, remote picker protocol design,
implementation, R07-001/R09, and runtime or visual acceptance claims.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, Workspace project identity and
worktree/root lifecycle, Explorer root presentation, native or project-aware
directory selection, and persisted Workspace/worktree state.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
action and adds selected roots to the active project's worktree state.
Filesystem (`labonair-filesystem`) supplies path and watcher behavior;
Explorer owns presentation of the resulting roots.
Canonical user entry point: File > Add Folder to Project; the project-root
context menu's Add Folders to Project entry; macOS `Cmd+Shift+A` and Linux/
Windows `Ctrl+Shift+A` while the Recent Projects surface or its nested picker
editor context is active. Labonair exposes Open Project, which replaces the
active Space's single project root, but no add-root route.
Public typed contract: Present a directory-only multi-select picker scoped to
the current project. Attempt to add each accepted directory to that project as
a visible root/worktree, retaining already open content and existing roots.
Cancel changes nothing. Reject adding folders to a collaboratively owned
project with a user-visible explanation. Preserve deterministic root/selection
state when several or nested directories are accepted. The pinned handler
continues after per-path failures and logs them; determine their user-visible
recovery rather than claiming atomic rollback. Non-collaborative remote-project
behavior and overlap with existing roots require explicit acceptance evidence.
Events and registry contributions: Keep the action in Workspace. Root changes
flow through the Workspace project's existing typed worktree lifecycle and
Explorer's root presentation; add no shell-wide file operation table or
duplicate Explorer-only owner.
Settings and persistence: No action-specific setting. Accepted roots become
current-project worktree state and participate in normal Workspace/project
serialization, rather than only the transient picker result. Verify reopening
the project restores its root set and missing/moved-root recovery.
Notifications and failure: Cancellation is silent. The pinned handler reports
the collaboration restriction and logs per-path open errors; whether picker
and root-creation failures are visible and how users retry remain to verify.
For multi-selection, retain successful roots if another path fails and record
the remaining paths and recovery state.
UI-kit: Use the native or project-aware shared directory picker and existing
Explorer/workspace surfaces; add no local duplicate picker style.
Focused inspection: Trace action registration and all entry points, context
matching, directory-only/multiple selection, current-project scope, local,
remote and collaborative cases, duplicate/nested roots, visible-root ordering,
per-path errors, cancellation, focus, and persisted reopen/recovery.
Visual evidence: Pending paired macOS checks for File menu and root context
menu, directory picker, multi-root Explorer state, cancellation, errors, and
narrow-window presentation.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action-ID/binding/context/owner/root/persistence/route
audit.
Documentation updates: One action row, a Workspace add-folder packet,
app.windows and project.explorer crosswalks, command/keymap crosswalk, parity
index, normative parity scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Native picker interaction, remote behavior,
and paired macOS captures are unavailable here.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace Add Folder to Project classification

- Added [`../../docs/parity/action-classification-workspace-add-folder.md`](../../docs/parity/action-classification-workspace-add-folder.md)
  for `ACT-MACOS-workspace-AddFolderToProject`, recording directory-only
  multi-selection, visible roots in the current project, collaboration
  rejection, worktree persistence, partial-path errors, and recovery states.
- Updated the stable action row with macOS/Linux/Windows bindings, Workspace,
  Filesystem, and Explorer ownership, and Labonair's missing multi-root path.
  Linked the packet from the parity index, application-window and
  project-explorer crosswalks, command/keymap crosswalk, and normative parity
  scope. Static inspection confirms Labonair Open Project re-roots a Space's
  single project root; it is not this action.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  186 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused action ID/bindings/context/owner/
  multi-root/persistence/route audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native picker
  acceptance, non-collaborative remote roots, partial-failure visibility,
  restore/recovery, and paired macOS captures remain open. Same-session
  analysis is independent redevelopment, not separated-room clean-room work.
  R07-001 and R09 remain unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace collaborator-follow actions

Request: Independently classify `FollowNextCollaborator` and `Unfollow`,
including active-pane scope, collaborator cycling, remembered targets,
disconnection/error states, and current Labonair ownership gaps.
Request type: Requirements and parity inventory; no runtime behavior change.
In scope: `ACT-MACOS-workspace-FollowNextCollaborator` and
`ACT-MACOS-workspace-Unfollow` with their default macOS/Linux/Windows
bindings. Record the related Vim/Helix overlay bindings for FollowNext as a
cross-reference without replacing the stable default-action IDs.
Out of scope: Account, channel, call, project sharing, collaboration transport,
Agent workflows, implementing a collaboration owner, and paired runtime or
visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, Workspace active-pane and focus
contracts, followable view state, collaborator membership/presence, and the
planned Collaboration capability contract.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
two key actions and active-pane follow projection. The planned Collaboration
capability owns collaborator identity, call membership, and follow transport;
it has no canonical Labonair crate yet and its typed Workspace boundary is
unresolved.
Canonical user entry points: Follow Next Collaborator is global: macOS
`Ctrl+Alt+Cmd+F`, Linux `Ctrl+Alt+Shift+F`, Windows `Ctrl+Shift+Alt+F`.
Unfollow is `Escape` in Workspace context on all three platforms. The Vim/
Helix overlay also binds Follow Next Collaborator to `[ f` and `] f` in
normal, visual, and operator modes. Labonair has no collaboration actions or
default bindings.
Public typed contract: Follow Next Collaborator targets the active center
pane. If that pane follows a project collaborator, move to the next available
target and wrap; when not following, restore its previous target if still
available or select the first available collaborator. With no available
target, leave follow state unchanged. Record the order presented to users;
do not assume it is alphabetical. Starting a follow synchronizes the pane to
the selected leader's active view. Unfollow identifies the leader associated
with the active center pane, stops that follow session, and otherwise does
nothing. It does not leave the call or unshare the project. If that session
also backs another follower surface, return that surface to its local state
as well. Test the one-collaborator case, where pinned cycling can resolve to
the current target, and Agent targets separately.
Events and registry contributions: Workspace owns the action adapter and
pane-local projection. Collaboration must provide typed membership, follow,
and stop-follow contracts; do not add transport or peer state to the shell or
store another module's private membership map in Workspace.
Settings and persistence: No setting is introduced by either action. Follow
state and a pane's previous leader are transient interaction state, not saved
Editor or project content. Account, project sharing, and call settings belong
to the future Collaboration capability. Verify what survives disconnect,
project switch, pane close, and session restart.
Notifications and failure: With no eligible collaborator, Follow Next is a
no-op. Pinned follow and unfollow network failures are logged by the detached
action path; determine whether another collaboration surface informs the user
and how to retry or clear stale follow UI. Removing a collaborator clears
their follow state. Local editing may itself leave follow mode and must not be
mistaken for a failed keyboard action.
UI-kit: Reuse shared collaborator identity, active-view, cursor, and status
surfaces supplied by the owning modules; this inventory slice adds no local
presence indicator or control.
Focused inspection: Trace action contexts and key precedence, center-pane
scope with multiple panes and focused docks, candidate order and wrap,
remembered target, no and one collaborator, Agent, stale/departed leader,
network failure, local edit, explicit Escape, call leave, pane close, and
session restore.
Visual evidence: Pending paired macOS checks for followed-leader identity,
active-view/cursor tracking, pane focus, Escape precedence, no peers, and
connection loss in narrow and multi-pane windows.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action-ID/binding/context/owner/active-pane/state/failure
audit.
Documentation updates: Two action rows, a collaborator-follow packet,
collaboration and command/keymap crosswalks, parity index, normative parity
scope if needed, and this task's progress record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Labonair has no current collaboration runtime;
native dispatch and paired macOS capture remain unavailable. R07-001 and R09
remain unstarted.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace collaborator-follow classification

- Added [`../../docs/parity/action-classification-workspace-collaborator-follow.md`](../../docs/parity/action-classification-workspace-collaborator-follow.md)
  for `FollowNextCollaborator` and `Unfollow`, recording active center-pane
  scope, target cycling and remembered targets, the one-participant edge case,
  transient state, failures, and the unresolved Workspace/Collaboration owner
  boundary.
- Updated two stable action rows with default platform bindings and owner
  routes. Cross-referenced both Vim/Helix overlay bindings and updated the
  collaboration and command/keymap crosswalks, parity index, and normative
  parity scope. Static repository inspection confirms no Labonair collaboration
  runtime, follow state, command, or default binding.
- Passed `python3 scripts/check_documentation.py` (43 normative documents,
  187 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused action ID/binding/context/owner/Vim
  overlay/crosswalk audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, and tests. They remain R07-000 exit gates. Native bindings,
  participant ordering, one/no-participant behavior, network recovery,
  collaboration owner resolution, and paired macOS captures remain open.
  Same-session analysis is independent redevelopment, not separated-room
  clean-room work. R07-001 and R09 remain unstarted.
- Next safe task: continue R07-000 with the next unclassified default-map
  action family and reconcile the remaining action catalog.

## Pre-change contract — classify Workspace Toggle Worktree Security

Request: Independently classify the Restricted Mode trust decision reached by
`ToggleWorktreeSecurity`, including default-deny behavior, trust scope
validation, persistence, consumers, and current Labonair security gaps.
Request type: Requirements and security-boundary inventory; no runtime change.
In scope: `ACT-MACOS-workspace-ToggleWorktreeSecurity`, its relationship to
unbound `ACT-REGISTRY-LINUX-workspace-ClearTrustedWorktrees`, and
`SET-general-session-trust-all-projects`. Record the default global key on
macOS, Linux, and Windows, the action's alternate meaning while the decision
modal is open, and the global auto-trust setting's default and path mismatch.
Out of scope: Implementing project trust or clearing trust in this slice,
other Settings beyond the global trust override, SSH host-key acceptance,
MCP tab grants, implementation, R07-001/R09, and runtime or visual acceptance.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, settings-requirements.md,
`docs/security/trust-boundaries.md`, Workspace project/worktree identity,
shared modal/dismissal contracts, persisted security decisions, and
Editor/MCP execution gates.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns the
action, Restricted Mode decision surface, and user-selected trust scope for
project/worktree paths. Settings stores the global auto-trust value only.
Editor, Agent, Git, and MCP consumers use a narrow typed trust decision before
project-controlled behavior runs. Keep trust state and persistence inside
Workspace; do not put the policy in settings or shell conditionals.
Canonical user entry point: macOS `Ctrl+Cmd+S`, Linux `Ctrl+Alt+Shift+S`, and
Windows `Ctrl+Shift+Alt+S`, global; the titlebar Restricted Mode warning and
Agent profile-picker warning also open the decision. When the decision is open,
the same action selects Stay in Restricted Mode. Labonair has no project-trust
action, mouse route, dialog, or default binding. SSH host trust is separate.
Public typed contract: If the current project contains restricted worktrees
or single-file roots, open a blocking decision that lists affected paths and
explains that Restricted Mode blocks project settings, language servers, and
MCP server installation, and asks the user to review project settings for
configured commands or extensions. Agent project-local skills and disallowed
tools are restricted, and Git repository workers receive trust state; the
global setting and explicit trust decisions can affect linked-worktree trust.
Explicit Trust and Continue trusts the listed paths.
The optional, unchecked parent-trust checkbox trusts a containing folder; for
one directory worktree it exposes a read-only-until-checked editable path that
defaults to the project parent. That path must be nonempty, absolute (or
home-relative with tilde expansion), and an ancestor or equal to the project.
Multiple directory worktrees use a static parent-folder scope; single-file
roots use a separate “Trust all single files” choice. Stay in Restricted Mode
leaves paths restricted. While the modal is active, the same action chooses
Stay and dismisses it. If nothing is restricted, the action is a no-op.
Escape and outside-click dismissal are blocked until a choice is made.
Events and registry contributions: Keep action presentation in Workspace.
Publish trust decisions through a narrow typed contract consumed by project
settings, language services, Agent, Git, and MCP owners before they expose or
execute project-controlled behavior. Keep this policy distinct
from SSH host-key trust and MCP per-tab grants.
Settings and persistence: The general Security switch “Trust All Projects By
Default” skips per-project Restricted Mode consent; its declared default is
false. The pinned Settings UI names its JSON path `session.trust_all_projects`,
while the backing settings value is named `trust_all_worktrees`; verify the
effective serialized path and live behavior. Preserve this global opt-in as a
Settings value whose effect is Workspace-owned. Individual trust decisions
remain security state, not project settings. Persist explicit trusted paths
with local or remote-host identity. The pinned Restricted Mode trust flow
covers local, SSH, and WSL projects; Docker and collaboration remotes use
provider-managed behavior that needs its own verification. Verify host
isolation, single-file and directory trust, optional parent scope, restart/reopen,
turning the global override on and off, and clearing trust.
`ClearTrustedWorktrees` is a separate unbound action that clears saved trust;
newly opened projects return to Restricted Mode, while already-open projects
require restart to apply the clearing. Persistence-failure recovery and the
restart outcome after a failed trust save are not established by static
evidence and must be specified and tested before implementation.
Notifications and failure: Default to Restricted Mode. Escape/outside-click
dismissal is blocked until the user chooses a button. Invalid parent scope
keeps the modal open and identifies the field error. Static source review
does not establish what the user sees after a failed trust save or whether
trust survives restart. Define explicit current-session and restart behavior,
visible retry/recovery, and fail-closed handling for a failed trust write
before implementation.
UI-kit: Use shared AlertModal, checkbox, input, button, path-list, focus, and
scroll components. Do not add a shell-local trust dialog or let dismissal
callbacks bypass explicit consent.
Focused inspection: Trace no/multiple restricted paths, directories and
single-file roots, local, SSH/WSL, Docker, and collaboration project identity, initial modal
behavior,
checkbox defaults, editable and static scopes, absolute/ancestor/~ validation,
Trust, Stay, Escape/outside click, action reuse while open, empty-state no-op,
global auto-trust enable/disable, persistence failure, project settings/LSP/
MCP restrictions, ClearTrustedWorktrees, restart, and reopen. Resolve the
Settings UI path/backing-field discrepancy with a saved-value trace.
Visual evidence: Pending paired macOS checks for warning text, affected paths,
checkbox/scope field, inline validation, explicit choices, blocked dismissal,
and focus/scroll behavior at narrow window sizes.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and exact action-ID/binding/owner/decision/scope/persistence/
consumer/security audit.
Documentation updates: One action row, the related unbound-action row and
global trust-setting record, a Workspace worktree-security packet,
`docs/security/trust-boundaries.md`, `privacy.security` and command/keymap
crosswalks, parity index, normative parity scope, and this task's progress
record.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Labonair lacks the project trust workflow;
native modal interaction and paired macOS captures are unavailable. R07-001
and R09 remain unstarted.
Next safe task: Continue R07-000 with the next unclassified default-map action
family after this slice passes its documentation gates.

## Progress record — 2026-10-10 — Workspace worktree-security classification

- Added the independent Restricted Mode packet for
  `ACT-MACOS-workspace-ToggleWorktreeSecurity`, including macOS/Linux/Windows
  global bindings, titlebar and Agent mouse entry points, explicit consent,
  single-file and directory roots, validated parent trust, global auto-trust,
  Agent and Git effects, remote-provider differences, and the unbound
  `ACT-REGISTRY-LINUX-workspace-ClearTrustedWorktrees` reset action.
- Updated the action matrix, unbound-action inventory, existing global trust
  setting record, parity index, privacy/keymap crosswalks, normative parity
  plan, and security trust-boundary contract. Labonair still has no connected
  project/worktree trust workflow; the Editor untrusted-root policy remains
  unconnected to Workspace.
- Static reference and Labonair source review only. Same-session work is
  independent redevelopment, not separated-room clean-room implementation.
- Passed: `python3 scripts/check_documentation.py` (43 normative docs /
  188 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused action ID/binding/owner/scope/settings/
  consumer/security audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native macOS action
  dispatch, titlebar/modal/Agent interactions, paired captures, global-setting
  saved-key behavior, Clear Trusted Worktrees discovery, and trust-save
  failure/retry behavior remain open.
- Other remote providers (Docker and collaboration) require their own trust
  policy verification. The global Settings UI path
  `session.trust_all_projects` differs from the backing value
  `trust_all_worktrees`; saved-value behavior is unverified.
- Next safe task: classify Workspace `ToggleZoom` (`Shift+Escape`, global)
  and continue the R07-000 action inventory. R07-001 and R09 remain unstarted.

## Pre-change contract — classify Workspace Toggle Zoom

Request: Independently classify the Workspace zoom toggle, including its
center-pane and dock-panel targets, focus and action-context routing,
appearance setting, persistence, and Labonair route.
Request type: Requirements and action ownership inventory; no runtime change.
In scope: `ACT-MACOS-workspace-ToggleZoom`, the more-specific Git CommitEditor
binding that shadows the global key, and related `ACT-REGISTRY-LINUX-workspace-ToggleEditorZoom`,
`ACT-REGISTRY-LINUX-workspace-ZoomIn`, `ACT-REGISTRY-LINUX-workspace-ZoomOut`,
and `SET-window-layout-zoomed-padding` records.
Out of scope: Implementing zoom behavior; fully classifying each related
unbound action; native OS window zoom; Editor font zoom; individual image,
diff, or preview zoom; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action-requirements.md, registered-actions-unbound.md,
settings-requirements.md, Workspace pane/dock state, the shared Panel contract,
Git CommitEditor context, current Workspace zoom controls, and session restore.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns
cross-pane and dock zoom state, focus routing, and the `ToggleZoom` action.
Pane and panel content owners expose their supported zoom transition through
the existing typed owner boundary. Do not add zoom policy to shell or a second
global action table.
Canonical user entry point: Global `Shift+Escape` on macOS, Linux, and
Windows. In the more-specific `CommitEditor > Editor` context the same key
invokes `git::ExpandCommitEditor`. Pane headers and supported Agent,
Terminal, and Debugger panel surfaces provide mouse zoom controls. Labonair
has no Workspace `ToggleZoom` action or default binding; its debug dock-zoom
command and native Window Zoom are separate routes.
Public typed contract: Toggle the zoom state of the focused zoom-capable Pane
or Panel; do nothing for an empty or non-zoomable target. One Workspace zoom
overlay is visible at a time. Zooming a center Pane overlays sibling content
and docks while retaining the Workspace layout; zooming a dock Panel overlays
the Workspace around that panel. Repeating the action restores the prior
layout. Keep this overlay distinct from `ToggleEditorZoom`, which maximizes
the active center Pane within the pane layout, and from OS Window Zoom.
Settings and persistence: the `zoomed_padding` setting defaults to true and
controls the inset around zoomed content. The pinned dock state persists its
zoom flag; center-Pane zoom is not part of serialized Pane state. Verify
restart behavior, panel registration/restoration, setting scope, save/reload,
and resets.
Events and registry contributions: Preserve Workspace as action and layout
owner. Route mouse and keyboard intent through the same typed Pane/Panel zoom
transition. The action has the three global default bindings above; the
CommitEditor context override takes precedence. Related `ToggleEditorZoom`,
`ZoomIn`, and `ZoomOut` are distinct unbound action records.
Notifications and failure: Zoom is local layout state with no expected I/O.
If focus, target teardown, or a panel refusing zoom makes the transition
unavailable, preserve the existing layout and focus and do not leave a stale
overlay target.
UI-kit: Reuse the shared icon button, tooltip, active/toggled state, panel
header, and focus styles. Do not add a shell-local zoom button or new setting.
Focused inspection: Verify center Pane and left/right/bottom Panel targets,
empty and non-zoomable content, active/inactive panes, focus transfer,
concurrent zoom requests, pane/panel close, tab changes, Git CommitEditor
precedence, the padding setting, dock restore, and session restart.
Visual evidence: Pending paired macOS checks for maximized and inset geometry,
overlap/occlusion, resize, focus, and narrow windows.
Workspace gates: Documentation checker, docs verification scope, queue check,
diff check, and focused action/binding/context/owner/setting/persistence/
distinction/current-route audit.
Documentation updates: ToggleZoom action row, Workspace zoom packet, parity
index, `workspace.docks` and `commands.keymap` crosswalks, the related
zoom-padding setting reference, normative parity plan, and this progress log.
Known limitations: Same-session analysis is independent redevelopment, not
separated-room clean-room work. Pinned key dispatch/focus precedence and
paired macOS captures are unavailable. R07-001 and R09 remain unstarted.
Next safe task: Continue R07-000 with the remaining unclassified actions.

## Progress record — 2026-10-10 — Workspace ToggleZoom classification

- Added an independent requirements packet for
  `ACT-MACOS-workspace-ToggleZoom`, covering the global Shift+Escape binding
  on macOS/Linux/Windows, the Git CommitEditor context override, center-Pane
  and dock-Panel overlays, the Zoomed Padding setting, per-target persistence,
  related unbound zoom actions, and Labonair's separate Debug dock route.
- Updated the bound action row, related setting record, parity index, Workspace
  docks/keymap/Settings crosswalks, normative parity plan, and this active
  task's contract and progress evidence.
- Static reference and Labonair source review only. The same-session work is
  independent redevelopment, not separated-room clean-room implementation.
- Passed: `python3 scripts/check_documentation.py` (43 normative docs /
  189 Markdown files), `python3 scripts/verify.py --scope docs`,
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 active),
  `git diff --check`, and the focused binding/context/setting/persistence/
  current-route audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native dispatch to
  the focused Pane/Panel, CommitEditor precedence, profile/user overrides,
  paired macOS captures, and settings live/reload behavior remain open.
- Current Labonair dock zoom is only a Debug command and resizes the dock
  within its region; the global action and center-Pane overlay are absent.
- Next safe task: classify the next unclassified Workspace action family,
  `worktree_picker::DeleteWorktree` and `ForceDeleteWorktree`. R07-001 and R09
  remain unstarted.

## Pre-change contract — classify Git worktree deletion

Request: Independently classify normal and forced deletion of a Git worktree,
including the picker entry points, open-worktree protections, dirty-tree
confirmation, destructive effects, asynchronous state, and failure recovery.
Request type: Requirements and action ownership inventory; no runtime change.
In scope: `ACT-MACOS-worktree_picker-DeleteWorktree` and
`ACT-MACOS-worktree_picker-ForceDeleteWorktree`; their macOS/Linux/Windows
bindings and `WorktreePicker || (WorktreePicker > Picker > Editor)` context;
the row/footer mouse affordances; and the distinction between deleting a Git
worktree and removing an open worktree from a window.
Out of scope: Implementing Git worktree management; classifying worktree
creation, switching, or open-in-new-window behavior beyond identifying the
picker entry point; deleting Git branches; trust policy; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action and unbound-action inventories; the Git capability
contract; Workspace's open-worktree visibility model; Git execution adapter;
the canonical notification center; shared picker, button, icon-button,
tooltip, and confirmation-dialog contracts.
Owner module and canonical crate: Git (`labonair-git`) owns worktree
discovery and lifecycle. `labonair-git-transport` executes the operation
through the configured local/remote Git adapter. Workspace provides a typed
view of worktree paths currently open across related windows so Git can
prevent deleting a live project root. The current `GitService` has no
worktree-list or worktree-remove operation; preserve one Git owner and do not
add a parallel Workspace deletion owner or registry without a discovery need.
Canonical user entry point: Open the Git Worktree picker from the titlebar
worktree selector or registered Git Worktree action. While the picker is open,
the selected eligible row offers Delete in the footer and a trash icon. The
icon's ordinary click deletes normally; holding Alt changes it to Force
Delete. The keyboard actions are available only in the WorktreePicker action
context. “Remove Worktree from Window” only closes another workspace view and
is not deletion.
Public typed contract: A request identifies one listed, non-main Git worktree
by repository identity and path and selects normal or forced removal. Reject
the main worktree, any path currently visible in a workspace in the same
project group, a missing/stale selection, and duplicate in-flight removal.
Normal removal asks Git to remove the worktree and its directory without
force. Force removal uses Git's force option, which can discard modified and
untracked files. Neither request deletes the branch. Completion reports
success or a descriptive Git failure; a successful removal invalidates the
worktree listing and removes the row. A dirty-normal-delete refusal offers an
explicit Force Delete / Cancel decision, then rechecks that the same worktree
still exists before retrying once with force.
Settings and persistence: No Labonair preference controls deletion. A
successful request changes Git's worktree metadata and filesystem; a force
request may permanently discard uncommitted and untracked files. These effects
survive restart. The branch lifecycle is separate and must not be changed by
this operation. Test repository refresh and restart discovery after success.
Events and registry contributions: Register both typed actions in the Git
picker context with the platform bindings recorded in
`action-requirements.md`. Use the Git repository's worktree-list-changed event
to refresh picker rows; keep one in-flight operation per path. Workspace
contributes only the typed open-path snapshot used for safety checks. No new
generic registry is required for one Git operation provider.
Notifications and failure: A dirty normal removal opens a warning with
Force Delete and Cancel. Cancel leaves the worktree intact. A declined or
failed operation clears its pending indicator and retains the row. Other Git
errors go through the notification center with the attempted command; retry
remains possible. If the worktree disappears while confirmation is open,
revalidate and avoid a duplicate removal. Perform Git I/O asynchronously and
never block the GPUI foreground thread.
UI-kit: Compose the picker, footer button, trash icon button, loading state,
tooltip, warning prompt, and error notification using existing shared UI-kit
and notification-center components. The force modifier must be visible in the
tooltip and destructive state, and keyboard and mouse paths must invoke the
same typed operation.
Focused inspection: Verify normal clean removal; normal dirty refusal and
Force Delete/Cancel; explicit force removal without a second prompt; main and
currently open worktree disabling; another-window “Remove Worktree from
Window” distinction; duplicate requests; a stale path during confirmation;
Git errors and retry; row refresh; and branch preservation. Check picker focus,
selection, keyboard bindings, Alt tooltip state, pending spinner, narrow
layout, and restart persistence.
Visual evidence: Pending paired native macOS captures and interaction checks
for the picker, selected/disabled rows, hover/Alt force affordance, warning,
loading, and error states. Record window bounds, theme, scale, focus, and
selected worktree. Linux host runtime and paired macOS capture access are not
available in this session.
Workspace gates: Documentation checker, docs verification scope, active
queue check, diff check, and a focused action/context/owner/safety/effect/
failure/current-route audit. Rust build, Clippy, and tests remain R07-000 exit
gates and are not run for this documentation-only slice.
Documentation updates: Both bound action rows, the independent Git worktree
deletion packet, parity index, `git.source-control` and `commands.keymap`
crosswalks, normative parity workflow, and this active task's progress record.
The existing Git capability remains the canonical owner; no new capability,
setting, or implementation queue task is added by this inventory slice.
Known limitations: Same-session source research and packet authoring are
independent redevelopment, not separated-room clean-room work. Pinned runtime
dispatch, native macOS captures, real Git failure recovery, and cross-window
open-path races remain open acceptance evidence. R07-001 and R09 remain
unstarted.
Next safe task: Continue R07-000 with the remaining unclassified actions.

## Progress record — 2026-10-10 — Git worktree deletion classification

- Added an independent requirements packet for
  `ACT-MACOS-worktree_picker-DeleteWorktree` and
  `ACT-MACOS-worktree_picker-ForceDeleteWorktree`, covering all three
  platform bindings, compact and modal picker routes, main/open-worktree
  protections, dirty-tree confirmation, force data loss, asynchronous state,
  error recovery, and the distinction from removing a worktree from a window.
- Recorded Git (`labonair-git`) as the lifecycle owner and Workspace as the
  source of the typed open-path guard. The current `GitService` has no
  worktree-list or removal API. No setting, capability, or new active queue
  item is introduced by this inventory slice.
- Updated both bound action rows, the parity packet index, Git and keymap
  crosswalks, normative parity workflow, and this task's pre-change contract
  and progress record.
- Static reference and Labonair source review only. The same-session work is
  independent redevelopment, not separated-room clean-room implementation.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 190 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused requirement-ID, binding, safety/failure,
  cross-reference, task-order, and Git API audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native keyboard
  dispatch, dirty-tree confirmation/cancellation, actual file/branch effects,
  cross-window open-path races, Git failure recovery, and paired macOS visual
  acceptance remain open.
- Next safe task: classify the adjacent bound font-size action pair,
  `zed::DecreaseBufferFontSize` and `zed::DecreaseUiFontSize`, within R07-000.
  R07-001 and R09 remain unstarted.

## Pre-change contract — classify font-size decrease actions

Request: Independently classify the bound buffer-font and UI-font decrease
actions, including their distinct scopes, context precedence, transient versus
persistent arguments, settings effects, and Labonair's prior no-op-command
removal.
Request type: Requirements and action ownership inventory; no runtime change.
In scope: `ACT-MACOS-zed-DecreaseBufferFontSize` and
`ACT-MACOS-zed-DecreaseUiFontSize`; their default macOS/Linux/Windows bindings,
the Onboarding/Welcome context overrides, `persist:false` behavior, the
buffer-size View menu action, the conditional Command/Ctrl+wheel buffer-size
gesture, and related `SET-appearance-buffer-font-size`,
`SET-appearance-ui-font-size`, `SET-editor-mouse-wheel-zoom`, and terminal
font-size records.
Out of scope: Implementing font zoom; classifying the separate Increase and
Reset action IDs; restoring the removed no-op `ZoomIn`/`ZoomOut`/`ZoomReset`
command IDs as-is; other font families/sizes such as Agent, Git Commit,
Markdown Preview, or terminal-specific controls; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action and settings inventories; the Themes typography owner;
Settings value and Settings UI contracts; Editor and Terminal font consumers;
ThemeStore live-update contract; keymap contexts and alternate profiles; the
historical R08-002 no-op command removal record.
Owner module and canonical crate: Themes (`labonair-theme`) owns global UI and
shared buffer typography state and the live adjustment contract. Settings
(`labonair-settings`) owns configured values; `labonair-theme-ui` applies
those values to the ThemeStore, and Editor/Terminal consume the shared buffer
metrics. A terminal-specific size may override the shared buffer value. Do
not put font-zoom policy in the shell or make Settings own action behavior.
Canonical user entry point: The pinned global `Command+-` action and View >
Zoom Out both decrease buffer text size. In the more-specific `Onboarding` and
`Welcome` contexts, `Command+-` invokes DecreaseUiFontSize and changes the
application UI font instead. Linux/Windows use `Ctrl+-` for both routes. The
JetBrains keymap adds an Editor-context `Ctrl+<` buffer decrease with
`persist:true`. In an eligible Editor, Command+wheel on macOS or Ctrl+wheel on
Linux/Windows also changes buffer size when the global mouse-wheel setting is
enabled; the UI-font action has no mouse route. Labonair has no matching
mouse-wheel setting or action. Its font-size Settings remain visible but the
old Zoom In/Out/Reset menu, palette, and default-keymap affordances were
removed under R08-002 because they had no handler and no product decision.
The active parity goal supplies that product requirement; it does not
authorize restoring the old dead command IDs.
Public typed contract: Each action lowers its own effective font size by one
pixel, bounded by the pinned 6–100 px clamp. DecreaseBufferFontSize adjusts
the shared buffer text size used by Editor and by Terminal when the separate
terminal size is unset. DecreaseUiFontSize adjusts UI font size, which affects
all UI elements and windows even though its default key route is available
only from Onboarding and Welcome. The more-specific UI context wins over the
global buffer action for the same key. With `persist:false`, store a transient
app-wide override and refresh all open windows without editing Settings; it
lasts until restart or a corresponding setting change. With `persist:true`,
write the clamped value to the user's `theme.buffer_font_size` or
`theme.ui_font_size` setting. Repeated decrease stops at the lower bound.
Settings and persistence: The pinned settings inventory records
`SET-appearance-buffer-font-size` default 15 and
`SET-appearance-ui-font-size` default 16, both user-scoped; a configured
terminal font size overrides buffer inheritance.
`SET-editor-mouse-wheel-zoom` is user-scoped and defaults to false;
Command/Ctrl+wheel is inactive until enabled, and some embedded Editor
surfaces disable the gesture themselves. The wheel gesture always makes a
transient buffer adjustment. The persistent argument updates the matching
user setting, while the shipped default binding remains transient. The local Settings model has `appearance.appFontSize` and
`appearance.bufferFontSize` plus separate editor and terminal font-size
fields; its defaults/ranges and ThemeStore bridge do not match the pinned
values one-to-one. Reconcile the canonical Labonair value path and effect
before implementation; do not add another size field or silently conflate
the existing ones.
Events and registry contributions: The Themes owner supplies typed decrease
actions and updates one ThemeStore typography state. A real settings write
uses the Settings value owner; a transient adjustment only changes runtime
state and refreshes windows. Editor owns its mouse-wheel gesture and the
`mouse_wheel_zoom` gate, routing an eligible gesture to the Theme-owned buffer
adjustment contract. Keep the global buffer action and scoped UI action
context-aware in Keymap. Do not add a generic font-action registry for this
single owner.
Notifications and failure: Transient adjustment has no I/O and should not
emit an operation notification. Persistent adjustment must respect the
Settings save result: on failure preserve a truthful displayed value and
surface recovery through the notification center. A successful decrease
needs no toast. The separate Reset action family restores the configured
base value and is classified independently.
UI-kit: Keep Settings field controls on the shared Settings/UI-kit components.
Use the existing application menu and Keymap surfaces for discoverability;
do not add local +/- controls solely for these actions. UI scale changes must
update shared tokens/spacing consistently with typography.
Focused inspection: Verify the effective one-pixel step and 6 px lower bound;
global buffer scope; terminal inheritance and explicit terminal override;
global UI effect across multiple windows; Cmd+- precedence in Onboarding and
Welcome; menu behavior; Command/Ctrl+wheel modifier mapping, the false-by-
default mouse-wheel setting, enabled and ineligible Editor surfaces; persisted
true versus default false; keymap-profile arguments; restart, live Settings
edits, and resets; persistent-write failure; and interaction with the existing
duplicate local buffer-size fields.
Visual evidence: Pending paired native macOS captures for normal and minimum
buffer size, UI scale across windows, Editor/Terminal inheritance and
override, Settings after persistent and transient changes, narrow layout, and
Welcome/Onboarding context behavior. Record build, OS, window bounds, theme,
scale, focused context, settings values, keymap profile, and `persist` arg.
Workspace gates: Documentation checker, docs verification scope, active
queue check, diff check, and a focused action/context/argument/scope/
settings/owner/current-route audit. Rust formatting, build, Clippy, and tests
remain R07-000 exit gates and are not run for this documentation-only slice.
Documentation updates: Both bound action rows, the independent decrease
packet, parity index, keymap/appearance/Editor/Terminal crosswalks, normative
parity workflow, related setting references, and this active task's progress
record. Keep the R08-002 no-op removal record historical and explicitly
document that the active parity goal now requires real font-size actions.
Known limitations: Same-session source research and packet authoring are
independent redevelopment, not separated-room clean-room work. Native action
precedence, actual transient/persistent runtime effects, Settings save failure,
and paired macOS captures remain pending. R07-001 and R09 remain unstarted.
Next safe task: Continue R07-000 with the separate buffer/UI font increase and
reset action family; do not begin R07-001 or R09 early.

## Progress record — 2026-10-10 — Font-size decrease action classification

- Added an independent requirements packet for
  `ACT-MACOS-zed-DecreaseBufferFontSize` and
  `ACT-MACOS-zed-DecreaseUiFontSize`, covering platform bindings, the
  Onboarding/Welcome context override, 1 px adjustment, 6–100 px clamp,
  transient and persistent arguments, and the Theme/Settings owner boundary.
- Included the optional Editor primary-modifier wheel gesture, gated by
  `SET-editor-mouse-wheel-zoom` (default false) and the Editor instance;
  recorded Terminal's inherited buffer size and separate override. The packet
  also notes R08-002's historical removal of no-op Zoom routes and the current
  Labonair duplicate font-size paths and missing adjustment action.
- Updated the action rows, packet index, Settings/Editor/Terminal/Appearance
  crosswalks, normative parity plan, and this task's pre-change contract and
  progress record. Static keymap inspection confirms the JetBrains Editor
  binding persists its buffer-size change while default routes are transient.
- Static reference and Labonair source review only. Same-session work is
  independent redevelopment, not separated-room clean-room implementation.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 191 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused keymap argument/context, Settings default, mouse
  gate/modifier, local-route, crosswalk, and task-order audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Actual buffer/UI
  size changes, key-context dispatch, wheel behavior and setting update,
  transient reset, persistent-save failure recovery, and paired macOS captures
  remain open.
- Next safe task: classify the separate buffer/UI font Increase and Reset
  actions within R07-000. R07-001 and R09 remain unstarted.

## Pre-change contract — classify font-size increase and reset actions

Request: Independently classify the bound buffer/UI font-size increase and
reset actions and the View-menu-only Reset All Zoom action, including action
arguments, context precedence, four-scope reset effects, settings, and the
historical removal of Labonair's nonfunctional Zoom command routes.
Request type: Requirements and action ownership inventory; no runtime change.
In scope: `ACT-MACOS-zed-IncreaseBufferFontSize`,
`ACT-MACOS-zed-IncreaseUiFontSize`, `ACT-MACOS-zed-ResetBufferFontSize`,
`ACT-MACOS-zed-ResetUiFontSize`, and registered but default-keymap-unbound
`ACT-REGISTRY-LINUX-zed-ResetAllZoom`; default macOS/Linux/Windows bindings,
Onboarding/Welcome overrides, View > Zoom In/Reset Zoom/Reset All Zoom,
transient and persistent argument behavior, the JetBrains persistent buffer
increase override, and the existing appearance/agent font-size values.
Out of scope: Implementing or restoring the removed `CommandId::Zoom*` route;
changing menu, palette, or global chrome structure; Git Commit/Markdown
Preview font actions; terminal-specific controls beyond buffer-size
inheritance; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: action/keymap and registered-action inventories; Themes
typography state and adjustment ownership; Settings values and persistence;
Keymap context precedence; the canonical global-menu contribution contract;
the historical R08-002 removal record.
Owner module and canonical crate: Themes (`labonair-theme`) owns shared buffer,
UI, and Agent typography state and adjustment behavior. Settings
(`labonair-settings`) owns configured values; `labonair-theme-ui` applies
them to ThemeStore, and Editor, Terminal, and Agent UI consume their matching
typography scopes. Keymap owns user bindings and context resolution. Keep
the menu contribution in its existing global-menu owner/registry and do not
move font behavior into the shell.
Sibling crates: `labonair-theme-ui` adapts configured Settings values to
ThemeStore; Editor, Terminal, and Agent UI consume the state without becoming
additional typography owners. Canonical user entries are the global and
scoped Keymap bindings and View > Zoom In, Reset Zoom, and Reset All Zoom.
Canonical user entry point: View > Zoom In adjusts buffer text;
View > Reset Zoom removes the transient buffer-size adjustment; View > Reset
All Zoom removes transient UI, buffer, Agent UI, and Agent buffer adjustments.
The global `cmd-=` / `cmd-+` and `ctrl-=` / `ctrl-+` bindings increase buffer
text, and `cmd-0` / `ctrl-0` resets it. More-specific Onboarding and Welcome contexts
route those same keys to UI-font actions. Default keymap arguments are
`persist:false`. The macOS and Linux JetBrains Editor profile adds
`Ctrl+>` with `persist:true` for buffer increase. Labonair has no working
increase/reset action route; R08-002 removed the previous no-op Zoom
commands, which must not be restored without a real typed handler.
Public typed contract: Increase changes the effective selected font scope by
one pixel, bounded to 6–100 px. A transient increase creates/updates a
runtime override, refreshes open windows, and does not edit Settings. A
persistent buffer increase writes the clamped size to the configured buffer
setting. Reset with `persist:false` removes the matching runtime override and
returns to the current configured base value; it does not force the built-in
default. Reset with `persist:true` clears the matching user override so the
configured fallback applies. Reset All follows the same distinction across
UI, buffer, Agent UI, and Agent buffer scopes. Reset Zoom from the View menu
targets buffer size only; Reset All is a separate registered action and menu
item without a default key binding. UI-font key context overrides the global
buffer action in Welcome and Onboarding.
State and persistence: Pinned configured defaults are buffer 15 px, UI 16 px,
Agent UI unset/inherited from UI, and Agent buffer 12 px. The terminal uses
the shared buffer size unless its specific terminal size is set. The local
Settings model exposes duplicate `appearance.appFontSize`,
`appearance.bufferFontSize`, `editor.editorFontSize`, and terminal font paths,
with different defaults/ranges and a ThemeStore bridge. Reconcile the
canonical Labonair values and the distinct Agent scopes before implementation;
do not add another size field or collapse an explicit terminal override.
Events and registry contributions: Provide typed Themes-owned increase and
reset operations; route default key bindings through Keymap's global versus
Onboarding/Welcome contexts; expose the required actions through the existing
menu contribution. Persistent changes use Settings' value owner. Do not add
a general font-action registry for one owning module.
Allowed dependency changes: None in this inventory slice; later implementation
must preserve the acyclic crate graph. Removal condition for temporary code:
None; keep the removed no-op Zoom command path absent rather than restoring
it as compatibility.
Notifications and failures: Transient changes do not perform I/O. A
persistent settings failure must leave the displayed value truthful and
provide recovery through the notification center; successful adjustments
need no toast. Verify the pinned write failure behavior and local recovery
route at runtime.
UI-kit: Reuse the existing application menu and Settings field components;
use UI-kit-backed controls only if the existing Settings UI exposes these
values. Do not add local +/- controls to substitute for missing actions.
Implementation order: Define the Themes adjustment/reset contract; reconcile
canonical Settings values and persistence; adapt ThemeStore; update Editor,
Terminal, and Agent consumers; contribute typed Keymap and menu routes; remove
superseded value/action paths; add focused tests and native evidence.
Focused inspection: Verify exact platform key/modifier routes and profile
arguments; the global buffer vs scoped UI context dispatch; 1 px increase and
6–100 px clamp; transient reset to configured base; persistent reset clearing
the user override; four-scope Reset All; Terminal inheritance and explicit
override; Agent UI/buffer inheritance; menu availability; live Settings
changes; save failure/retry behavior; and that no removed no-op enum path is
used.
Visual evidence: Pending paired native macOS captures for increased and reset
buffer/UI sizes, Settings base values before/after transient and persistent
actions, all four Reset All scopes, Terminal inheritance/override, Agent
scopes, Welcome/Onboarding context behavior, and narrow layout. Record exact
build, OS, window bounds, theme, scale, focus/context, Settings values,
keymap profile, and action `persist` argument.
Workspace gates: Documentation checker, docs verification scope, active
queue check, diff check, and focused action/context/argument/scope/settings/
owner/current-route audit. Rust formatting, workspace check, Clippy, and
tests remain R07-000 exit gates and are not run for this documentation-only
slice.
Security impact: N/A for requirements-only documentation; typography values
contain no secrets and this slice introduces no permission boundary.
Performance impact: No runtime change in this slice. Future transient changes
should remain in memory and must not block GPUI; measure persistent I/O and
multi-window refresh cost in the implementation.
Documentation updates: Four bound action rows; the unbound ResetAllZoom
catalog row; a standalone increase/reset classification packet; parity index;
Keymap, Settings, Appearance, Editor, Terminal, and Agent crosswalks; the
normative parity workflow; and this task's progress record. Preserve R08-002
as historical evidence that the old Zoom identifiers were no-op paths; the
active parity objective requires real behavior now.
Known limitations: Same-session reference research and requirements writing
are independent redevelopment, not separated-room clean-room implementation.
Native dispatch, settings-write failure recovery, actual scope effects, and
paired macOS captures remain unverified. R07-001 and R09 remain unstarted.
Next safe task: Complete the increase/reset action-family classification and
continue R07-000 with its remaining inventory; do not begin R07-001 or R09.

## Progress record — 2026-10-10 — Font-size increase and reset classification

- Added the independent requirements packet for the global buffer increase,
  scoped UI increase, both scoped reset bindings, View > Zoom In/Reset Zoom,
  and registered View > Reset All Zoom. It records transient and persistent
  arguments, the JetBrains persistent buffer-increase override, the four
  typography scopes reset by Reset All, and unaffected Git Commit/Markdown
  Preview scopes.
- Updated four bound action rows and the ResetAllZoom unbound-action row with
  semantics, owner, current Labonair route, and runtime status; linked the
  packet from the decrease packet, parity index, feature crosswalk, and
  normative parity workflow. Updated this task with the full change contract.
- Static pinned keymap/menu/action and Labonair Settings/Theme source review
  only. Same-session research and packet authoring are independent
  redevelopment, not separated-room clean-room implementation.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 192 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused four-bound-row/ResetAllZoom scope, context,
  argument, owner, and current-route audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native action/menu
  dispatch, rendered increase/reset effects, persistent-write recovery,
  four-scope Reset All behavior, and paired macOS captures remain open.
- Next safe task: Continue classifying R07-000's remaining action and
  workflow inventory; do not begin R07-001 or R09.

## Pre-change contract — classify Agent Reset Zoom

Request: Classify the registered Agent `ResetAgentZoom` action and distinguish
its Agent-only transient reset from Zed's View-menu Reset All action.
Request type: Requirements and registered-action ownership inventory; no
runtime change.
In scope: `ACT-REGISTRY-LINUX-agent-ResetAgentZoom`; action availability when
the current Workspace has an AgentPanel; Agent UI and Agent buffer typography
fallbacks; relation to `ACT-REGISTRY-LINUX-zed-ResetAllZoom`; and the missing
Labonair AI/Agent UI route.
Out of scope: Implementing the AI/Agent panel, changing global-menu or palette
structure, classifying other Agent panel actions, changing configured font
values, or starting R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: The registered-action inventory, AI capability owner, Themes
typography state contract, Appearance Agent-size settings, and the completed
font-size increase/reset packet.
Owner module: AI (`labonair-ai`) owns the Agent-panel action and workflow;
Themes (`labonair-theme`) owns Agent UI/buffer runtime adjustments and reset
semantics. Settings owns configured fallbacks. Agent UI dispatches through a
typed Themes contract; it does not create a second typography owner.
Canonical user entry point: The pinned Linux catalog lists
`agent::ResetAgentZoom`, and AgentPanel source registers its handler in the
current Workspace; it acts only when that Workspace has an AgentPanel. No
default key binding or menu route was found; command-palette discoverability
and macOS runtime registration remain unverified.
Public typed contract: No arguments. Remove the transient Agent UI and Agent
buffer font-size overrides, restoring their configured fallbacks. Do not
change Settings. If the current Workspace has no AgentPanel, the action has
no effect. This is narrower than Reset All Zoom, which also clears shared UI
and buffer font adjustments.
State and persistence: Agent UI size defaults unset and inherits shared UI
size; Agent buffer size defaults to 12 px. The action only clears the two
runtime Agent adjustments and refreshes windows; no settings save occurs.
Events and registry contributions: AI contributes its typed action in the
existing Agent action namespace and invokes the Themes-owned reset operation.
No general font registry or shell dispatch conditional is needed.
Allowed dependency changes: None for the documentation-only slice. Temporary
compatibility code: none; no stale path is introduced.
Notifications and failure: Reset is in-memory and needs no notification.
Dispatch with no AgentPanel remains an evidenced no-op; no I/O failure path
applies.
UI-kit: No UI control is established from the current evidence. Any eventual
discoverability surface must use the existing AI/Agent and UI-kit contracts.
Focused verification: Confirm action schema has no arguments, default and
profile keymaps lack a binding, no View menu item exists, the AgentPanel guard
and two Theme reset targets, runtime override removal/refresh behavior, and
Labonair's missing AI/Agent panel and Theme API.
Visual evidence: Pending paired macOS Agent panel captures before/after Agent
UI and Agent buffer adjustments, with shared UI/buffer values held constant;
capture with Agent panel present and absent, plus any confirmed action surface.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused action/owner/current-route audit. Rust formatting,
workspace check, Clippy, and tests remain R07-000 exit gates and are not run
for this documentation-only slice.
Documentation updates: Font increase/reset packet, ResetAgentZoom unbound
action row, AI capability crosswalk and action counts, normative parity note,
and this task's progress record.
Security impact: N/A; this action changes only local presentation settings
and introduces no permission boundary. Performance impact: No runtime change;
the future reset must avoid blocking GPUI and refresh only necessary windows.
Known limitations: Same-session source research and requirements authoring are
independent redevelopment, not separated-room clean-room work. Command-palette
discoverability, macOS registration, native dispatch, actual Agent font
changes, and paired macOS captures remain pending. R07-001 and R09 remain
unstarted.
Next safe task: Complete the Agent Reset Zoom classification in R07-000, then
continue with the next unclassified action or workflow; do not start R07-001
or R09.

## Progress record — 2026-10-10 — Agent Reset Zoom classification

- Classified `ACT-REGISTRY-LINUX-agent-ResetAgentZoom` as an AgentPanel action
  that clears only app-global transient Agent UI and Agent buffer font
  overrides. Distinguished its current-Workspace AgentPanel guard from
  View > Reset All Zoom's four-scope reset and noted that no Settings write
  occurs.
- Updated the font-size increase/reset packet, registered-action row, AI
  crosswalk, action counts, and normative parity note. Recorded that command-
  palette reachability and macOS runtime registration are unverified.
- Static Linux inventory and pinned AgentPanel/Theme handler inspection plus
  local capability/source audit only. Same-session work is independent
  redevelopment, not separated-room clean-room implementation.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 192 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused action schema, default/profile binding, menu,
  AgentPanel guard, reset-scope, owner, and local-route audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native/macOS action
  registration and dispatch, palette reachability, visible Agent typography
  reset, and paired macOS captures remain open.
- Next safe task: Continue classifying the remaining R07-000 action and
  workflow inventory; do not begin R07-001 or R09.

## Pre-change contract — classify related Workspace zoom actions

Request: Complete the existing Workspace zoom classification for its three
registered, default-keymap-unbound actions and distinguish them from the
global ToggleZoom action and native window zoom.
Request type: Requirements and registered-action inventory; no runtime change.
In scope: `ACT-REGISTRY-LINUX-workspace-ToggleEditorZoom`,
`ACT-REGISTRY-LINUX-workspace-ZoomIn`, and
`ACT-REGISTRY-LINUX-workspace-ZoomOut`; their target selection, eligibility,
focus effects, action discoverability, and current Labonair routes.
Out of scope: Changing global `ACT-MACOS-workspace-ToggleZoom`; implementing
Pane/Panel or Editor zoom; classifying native window or image zoom; changing
the accepted Pane/tab model; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: The existing Workspace zoom packet, registered-action and
keymap inventories, Pane/Panel zoom capability contracts, Workspace layout
state, and the local Debug dock zoom record.
Owner module and canonical crate: Workspace (`labonair-workspace`) owns zoom
target selection and Workspace-level layout state. Pane and Panel owners
provide eligibility and their content transitions. No shell action table or
new capability owner is introduced.
Sibling crates: Workspace has none. `labonair-panel` defines the shared
UI-free panel contract, and feature panel owners supply their own capabilities;
Pane state remains Workspace-owned. Canonical entry point: typed
`workspace::ToggleEditorZoom`,
`workspace::ZoomIn`, and `workspace::ZoomOut` actions; no default key binding
or shipped menu/pointer entry was found for these three action IDs.
Public typed contract: `ToggleEditorZoom` maximizes/restores the active center
Pane within the split tree, hiding sibling center Panes while leaving docks
and Panels in place. If an overlay Pane/Panel zoom is active, clear it before
maximizing. `ZoomIn` and `ZoomOut` enter/leave the active eligible Pane's
zoom state. A Pane hosted inside a panel may forward its event to that Panel;
do not assume every Panel handles these action IDs. Empty/non-zoomable Panes
propagate/no-op; ZoomIn focuses the target when required and is idempotent
when already zoomed; ZoomOut does nothing unless zoomed. These overlay actions
share Workspace target/padding behavior with ToggleZoom, while Editor
maximization remains a different transition.
State and persistence: No additional setting is defined for these action
IDs. Direct dock-panel zoom uses existing dock zoom persistence; center-Pane
overlay and Pane maximization follow the existing Workspace zoom packet's
restart behavior. The zoom-padding setting applies to eligible overlay zoom,
not as an Editor font-size or native-window scale value.
Events and registry contributions: Workspace owns the three typed actions
and target routing; Pane/Panel zoom events update the existing Workspace
target state.
Do not add another zoom state or a generic registry for these three routes.
Allowed dependency changes: None for this inventory slice. Removal condition
for compatibility code: N/A; no temporary code is added.
Notifications and failure: No I/O is expected. If a target disappears during
transition, clear only the stale zoom target and preserve surviving layout;
verify focus recovery against the existing zoom packet.
UI-kit: Any shipped route must reuse the existing Pane/Panel header controls
and shared UI-kit buttons/tooltips; no new local zoom control is part of this
documentation-only slice.
Focused verification: Confirm action schemas have no arguments; no default or
alternate-profile key binding; Pane/Panel eligibility and focus behavior;
ToggleEditorZoom overlay restoration and sibling-pane hiding; local internal
Dock zoom events versus its Debug-only command; and absence of equivalent
typed Labonair routes.
Visual evidence: Pending paired macOS states for a split Editor with two or
more Panes, existing zoom overlay followed by Editor maximize, eligible and
non-eligible Pane/Panel targets, focus restoration, narrow layout, and
restart behavior. Record builds, OS, bounds, scale, target owner, pane/dock
state, and focus.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused action/owner/target/current-route audit. Rust
formatting, workspace check, Clippy, and tests remain R07-000 exit gates and
are not run for this documentation-only slice.
Documentation updates: Existing Workspace zoom packet and three unbound
action rows; command/keymap counts and Workspace crosswalk; normative parity
note; and this task's progress record.
Security impact: N/A; actions affect local presentation state and no
permission boundary. Performance impact: No runtime change; future layout
transitions must remain synchronous-light and must not block GPUI.
Known limitations: Same-session source research and requirements writing are
independent redevelopment, not separated-room clean-room work. Native action
dispatch, palette or user-keymap reachability, layout restoration, and paired
macOS captures remain unverified. R07-001 and R09 remain unstarted.
Next safe task: Classify these three related Workspace zoom actions in R07-000,
then continue with remaining inventory; do not start R07-001 or R09.

## Progress record — 2026-10-10 — Related Workspace zoom action classification

- Completed the existing Workspace zoom packet's classification for
  `ToggleEditorZoom`, `ZoomIn`, and `ZoomOut`: center-Pane split maximization
  is distinct from overlay zoom; direct ZoomIn/Out act on the active eligible
  Pane, with a Pane-hosted Panel allowed to forward its own zoom events.
- Updated the three Linux registered-action rows with semantic outcomes,
  Workspace/Panel ownership, absent local typed routes, and Linux-only versus
  unverified macOS runtime status. Updated the packet index, command/keymap
  count, Workspace dock crosswalk, and normative parity note. Corrected the
  task contract to match the repository's Workspace-owned Pane boundary.
- Static pinned action declaration/handler/keymap and local Workspace/panel
  source review only; no runtime test was run. Same-session research is
  independent redevelopment, not separated-room clean-room work.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 192 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused three-action argument/binding/target/owner/local-
  route audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native action
  dispatch, command-palette/keymap reachability, Mac registration, Pane/Panel
  focus and layout restoration, and paired macOS captures remain open.
- Next safe task: Continue the remaining R07-000 action and workflow
  inventory; do not begin R07-001 or R09.

## Pre-change contract — classify Image Viewer zoom actions

Request: Classify the Image Viewer zoom action family across keyboard,
toolbar, percentage editing, wheel, pinch, and drag inputs; state the
distinct Fit-to-View and 100% reset outcomes and the view-state persistence.
Request type: Requirements and default-bound action ownership inventory; no
runtime change.
In scope: `ACT-MACOS-image_viewer-FitToView`,
`ACT-MACOS-image_viewer-ResetZoom`, `ACT-MACOS-image_viewer-ZoomIn`,
`ACT-MACOS-image_viewer-ZoomOut`, and
`ACT-MACOS-image_viewer-ZoomToActualSize`; their ImageViewer context and
macOS/Linux/Windows bindings, toolbar controls, zoom limits/steps, pan input,
view-state persistence, and current Labonair route.
Out of scope: Image decoding and format support; preview open/save lifecycle;
adding a Preview crate; changing the toolbar or app menu; editing image
pixels; Workspace and text-font zoom; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Default action/keymap inventory, Preview feature crosswalk,
UI-kit/toolbar ownership, image-viewer setting inventory, and file item
serialization contract.
Owner module: Planned Preview capability (`file.previews`) owns Image Viewer
rendering, zoom state, actions, and toolbar. Editor/Workspace owns opening and
hosting the view; Keymap owns action bindings. No Labonair canonical Preview
crate currently exists or is assigned, so the implementation task must settle
that boundary before code without inventing it in this inventory packet.
Canonical user entry point: `ImageViewer` context actions are available only
while the Image Viewer is focused. The active view toolbar provides Zoom Out,
an editable percent label, Zoom In, and Fit to View. Right-clicking the percent
label resets to 100%. No toolbar button for ZoomToActualSize was found.
Public typed contract: `ZoomIn`/`ZoomOut` multiply/divide the current scale by
1.1 and clamp action-driven scale to 10%–2000%. `ResetZoom` and
`ZoomToActualSize` both set 100% and clear pan offset. `FitToView` selects the
smaller width/height fit ratio, never upscales beyond 100%, and clears pan; it
does nothing until container bounds and image dimensions are available. The
first rendered layout also starts at fit-to-view scale. Fit scale is computed
separately from `set_zoom`; extremely small fit scales may fall below the
10% clamp and require runtime verification.
State and persistence: Current zoom and pan live in the ImageView. The
serializable item stores the image path, not zoom/pan, and zoom actions do not
write Settings. `ImageViewerSettings.unit` only selects binary/decimal file
size labels; it is unrelated to zoom. After restore/new first layout, fit
behavior should be checked; do not claim zoom persistence.
Events and registry contributions: Preview contributes its five typed
actions and active-view toolbar controls. Keymap contributes the platform
default bindings below; no global menu or zoom registry is required.
Allowed dependency changes: None for this inventory slice. Compatibility
code: none; do not add a parallel local zoom action path.
Notifications and failure: Zoom/pan are synchronous view state and need no
notification. Missing image dimensions/container bounds disable the fit
effect; malformed percent input cancels without altering the prior view.
Image load errors belong to Preview's separate open/decode workflow.
UI-kit: Reuse shared `IconButton`, `Tooltip`, `Label`, and the Settings/UI-kit
numeric input contract for the toolbar and editable percentage field. Keep all
controls in the owning Preview surface.
Focused verification: Confirm all five default bindings and ImageViewer
context; toolbar and right-click routes; 1.1 multiplicative step and
10%–2000% input/action limits; fit-to-view no-upscale rule and extreme-small
fit edge; two equivalent 100% reset effects; scroll modifier and cursor anchor;
wheel-pan, pinch, left/middle drag pan; cancellation/invalid percent input;
image-view restore and no Settings writes; and current Labonair owner/routes.
Visual evidence: Pending paired native macOS captures for first-fit landscape
and portrait images, 100% and extreme scale bounds, fit/actual-size contrast,
editable and invalid percentages, pointer-focused zoom/pan, pinch, narrow
toolbar layout, and restored Image Viewer item. Record build, OS, window/image
bounds, scale, zoom percentage, pan offset, input device, and focus context.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused action/owner/input/persistence/local-route audit.
Rust formatting, workspace check, Clippy, and tests remain R07-000 exit gates
and are not run for this documentation-only slice.
Documentation updates: Five action rows; a standalone Image Viewer zoom
classification packet; parity index; Preview capability crosswalk and
normative parity note; and this task's progress record.
Security impact: N/A; zoom changes local presentation only. Performance
impact: No runtime change; future image scaling/rendering must remain
non-blocking and reuse cached image resources.
Known limitations: Same-session reference research and requirements writing
are independent redevelopment, not separated-room clean-room work. Native
action dispatch, toolbar visuals, modifier interpretation, restoration, and
paired macOS evidence remain unverified; Preview crate ownership is open.
R07-001 and R09 remain unstarted.
Next safe task: Complete this Image Viewer zoom classification within R07-000,
then continue with remaining inventory; do not start R07-001 or R09.

## Progress record — 2026-10-10 — Image Viewer zoom classification

- Added the independent Image Viewer packet for five bound actions:
  FitToView, ResetZoom, ZoomIn, ZoomOut, and ZoomToActualSize. It specifies
  their `ImageViewer` context, platform bindings, 100% resets versus capped
  FitToView, 1.1 keyboard/button steps, 10%–2000% adjusted scale, percentage
  editing, modifier-wheel/pinch zoom, drag/wheel pan, and path-only item
  serialization.
- Updated all five action rows with semantics, the planned Preview owner and
  unassigned crate boundary, current missing Labonair routes, the Preview
  crosswalk, parity action counts/index, and this task's change contract.
- Static pinned keymap/view/control/serialization and local capability/source
  review only. Same-session research and packet authoring are independent
  redevelopment, not separated-room clean-room implementation.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 193 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused five-action/context/scale/input/persistence/
  owner/local-route audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native ImageViewer
  dispatch, toolbar behavior, input-modifier behavior, restored state, fit
  scale edge cases, and paired macOS captures remain open.
- Next safe task: Continue classifying the remaining R07-000 action and
  workflow inventory; do not begin R07-001 or R09.

## Pre-change contract — classify application and window lifecycle actions

Request: Classify the bound native Hide, Hide Others, Minimize, Quit, and
Toggle Full Screen actions, including their platform/context bindings, menu
routes, quit preparation and settings. Add the omitted JSON-only quit
confirmation value to the pinned Settings field register.
Request type: Action/owner inventory with one source-backed Settings
requirements ledger correction; no runtime change.
In scope: `ACT-MACOS-zed-Hide`, `ACT-MACOS-zed-HideOthers`,
`ACT-MACOS-zed-Minimize`, `ACT-MACOS-zed-Quit`,
`ACT-MACOS-zed-ToggleFullScreen`, their app/window menu and key routes,
`workspace.confirm_quit`, existing `fullscreen_mode`, current Labonair
handlers and quit safeguards, and the neighboring unbound `zed::ShowAll`
menu action for route comparison.
Out of scope: Workspace close action semantics already recorded in the Close
Window packet; other app menus/actions; changing application shutdown,
fullscreen, Settings, or menu code; the unbound `zed::Zoom` action; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Pinned default action/keymap inventory, `app.windows` and
`platform.integration` crosswalks, native app menu contract, Settings field
register, and Workspace close-window packet.
Owner module: Platform integration supplies stateless native Hide,
HideOthers, Minimize, and Quit behavior; the current architecture has no
separate lifecycle capability owner. Workspace owns the fullscreen
presentation action contract and Workspace close preparation; Settings owns
the typed values and not the action behavior. `labonair-shell` remains the
composition adapter and is not assigned lifecycle feature ownership.
Canonical capability crates: N/A for app-level GPUI lifecycle calls because
they have no product-owned state; `labonair-shell` composes current menu/window
actions, `labonair-workspace` owns Workspace close/presentation state, and
`labonair-settings` stores the persisted values. No new capability or crate
is assigned by this inventory.
Canonical user entry points: App menu for Hide, Hide Others, Show All, and
Quit; Window menu/key for Minimize; View menu and Workspace-context keymap
bindings for Toggle Full Screen. The global App action remains distinct from
closing a Workspace window or project.
Public typed contract: All five actions take no arguments. Hide hides the
current application; Hide Others hides other applications; Minimize minimizes
the current window; Quit prepares all workspace windows and only exits after
confirmation and save/close preparation succeeds; Toggle Full Screen changes
the current Workspace window's fullscreen mode. Preserve the macOS
Workspace-context distinction and the Linux/Windows F11 binding.
State and persistence: `workspace.confirm_quit` defaults false and controls
the app-level confirmation prompt. `fullscreen_mode` defaults to `native`;
on macOS, `simple` covers the full display bounds while `native` uses the
system fullscreen Space. Both values are Settings-owned user values; action
invocation stores no additional state. The local
`confirmQuitWithActiveSessions` is a separate terminal-session window-close
guard and does not replace quit/save preparation.
Events and registry contributions: Pinned Zed exposes typed actions and
platform keymap/menu entries. Labonair already has app-menu GPUI actions and a
shell-owned native-window command adapter. No new registry or event is
required for this classification.
Commands and default bindings: macOS: `cmd-h` Hide, `alt-cmd-h` Hide Others,
`cmd-m` Minimize, `cmd-q` Quit, `fn-f` and `ctrl-cmd-f` Toggle Full Screen in
Workspace. Linux/Windows: `ctrl-q` Quit and `f11` Toggle Full Screen in
Workspace. Hide/Hide Others/Minimize are macOS-only defaults. Terminal
contexts may shadow `ctrl-q` for terminal input.
UI-kit: No reusable product control; these actions use the native application
menu and window manager.
Allowed dependency changes: None. Compatibility code: none; do not add
parallel menu or shortcut tables.
Notifications and failure: Hide/Minimize/fullscreen need no notification.
Quit cancellation, save-preparation refusal, and preparation errors keep the
application open. Labonair's existing active-shell guard notifies on the
first close request and allows retry within three seconds.
Focused verification: Compare pinned action definitions, default keymaps,
quit-confirmation setting/default, multi-window save preparation and
fullscreen mode with local app-menu handlers, fixed/default shortcuts,
close callback and the existing Settings field inventory. Confirm the new
JSON-only field has a stable requirement ID and an explicit missing-control
status. Audit no conflation with Workspace Close Window or `zed::Zoom`.
Visual evidence: Pending paired native macOS App/Window/View menu captures,
Hide/Hide Others/Minimize/Quit outcomes, fullscreen native/simple modes, and
multi-window quit cancellation. Linux/Windows shortcut and fullscreen runtime
acceptance also pending. Record build, OS, window state, active workspace,
settings values, and running-shell/dirty-buffer state.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, settings-row count/link validation, and focused action/settings/
local-route audit. No Rust gates for this documentation-only slice.
Documentation updates: Five action rows; one app/window lifecycle packet;
`confirm_quit` JSON-only Settings row and inventory count; app-window,
platform, and Settings crosswalk references; parity index and normative parity
note; this task contract and progress record.
Security impact: N/A; actions are local app/window controls. Performance
impact: N/A; no runtime path changes.
Known limitations: Same-session reference review and independently worded
requirements are redevelopment, not separated-room clean-room work. Pinned
native dispatch, quit-dialog flow, local dirty-buffer behavior, fullscreen
rendering, non-macOS runtime and paired captures remain unverified.
Next safe task: Complete this bounded lifecycle classification, then continue
the remaining R07-000 inventory; do not begin R07-001 or R09.

## Progress record — 2026-10-10 — Application/window lifecycle actions

- Classified five bound actions: Hide, Hide Others, Minimize, Quit, and
  Toggle Full Screen, plus the registered unbound Show All menu action.
  Recorded platform/context shortcuts, app/window entry points, fullscreen
  mode policy, app-wide quit confirmation and all-workspace save preparation,
  and Labonair's existing active-shell close guard and native menu routes.
- Added `SET-settings-json-only-workspace-confirm-quit` (default `false`,
  user/global scope). The pinned Settings UI has no control for this value;
  updated the register total to 488 requirements / 384 static fields / five
  JSON-only values and updated the Settings crosswalk.
- Updated five bound action rows, the unbound Show All row, app-window and
  platform crosswalks, the Settings crosswalk, parity index, normative parity
  record, and this task contract. `labonair-shell` is recorded as a current
  composition adapter, not a lifecycle feature owner.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 194 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused pinned action/keymap/quit/settings/local-route,
  inventory-count, and ownership-boundary audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native menu and
  accelerator behavior, quit cancellation/save preparation, fullscreen modes,
  cross-platform dispatch, and paired captures remain open.
- Same-session source research and requirements writing are independent
  redevelopment, not separated-room clean-room implementation.
- Next safe task: Continue the remaining R07-000 action/workflow inventory;
  do not begin R07-001 or R09.

## Pre-change contract — classify Extensions action

Request: Classify the default `zed::Extensions` action, its Workspace context,
platform bindings, menu/Welcome entry points, optional category/extension
targets, view activation, and current Labonair route.
Request type: Requirements and capability-entry inventory; no runtime change.
In scope: `ACT-MACOS-zed-Extensions`, its macOS/Linux/Windows defaults, typed
optional category and extension-ID parameters, Extensions page open/reuse
behavior, related menu/Welcome/owner-contributed entry points, and the
planned Extensions capability's current Labonair gap.
Out of scope: Implementing an extension catalog, package install/update/remove
lifecycle, marketplace service, trust or distribution policy, language server
setup, Theme import, unbound extension administration actions, or R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Default action/keymap inventory, `extensions` and
`language.services` feature crosswalks, capability and feature lifecycle
contracts, Workspace item entry, and shared UI-kit ownership.
Owner module: Planned Extensions owns extension discovery/management and the
Extensions page/action; Workspace hosts the page as an active-pane item;
Keymap owns the Workspace-context binding. The canonical Labonair Extensions
crate is unassigned and must be resolved before implementation. Current
roadmap defers extension/distribution work until the core is stable; this
packet records parity requirements without changing that disposition.
Canonical capability crate: Planned Extensions owner; no crate assigned.
Current `labonair-shell` remains a composition root and has no extension
feature ownership.
Canonical user entry points: Workspace `cmd-shift-x` on macOS and
`ctrl-shift-x` on Linux/Windows; App menu and titlebar global menu; Welcome
page's Explore Extensions. Category owners may dispatch the same typed action
with a filter or extension ID, which delegates to the Extensions page.
Public typed contract: Default `zed::Extensions` supplies no filter and no
extension ID, opening the all-category page. Optional typed category filters
select themes, icon themes, languages, grammars, language servers, context
servers, snippets, or debug adapters; an optional ID focuses a specific
extension. Open or reuse an Extensions page in the active pane and activate it.
State and persistence: Opening the page does not change Settings or the
installed-extension catalog. Search/filter and focused extension state belong
to the Extensions view; persistence and restore behavior need verification.
Installed extension state and distribution remain Extension-owner contracts.
Events and registry contributions: Extensions owns catalog data and the
shared manager. Before implementation, decide whether actual provider and
consumer counts require a registry; use a direct typed contract for one
provider/consumer. This inventory selects or adds no Labonair registry.
Category-specific caller actions delegate to the same page action.
Commands and default bindings: `cmd-shift-x` in Workspace on macOS;
`ctrl-shift-x` in Workspace on Linux/Windows. The default action has no args.
Settings: None identified for opening the page or selecting an extension.
Notifications: Opening the view is local navigation; fetch/install failures
belong to the planned Extensions workflow and must use the notification
center where a passive message is appropriate.
UI-kit: The future Extensions view must use shared search/input/list/menu and
button primitives; Workspace hosts it but does not own extension state.
Allowed dependency changes: None for this inventory. Compatibility code:
none; do not treat the Theme JSON import adapter as an extension manager.
Focused verification: Compare the action schema/defaults, three platform
bindings, App/titlebar/Welcome entry points, active-pane page open/reuse and
activation, category/ID variant routes, current capability crosswalk, and the
absence of a Labonair Extensions view/action/store route.
Visual evidence: Pending paired native macOS menu/Welcome entry captures,
active-pane creation/reuse, all-category and prefiltered view states, search,
loading/empty/error and narrow-window states, plus extension installation
outcomes. Record build, OS, extension category/ID, window/pane state, focus,
and network state.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused action/schema/binding/entry/owner/local-route audit.
Rust workspace gates remain R07-000 exit gates; no runtime code is changed.
Documentation updates: Action row, independent Extensions action packet,
Extensions capability crosswalk and action counts/index, normative parity
note, and this task contract/progress record.
Security impact: No runtime change. Extension installation trust, code
execution, permissions, and distribution risks remain part of the deferred
Extensions capability contract.
Performance impact: No runtime change. The eventual catalog/search surface
must define loading and bounded-list behavior.
Known limitations: Same-session source research and independent requirements
writing are redevelopment, not separated-room clean-room work. Action
dispatch, active-pane behavior, extension discovery, view restore, network
failure, installation, and paired native captures remain unverified.
Next safe task: Complete this bounded Extensions action classification and
continue R07-000; do not begin R07-001 or R09.

## Progress record — 2026-10-10 — Extensions action

- Classified `zed::Extensions` and `ACT-MACOS-zed-Extensions`: Workspace
  bindings on all three platforms, App/titlebar/Welcome entry points, typed
  category and extension-ID variants, and active-pane Extensions page
  creation/reuse/activation. Recorded its All/Installed/Not Installed and
  category/search surfaces plus loading, failure, empty, and restore questions.
- Updated the action requirement, Extensions and language-services
  crosswalks, parity action counts/index, normative parity record, and this
  task's pre-change contract. Labonair has no Extensions action/view/store or
  canonical Extensions crate; extension distribution remains deferred until
  the core is stable.
- Passed: `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative docs / 195 Markdown files), `python3 scripts/verify.py
  --scope docs`, `python3 scripts/check_rework_queue.py` (46 tasks; R07-000
  active), and the focused action schema/binding/entry/page-state/local-route
  audit.
- Not run for this documentation-only slice: Rust formatting, workspace
  check, Clippy, or tests. They remain R07-000 exit gates. Native action
  dispatch, extension catalog/network/install behavior, explicit retry,
  item-state restore, and paired macOS evidence remain open.
- Same-session source research and independently worded requirements are
  redevelopment, not separated-room clean-room implementation.
- Next safe task: Continue classifying the remaining R07-000 action/workflow
  inventory; do not begin R07-001 or R09.

## Pre-change contract — classify Settings entry actions

Request: Classify the bound Open Settings and Open Settings File actions and
their related menu-only Open Project Settings, Open Project Settings File,
and Open Default Settings routes.
Request type: Settings-surface entry and action ownership inventory; no
runtime change or new Settings value.
In scope: `ACT-MACOS-zed-OpenSettings`,
`ACT-MACOS-zed-OpenSettingsFile`, their platform/context defaults, and the
three related registered-unbound settings menu actions; distinguish the
Settings UI, user JSON, project Settings UI/file, and bundled defaults routes.
Out of scope: Settings field definitions and category/layout acceptance
already tracked in the Settings requirements/crosswalk; changing Settings,
Editor, keymap, menu, or persistence code; Open Keymap actions; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Pinned action/keymap inventories, Settings surface and value
ownership contracts, Workspace file-open contract, Settings crosswalk, and
current local command providers/window entry.
Owner module: Settings owns Settings UI and values; Workspace owns opening a
settings JSON document as an Editor item and project-file association; Keymap
owns the bound shortcuts. Native menus are composed by the app root and do
not own Settings state.
Canonical capability crates: `labonair-settings-ui` owns the Settings window;
`labonair-settings` owns typed values and persistence; `labonair-workspace`
opens user/project Settings JSON files. The default-settings reference has no
Labonair route. No new crate is assigned.
Canonical user entry points: App/Settings menu and Workspace welcome entry
for Open Settings; App > Settings > Open Settings File; App > Settings > Open
Project Settings, Open Project Settings File, and Open Default Settings.
Bound shortcuts are `cmd-,`/`ctrl-,` for Open Settings outside
`SettingsWindow`, and `cmd-alt-,`/`ctrl-alt-,` globally for Open Settings
File.
Public typed contracts: Open Settings takes no arguments and opens or focuses
the Settings UI. Open Settings File takes no arguments and opens the user
Settings JSON in a Workspace editor document, creating the initial file when
needed. Open Project Settings targets the active project's visible root in
the Settings UI. Open Project Settings File opens that project's JSON
document. Open Default Settings opens the bundled baseline document, distinct
from editable user/project Settings.
State and persistence: No action stores state. Settings values persist via
Settings; user/project JSON files retain their respective scopes. Settings
window visibility is transient. Bundled defaults are not the user's editable
file. The local Settings window exposes user/project scope, and its JSON
button opens the corresponding file in the main Workspace.
Events and registry contributions: No new event or registry. The Settings
owner contributes the Settings command; Keymap supplies platform defaults;
Workspace opens JSON items through its existing Editor route.
Settings: No field/value additions. Preserve the distinction between
`OpenSettingsFile` (user JSON), project Settings UI, project JSON, and
bundled defaults.
Notifications and failure: UI opening should focus an existing Settings
window where appropriate. File-creation/open failures use the Settings or
Workspace error path; do not create duplicate toasts or menus.
UI-kit: Settings UI uses shared Settings/UI-kit controls; JSON files use the
Workspace Editor. Native menu entries remain platform menu actions.
Allowed dependency changes: None. Compatibility code: none; do not add
parallel Settings JSON routes or duplicate Settings management surfaces.
Focused verification: Compare all five action/registry rows, keymap contexts,
menu and Welcome entries, pinned target selection and bundled-file behavior,
local Settings window/command/JSON-button routes, user/project scope, and
missing bindings/menu items. Do not conflate the JSON editor with the Settings
UI or project settings with default settings.
Visual evidence: Pending paired macOS Settings menu/window captures, existing
Settings window refocus, user/project scope, Settings JSON document opening,
bundled defaults read-only state, and no-project/error states. Record OS,
window count/focus, project roots, file target, and Settings scope.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused action/owner/target/local-route audit. Rust gates
remain R07-000 exit gates; no implementation change is planned.
Documentation updates: Two bound action rows; three registered-unbound
settings menu rows; Settings entry packet, action counts/index, Settings
crosswalk, normative parity note, and this task contract/progress record.
Security impact: N/A; no runtime file access changes. User/project settings
scope remains explicit. Performance impact: N/A; no runtime change.
Known limitations: Same-session research and independent requirements are
redevelopment, not separated-room clean-room work. Native menu dispatch,
Settings window focus, current JSON actions, read-only defaults, and paired
macOS behavior remain unverified.
Next safe task: Complete this bounded Settings-entry classification, then
continue R07-000; do not begin R07-001 or R09.

## Progress — classify Settings entry actions

- Classified the two bound Settings actions and three related menu-only routes
  in [`action-classification-settings-entry.md`](../../docs/parity/action-classification-settings-entry.md).
  The packet distinguishes the Settings UI, user Settings JSON, project
  Settings UI, project JSON, and bundled default document, and records each
  current Labonair route and gap.
- Updated the bound and registered-unbound action ledgers, parity packet index,
  feature crosswalk, and normative Zed parity summary. Added this task contract
  and progress record before implementation work.
- Focused static audit matched the pinned macOS, Linux, and Windows keymap
  defaults and Settings menu entries, then checked local Settings commands,
  native menu routes, and Workspace JSON open routes. Key dispatch, native
  window focus, bundled-document behavior, and paired visual acceptance remain
  unverified.
- Passed `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative documents and 196 Markdown files),
  `python3 scripts/verify.py --scope docs`, and
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 remains earliest
  active). Rust gates were not run for this documentation-only slice and remain
  R07-000 exit gates.
- Same-session source research and independent requirements are redevelopment,
  not separated-room clean-room implementation.
- Next safe task: Continue R07-000 action/workflow classification; do not begin
  R07-001 or R09.

## Pre-change contract — classify Keymap entry actions

Request: Classify the bound Open Keymap and Open Keymap File actions and the
related menu-only Open Default Key Bindings route.
Request type: Keymap surface entry and action ownership inventory; no runtime
change, new command, or new Keymap setting.
In scope: `ACT-MACOS-zed-OpenKeymap`,
`ACT-MACOS-zed-OpenKeymapFile`, their platform/context defaults, and
`ACT-REGISTRY-LINUX-zed-OpenDefaultKeymap`; distinguish the Keymap management
surface, editable user keymap JSON, and bundled default key bindings.
Out of scope: binding editor controls, conflict semantics, profiles, Vim or
Helix layers, keymap migration, and changing Settings, Workspace, Editor,
command, keymap, or menu code; R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Keymap ownership and command contracts, the default and overlay
keymap inventories, Workspace Editor file-open behavior, existing Keymap
surface/menu composition, and local Keymap provider/handler.
Owner module: Keymap owns binding state, action discovery metadata, keymap
composition, and the management surface contract. Workspace hosts the
management surface and editable JSON document as Workspace items. The app
root composes native menu actions; it does not own keymap state.
Canonical capability crates: `labonair-keymap` owns keymap values, files, and
runtime composition; `labonair-keymap-ui` owns management presentation;
`labonair-workspace` hosts the management tab and opens the raw JSON Editor
item. No new crate is assigned.
Canonical user entry points: App > Settings > Open Keymap, Open Keymap File,
and Open Default Key Bindings; `cmd-k cmd-s`/`ctrl-k ctrl-s` opens the Keymap
surface, and `cmd-e`/`ctrl-e` opens the editable file from KeymapEditor.
Welcome Customize Keymaps, titlebar/global menu, Settings Keymap link, and
Command Palette routes also reach the management surface.
Public typed contracts: Open Keymap takes no arguments and opens or activates
the Keymap management surface in the active Workspace. Open Keymap File takes
no arguments and opens/initializes the user's editable keymap JSON in an
Editor item. Open Default Key Bindings opens the bundled baseline document for
inspection. These targets are distinct.
State and persistence: User bindings persist in the user keymap file and are
composed with platform/profile/mode layers by Keymap. Opening the management
surface does not change bindings. The bundled default document is distinct
from editable user bindings.
Events and registry contributions: No new event or registry. Reuse the
existing Keymap command provider, Workspace handler, Keymap Settings-surface
contribution, and action/menu composition.
Settings: No value addition. Profile and modal-mode values and loading
behavior remain in their existing crosswalks.
Notifications and failure: Malformed or unwritable user keymap data uses the
Keymap notification path; Workspace Editor open failures use existing
Workspace errors. Do not add duplicate file editors or passive messages.
UI-kit: The management surface uses `labonair-ui-kit` controls; raw user and
bundled JSON documents open through the Workspace Editor.
Allowed dependency changes: None. Compatibility code: none; document the
existing `OpenKeymapJson` internal command identity only as needed to explain
its action mapping, and do not add parallel actions.
Focused verification: Compare the three pinned action/menu routes and
platform/context bindings; check local menu/Welcome/Settings/palette entries,
shortcut defaults and identity aliases, management-tab reuse, JSON creation,
and the missing bundled-document route. Keep visual editor and raw JSON
targets distinct.
Visual evidence: Pending paired macOS App Settings menu, Keymap management
surface, user JSON document, bundled default document, and existing-tab
activation evidence. Record OS, active Workspace/pane, document target, and
focus state.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused pinned-action/local-owner/route audit. Rust gates
remain R07-000 exit gates; no implementation change is planned.
Documentation updates: Two bound action rows; one registered-unbound default
keymap row; Keymap entry packet, action counts/index, command/keymap
crosswalks, normative parity note, and this task contract/progress record.
Security impact: N/A; no runtime file access changes. User keymap path and
default document remain distinct. Performance impact: N/A; no runtime change.
Known limitations: Same-session source research and independently written
requirements are redevelopment, not separated-room clean-room implementation.
Native dispatch, menu composition, existing-item activation, file errors, and
paired macOS behavior remain unverified.
Next safe task: Complete this bounded Keymap-entry classification, then
continue R07-000; do not begin R07-001 or R09.

## Progress — classify Keymap entry actions

- Classified the bound Open Keymap and Open Keymap File actions and related
  Open Default Key Bindings menu route in
  [`action-classification-keymap-entry.md`](../../docs/parity/action-classification-keymap-entry.md).
  The packet distinguishes the interactive management surface, editable user
  JSON document, and bundled default document, and records current Labonair
  routes and gaps.
- Updated the bound and registered-unbound action ledgers, Keymap layer
  crosswalk, parity packet index, feature crosswalk, normative Zed parity
  summary, and this task contract/progress record.
- Focused source audit passed for pinned menu routes, macOS/Linux/Windows
  shortcut contexts, active-pane item activation/creation, and current local
  command aliases, user-file initialization, and missing routes. Native
  dispatch, document behavior, and paired visual acceptance remain unverified.
- Passed `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative documents and 197 Markdown files),
  `python3 scripts/verify.py --scope docs`, and
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 remains earliest
  active). Rust gates were not run for this documentation-only slice and remain
  R07-000 exit gates.
- Same-session source research and independently written requirements are
  redevelopment, not separated-room clean-room implementation.
- Next safe task: Continue R07-000 action/workflow classification; do not begin
  R07-001 or R09.

## Pre-change contract — classify task-definition file entry actions

Request: Classify the bound Open Worktree Setup Tasks action and the related
global/project task and debug-configuration file routes.
Request type: Task/debug configuration entry and ownership inventory; no
runtime change, Task runner, debugger session, or new setting.
In scope: `ACT-MACOS-zed-OpenWorktreeSetupTasks`,
`ACT-REGISTRY-LINUX-zed-OpenTasks`,
`ACT-REGISTRY-LINUX-zed-OpenDebugTasks`,
`ACT-REGISTRY-LINUX-zed-OpenProjectTasks`, and
`ACT-REGISTRY-LINUX-debugger-OpenProjectDebugTasks`; distinguish user-global
tasks, user-global debug scenarios, project tasks, project debug scenarios,
and the worktree setup task example.
Out of scope: Task execution, shell integration, debugger lifecycle/protocols,
task discovery/picker behavior, task profile settings, and runtime changes to
Task, Debugger, Workspace, Settings, Editor, Keymap, or native menus;
R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: `terminal.tasks` and `debugger` feature crosswalks, pinned
action/keymap inventories, global and project file ownership contracts,
Workspace Editor file-open behavior, and current Task/Debugger capability
inventory.
Owner module: Task definitions belong to the planned Tasks owner; debug
scenario definitions belong to the planned Debugger owner. Workspace hosts
the files as Editor items, while Keymap owns the bound Worktree Picker
shortcut. No current Labonair Task or Debugger owner/crate is established;
their canonical contracts remain an explicit implementation prerequisite.
Canonical capability crates: None exists for Task or Debugger in the current
tree. Do not assign these workflows to Settings or create a facade in this
entry-only inventory. Workspace's existing Editor owns document presentation.
Canonical user entry points: Global `zed: open tasks` and `zed: open debug
tasks` actions open user configuration files. Run > Edit tasks.json and
Edit debug.json open project files. `cmd-shift-c`/`ctrl-shift-c` on macOS/Linux
and `ctrl-shift-,` on Windows in WorktreePicker contexts opens project tasks
and supplies a create-worktree hook example.
Public typed contracts: All five actions take no arguments. Global task/debug
actions open or initialize their user config JSON documents. Project actions
open or initialize `.zed/tasks.json` and `.zed/debug.json` in the active
project. The worktree setup action opens `.zed/tasks.json` and inserts one
safe create-worktree example only when the file has no existing hook.
State and persistence: Global task/debug definitions persist in user config;
project task/debug definitions persist in their respective worktree files.
The worktree setup entry mutates project task JSON only to seed a missing hook
example. Task execution state and debugger session state are out of scope.
Events and registry contributions: No new event or registry is needed to
classify file-open routes. Future Task/Debugger discovery and execution
contracts require their own owner and consumer-count analysis.
Settings: No setting/value additions. The global file's initial contents are
separate from Settings values.
Notifications and failure: Project-scoped open with no visible project
worktree publishes an explanatory notification. File creation/open failures
use Workspace errors; malformed content remains in the owning Task/Debugger
validation contract.
UI-kit: Config JSON uses the Workspace Editor; menu/picker controls use their
own surface and shared UI-kit components.
Allowed dependency changes: None. Compatibility code: none; do not add a
second Task/Debugger file editor or move configuration ownership to Settings.
Focused verification: Match all five action rows, global/project/worktree
targets, three-platform shortcut contexts, project/no-project behavior,
initial file creation, existing-hook suppression, and current Labonair owner
and route absence. Keep task definitions separate from Settings and from
runtime task/debug execution.
Visual evidence: Pending paired macOS Run-menu project file entry, Worktree
Picker route, created JSON editor item, existing-hook behavior, and no-project
notification. Record OS, active project/worktree, target file, and focus.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused pinned-action/file-target/current-owner audit. Rust
gates remain R07-000 exit gates; no implementation change is planned.
Documentation updates: One bound action row; four registered-unbound task and
debug rows; task-definition entry packet, action counts/index, feature
crosswalks, normative parity note, and this task contract/progress record.
Security impact: N/A; no runtime file access changes. Project config files
remain project-scoped. Performance impact: N/A; no runtime change.
Known limitations: Same-session source research and independent requirements
are redevelopment, not separated-room clean-room implementation. Canonical
Task/Debugger crates, native dispatch, file behavior, no-project recovery, and
paired macOS acceptance remain unresolved.
Next safe task: Complete this bounded task-definition entry classification,
then continue R07-000; do not begin R07-001 or R09.

## Progress — classify task-definition file entry actions

- Classified the Worktree Setup Tasks shortcut and four related global and
  project task/debug JSON routes in
  [`action-classification-task-definition-entry.md`](../../docs/parity/action-classification-task-definition-entry.md).
  The packet records each file scope, initial-file behavior, project
  worktree selection, conditional hook insertion, and the absent Labonair
  Task/Debugger owners and routes.
- Updated the bound and registered-unbound action ledgers, task and debugger
  feature crosswalks, parity packet index, and normative Zed parity summary.
- Focused source audit passed for the pinned global and project file targets,
  project/no-folder recovery, conditional worktree-hook seeding, and
  macOS/Linux/Windows shortcut actions in the WorktreePicker contexts. No
  corresponding local action routes or Task/Debugger owner crates exist.
  Native dispatch, execution, debug launch, and paired visual evidence remain
  open.
- Passed `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative documents and 198 Markdown files),
  `python3 scripts/verify.py --scope docs`, and
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 remains earliest
  active). Rust gates were not run for this documentation-only slice and remain
  R07-000 exit gates.
- Same-session source research and independently written requirements are
  redevelopment, not separated-room clean-room implementation.
- Next safe task: Continue R07-000 action/workflow classification; do not begin
  R07-001 or R09.

## Pre-change contract — classify worktree navigation and creation actions

Request: Classify the bound Git Worktree picker action and its typed
create, switch, and open-in-new-window actions.
Request type: Git worktree navigation and lifecycle action inventory; no
runtime change, new setting, or worktree implementation.
In scope: `ACT-MACOS-git-Worktree`,
`ACT-REGISTRY-LINUX-git-CreateWorktree`,
`ACT-REGISTRY-LINUX-git-SwitchWorktree`, and
`ACT-REGISTRY-LINUX-git-OpenWorktreeInNewWindow`; define picker access,
selection/create targets, switching, and separate-window opening.
Out of scope: Delete and force-delete (already classified), Restricted Mode
and trusted-path decisions (already classified), worktree setup task seeding
(classified in the task-definition entry packet), branch management, and
runtime changes to Git, Workspace, Keymap, menus, or project persistence;
R07-001/R09.
Active task: R07-000-zed-parity-baseline-and-inventory.
Dependencies: Git ownership contracts, pinned Git and Worktree Picker action
inventories, current Git repository API and Workspace hosting model, and the
Git worktree deletion/security and task-definition entry packets.
Owner module: Git owns repository worktree discovery, create/switch operations,
and picker state/behavior. Workspace owns project activation and window
hosting. Keymap owns the global Workspace shortcut. The shell/app root only
composes platform window services and menu/toolbar surfaces.
Canonical capability crates: `labonair-git` is the existing Git contract
crate, but it currently has no worktree lifecycle API. No Git Worktree UI
crate or local worktree implementation is present; do not create another owner
or claim the contract exists.
Canonical user entry points: The titlebar worktree trigger opens Worktree
Picker; `cmd-ctrl-w` on macOS, `alt-ctrl-shift-w` on Linux, and `shift-alt-w`
on Windows invoke `git::Worktree` in Workspace. The picker selects existing
worktrees, offers creation targets, and has a separate Open in New Window
control for non-current worktrees.
Public typed contracts: `Worktree` takes no arguments. `CreateWorktree` takes
an optional worktree name and a CurrentBranch, ExistingBranch, or RemoteBranch
target; the target determines the detached starting point, and success opens
and activates the new worktree Workspace. `SwitchWorktree` takes a path and
display name and switches the current Workspace to that linked worktree.
`OpenWorktreeInNewWindow` takes a path and opens that existing worktree in a
separate window.
State and persistence: Worktree existence and branch/ref targets persist in
Git. Worktree switching must account for the current Workspace's open items,
docks, and focus; creation opens a new Workspace on the new worktree. No
Settings value is added by these entry actions.
Events and registry contributions: The reference dispatches typed Git
actions from Worktree Picker selection. No new Labonair event or registry is
added in this classification; define operation, lifecycle, and multi-consumer
contracts before implementation.
Settings: None added. Existing workspace/tab/session persistence effects
remain to be measured for switch, create, and new-window paths.
Notifications and recovery: Repository absence, collaboration restrictions,
remote fetch/open failures, path conflicts, concurrent creation, and canceled
switch/create operations require visible owner-routed recovery. Labonair user
messages must use Notifications; the reference's transient errors still need
native observation.
UI-kit: Worktree Picker reuses shared picker, list, button, and icon controls;
new Labonair controls would use `labonair-ui-kit`.
Allowed dependency changes: None. Compatibility code: none; do not present
branch switching as worktree switching or the existing Worktree deletion
classification as full worktree parity.
Focused verification: Compare the four action rows and parameter types,
titlebar/keyboard entry routes, create targets, existing-worktree switch,
new-window path, repository/collaboration restrictions, remote operations,
concurrent/canceled operations, and current Labonair Git API/UI absence.
Visual evidence: Pending paired macOS titlebar trigger, picker list/search,
create targets, switch, in-new-window, disabled/error, and restored Workspace
state captures. Record OS, repository/worktrees, active target, Workspace
window count, selected branch target, and focus.
Workspace gates: Diff check, documentation checker, docs verification scope,
queue check, and focused action/parameter/owner/local-API audit. Rust gates
remain R07-000 exit gates; no implementation change is planned.
Documentation updates: One bound action row; three registered-unbound Git
rows; worktree navigation packet, action counts/index, Git crosswalk, normative
parity note, and this task contract/progress record.
Security impact: N/A; no file or repository mutation changes. Preserve Git
repository boundaries and the existing trust/deletion contracts.
Performance impact: N/A; no runtime change. Picker population and remote
repository latency remain acceptance requirements.
Known limitations: Same-session source research and independent requirements
are redevelopment, not separated-room clean-room implementation. Native
dispatch, operation outcomes, error recovery, Workspace restoration, and
paired macOS behavior remain open.
Next safe task: Complete this bounded worktree navigation/create
classification, then continue R07-000; do not begin R07-001 or R09.

## Progress — classify Git worktree navigation and creation actions

- Classified the bound Worktree Picker action and typed create, switch, and
  open-in-new-window actions in
  [`action-classification-git-worktree-navigation.md`](../../docs/parity/action-classification-git-worktree-navigation.md).
  The packet records create targets, collaboration/concurrency guards,
  separate-window behavior, Workspace-state requirements, and the missing
  Labonair lifecycle API/UI.
- Updated the bound and registered-unbound action ledgers, Git feature
  crosswalk, parity packet index, and normative Zed parity summary.
- Focused source audit passed for the titlebar entry, macOS/Linux/Windows
  shortcuts, typed parameters, Git service actions, and the absence of matching
  Labonair worktree routes. Picker dispatch, remote behavior, operation
  recovery, Workspace restoration, and paired visual acceptance remain open.
- Passed `git diff --check`, `python3 scripts/check_documentation.py`
  (43 normative documents and 199 Markdown files),
  `python3 scripts/verify.py --scope docs`, and
  `python3 scripts/check_rework_queue.py` (46 tasks; R07-000 remains earliest
  active). Rust gates were not run for this documentation-only slice and remain
  R07-000 exit gates.
- Same-session source research and independently written requirements are
  redevelopment, not separated-room clean-room implementation.
- Next safe task: Continue R07-000 action/workflow classification; do not begin
  R07-001 or R09.

## Exit condition

This task is complete only when the full feature inventory has no unclassified
areas, all prior Settings attempts have an evidence-based disposition, the
source-blind implementation packet is ready, and the next implementation task
can begin without inventing a missing product or ownership decision. If native
capture access remains blocked, the blocker and owner must be explicit; visual
parity itself cannot be claimed from incomplete evidence.
