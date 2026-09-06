# Labonair Architecture Rework Roadmap

**Status:** Current implementation plan
**Version:** 3

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

**Exit:** all permanent UI has one documented location and layout behavior is consistent.

## Phase 3 — Notifications and transfers

- replace the toast implementation with the notification center/dropdown;
- migrate inline user-facing errors;
- make transfers a first-class statusbar module;
- preserve actions, details, deduplication, and scrollable history.

**Exit:** every user-facing message is retained in the notification registry
and shown through the statusbar dropdown; no product code uses a toast or
feature-local user-facing error surface.

## Phase 4 — Command palette and keymap

- replace static palette tables and duplicate shell dispatch registries with
  one typed command registry;
- add contexts, dynamic submenus, and stable action IDs;
- register commands from owning modules;
- build the keymap file/editor and conflict handling.

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

## Phase 6 — Settings reduction

- audit every field against real consumers;
- remove dead, duplicate, and misplaced settings;
- migrate only retained values;
- verify global/project scopes.

**Exit:** Settings contains values only and categories have clear ownership.

## Phase 7 — Feature module migration

- migrate terminal, editor, SSH, SFTP, explorer, Git, snippets, and AI to their ownership boundaries;
- remove temporary shell/workspace compatibility paths;
- add module-level tests and visual checks.

**Exit:** the dependency graph and source layout match the architecture contract.

## Phase 8 — Product refinement

- improve terminal and remote workflows;
- rebuild AI on the new contracts;
- evaluate projects/workspace enhancements;
- add extensions or downloads only after the core is stable.

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
