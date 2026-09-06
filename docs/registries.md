# Registry Contracts

**Status:** Normative

Registries are the extension mechanism for capabilities used by more than one consumer. A registry owns discovery and metadata; the owning module owns the behavior behind each entry.

## Command registry

The command palette consumes one command registry. Feature modules register commands with:

- stable action ID;
- title and searchable aliases;
- category or submenu;
- applicable contexts;
- default keybinding metadata;
- execution callback or typed action payload.

The palette owns search, filtering, navigation, preview, and selection. It does not own feature state or a giant dispatch table.

Submenus are registered providers. Dynamic entries such as hosts, themes, and tabs are supplied by providers that expose a snapshot and an action.

## Keymap registry

Every command-capable action may register a keybinding descriptor. The keymap system owns:

- keymap file loading and persistence;
- contexts and precedence;
- conflict detection;
- display formatting;
- user overrides.

The keymap system does not contain feature behavior. A feature owns the action it registers.

## Theme registries

Color themes and icon themes have separate registries. Each entry contains a stable ID, display name, metadata, and a complete or layered definition. Preview is transactional: navigation applies a temporary preview, while confirmation persists the selected ID.

Initial themes are built in. Downloading and extensions are future modules, not part of the initial registry contract.

## Panel and status-item registries

Panels register identity, title, icon, supported docks, and a view factory. Status items register identity, placement metadata, badge behavior, and a view factory. The shell provides the host surface; feature modules own their content and actions.

## Notification center

The notification center is the single user-message sink. It accepts structured notifications with severity, title, summary, optional details, source, timestamp, actions, and deduplication metadata. It does not render toasts and does not contain feature-specific error handling.

## Registration rules

- Registration is explicit at application startup.
- IDs are stable and globally unique within their registry.
- Registries reject duplicate IDs with a descriptive startup error.
- Providers expose immutable snapshots to consumers.
- Registries must be testable without launching the whole application.
- Adding a registry entry must not require editing an unrelated central feature table.
