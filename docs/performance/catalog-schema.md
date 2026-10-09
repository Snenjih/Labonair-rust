# Performance Catalog Schema

**Status:** Normative
**Version:** 1
**Related:** [`budgets.md`](budgets.md), [`../performance.md`](../performance.md), [`../perf-baseline.md`](../perf-baseline.md), [`../testing/evidence-model.md`](../testing/evidence-model.md)

The structured performance catalog separates a target budget from a measured
baseline. A budget is never marked `Verified` merely because the source code
contains a guard or because Rust tests pass.

Each record identifies the owner, unit, target, measurement procedure,
supported environment, current evidence, source references, limitation, and
next measurement action. Native GUI, frame-time, memory, and release-build
claims remain `Pending` until the documented supported-host procedure records
an artifact.

Run the check with:

```text
python3 scripts/verify.py --scope performance
```
