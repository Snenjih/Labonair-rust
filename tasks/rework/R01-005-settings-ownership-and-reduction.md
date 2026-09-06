# R01-005 — Reduce Settings to values and typed module contracts

## Status

`⬜ Todo`

## Owner

- Module: `settings`
- Capability matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Canonical surfaces: Settings values window; module-owned Hosts, Themes,
  Keymap, Notifications, and Personalization surfaces

## Dependencies

- `R01-004-command-palette-keymap-registry`

## Goal

Make Settings a value-oriented capability instead of an integration hub.
Remove duplicate or obsolete settings models and migrate settings-owned UI
away from direct backend, workspace, MCP, and registry access.

## Scope

- Keep typed user/project values, layer precedence, persistence, validation,
  and migration that have real consumers.
- Move or isolate host, theme, icon-theme, keymap, notification, transfer,
  and statusbar-management workflows behind their owning capabilities.
- Remove user-facing inline validation/error banners from Settings and publish
  diagnostics through the notification registry, while retaining internal
  field validation needed to prevent invalid writes.
- Establish narrow typed contracts for MCP/agent settings and statusbar
  customization before changing consumers.
- Audit every retained setting against a real consumer; mark unused values for
  removal or explicit deferral.

## Ownership and dependency rules

- `labonair-settings` owns values and layer mechanics, not feature behavior.
- `settings-ui` may render values and invoke typed settings contracts, but may
  not hold `Backend`, `Workspace`, or feature registries.
- Legacy `Preferences`/MCP/host projections remain only behind named
  migrations and are not a second runtime source of truth.
- User-visible diagnostics use `labonair-notifications`; no Settings toast or
  permanent inline error surface is introduced.

## Migration order

1. Inventory every Settings field and direct dependency with tests.
2. Define the canonical value owner and the narrow contract for each external
   workflow.
3. Move adapters and consumers into their owning modules.
4. Remove duplicate models, dependencies, and unreachable panes.
5. Verify user/project precedence, migration fixtures, and notification
   delivery.

## Acceptance criteria

- [ ] Settings navigation contains values only; capability-management entries
  are absent.
- [ ] `settings-ui` no longer stores or calls the broad backend/workspace
  integrations for MCP, Hosts, or personalization.
- [ ] Duplicate runtime settings models have one canonical owner; migrations
  are explicit and covered by fixtures.
- [ ] User-visible Settings diagnostics reach the notification registry and no
  longer render as inline error banners.
- [ ] Every retained setting has a tested consumer or a documented removal /
  deferral decision.
- [ ] Focused Settings and notification tests plus all workspace verification
  gates pass.
