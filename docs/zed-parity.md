# Zed Clean-Room Parity Plan

**Status:** Normative product and specification plan  
**Version:** 1  
**Owner:** Product architecture

This document defines how Labonair will target complete user-visible feature,
workflow, keyboard, and UI/UX parity with one fixed Zed reference while keeping
Labonair's implementation, product identity, and architecture independent.
It does not authorize copying source code, assets, or private implementation
details. It is the canonical scope and specification plan; the active task
queue remains `tasks/rework/`.

## Product decision

Labonair is **not a Zed fork**. It remains an independently implemented native
Rust/GPUI application. The target is complete parity with the feature and
interaction surface of the pinned reference, subject to the explicit Labonair
tab-model decision below. “Not a fork” does not mean “not feature complete.”

The current capability and roadmap records contain earlier deferrals and
non-goals that predate this decision. They are not parity exclusions. The
first parity inventory must crosswalk every such decision to the pinned
reference and either replace it with an owner-backed target or record a
specific, user-approved divergence.

### Reference and platform baseline

| Property | Pinned target |
|---|---|
| Local Zed source reference | `zed-refrence/zed` at `3569541038dd51524b03998ba4d38d253cb54f80`; the `nightly-9` label is not confirmed by a local Git tag |
| Initial visual and workflow acceptance platform | macOS |
| Other platforms | Inventory platform-specific behavior in the specification; lock the supported release matrix before cross-platform implementation sign-off |
| Public documentation snapshot consulted | Zed documentation as observed 2026-10-09; newer docs must be checked against the pinned runtime before they change the target |

The referenced Zed working tree is clean at the pinned commit. The parent
repository maps the `zed-refrence/zed` gitlink in [`.gitmodules`](../.gitmodules)
to the upstream Zed repository. Clean-clone acquisition passed from parent
commit `bf70e41`: `git submodule update --init --depth 1 -- zed-refrence/zed`
checked out the exact pinned hash. Do not silently update the pinned commit. A
target update requires a new baseline record and a reviewed feature-catalog
diff. The local checkout is on `main`; no local tag named `nightly-9` points at
this commit.

R07-000 working records are indexed in [`docs/parity/README.md`](parity/README.md).
The Settings source analysis is research-side material in
[`docs/reports/zed-parity-settings-analysis-2026-10-09.md`](reports/zed-parity-settings-analysis-2026-10-09.md).
The source-independent coverage map is
[`docs/parity/feature-crosswalk.md`](parity/feature-crosswalk.md); it is a
draft and does not claim the inventory is complete.

## Clean-room working boundary

1. A reference-analysis role may inspect the pinned Zed runtime, public
   documentation, and source when needed to discover candidate behavior.
2. The analysis output must be rewritten as Labonair requirements: user goals,
   observable state transitions, input/output behavior, measurements, and
   acceptance evidence. Do not transfer Zed code, comments, file/type layout,
   or source-specific algorithms into implementation instructions.
3. Implementation work receives the independent requirements package and
   Labonair contracts only. It does not receive the Zed checkout or
   source-level comparison reports. Existing reports that inspect Zed source
   remain research-side material and are not clean-room implementation specs.
   Enforce this boundary with a source-blind implementation checkout that
   omits the Zed gitlink and source-level comparison reports; review incoming
   patches for provenance before integrating them.
4. Labonair's existing implementation gets a provenance review before it is
   represented as clean-room work. The process cannot retroactively erase
   prior source exposure.
5. Original assets, branding, extension contents, service protocols, and
   distribution obligations receive a separate license and product review.
   This engineering process does not claim legal clearance by itself.

If the same contributor performs both source analysis and implementation,
record the work as an independent rewrite, not as a separated clean-room
implementation.

## Scope inventory

The inventory must cover the pinned app as a whole, including at least:

- application startup, windows, projects, worktrees, workspace lifecycle, and
  session restore;
