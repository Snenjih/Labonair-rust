# Extensions System — Concept

**Status:** Idea (non-normative, see [`README.md`](README.md))
**Scope:** Introduces a universal extensions capability that eventually
subsumes themes and icon themes, and later grows into a VSCode-scale
extension surface (new UI, new functionality, third-party distribution).

This document only records what was discussed and decided in conversation. It
is not binding until incorporated into `docs/capabilities.md`,
`docs/modules.md`, `docs/registries.md`, and the roadmap, or accepted as an
ADR.

## Motivation

Themes and icon themes are already a de-facto extension category: they have
their own store (`crates/theme/src/store.rs`, `crates/theme/src/icon_theme.rs`)
and their own command-provider registration. Rather than growing more
one-off "pluggable content" mechanisms per feature (themes today, snippets or
keymap packs tomorrow), we build one generic extensions capability that:

- owns installation, enablement, versioning, and update of extension packages;
- lets themes and icon themes become the first two extension kinds instead of
  a special-cased subsystem;
- is designed from the start to grow toward code-carrying extensions (new
  status bar items, panels, settings pages, editor/terminal/SFTP
  contributions, keymaps) without a rewrite of the base.

## Non-goals (for now)

- No WASM/code-execution sandbox in phase 1 or 2. Phase 1/2 extensions are
  **data-only** (manifest + assets). Code-carrying extensions are phase 3+
  and are a separate, much larger design (see "Open questions").
- No public marketplace UI/backend build-out before the local extension
  format is stable. The registry described below starts as an index, not a
  hosting platform with accounts, ratings, payments, etc.
- No attempt to match the full VSCode extension API surface up front. VSCode-level
  richness is the long-term direction, not a phase 1/2 requirement.

## Ownership

Per `docs/modules.md`, this is one product capability with one owning module.

- **Owning module:** `extensions`
- **Canonical capability crate:** `crates/extensions` (domain model, manifest
  schema, local registry/store, install/enable/disable/update lifecycle,
  persistence)
- **Sibling crates**, added only when a real boundary exists (per module
  rule 3):
  - `crates/extensions-ui` — the Extensions tab (browse installed, browse
    index, install/uninstall/enable/disable, update) and, in later phases,
    the per-extension settings sub-pages.
  - `crates/extensions-registry-client` — talks to the remote index (fetch
    manifest list, download packages, verify checksums/signatures). Split
    from the core crate because it has network I/O and can be tested/mocked
    independently.

`theme` and a new `icon-theme` ownership stay themselves the *behavioral*
owners of what a theme/icon theme does once loaded (parsing, applying tokens,
command-palette submenu, etc. — see `docs/registries.md` "Dynamic entries such
as hosts, themes, and tabs"). `extensions` owns *packaging, discovery,
lifecycle*. This mirrors the existing rule that a registry owns discovery and
the owning module owns behavior behind each entry.

## Phase 1 — Extension base + themes/icon themes only

Goal: lay the generic foundation, but the only two extension **kinds**
implemented are `theme` and `icon-theme`.

### Manifest format

A minimal manifest per extension, versioned from day one so phase 2/3 kinds
can be added without breaking phase 1 packages:

- `id` (stable, namespaced, e.g. `publisher.name`)
- `name`, `version` (semver), `publisher`
- `kind`: enum, extensible — phase 1 only allows `theme` | `icon-theme`
- `engine` / `api_version`: minimum Labonair extensions-API version the
  package targets, so future phases can reject incompatible packages instead
  of silently misbehaving
- kind-specific payload block (for `theme`/`icon-theme`: path(s) to the
  existing theme/icon-theme asset files, reusing current parsing in `theme`)
- optional `license`, `repository`, `description`, `icon`

### Local registry/store (`crates/extensions`)

- Stable ID namespace, duplicate-ID is an error (same rule as command
  registry).
- Install = copy/validate package into a local extensions directory +
  register manifest in the store.
- Enable/disable is a persisted flag per extension; disabling does not
  uninstall.
- Update = fetch newer manifest+package from the registry client, validate
  `engine` compatibility, replace.
- Snapshot API for consumers (`extensions-ui`, and internally `theme`/
  `icon-theme` loaders), same "consumers receive snapshots, don't mutate
  registry-owned state directly" rule as other registries.
- Persistence: local settings/state store, following the existing
  settings persistence conventions (see `docs/settings.md`).

### theme / icon-theme integration

`theme` stops being the sole loader of theme/icon-theme assets from a
hardcoded location; instead it resolves themes contributed by:
1. built-in themes (unchanged, shipped with the app), and
2. themes contributed by installed+enabled `theme`/`icon-theme` extensions,
   via the extensions store snapshot.

This is additive — existing built-in theme loading and the theme command
submenu are not rewritten, they gain a second source.

### Extensions tab (`crates/extensions-ui`)

- New top-level shell tab/panel, wired by the composition root like other
  panels (no static shell-wide panel table, per `CLAUDE.md`).
- Two views to start: **Installed** (list, enable/disable/uninstall toggle,
  update-available indicator) and **Browse/Index** (list from the remote
  registry index, install action).
- Search/filter reuses existing UI-kit list/search components, no new
  bespoke widgets.

### Registry indexing — GitHub workflow

Per the decision made in conversation: no custom index server for phase 1.

- Extension authors submit a manifest (and package reference) via PR to a
  dedicated `labonair-extensions-index` repository (or a directory in an
  existing repo — TBD, see open questions).
- A GitHub Actions workflow validates the manifest schema (`id` uniqueness,
  semver, required fields, `engine` compatibility) and, on merge to the
  index's default branch, regenerates a static index file (e.g.
  `index.json` or per-kind indices) and publishes it (GitHub Pages, or a
  release asset).
