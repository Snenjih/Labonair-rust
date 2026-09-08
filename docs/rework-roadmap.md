# Labonair Architecture Rework Roadmap

**Status:** Current implementation plan
**Version:** 4

The rework is through the completed R06 backend-adapter eradication. The
capability-owned registries and surfaces, shell/workspace identity, settings
reduction, and explicit module boundaries are established. R07 is now the
active acceptance and cleanup phase: prove that documentation, ownership,
dependency direction, and permanent product surfaces agree, then remove the
remaining transitional shell adapters.

This roadmap replaces the historical task order. Existing completed work remains valuable, but old tasks do not override the contracts in `docs/`.

Work proceeds in dependency order. A later phase may not introduce a second
owner or bypass an unfinished contract from an earlier phase. The roadmap is a
sequence of bounded migrations, not a commitment to preserve every feature in
the predecessor.

## Phase 0 — Documentation and inventory

- establish product, architecture, module, registry, design, workspace, and settings contracts;
- archive superseded plans;
- map every current crate and cross-module dependency;
- publish the baseline in [`audits/architecture-inventory.md`](audits/architecture-inventory.md);
- classify each current feature as keep, redesign, defer, or remove;
- generate a migration matrix from current crates to target modules.

**Exit:** the target graph and ownership of every current feature are documented.

## Phase 1 — Foundation contracts

- establish UI-kit token and component boundaries;
- define typed identifiers, events, service traits, and registration APIs;
- split or reduce the all-purpose backend; no new capability code enters it;
- enforce dependency direction in CI.

**Exit:** a new feature can be added to one owning capability crate and
registered without editing unrelated feature internals or a shell-wide table.

## Phase 2 — Shell and workspace

- simplify titlebar, tabs, docks, workspace, and statusbar;
- allow empty and standalone workspaces;
- fix popover/dropdown anchoring;
- move shell logic into composition only.

The first bounded implementation is
[`R02-001-global-menu-and-theme-entrypoints.md`](../tasks/rework/R02-001-global-menu-and-theme-entrypoints.md):
the titlebar publishes typed global-menu events and delegates Settings,
Keymap, Themes, Icon Themes, and Hosts to their owning surfaces.
The completed bounded implementation is
[`R02-002-shell-composition-and-standalone-workspaces.md`](../tasks/rework/R02-002-shell-composition-and-standalone-workspaces.md):
it narrowed one remaining shell/workspace behavior slice and verified empty,
project, and standalone workspace transitions.
The follow-up implementation is also complete:
[`R02-003-project-entry-and-workspace-transitions.md`](../tasks/rework/R02-003-project-entry-and-workspace-transitions.md):
it completes session identity persistence, explicit return-to-standalone
behavior, and the removal audit for cwd-based project inference.

**Exit:** all permanent UI has one documented location and layout behavior is consistent.

## Phase 3 — Notifications and transfers

- replace the former toast implementation with the notification center/dropdown;
- migrate inline user-facing errors;
- make transfers a first-class statusbar module;
- preserve actions, details, deduplication, and scrollable history.

**Exit:** every operation message is retained in the notification registry and
shown through the statusbar dropdown; no product code uses a toast or duplicate
feature-local error surface. Explicitly actionable dialogs and field
validation affordances may retain the details and controls required to finish
the current task.

## Phase 4 — Command palette and keymap

- replace static palette tables and duplicate shell dispatch registries with
  one typed command registry;
- add contexts, dynamic submenus, and stable action IDs;
- register commands from owning modules;
- build the keymap file/editor and conflict handling.

The implementation was split into two bounded tasks after the workspace
identity work, and both are complete:

- [`R03-001-command-palette-provider-registry.md`](../tasks/rework/R03-001-command-palette-provider-registry.md)
  established the provider-owned metadata and dynamic-submenu snapshot
  contract. Ordinary command handlers and dynamic submenu execution now use
  owner contributions; R07-002 completed the final typed action-handler
  boundary, so the shell only forwards opaque actions.
- [`R03-002-keymap-runtime-and-editor.md`](../tasks/rework/R03-002-keymap-runtime-and-editor.md)
  made the keymap a runtime command-binding capability with contexts, conflicts,
  persistence, and a dedicated editor surface.

**Exit:** palette and keymap changes are localized to their registries and
owning modules; adding a command does not require editing an unrelated central
table.

## Phase 5 — Themes and hosts

