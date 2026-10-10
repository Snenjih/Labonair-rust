# Settings Contract

**Status:** Normative
**Version:** 2

## What settings are

Settings are typed, persisted values that change application behavior or presentation. They may have global, user, and project/workspace layers when that scope is meaningful.

Settings are a value service, not a feature directory. A setting belongs here
only when changing the value changes behavior or presentation and the owning
feature can consume it through the typed settings contract.

## What settings are not

The settings system is not the owner of:

- saved hosts or credentials;
- theme or icon-theme catalogs;
- keymap definitions and shortcut editing (the Keymap page links to the
  Keymap-owned editor);
- notifications;
- transfer history;
- command registration;
- feature management screens.
- MCP/agent bridge runtime configuration;
- statusbar item placement and panel visibility.

There is no `Shortcuts`, `Hosts`, `Themes`, or `Icon Themes` settings
category. The pinned Settings taxonomy has a `Keymap` page that opens the
Keymap owner's canonical editor; it does not add a second binding editor or
move binding state into Settings. Host and theme management remain separate
capability surfaces.

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

## Settings pages

The visible Settings navigation is a UI taxonomy, independent of the seven
persisted `SettingsContent` groups. Page descriptors route stable JSON field
paths to sections and search results; they do not rename stored keys or take
ownership away from the capability that consumes each value. A field has one
canonical edit page even when its persisted group supplies values to multiple
pages. Capability-owned links use `SettingsSurfaceRegistry`: each contribution
provides a stable ID, searchable title and description, canonical action label,
and page placement. The registry validates IDs and page metadata, then supplies
the same snapshot to navigation and search. Settings invokes the contribution's
owner action; it does not add a feature-specific renderer branch.

The pinned-reference target has fifteen page slots, recorded with each
capability owner and current implementation status in the
[Settings crosswalk](parity/settings-crosswalk.md): General, Appearance,
Keymap, Editor, Languages & Tools, Search & Files, Window & Layout, Panels,
Debugger, Terminal, Version Control, Collaboration, AI, Network, and
Developer. A page appears in the runtime navigation only when it has
owner-backed settings or links to its owner's canonical editor when the page
is part of the pinned Settings taxonomy; missing
capabilities remain explicit crosswalk work and must not be represented by
empty or inert placeholder controls.

The current runtime pages are General, Appearance, Keymap, Editor, Languages
& Tools, Search & Files, Window & Layout, Terminal, Version Control, and
Network.

Appearance links to the Theme owner's app-theme and icon-theme pickers and the
Background owner's image editor from its Theme section. These actions open the
canonical owner surfaces; Settings does not maintain duplicate catalogs,
preview state, image state, or persistence paths.

`Network` holds global SSH connection-behaviour defaults (handshake timeout,
keep-alive interval, keep-alive failure tolerance) — the fallback a host
record falls back to when it doesn't set its own value. It does not manage
hosts, credentials, or the connection list; that remains the Hosts module's
surface.

Update policy is a General field and is grouped under the General page's
Updates section; it is not a separate management category.

Every JSON-backed field row can copy a `labonair://settings/<json-path>` link.
The macOS application receives that URL through GPUI's open-URL callback,
validates the path against the Settings field registry, and opens Settings at
the owning page and field. Unknown paths are ignored.

Theme and icon-theme selection and host management are not Settings pages even
when their selected IDs or defaults are persisted through the settings storage
layer. The Keymap page is a navigation surface for the existing Keymap-owned
editor; keymap editing and persistence remain outside Settings.

The Keymap editor is reachable from both the Settings navigation and the
titlebar global menu; both entry points open the same Workspace-owned Keymap
tab. Themes and Icon Themes open their respective pickers, and Hosts opens
host selection/management. The themes and hosts modules own those flows;
Settings may persist only a typed preference exposed by their contracts.

Categories may be added for a capability-owned editor only when that category
exists in the pinned Settings taxonomy. The page links to the canonical owner
surface and never duplicates its editor, state, or persistence. Other
management workflows remain outside Settings.

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

The native Settings window has one dense tree rail and one content surface:

- its initial outer bounds are 900×750 logical pixels scaled by
  `ui_font_size / 16`; the minimum is 626×240 logical pixels;
- the rail is 226 logical pixels wide and contains the owner-backed pages
  listed above; persisted `SettingsContent` groups and group headings are not
  a second navigation taxonomy;
- the rail uses 10 logical pixels of side and bottom inset and a 40-pixel
  macOS top inset below the native titlebar. Its shared search field is 28
  pixels high with a 12-pixel gap before navigation;
- the rail spans the full settings surface; the scope selector and JSON action
  sit in the header of the right content pane, above its scrolling page;
- the content surface keeps at least 400 logical pixels for field descriptions
  and controls, with 24 pixels of content inset;
- generated field rows span the available content width. Labels and
  descriptions wrap in a flexible leading column; a modified setting shows
  its reset action and source beside the title, while the value control stays
  aligned in the trailing column. Each row's divider spans the content width
  and follows the row's full wrapped height, with vertical padding separating
  it from the next setting;
- the navigation search is inset below the native titlebar controls, while the
  right content header begins at the top of the content pane;
- the header exposes visible `User` / `Project` segments when a project is
  active and an outlined action opens that scope's JSON file; the scope control
  uses the shared medium button geometry and supports click, Tab, Enter/Space,
  and Left/Right navigation;
  project writes are rejected by the Settings owner unless the key is on
  `PROJECT_SETTINGS_WHITELIST`;
- reset removes the selected override from its sparse JSON layer so the next
  lower-precedence value becomes effective. It must not write a duplicate
  default value;
- search, section navigation, and field pages use bounded or virtualized lists;
  long pages must not eagerly materialize every row;
- ordinary controls are UI-kit controls: shared search-field chrome, the
  standard text-field frame with native text/number editors, select
  triggers/popovers, switches, badges,
  disclosure/tree rows, and keyboard hints. Errors go to the notification
  center rather than a second Settings-only toast/banner system.
- system-font discovery exposes its loading state, reports failures through
  the notification center, and offers an in-place retry action.

The Settings UI targets the pinned Zed reference for observable layout,
navigation, focus, field behavior, and state coverage while keeping Labonair's
settings ownership and typed persistence contracts. It must not copy Zed source
code or introduce management pages without an owning Labonair capability.
