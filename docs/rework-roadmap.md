# Labonair Architecture Rework Roadmap

**Status:** Current implementation plan
**Version:** 2

This roadmap replaces the historical task order. Existing completed work remains valuable, but old tasks do not override the contracts in `docs/`.

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
- split or reduce the all-purpose backend;
- enforce dependency direction in CI.

**Exit:** a new feature can be added without editing unrelated feature internals.

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

**Exit:** no product code uses a toast or feature-local user error surface.

## Phase 4 — Command palette and keymap

- replace static palette tables with registry providers;
- add contexts, dynamic submenus, and stable action IDs;
- register commands from owning modules;
- build the keymap file/editor and conflict handling.

**Exit:** palette and keymap changes are localized to registries and owning modules.

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

## Definition of done

A phase is done only when its code, documentation, tests, dependency graph, and visual behavior agree. A passing compiler alone is not sufficient.
