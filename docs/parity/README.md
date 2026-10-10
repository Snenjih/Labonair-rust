# Zed Parity Working Records

**Status:** Working records for R07-000; incomplete
**Owner:** Product architecture and capability owners

This directory separates the independent Labonair requirements package from
source-side research. It records current evidence without claiming parity.

## Records

- [`feature-crosswalk.md`](feature-crosswalk.md) — draft user-workflow coverage
  map with proposed/current owners and explicit missing areas. It is not an
  exhaustive action or settings inventory yet.
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

## Clean-room handoff rule

Implementation work may use only the independent feature records, Labonair's
normative contracts, and its current code. It must not receive the Zed
checkout or source-side comparison reports. The reference checkout is pinned
through the parent gitlink and `.gitmodules`, but the source-blind implementation
checkout has not yet been created. Before implementation starts, create a
separate checkout at the accepted parent revision, leave the Zed submodule
uninitialized and absent from the worktree, and omit the Zed-specific files in
`docs/reports/`. Verify the checkout contains neither `zed-refrence/zed` nor
the source-side Settings report. Give the implementation owner only this
requirements directory and Labonair's normative contracts. R07-000 remains
open until the packet and the source-blind checkout have both been reviewed.
