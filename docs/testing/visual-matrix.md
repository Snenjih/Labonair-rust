# Visual Evidence Matrix

**Status:** Normative
**Version:** 1
**Related:** [`../visual-verification.md`](../visual-verification.md), [`../audits/product-surface-acceptance.md`](../audits/product-surface-acceptance.md), [`evidence-model.md`](evidence-model.md)

The product-surface audit is the canonical current-state matrix for native
visual acceptance. This document defines how future surface descriptors link to
that matrix.

The machine-readable screenshot index is
[`visual-evidence.toml`](visual-evidence.toml), with its generated
per-surface view at
[`../generated/visual-evidence.md`](../generated/visual-evidence.md). Keep the
state labels in that source aligned with `product/surfaces.toml`; the checker
rejects aliases, missing states, and untracked surface records.

## Required state coverage

Each surface declares the states that apply to it. At minimum, applicable
surfaces cover:

- normal/ready;
- focused/keyboard-focused;
- empty/no-data;
- narrow or constrained layout;
- loading/in-progress;
- error/failure;
- expanded details or secondary interaction where applicable.

Each matrix cell records the exact native binary, platform, window/PID scope,
state label, artifact path, date, and result. Use `Pending` when the native
capture environment is unavailable. Do not infer a screenshot from source
inspection or a non-native web/reference rendering.

The current R07 matrix remains open for native acceptance. Its pending cells
are a deliberate visible limitation in the generated scorecard.
