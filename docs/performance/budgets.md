# Performance Budgets

**Status:** Normative
**Version:** 1
**Related:** [`../performance.md`](../performance.md), [`../perf-baseline.md`](../perf-baseline.md), [`../testing/evidence-model.md`](../testing/evidence-model.md)

The machine-readable source is [`budgets.toml`](budgets.toml), and the
generated index is [`../generated/performance.md`](../generated/performance.md).
Run `python3 scripts/verify.py --scope performance` before changing a budget
or claiming a measurement.

These are target budgets, not measured claims. A budget becomes `Verified` only
after a reproducible release-build measurement is recorded on the supported
native host.

| Area | Target | Current evidence |
|---|---|---|
| Cold start to visible window | `< 400 ms` on macOS Apple Silicon release build | Pending native measurement. |
| Start to interactive terminal | `< 700 ms` | Pending native measurement. |
| Idle RSS | `< 150 MB` with one tab | Pending native measurement. |
| Terminal output | No visible hitch at 200k lines; bounded scrollback | Source guards exist; native workload pending. |
| Large lists | 5k-row Explorer/SFTP/Git views remain virtualized and responsive | Source guards exist; native workload pending. |
| Git status | One poll per configured interval, no overlap | Source guard and tests; runtime measurement pending. |
| UI foreground | No filesystem, process, network, or database blocking | Code review and async boundaries; profiler evidence pending. |

The container has no display server and cannot provide credible GPU/frame/RSS
evidence. Keep these items pending until the documented macOS measurement
procedure runs.
