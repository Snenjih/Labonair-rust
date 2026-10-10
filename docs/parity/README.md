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
  for each of the 585 default macOS bound actions, with binding/context
  inventory and per-action semantic, owner, route, and runtime gates. All
  action-level acceptance remains open.
- [`titlebar-tab-strip.md`](titlebar-tab-strip.md) — independent titlebar tab
  appearance, input/state matrix, ownership, and evidence packet.
- [`settings-crosswalk.md`](settings-crosswalk.md) — independent Settings
  page-to-owner map and implementation order; its field-level inventory is
  still incomplete.
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
