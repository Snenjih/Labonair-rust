# Settings Surface Crosswalk

**Status:** Initial independent implementation packet; incomplete
**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`
**Owner:** Settings and capability owners

This record defines the Settings pages Labonair must account for and maps each
page to its current capability owner. It describes user-visible scope and
implementation status only; it contains no reference source paths or copied
implementation details. `Partial` means the owner or some matching values
exist, not that the page is accepted.

## Page and owner crosswalk

| Order | Settings page | Canonical Labonair owner(s) | Current values or surface | Status and dependency |
|---:|---|---|---|---|
| 1 | General | Settings, Workspace, Updater | Startup choice, session restore, window restore, update checks, quit behavior | Partial; complete field/default/effect crosswalk and match remaining runtime behavior |
| 2 | Appearance | Settings, Themes, Backgrounds | App typography, density, motion, corner scale, tab placement, theme selection, background image presentation | Partial; Settings links to Theme and Background owner surfaces; finish typography coverage and visual acceptance |
| 3 | Keymap | Keymap | Key binding editor and keymap selection | Partial; the Settings page now opens the canonical Workspace Keymap tab; editor state and persistence remain Keymap-owned |
| 4 | Editor | Editor | Typography, wrapping, line numbers, indentation, Vim, caret and current-line behavior | Partial; implement remaining editor behavior before exposing its values |
| 5 | Languages & Tools | Editor, Snippets, planned Extensions | Language services, toolchains, formatters, language-specific support | Missing; language-service lifecycle and extension owner are dependencies |
| 6 | Search & Files | File Finder, Filesystem, Explorer, SFTP | File search, Explorer behavior, SFTP browser presentation | Partial; map search and Explorer settings, retain SFTP values with their owning workflow |
| 7 | Window & Layout | Workspace | Startup/session choices, tab placement, zen presentation; runtime layout is Workspace-owned | Partial; define the universal tab/split/layout setting boundary and persistence |
| 8 | Panels | Workspace and panel owners | Panel visibility, docking, and panel presentation | Missing as a Settings page; depends on the accepted dock and owner-contribution contract |
| 9 | Debugger | Planned Debugger | No current debugger settings owner | Missing; debugger capability, protocol, lifecycle, and settings contract come first |
| 10 | Terminal | Terminal, Workspace | Shell, typography, cursor, scrollback, input, environment, and close behavior | Partial; finish terminal value/effect coverage and visual states |
| 11 | Version Control | Git, Source Control, Git Graph | Explorer Git decorations and SCM tree preference | Partial; map Git behavior settings and keep repository operations owned by Git |
| 12 | Collaboration | Planned Collaboration | No current collaboration settings owner | Missing; collaboration lifecycle, identity, privacy, and permissions come first |
| 13 | AI | AI, MCP | AI capabilities and provider/session preferences; credentials remain capability-owned | Partial; define safe non-secret values and compose the AI owner's configuration surface |
| 14 | Network | SSH, Connections, planned Remote Development | SSH timeout and keep-alive fallbacks | Partial; inventory proxy, transport, and remote workspace behavior with explicit secret boundaries |
| 15 | Developer | Planned Developer diagnostics and developer tooling | No current Settings value group | Missing; add only values with a live consumer and a defined support workflow |

The current seven `SettingsContent` fields (`general`, `appearance`,
`terminal`, `editor`, `fileManager`, `workspace`, and `connections`) describe
persisted value groups. They are not the target navigation taxonomy. One value
group may supply fields to more than one page, and one page may collect fields
from more than one owning capability. The navigation model must therefore
resolve stable field IDs rather than assume that one persisted struct equals
one visible page.

No page may be populated with inert placeholder settings. A value is visible
only when its owner, type, default, scope, validation, reset behavior, and
runtime effect are known. A management workflow stays with its capability;
Settings may embed or navigate to that owner's canonical editor without
creating a second editor or persistence path.

Owner UI modules register links with stable IDs and metadata through the
Settings surface contract. One registry snapshot drives both their page rows
and Settings search, so a new link does not require feature-specific
navigation or search code. The composition root connects the contribution's
action to its canonical owner surface.

## Current persisted-value placement

The Settings page resolver currently routes registered values as follows.
This changes navigation only: ownership, JSON paths, scope, and migrations
remain with their existing contracts. Explicit field placement takes
precedence over a storage-group fallback, so a value appears on one canonical
edit page.

| Current value group | Target page placement | Existing ownership constraint |
|---|---|---|
| `general` | General; `theme` is on Appearance and `restoreWindowState` is on Window & Layout | Keep window/session/update values with their live consumers |
| `appearance` | Appearance; tab and zen presentation values are on Window & Layout | Themes and Backgrounds own catalogs, previews, and their runtime state |
| `terminal` | Terminal | Terminal and Workspace consume typed values; credentials stay outside Settings |
| `editor` | Editor, Languages & Tools, and Version Control | Editor owns document behavior and language-service state |
| `fileManager` | Search & Files and Version Control | Explorer, Filesystem, Git, and SFTP retain their behavior; do not create a duplicate browser settings store |
| `workspace` | General for palette/quit values; Window & Layout is the fallback for future owner-backed values | Workspace layout/session state remains runtime state, not a Settings override |
| `connections` | Network | SSH/Connections own transport behavior; Settings stores only non-secret fallback values |

The current runtime exposes ten pages, including the owner-linked Keymap
page. Panels, Debugger, Collaboration, AI, and Developer stay in the target
inventory until their owner-backed Settings contribution or canonical
embedded surface exists.

## Required field record

Before a field is accepted into a page, record all of the following:

- stable field ID and visible label;
- owning capability, typed value, and runtime consumer;
- default, valid values/range, and validation feedback;
- supported scopes and project whitelist decision;
- control type, compact geometry, keyboard behavior, and reset behavior;
- persistence, migration, and live-update or restart behavior;
- normal, focused, empty, loading, error, disabled, and narrow-viewport states;
- paired macOS reference and Labonair evidence, or a named capture blocker.

Search results, deep links, and keyboard navigation use the same stable field
ID and page descriptor as ordinary navigation. Reset clears the current sparse
override. A field shown in more than one workflow has one canonical edit page;
other surfaces link to that page.

## Implementation order

1. Complete the field-level reference crosswalk and record missing runtime
   consumers. Keep source research separate from this independent packet.
2. Separate visible Settings page metadata from `SettingsContent` persistence
   groups. Preserve existing JSON keys and migrations while navigation moves
   to stable field IDs.
3. Finish the pages whose owners and current values already exist: General,
   Appearance, Editor, Search & Files, Window & Layout, Terminal, Version
   Control, and Network.
4. Route the Keymap page to the canonical Keymap owner surface rather than
   maintaining a second key binding editor in Settings.
5. Add Languages & Tools, Panels, Debugger, Collaboration, AI, and Developer
   content only as their owner contracts and real settings values are
   implemented. Do not mark those pages complete while their features are
   absent.
6. Capture every page under the same macOS build, window, theme, font, and
   scale conditions. Verify search, scope changes, reset, focus traversal,
   narrow layout, and field states against the universal checklist.

This packet is incomplete until all fields and conditional pages in the
pinned baseline have a record and each category has owner-backed content.
