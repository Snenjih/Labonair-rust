# R05-001 — Reduce Settings to validated values

## Status

`🔄 In Progress`

## Owner

- Module: `settings`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-settings-ui`

## Dependencies

- `R04-001-static-theme-registries-and-preview`
- `R04-002-host-management-and-connection-pickers`

## Goal

Make every remaining Settings value intentional, typed, scoped, consumed, and
discoverable. Remove dead, duplicate, and misplaced fields without preserving
management UI in the Settings system.

## Scope

- In scope: field inventory, defaults, global/project scope, schema, generated
  UI, migrations, validation, search, and removal of obsolete categories.
- Out of scope: feature management views, command registration, host/theme/
  keymap registries, and unrelated feature redesign.

## Contracts and ownership

- `labonair-settings-content` owns typed value data and defaults.
- `labonair-settings` owns layered state, persistence, and change lifecycle.
- `labonair-settings-ui` owns value navigation and field rendering only.
- External modules consume narrow typed settings contracts or immutable values;
  Settings never owns their state or UI.

## Dependencies

- Existing edges removed: settings-to-hosts, settings-to-theme-management,
  settings-to-keymap-management, and settings-to-notification behavior.
- New edges: none unless a narrow value-discovery contract has a current
  consumer and is documented in the inventory.
- Dependency verifier change: remove obsolete edges and migration-only aliases
  after fixtures prove they are no longer needed.

## Persistence and migration

- Settings: retain only fields with a tested consumer and explicit scope.
- Storage: preserve user data with versioned migrations and atomic writes.
- Compatibility: unknown fields remain round-trippable during migration;
  removed fields are ignored or transformed only when their data meaning is
  documented.

## User-visible behavior

- Canonical entry point: Settings window for values and global search for value
  fields.
- Notifications: parse, schema, and write failures use structured
  notifications with details; field validation remains local only to prevent
  invalid writes.
- Inline errors/toasts: no passive banners or toasts.

## Implementation plan

1. Produce a field-to-consumer/scope inventory and mark keep, redesign,
   defer, or remove → verify every row has evidence.
2. Normalize the typed model, schema, generated UI, and migrations → verify
   global/project precedence and round-trip fixtures.
3. Remove dead fields, duplicate models, management categories, and obsolete
   dependencies → verify no stale registration or surface remains.

## Acceptance criteria

- [ ] Every retained setting has a real consumer, type, default, and scope.
- [ ] Hosts, themes, icon themes, keymap editing, transfers, notifications,
      and command registration are absent as Settings categories.
- [ ] Global/project values merge deterministically and preserve user data.
- [ ] Settings UI is generated from the typed model and uses UI-kit controls.
- [ ] Malformed files and writes use notifications; invalid fields remain
      actionable without duplicating passive errors.
- [ ] Focused fixtures and all repository verification gates pass.

## Removal condition

This task is complete only when Settings can be described entirely as a typed
value store plus its standard value editor, with no capability-management
state or duplicate runtime source of truth.

## Notes and follow-ups

New settings fields require a consumer and a scope before they can be added to
the model. Unused values are parked in the inventory, not added speculatively.

## Progress

- [x] Produced [`docs/settings-inventory.md`](../../docs/settings-inventory.md)
      with the current field-to-consumer, scope, and keep/move/remove/review
      decision for all typed Settings fields.
- [x] Removed the untyped `general.notifyOnErrors` key from the shipped
      defaults and added a shape-drift test so future default assets cannot
      contain values outside `SettingsContent`.
- [x] Moved the five duplicated Background values out of `AppearanceContent`
      and into the existing Background-owned top-level storage path, including
      migrations for legacy v1 and already-sparsified v2 files.
- [x] Moved dock/sidebar runtime persistence behind the Workspace owner in
      `workspace-layout.json`; Settings UI and project scope no longer expose
      those fields, and legacy values are imported and removed idempotently.
- [x] Remove the typed dock/sidebar compatibility fields. The app-level
      migration reads both legacy v1 `preferences` and v2 `workspace` input
      directly into the Workspace layout owner before Settings conversion.
- [x] Remove confirmed unsupported terminal compatibility fields from the
      typed model and generated UI, while keeping the legacy Preferences wire
      shape readable for migration.
- [x] Remove the unused general autostart, credential-encryption, and
      SSH-quit-confirmation values from the typed model and generated UI while
      retaining the legacy wire shape for old files.
- [x] Remove the unused `startupTerminalCount` value from the typed model,
      generated UI, project whitelist, and defaults while retaining the legacy
      Preferences wire shape for migration.
- [x] Remove legacy `appCornerRadius` from the typed model and UI; v1 and v2
      values migrate losslessly to the current `cornerRadiusScale` value.
- [x] Remove Appearance layout values without native consumers:
      `sidebarTabInfoLine`, `sidebarGroupByFolder`, `sidebarGroupSingleTabs`,
      `badgesAlwaysVisible`, and `titlebarsIconsPosition`.
- [x] Remove unwired Editor values (`editorAutoSave`,
      `editorAutoSaveDelay`, `editorAutocompleteDebounceMs`, and
      `editorMaxFileSizeMb`) while retaining their legacy wire shape.
- [x] Remove SFTP/transfer values from the Settings model; transfer runtime
      policy remains owned by the transfer worker and is no longer duplicated
      in Settings.
- [x] Remove the empty Connections Settings area and its unconsumed SSH/
      Explorer timing values; transport and Hosts remain the future owners of
      connection policy.