- titlebar, global menu, status surfaces, docks, panels, overlays, dialogs,
  dropdowns, context menus, tooltips, inputs, buttons, icons, focus, and
  responsive layouts;
- Commands, Command Palette, keymap schemes, contexts, action availability,
  key recording, conflict behavior, and discoverability;
- all Settings categories and fields, defaults, scopes, validation, editing,
  persistence, search, navigation, and reset behavior;
- a shared Tab and Split workspace for all Labonair tab content, including
  Editor, Terminal, Source Control, Git Graph, DiffView, Settings, and other
  registered views;
- Editor, text/buffer lifecycle, search/navigation, syntax, language servers,
  diagnostics, completions, formatting, debugging, and language support;
- Project Explorer, file operations, terminal, tasks, debugger, REPL, Git,
  Source Control, Git Graph, and diff/review workflows;
- themes, icon themes, theme extensions, language extensions, debugger and
  other extension capabilities, extension discovery, installation, update,
  permission, and failure flows;
- AI and agent workflows, MCP, collaboration, remote development, accounts,
  hosted services, privacy/security, update flows, CLI, and platform-specific
  behavior whenever present in the pinned reference.

The current Zed public references are discovery inputs, not a substitute for
the pinned runtime: [UI/UX checklist](https://zed.dev/docs/development/ui-checklist),
[development glossary](https://zed.dev/docs/development/glossary),
[all settings](https://zed.dev/docs/reference/all-settings),
[all actions](https://zed.dev/docs/all-actions),
[language extensions](https://zed.dev/docs/extensions/languages), and
[extension capabilities](https://zed.dev/docs/extensions/capabilities).
Record any difference between those pages and the pinned build.

## Shared Labonair contracts to specify

### Feature contributions

Each feature owns its state, behavior, UI, commands, settings, persistence,
and tests. It contributes typed descriptors through the smallest appropriate
owner registry. The composition root wires owners; it does not repeat feature
metadata in shell-wide tables. A feature addition must not require separate
manual entries for the same command, palette row, keymap, panel, tab kind,
settings field, and menu item in unrelated registries.

The inventory must decide which shared registries are genuinely multi-owner
and define typed contracts for command/keymap contributions, tab kinds, panels
and docks, settings metadata, menus/pickers, language services, extensions,
themes, and icon themes. Single-consumer behavior remains a direct typed
contract rather than a speculative registry.

### Workspace, tabs, panels, and diffs

- One Labonair tab strip manages every registered tab kind. A tab descriptor
  provides identity, label, icon, dirty/busy state, close policy, context
  actions, persistence payload, and its owner renderer.
- Workspace owns cross-capability placement, focus, tab ordering, drag/drop,
  splitting, merging, movement, and session layout. Feature owners own the
  tab contents and their internal state.
- Panels remain docked surfaces with owner-provided descriptors and shared
  dock mechanics. Dialogs, menus, pickers, and transient overlays do not become
  tabs unless the parity inventory proves that workflow requires a durable
  view.
- DiffView is one reusable tab capability. Git, Editor, and other producers
  provide typed diff inputs; DiffView owns rendering, navigation, and common
  review interactions.
- The universal tab bar is an intentional Labonair product decision and an
  explicit divergence from reference-specific tab groupings. It must preserve
  the same user workflows while making all tab kinds discoverable and
  movable through one consistent system.

#### Tab and split lifecycle contract

- A tab kind has one stable owner ID. Each open instance has a distinct
  instance ID, owner label/icon, dirty and busy snapshot, close policy,
  context actions, versioned persistence payload, and owner renderer.
- Workspace owns ordered tabs, active tab, focused pane, split tree, split
  ratios, tab movement between panes, and the serialized layout. Feature
  owners own the document or session state carried by each tab.
- Opening and restoring a tab must resolve its owner and payload version.
  Unsupported or corrupt payloads produce a recoverable item-level failure;
  they must not silently erase the remaining workspace session.
- Closing a tab asks its owner whether close is allowed. Dirty or busy state
  may require an owner-provided decision. Cancel leaves the tab and focus
  unchanged; a confirmed close selects the nearest surviving tab in the same
  pane, then the nearest pane, or the documented empty-pane state.
- Moving a tab preserves its instance ID and owner state. A split creates a
  new pane with a defined orientation and ratio, focuses the destination, and
  retains the existing tab. Merging panes moves tabs in visible order and
  focuses the selected surviving tab. Closing a pane follows the same owner
  close checks as closing its tabs.
- Restoring a session preserves pane order, tab order, active tab, split
  ratios, and focus when the referenced owners and versions are available.
  Missing owners or tabs are reported through the owning recovery surface and
  do not prevent unrelated tabs from restoring.

These rules are a target proposal. R07-000 still needs contract review, typed
API definitions, and acceptance scenarios before they are considered accepted
for implementation.

#### Shared DiffView contract

- A diff producer supplies a typed input containing stable producer identity,
  comparison identity, display labels, optional revision identities, and the
  two comparable content sides or a typed content provider.
- DiffView owns presentation, hunk navigation, side-by-side/unified mode,
  synchronized scrolling where applicable, and common review state. The
  producer owns domain actions such as staging, applying, or saving.
- Diff inputs declare whether they are read-only and which producer actions
  are available. DiffView never infers a Git action from a file path or label.
- Missing content, an expired provider, and unsupported comparison versions
  render a recoverable error while preserving the tab identity and other
  workspace state.

This contract is also a target proposal. Its input shape, navigation behavior,
and owner actions remain open until the crosswalk and typed contract are
reviewed.

### Shared interaction and visual system

The UI-kit specification must define common dimensions and variants for
buttons, icon buttons, text/search inputs, selects, list rows, tabs, menus,
dropdowns, dialogs, tooltips, dividers, and empty/loading/error views. For each
control record geometry, typography, icon sizing, spacing, semantic tokens,
hover/pressed/selected/focused/disabled/error states, keyboard behavior, and
mouse behavior.

Every popup uses one shared positioning contract based on its triggering
element's bounds, viewport collision handling, and keyboard focus. Every menu
uses one shared item model for enabled/disabled, selected/checked, nested,
destructive, and separator states. Owners contribute content; UI-kit owns
rendering mechanics and visual variants.

## Specification record

Every feature or surface entry in the parity catalog records:

```text
Reference ID and pinned build evidence:
Labonair owner and canonical crate:
User goal and end-to-end workflow:
Entry points, commands, contexts, and key bindings:
Views, control geometry, and visual variants:
Normal, focused, empty, loading, error, disabled, narrow, and platform states:
Domain state, lifecycle, cancellation, retry, and recovery:
Settings, defaults, scope, validation, and reset behavior:
Persistence, migration, and session restore:
Registry contributions and extension points:
Cross-capability typed contracts and external services:
Security, permissions, and secret handling:
Performance budget and measurement fixture:
Keyboard-only and mouse acceptance steps:
Reference and Labonair visual evidence:
Parity status: present / partial / missing / intentional divergence:
Dependencies and implementation order:
```

The independent universal UI checklist in
[`docs/parity/universal-checklist.md`](parity/universal-checklist.md) is
attached to every UI or interaction task. It records measurements and
acceptance steps for controls, menus, popups, responsive layouts, keyboard and
mouse input, light/dark themes, workload states, accessibility, failure and
offline behavior, undo, and progressive discovery. Its values remain pending
until paired native evidence exists.

## Settings parity pilot

Settings is the first detailed specification pilot because it has already
been reworked several times without accepted Zed-to-Labonair visual evidence.
The pilot does not begin with another implementation attempt.

1. Correlate the four reported attempts with their commits, screenshots, and
   captured viewport/platform. Preserve those artifacts as a failure review.
   Current commit and artifact findings are recorded separately; the causes
   of the visual misses remain unknown until paired evidence exists.
2. Capture the pinned Zed Settings surface and Labonair Settings under the same
   macOS build, window bounds, theme, scaling, and scroll positions.
3. Inventory every reference category and field, including displayed label,
   setting key, type, default, scope, input control, validation, dependencies,
   reset behavior, and actual effect.
4. Specify navigation hierarchy and measurements: rail width, header and
   section spacing, row heights, label/help alignment, control widths, icon
   geometry, selected/focused styling, scrolling, and narrow-window behavior.
5. Record paired evidence for navigation, search, text entry, selects,
   toggles, numeric controls, scope changes, invalid values, save failures,
   resets, and keyboard-only use.
6. Fix shared components and tokens before composing the full Settings view;
   accept the result only against the paired reference evidence.

The current Settings evidence catalog declares only five generic states, all
`Pending`. It is not proof of reference parity. The R07-000 specification task
must also identify why each of the four earlier attempts missed the target;
do not infer a cause without matching artifacts.

## Work sequence and exit gates

| Stage | Work | Exit condition |
|---|---|---|
| 0 | Lock the pinned reference, macOS capture setup, source boundary, and provenance process | Builds, capture conditions, and remaining blockers are recorded reproducibly |
| 1 | Complete the Settings pilot and review all four earlier attempts | Settings navigation, fields, controls, states, and measurable visual evidence are specified; unknown causes remain labeled |
| 2 | Build exhaustive feature, surface, command, setting, extension, platform, and glossary inventories; crosswalk current Labonair state | No reference area or current Labonair capability is unclassified |
| 3 | Define typed owner-contribution contracts and universal UI/interaction checklist | New features can contribute once through owners; central duplicate metadata is prohibited by contract |
| 4 | Specify shared UI-kit, icon/token, popup/menu, and control geometry | Each reusable primitive has measurable visual and input acceptance evidence |
| 5 | Specify Workspace, universal tabs, split tree, docks, persistence, and cross-view movement | Every tab kind has lifecycle and layout acceptance scenarios |
| 6 | Implement the accepted Settings parity specification on shared controls | Every Settings field and required view state is mapped and visually accepted |
| 7 | Complete Editor, languages/LSP, Explorer, Terminal, Git/Graph, and DiffView contracts | End-to-end workflows, failure states, and module ownership are covered |
| 8 | Complete extensions, themes, remote/collaboration, AI/agent, hosted-service, and platform contracts | Required external services and permission boundaries have independent owners and acceptance evidence |
| 9 | Run full parity acceptance and release review | No unexplained `missing` or `unknown` entries remain; intentional divergences are listed and approved |

Stages 0–5 are tracked by active task `R07-000`. Later implementation tasks
are created from the completed inventory and ordered by the resulting
dependency graph; the existing R09 Editor sequence must be reconciled against
that graph before any R09 task starts.

## Acceptance and evidence

- Every feature documented for the pinned build has a parity record. No item
  may disappear because an earlier Labonair plan marked it deferred.
- Every visible action has a stable command identity where applicable,
  context-aware keybinding behavior, discoverability, and keyboard-only
  acceptance steps.
- Every surface records its applicable normal, focused, empty, loading, error,
  disabled, constrained-layout, and overlay states against the reference.
- Visual evidence identifies both exact builds, OS, viewport, scale, theme,
  and reference/Labonair capture pair. Native captures remain pending when
  capture permissions or the supported host are unavailable.
- Large-project and large-file behavior, responsiveness, and performance are
  measured against fixed fixtures rather than qualitative claims.
- Extension, language-server, remote, collaboration, AI, and hosted-service
  workflows include allow/deny, offline, cancellation, recovery, and secret
  redaction cases.
- A complete parity claim requires every catalog entry to be present or an
  explicitly approved Labonair divergence. Legal and distribution review is
  recorded separately from engineering parity.
