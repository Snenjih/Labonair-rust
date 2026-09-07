# Registry Contracts

**Status:** Normative

Registries are the extension mechanism for a capability that has multiple
providers or consumers and therefore needs discovery. A registry owns
discovery, stable metadata, and lifecycle; the owning module owns the behavior
behind each entry. For a single provider and consumer, prefer a direct typed
trait or event instead of inventing a registry.

## Common registry contract

Every registry must document:

- its owner and lifecycle (startup, workspace-scoped, or long-lived);
- its stable ID namespace and duplicate-ID behavior;
- whether providers are static or asynchronous;
- snapshot, refresh, and invalidation semantics;
- action context and result/error handling;
- persistence and migration behavior, if any;
- focused contract tests and an explicit removal path for temporary adapters.

Consumers receive snapshots or typed handles. They do not mutate another
module's registry-owned state directly.

Registration is performed by the owning module through a typed API. The
composition root invokes those registration functions and connects concrete
services, but it must not become a second registry owner or maintain a
parallel list of entries. IDs are stable and duplicate registration is an
error.

The command registry contract is intentionally UI-free. Capability crates may
depend on `labonair-command-palette-core` to publish metadata, while the
palette UI depends on the contract and only renders its snapshots. Executable
owner callbacks use the separate `labonair-command-palette-runtime` bridge;
that bridge knows GPUI but not the shell or any product module. Stable shortcut
identities live in `labonair-interaction-contracts`, so the command core,
runtime, and keymap can depend on the same foundation without a cycle.

## Command registry

The command-palette module owns one command registry. Feature modules register
their own commands with:

- stable action ID;
- title and searchable aliases;
- category or submenu;
- applicable contexts;
- default keybinding metadata;
- execution callback or typed action payload.

The palette owns search, filtering, navigation, preview, and selection. It
does not own feature state, feature behavior, a giant dispatch table, or a
second static command table in the shell. A submenu is a registry/provider
contribution, not a special case in the palette view.

Adding a command must be a contribution from its owning module; it must not
require editing a palette-owned list of every feature command. The stable ID
is namespaced and opaque to the view, and execution is resolved by the owning
module after selection.

Default bindings follow the same ownership rule. A provider attaches defaults
to its `CommandDescriptor` with `with_default_binding`; the keymap module
materializes those typed descriptors into its built-in default layer. The
shipped JSONC asset remains a compatibility layer during migration and the
user keymap remains the override layer. Adding a command or changing its
default therefore does not require editing a shell-owned keymap table.

Submenus are registered providers. Dynamic entries such as hosts, themes, and
tabs are supplied as immutable snapshots with typed primary and secondary
actions. The capability owns loading and action semantics; the palette owns
only common filtering, focus, preview, and picker interaction. Snapshot
builders now live in the workspace, hosts, editor, theme, snippets, and Git
providers. The composition root only supplies live values and registers the
resulting snapshots. Hidden status-bar state and labels are owned by the
workspace status registry and use the same snapshot contract.

## Keymap registry

Every command-capable action may register a keybinding descriptor with the
keymap module. The keymap system owns:

- keymap file loading and persistence;
- the versioned JSONC document shape and built-in default assets;
- contexts and precedence;
- conflict detection;
- display formatting;
- user overrides.

Shortcut identity is defined by `labonair-interaction-contracts`, below both
the keymap and command registries. This lets keymap publish its own command
metadata without making the command contract depend back on keymap.

The keymap system does not contain feature behavior. A feature owns the action
it registers and supplies the stable command ID; keymap resolution only maps
that ID to user input. The keymap editor is a keymap surface, not a Settings
category.

`keymap::adapter::load` is the module-owned loading boundary. It accepts the
command registry, builds the known-action vocabulary and owner-default layer,
and returns one immutable snapshot containing effective bindings and current
diagnostics. A platform shell may install that snapshot into GPUI, but it must
not reimplement loading, validation, recovery, or layer composition.

