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

The first post-acceptance boundary task is prepared but not started:
[`R07-004-explorer-host-contract.md`](../tasks/rework/R07-004-explorer-host-contract.md)
will remove the direct Explorer-to-Workspace view dependency described as B01
in [`audits/remaining-boundaries.md`](audits/remaining-boundaries.md).

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