- create static color and icon-theme registries;
- implement preview and confirmation;
- establish HostStore and host management surface;
- connect palette Enter/Shift+Enter behavior;
- keep jump hosts inside SSH connection configuration.

**Exit:** themes, icon themes, and hosts are absent from Settings and fully accessible through their intended surfaces.

The existing global-menu entrypoints are only navigation. Theme selection and
host management are now implemented through their bounded tasks; they must not
grow new Settings categories.

The bounded tasks are complete:

- [`R04-001-static-theme-registries-and-preview.md`](../tasks/rework/R04-001-static-theme-registries-and-preview.md)
  completed built-in color/icon registries, preview, and confirmation.
- [`R04-002-host-management-and-connection-pickers.md`](../tasks/rework/R04-002-host-management-and-connection-pickers.md)
  gave saved hosts one management surface and one SSH/SFTP picker flow.

## Phase 6 — Settings reduction

- audit every field against real consumers;
- remove dead, duplicate, and misplaced settings;
- migrate only retained values;
- verify global/project scopes.

**Exit:** Settings contains values only and categories have clear ownership.

The executable migration is
[`R05-001-settings-audit-and-value-normalization.md`](../tasks/rework/R05-001-settings-audit-and-value-normalization.md).

R05-001 is complete. Settings now contains only typed, consumer-backed values;
legacy compatibility input remains isolated in the Settings migration wire
shape, while hosts, themes, keymaps, notifications, transfers, and layout
state are owned by their respective capabilities.

## Phase 7 — Feature module migration

- migrate terminal, editor, SSH, SFTP, explorer, Git, snippets, and AI to their ownership boundaries;
- remove temporary shell/workspace compatibility paths;
- add module-level tests and visual checks.

**Exit:** the dependency graph and source layout match the architecture contract.

The backend-removal boundary
[`R06-001-backend-adapter-eradication.md`](../tasks/rework/R06-001-backend-adapter-eradication.md)
is complete. Remaining surface-contribution discrepancies are audited by
[`R07-001-product-surface-acceptance.md`](../tasks/rework/R07-001-product-surface-acceptance.md)
and the remaining explicit dependency edges are ordered in
[`audits/remaining-boundaries.md`](audits/remaining-boundaries.md). They are
implemented only through bounded follow-up tasks.

## Phase 8 — Product refinement

- improve terminal and remote workflows;
- rebuild AI on the new contracts;
- evaluate projects/workspace enhancements;
- add extensions or downloads only after the core is stable.

Remote theme downloads, marketplace behavior, and extension hosting remain
deferred until a later product decision adds a concrete workflow and owner.

The cross-module visual and workflow gate remains open:
[`R07-001-product-surface-acceptance.md`](../tasks/rework/R07-001-product-surface-acceptance.md).
The owner-contribution task is complete and closes the dynamic palette
execution residue:
[`R07-002-owner-registered-surface-contributions.md`](../tasks/rework/R07-002-owner-registered-surface-contributions.md).

The acceptance launch also recorded a bounded Settings compatibility finding;
[`R07-003-settings-legacy-warning-disposition.md`](../tasks/rework/R07-003-settings-legacy-warning-disposition.md)
now records how known removed keys are classified without weakening warnings
for genuinely unknown future keys.

The first post-acceptance boundary task,
[`R07-004-explorer-host-contract.md`](../tasks/rework/R07-004-explorer-host-contract.md),
has landed its structural migration: the direct Explorer-to-Workspace view
dependency (B01 in
[`audits/remaining-boundaries.md`](audits/remaining-boundaries.md)) is removed.
`labonair-panel-explorer` now reaches the workspace only through the injected
`labonair-explorer-host::ExplorerHost` contract, and the shared drag/preview
value types live in the new leaf `labonair-explorer-host` crate. All code,
dependency, and test gates pass; the native Explorer visual-state recording is
the only remaining item and folds into the R07-001 visual matrix.

The second boundary task,
[`R07-005-background-presentation-boundary.md`](../tasks/rework/R07-005-background-presentation-boundary.md),
has landed the same way for B02: `labonair-workspace` no longer depends on
`labonair-background`. `Workspace` and `TerminalView` render through the
injected `labonair-background-host::BackgroundHost` contract (a layer
callback plus a `BackgroundPulse` repaint entity); `labonair-background`
keeps sole ownership of image storage, import/delete, persistence, decoding,
and rendering policy. All gates pass; only the native background-layer
visual-state recording remains, again folding into the R07-001 visual matrix.

