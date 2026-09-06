# R01-005 — Reduce Settings to values and typed module contracts

## Status

`✅ Done`

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

- [x] Settings navigation contains values only; capability-management entries
  are absent.
- [x] `settings-ui` no longer stores or calls the broad backend/workspace
  integrations for MCP, Hosts, or personalization.
- [x] Duplicate runtime settings models have one canonical owner; migrations
  are explicit and covered by fixtures.
- [x] User-visible Settings diagnostics reach the notification registry and no
  longer render as inline error banners.
- [x] Every retained setting has a tested consumer or a documented removal /
  deferral decision.
- [x] Focused Settings and notification tests plus all workspace verification
  gates pass.

## Outcome

- `SettingsContent` now contains only typed application values. Hosts, keymaps,
  MCP runtime preferences, and statusbar/panel presentation state are not
  serialized as Settings areas.
- Legacy host/MCP wire types remain only behind named migration code. The
  migration preserves standalone MCP data, skips obsolete host-manager values,
  and keeps host-store migration separate from SettingsContent.
- Settings UI has no backend, Hosts UI, panel, or personalization integration
  and publishes parse/schema diagnostics through Notifications.
- Product-specific SCM presentation code moved out of `ui-kit` into
  `panel-scm`; shared UI primitives remain in `ui-kit`.
