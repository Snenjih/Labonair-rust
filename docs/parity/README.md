# Zed Parity Working Records

**Status:** Working records for R07-000; incomplete
**Owner:** Product architecture and capability owners

This directory separates the independent Labonair requirements package from
source-side research. It records current evidence without claiming parity.

## Records

- [`feature-crosswalk.md`](feature-crosswalk.md) — draft user-workflow coverage
  map with proposed/current owners and explicit missing areas. It is not an
  exhaustive settings or runtime inventory yet.
- [`action-requirements.md`](action-requirements.md) — stable requirement IDs
  for all 595 action identifiers across the default macOS, Linux, and Windows
  keymaps, with 2,731 binding/context records and four null entries. It records
  five deprecated-alias resolutions from the pinned Linux action registry.
  The 585 macOS IDs remain stable. Independent static intent and owner-route
  packets currently cover 23 Search actions, 22 Pane actions, one Editor Go
  to Line action, one File Finder action, two Workspace Open/OpenFiles
  actions, and one Workspace New Window action; pinned dispatch and native
  interaction acceptance remain open for those and all other actions.
- [`action-classification-search.md`](action-classification-search.md) —
  independently stated user goals, static Labonair owner routes, code coverage,
  and runtime/evidence gaps for 23 default-macOS Search-family actions. No
  action is marked runtime-observed.
- [`action-classification-pane.md`](action-classification-pane.md) —
  independently stated user goals, Workspace/Editor owner boundaries, current
  tab/pane routes, and evidence gaps for 22 default-macOS Pane actions. The
  split between universal tabs, outer panes, and Editor groups remains explicit;
  runtime acceptance is pending.
- [`action-classification-go-to-line.md`](action-classification-go-to-line.md) —
  independently stated intent, Editor owner, current line-jump route, and
  evidence gap for the default-macOS Go to Line action. The arbitrary
  line/column prompt and its runtime acceptance remain open.
- [`action-classification-file-finder.md`](action-classification-file-finder.md) —
  independently stated intent, Editor/Workspace ownership, current Open File
  route, related Settings targets, and runtime/evidence gaps for the single
  default-macOS File Finder action.
- [`action-classification-workspace-open.md`](action-classification-workspace-open.md) —
  independently stated intent, Workspace/Editor ownership, distinct general
  Open and platform Open Files routes, related Settings targets, and runtime
  gaps for the macOS-bound Workspace Open action and Linux/Windows-only
  Workspace OpenFiles action.
- [`action-classification-workspace-new-window.md`](action-classification-workspace-new-window.md) —
  independently stated intent, Workspace/application-window ownership,
  `On New Window` setting behavior, the missing Labonair action route, and
  runtime/evidence requirements for the default-macOS New Window action.
- [`keymap-overlay-requirements.md`](keymap-overlay-requirements.md) — stable
  IDs for 1,402 bindings across seven macOS base-profile overlays, conditional
  Vim/Helix bindings, and macOS-specific overrides. Profile behavior, action
  semantics, and native macOS dispatch remain open. It also records 133
  default-map context-block flags for key-equivalent handling.
- [`keymap-nonmacos-requirements.md`](keymap-nonmacos-requirements.md) — stable
  IDs for 446 shared Linux/Windows profile and override bindings, plus 259
  default-map key-equivalent block records. Windows runtime acceptance and
  both platforms' action semantics and dispatch remain open.
- [`registered-actions-unbound.md`](registered-actions-unbound.md) — stable IDs
  for 813 actions registered by the pinned Linux runtime but absent from all
  three default keymaps. Reachability, semantic classification, Labonair owner
  and route, and runtime behavior remain open. The complete pinned registry
  metadata and schemas are in [`evidence/zed-actions-3569541-linux-x86_64.json`](evidence/zed-actions-3569541-linux-x86_64.json).
  The JSON under `evidence/` is machine-generated reference metadata, not an
  implementation brief or proof of user-visible behavior.
- [`titlebar-tab-strip.md`](titlebar-tab-strip.md) — independent titlebar tab
  appearance, input/state matrix, ownership, and evidence packet.
- [`settings-requirements.md`](settings-requirements.md) — 486 stable IDs for
  static Settings fields, per-language values, shipped profiles, dynamic setup
  pages, and core Settings flows. Field effects, exact owners/routes, runtime
  behavior, and visual acceptance remain open.
- [`settings-crosswalk.md`](settings-crosswalk.md) — independent Settings
  page-to-owner map and implementation order. It records page ownership;
  field-level acceptance remains incomplete.
- [`../reports/zed-parity-settings-analysis-2026-10-09.md`](../reports/zed-parity-settings-analysis-2026-10-09.md)
  — research-side Settings source and commit review. Do not use this report as
  an implementation brief.
- [`../reports/zed-titlebar-tabbar-analysis-2026-10-10.md`](../reports/zed-titlebar-tabbar-analysis-2026-10-10.md)
  — source-side analysis of the supplied Editor tab-bar reference and current
  Labonair titlebar. The implementation packet above is the user-facing
  requirements record.
- [`../zed-parity.md`](../zed-parity.md) — normative target, clean-room rules,
  required feature-record fields, and exit criteria.

## Evidence vocabulary

| Label | Meaning |
|---|---|
| `Source checked` | The pinned tree or current Labonair source was inspected. It proves structure, not rendered behavior. |
| `Runtime observed` | The exact pinned runtime was opened and the named behavior was inspected. |
| `Capture verified` | An inspected native capture records the exact build and matching environment. |
| `Public-doc only` | A current public documentation page describes the behavior; its match to the pinned build is unverified. |
| `Pending` | Evidence is missing or the comparison has not been completed. |

## Provenance and source boundary

The current R07-000 analysis and implementation were performed in the same
contributor session. A source-blind implementation checkout was not used, so
this work is recorded as independent redevelopment and makes no claim of
separated-room clean-room provenance. That limitation applies to prior R07-000
implementation changes too; it cannot be changed retroactively by creating a
new checkout. Do not copy Zed code, comments, assets, fonts, or implementation
details.

The source-side reports remain research records and are not implementation
briefs. Future work may use a source-blind checkout when its contributor has
not received source-level reports. Implementation packets in this directory
must contain independently written user goals, observable flows, states,
owner/contracts, settings and persistence, dependencies, and acceptance
evidence. The independent package is not complete while any field is missing
or marked pending. Runtime evidence and action-level parity remain open under
R07-000.