### Post-B02 audit — Keymap boundary, Workspace/Terminal/Editor ownership, AI UI, B03–B06 (2026-09-07)

With B01 and B02 resolved, the remaining rework-prompt steps were audited
against the current source tree rather than assumed complete or re-litigated
from scratch:

- **Keymap platform boundary.** `crates/shell/src/keymap_loader.rs` and
  `crates/shell/src/menu.rs` were re-read end to end. Loading, merging,
  validation, default layers, alias resolution, and conflict handling live
  entirely in `labonair-keymap` (`adapter::load_descriptors`,
  `runtime::command_for_action`, 36 focused tests covering registration,
  resolution, context precedence, conflict reporting, and persistence). The
  shell only turns the resulting immutable snapshot into concrete
  `gpui::KeyBinding`s/`Action`s (`menu::apply_keymap`/`action_for`) and a
  display-hint global, and owns the `keymap.json` file-watch via
  `labonair_settings::watch_file`. This matches the already-normative
  T19-008 rationale in `scripts/check_crate_deps.py` ("shell also depends on
  `labonair-settings` directly — it owns the concrete `menu::` GPUI Actions").
  No duplicate legacy keymap table exists (`KeybindMap`/old `apply_keybinds`
  are gone, referenced only in a historical doc comment). A single
  `CommandId` enum (`labonair-command-palette-core`, ~113 variants) is the
  sole command registry; no competing per-module command/action enum was
  found. Conclusion: **compliant, no extraction required.**
- **Workspace/Terminal/Editor ownership.** `labonair-terminal` and
  `labonair-editor` already own their engine algorithms (session/PTY
  lifecycle, ANSI batching, input mapping, Vim, syntax, symbols — e.g.
  `crates/editor/src/vim.rs` at 1603 lines is entirely in the editor crate).
  `crates/workspace/src/views/{terminal,editor}.rs` are the GPUI adapters
  Workspace needs to host them as tabs; they call into the engine crates
  (`batch_runs`, `key_to_bytes`, session registries) rather than
  reimplementing them, and cannot move into the UI-free engine crates
  without violating rule 4 (no UI dependency). `Workspace::spawn_session`
  translates typed Settings into `SessionOptions` and calls
  `TerminalRegistry::create`; it holds no PTY/session code itself.
  Conclusion: **the current engine/view split already matches the target
  ownership boundary.** No further extraction is identified.
- **AI UI rebuild.** The AI frontend is intentionally parked (see
  `memory/ai-frontend-parked.md`, tag `ai-ui-v1`); `labonair-ai` backend
  contracts are retained and `labonair-workspace`'s AI live-bridge
  (`live_bridge.rs`) implements the typed `labonair_ai::LiveBridge` contract
  without owning AI state or provider logic. Rebuilding the AI UI is a
  separate product decision, not mandated by this audit; Step 6's contract
  guidance applies whenever that rebuild starts.
- **B03–B06 re-review.** Each retained edge in
  [`audits/remaining-boundaries.md`](audits/remaining-boundaries.md) was
  checked against its actual current consumer (e.g. B03 confirmed:
  `workspace → ai` is exactly the one typed `LiveBridge` implementation, no
  broader AI state). All four decisions (`Narrow when needed`) still hold;
  the audit file now records the review date.
- **Step 8 anti-pattern sweep.** Source-wide search found: no toast
  renderer or timer-based passive popup (the two `Toast`/`toast` hits are
  doc comments asserting the absence); no duplicate command/action registry;
  no Jump Hosts menu/badge surface (the only `JumpHost` hits are the SSH
  connection/tunnel implementation, matching the product decision that jump
  hosts stay inside SSH connection config); no remote theme
  download/marketplace scaffolding (`"Community Neon"` is a shipped
  fixture theme name, not a network feature); Transfers is registered as an
  owner status item and is reachable, not hidden; `crates/shell` has no file
  over 735 lines and each one is composition/registration, not feature
  state. This was a representative source-and-grep sweep, not a literal
  per-action enumeration of every command in the app; no violation was found
  large enough to justify a new crate, facade, or removal.

No code change resulted from this audit pass beyond B01/B02 (R07-004,
R07-005) — every check confirmed the existing architecture already matches
the normative contract. This is recorded here, rather than left implicit, so
the next session does not re-run the same audit from zero.

### Follow-up audit — module-boundary and product-surface review (2026-09-08)

A second, deeper independent review (`your-task-2.md`) went past the
representative sweep above and read each foundation/feature crate boundary in
full. It found boundary and ownership issues the grep-level pass did not
surface, so the "no violation large enough to act on" conclusion above is
**superseded for the specific edges listed below**. The green dependency
verifier is necessary but not sufficient: several allow-listed edges are
acyclic yet architecturally undesirable.

Confirmed against current source:

- `labonair-ui-kit` (foundation) depends on `labonair-theme` (feature) and
  imports concrete `ThemeStore` / gallery types (P1.1).
- `labonair-workspace` stores `Entity<HostManagerView>` /
  `Entity<ConnectionStatusStore>` and mutates Hosts-UI state directly (P1.2).
- `panel-snippets` and `transfers-ui` store `Entity<Workspace>` and call it
  directly for execution / SFTP refresh (P1.3, P1.4).
- `settings-ui` owns theme preview / activation / persistence policy (P1.5).
- The pre-migration `ShortcutId` / `SHORTCUTS` keymap model is inert but still
  present and re-exported (P1.6). **Resolved (R08-007)** — deleted, along with
  the now-empty `labonair-interaction-contracts` crate.
- Zoom In/Out/Reset and Format Document are visible palette/menu commands with
  no execution handler (P1.7).
- Titlebar and statusbar popovers anchor to the pointer, not the trigger
  bounds (P2.1).

These are being worked as bounded follow-up tasks (R08-series) in the order
recorded in `tasks/rework/README.md`. Resolved so far: R08-001 (P1.4,
transfers-ui → workspace edge removed); R08-002 (P1.7, the no-op Zoom
In/Out/Reset, Adjust Font Size, and Format Document affordances were removed
from the palette, native menu, and default keymap); R08-003 (P1.3,
panel-snippets → workspace edge removed via the leaf
`labonair-snippets-host::SnippetExecutionHost` contract); R08-004 (P2.1, the
titlebar global menu, the Notifications / Agent Access statusbar dropdowns, and
the Settings select/font dropdowns now anchor to their trigger's rendered
bounds instead of the click position); R08-005 (P1.5, the settings→`ThemeStore`
application policy and the palette theme-action handler moved from `settings-ui`
into the new `labonair-theme-ui` sibling — `settings-ui` owns no theme
preview/activation/persistence policy, and `labonair-theme` keeps no
`labonair-settings` dependency). R08-006 (P2.4, the `AppComposition` bundle lost its blanket `Deref` — now one
explicit accessor per capability — and the `BackendSsh*` transport adapters
were renamed to `Ssh*Adapter`; `main.rs` / `bootstrap` name the bundle
`composition`, not `backend`); R08-007 (P1.6, the pre-migration `ShortcutId` /
`SHORTCUTS` cheat-sheet model and every `.with_shortcut()` call site were
deleted, the now-empty `labonair-interaction-contracts` crate removed, and the
`command_palette.rs` re-exports trimmed to `keystroke_tokens`). Right-edge
collision behavior for P2.1 is left to the R07-001 visual matrix. Task/acceptance status in R01-004,
R03-002, R04-002, R05-001, and R06-001 was reconciled the same day: their
non-visual acceptance criteria are ticked with evidence, residual items carry
an explicit follow-up pointer, and visual criteria are delegated to the
still-open R07-001 matrix.

## Change and removal gates

Every phase task must state its owner, canonical capability crate, affected
contracts, canonical surface, dependencies, and removal condition for any
compatibility code. Update `docs/capabilities.md` and
`docs/audits/architecture-inventory.md` with the same change when ownership or
crate edges change.

Before adding a new permanent surface or abstraction, show the current user
workflow it serves and why an existing command-palette, workspace, dock,
statusbar, overlay, or UI-kit surface cannot serve it. Features without a
current workflow are marked `defer` or `remove`; they do not receive shell
chrome, settings, or registry infrastructure.

For migrations, the required order is: inventory current consumers, define the
target contract, move the owning state, move adapters, move consumers, delete
obsolete registrations and dependencies, then run the full verification
gates. A phase is not complete while old and new implementations both remain
authoritative.

## Definition of done

A phase is done only when its code, documentation, tests, dependency graph, and visual behavior agree. A passing compiler alone is not sufficient.
