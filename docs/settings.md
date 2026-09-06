# Settings Contract

**Status:** Normative

## What settings are

Settings are typed, persisted values that change application behavior or presentation. They may have global, user, and project/workspace layers when that scope is meaningful.

## What settings are not

The settings system is not the owner of:

- saved hosts or credentials;
- theme or icon-theme catalogs;
- keymap definitions and shortcut editing;
- notifications;
- transfer history;
- command registration;
- feature management screens.

Those capabilities have their own modules and entry points.

## Settings categories

The initial categories are intentionally small:

- General
- Appearance
- Terminal
- Editor
- Workspace
- File Manager
- Connection defaults
- Updates

Categories may be added only when they contain real configurable values. A category may not exist solely to host a management UI.

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

## UI rules

Generated field UI is preferred for ordinary values. A custom view is allowed only when the interaction cannot be represented as a field, and it must still use the standard settings chrome and UI-kit components.
