# Settings Contract

**Status:** Normative
**Version:** 1

## What settings are

Settings are typed, persisted values that change application behavior or presentation. They may have global, user, and project/workspace layers when that scope is meaningful.

Settings are a value service, not a feature directory. A setting belongs here
only when changing the value changes behavior or presentation and the owning
feature can consume it through the typed settings contract.

## What settings are not

The settings system is not the owner of:

- saved hosts or credentials;
- theme or icon-theme catalogs;
- keymap definitions and shortcut editing;
- notifications;
- transfer history;
- command registration;
- feature management screens.
- MCP/agent bridge runtime configuration;
- statusbar item placement and panel visibility.

There is no `Shortcuts`, `Hosts`, `Themes`, or `Icon Themes` settings
category. Keymap, host, and theme management are separate capability surfaces.

Those capabilities have their own modules and entry points.

Persisting a value does not transfer ownership to Settings. For example, the
active color-theme ID or icon-theme ID may be stored as a user preference, but
the theme module owns the registry, preview, validation, and selection flow.
Likewise, keymap data is owned by the keymap module and host records by the
hosts module; neither becomes a Settings category merely because it is saved
to disk. MCP preferences and statusbar/panel layout follow the same rule and
are loaded by their owning capabilities.

Workspace dock and sidebar state is persisted by `labonair-workspace` in
`workspace-layout.json`. The legacy `workspace.sidebar*`, `workspace.dockLayout`,
and v1 `preferences` counterparts are migration input only; they are not
editable Settings fields and must not be added to the project-settings
whitelist. Layout migration runs before Settings conversion for v1 input and
also handles existing v2 files. It is idempotent: a valid workspace layout
file is never overwritten.

## Settings categories

The initial categories are intentionally small and currently consist of:

- General
- Appearance
- Terminal
- Editor
- Workspace
- File Manager

Update policy is a General field and is grouped under the General page's
Updates section; it is not a separate management category.

Theme and icon-theme selection, keymap editing, and host management are not
Settings pages even when their selected IDs or defaults are persisted through
the settings storage layer.

Their canonical entry points are the titlebar global menu and its command
palette surfaces: Keymap opens keymap management, Themes and Icon Themes open
their respective pickers, and Hosts opens host selection/management. The
themes and hosts modules own those flows; Settings may persist only a typed
preference exposed by their contracts.

Categories may be added only when they contain real configurable values. A category may not exist solely to host a management UI.

Do not add a category for a registry, resource catalog, connection list,
history view, or feature workflow. Those belong to the owning module's
surface. If a category becomes empty, duplicate, or unused, remove it and
migrate only values that still have a consumer.

## Field rules

Every setting has:

- a typed field;
- a stable key;
- a default;
- documented scope;
- validation;
- reset behavior;
- a visible description.

Unknown values should survive migrations when safe. Removed settings require a deliberate migration or an explicit compatibility decision.

Before retaining a field, identify its consumers and scope. A field with no
current consumer is removed or explicitly deprecated with a removal condition;
it is not kept as a placeholder for a possible future feature. Settings
migrations must be idempotent, preserve user data where safe, and never move
ownership of hosts, themes, keymaps, notifications, or transfers into the
settings crate.

## UI rules

Generated field UI is preferred for ordinary values. A custom view is allowed only when the interaction cannot be represented as a field, and it must still use the standard settings chrome and UI-kit components.