- `extensions-registry-client` fetches that static index — this means the
  "registry" phase 1 is a static file behind CI validation, not a running
  service. No accounts, no upload endpoint, no database to operate.
- Package hosting for phase 1: either the extension repo itself (index
  entry points at a GitHub release asset / raw file) or a thin CDN in front
  of the same GitHub-hosted artifacts. No custom storage service needed yet.

### Phase 1 success criteria

1. `theme`/`icon-theme` extensions can be installed from a local `.zip`/
   folder and take effect → verify: install a sample theme extension,
   confirm it appears in the theme command-palette submenu and applies.
2. Extensions tab lists installed extensions and can enable/disable/
   uninstall them → verify: toggle state persists across restart.
3. Extensions tab can browse the GitHub-workflow-generated index and install
   from it → verify: index entry → download → install → appears in
   Installed.
4. Duplicate `id` registration is rejected with a clear error → verify: unit
   test in `crates/extensions`.

## Phase 2 — Status bar items, panels/dropdowns, per-extension settings

Only starts once phase 1's base (manifest format, store, registry client,
tab) is in place and stable.

Adds new extension `kind`s that are still **data/config-driven**, not
arbitrary code:

- **Status bar item contributions**: an extension manifest can declare a
  status bar entry (label/icon, and an action: open a dropdown menu with
  declared static entries, or open a sidebar panel). Behind the scenes this
  is a contribution to the existing workspace status registry
  (`docs/registries.md`: "Hidden status-bar state and labels are owned by
  the workspace status registry and use the same snapshot contract") — the
  extensions module registers on behalf of installed extensions, it does not
  become a second status registry owner.
- **Sidebar panel contributions**: declarative panel definition (what it
  shows — initially likely a constrained set of declarative content, not
  arbitrary GPUI code, since there's still no code sandbox in phase 2).
- **Settings contributions**: introduces a new **top-level settings
  category "Extensions"**. Its content is one **sub-level category per
  installed extension** that declares settings (each extension's manifest
  declares its own settings schema: keys, types, defaults, labels). This
  follows the existing settings-guidelines top-level/sub-level pattern
  (`docs/settings-guidelines.md`) rather than inventing a new settings UI
  pattern.

Open design question carried into phase 2 (do not resolve now, flag when
phase 2 starts): how much of "dropdown"/"panel" content phase 2 allows to be
declarative-only vs. how much pulls forward some of phase 3's code surface
early. The conversation's intent is that phase 2 stays config-driven; phase 3
is where real code/UI injection begins.

## Phase 3 — Broader surface + custom keymaps

Only starts once phase 2's status bar/panel/settings contribution model is
in place and stable.

- Extend the contribution surface into more app areas: editor, terminal,
  SFTP file manager, explorer — extensions can add and modify things in
  these areas (exact mechanism TBD: likely where a real extension API /
  sandbox becomes unavoidable, since "modify" behavior in these areas is
  where VSCode-style extensions stop being declarative).
  This is the phase where the "non-goal: no code sandbox" from phase 1/2
  is revisited and where a WASM-based execution model (see open questions)
  most plausibly gets designed.
- Custom keymaps contributed by extensions: an extension can ship its own
  keybinding defaults. Integrates with the existing keymap default-layer
  mechanism described in `docs/registries.md` ("a provider attaches
  defaults ... the keymap module materializes those typed descriptors into
  its built-in default layer") — extensions become another default-binding
  provider, not a parallel keymap table.

## Open questions (not decided yet)

- Exact repository shape for the extensions index (dedicated repo vs.
  directory in an existing repo) and where built package artifacts are
  hosted long-term.
- Trust/signing model for third-party packages before phase 3 introduces
  code execution — becomes load-bearing once extensions can run code, not
  strictly needed while phase 1/2 stay data-only.
- Code-execution model for phase 3 (WASM component model is the most likely
  Rust-native fit for sandboxing; not evaluated in detail yet).
- Whether phase 3's "modify editor/terminal/SFTP" contributions need a
  typed extension-facing API crate (analogous to how `command-palette-core`
  is UI-free and typed) before any code sandbox is built, so the API
  surface is designed once rather than organically grown.

## Relationship to existing docs

This idea, if accepted, would eventually need:

- a new module entry in `docs/modules.md` and `docs/capabilities.md`
  (`extensions` module, `crates/extensions` canonical crate);
- a new registry section in `docs/registries.md` describing the extensions
  registry's ID namespace, lifecycle, and snapshot contract;
- a new top-level settings category entry once phase 2 lands, per
  `docs/settings-guidelines.md`;
- roadmap/task entries under `docs/rework-roadmap.md` /
  `tasks/rework/README.md` when phase 1 is scheduled for implementation.

None of these documents are edited by this idea file itself — per
`docs/documentation-governance.md`, an idea becomes binding only once
incorporated into the normative documents or accepted via ADR.