The editor-facing `KeymapDocument` keeps the original user source as the
authoritative value and exposes parsing/validation as derived state. Saving
must write that source unchanged; invalid or unknown entries are therefore
diagnosed without being silently normalized away.

`labonair-keymap-ui` is the keymap module's sibling presentation boundary. It
receives a `KeymapManagementSnapshot`, provides filtering and diagnostics, and
offers the raw JSONC editor as an explicit action. It does not read files on
the GPUI thread, resolve commands, or maintain a second shortcut registry.

The file/GPUI adapter must cross this boundary once: persisted action names are
resolved through `keymap::runtime::command_for_action` into `CommandId`, and
only then mapped to a concrete platform action. GPUI context predicates stay
in the adapter because they are richer than the keymap runtime's portable
context identifiers; the adapter must not duplicate action-name aliases.

Palette and other UI surfaces consume effective bindings by `CommandId`. The
legacy `ShortcutId` table remains exported only for migration of older callers;
it must not be used as the source for new command rows or keymap management
views.

The legacy action name `settings::OpenShortcuts` is accepted only as a
migration alias for existing user keymap files and resolves to the canonical
`Open Keymap (JSON)` action. It is not registered as a separate palette entry,
native menu surface, or Settings page. New actions must use the keymap/command
contracts rather than the legacy name.

## Theme registries

The themes module owns separate color-theme and icon-theme registries. Each
entry contains a stable ID, display name, metadata, and a complete or layered
definition. Preview is transactional: navigation applies a temporary preview,
while confirmation persists the selected ID. The global menu exposes both
surfaces through the command palette; neither is a Settings management page.

Initial themes are built in. Downloading and extensions are deferred until a
concrete workflow and owner exist; they are not part of the initial registry
contract.

## Panel and status-item registries

Panels register identity, title, icon, supported docks, and a view factory. Status items register identity, placement metadata, badge behavior, and a view factory. The shell provides the host surface; feature modules own their content and actions.

## Notification center

`labonair-notifications-core` is the single user-message registry. It accepts
structured notifications with kind, title, summary, optional details, source,
timestamp, actions, and deduplication metadata. Records remain available until
dismissed or cleared and expose read state for the statusbar badge. It does not
render toasts, run timers, or contain feature-specific error handling.

The GPUI notification adapter may temporarily bridge callback actions for
existing callers. New actions must use stable IDs and be interpreted by the
owning module or command registry.

User-visible errors are notifications too. A feature may keep an internal
error state for retry logic, but it must not render a second feature-local
error banner for the same user-facing failure.

Publishing a notification is the complete user-message path. A caller must
not publish the same event and also render a toast, inline banner, or feature
local message. The statusbar dropdown is the sole global notification
presentation surface.

## Transfer registry

`labonair-transfers::TransferRegistry` is the sole owner of transfer lifecycle
state. It is long-lived for the application session and retains queued,
active, paused, failed, cancelled, and completed records until the user clears
terminal history. It consumes typed `TransferEvent` values and exposes
immutable `TransferSnapshot` values to the UI.

`TransferService` is the action contract for enqueue, cancellation, and
conflict/file-error resolution. `TransferEventSource` is the read-only event
boundary. The raw adapter event bus is translated once by
`labonair-transfers-ssh`; no workspace or statusbar code decodes raw event
names. The statusbar-hosted `labonair-transfers-ui` view is the canonical presentation
surface and emits only a typed completion signal for the SFTP pane refresh.
Transfer history is intentionally in-memory for this migration; durable
history requires a separate persistence decision and schema task.

## Registration rules

- Registration is explicit at application startup.
- IDs are stable and globally unique within their registry.
- Registries reject duplicate IDs with a descriptive startup error.
- Providers expose immutable snapshots to consumers.
- Registries must be testable without launching the whole application.
- Adding a registry entry must not require editing an unrelated central feature table.
