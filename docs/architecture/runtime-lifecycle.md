# Runtime Lifecycle

**Status:** Normative
**Version:** 1
**Related:** [`../architecture.md`](../architecture.md), [`composition-root.md`](composition-root.md), [`../visual-verification.md`](../visual-verification.md)

This is the canonical lifecycle vocabulary for implementation tasks, audits,
and agent handoffs. A feature is not considered integrated because its crate
compiles; it must be traceable through the relevant lifecycle states.

## Lifecycle states

| State | Owner | Required evidence |
|---|---|---|
| `construct` | Composition root | Concrete service and owner contract are wired without a duplicate owner. |
| `register` | Owning module | Commands, panels, status items, notifications, and palette providers use canonical registries. |
| `load` | Owning module plus injected services | Settings, persistence, or remote state is loaded asynchronously and failure is typed. |
| `render` | Owning UI crate | The view uses `labonair-ui-kit`, preserves shell zones, and has native visual evidence when UI changes. |
| `interact` | Owning module | User actions update owner state and emit typed events for other modules. |
| `persist` | Owning module | Values and lifecycle state are written through the capability's persistence boundary. |
| `shutdown` | Composition root and service owner | Tasks, sessions, watchers, and windows release resources without blocking the GPUI thread. |

## Agent checklist

For a lifecycle change, record:

1. the owning module and canonical crate;
2. the construction site and injected contract;
3. the user entry point and all affected shell zones;
4. state transitions, persistence, notifications, and emitted events;
5. failure, retry, cancellation, and shutdown behavior;
6. source, test, or native visual evidence for each claim.

I/O, process execution, network work, and database operations must not block
the GPUI foreground thread. Use async tasks or `spawn_blocking` and return
descriptive errors for predictable failures.

## Evidence vocabulary

Use `Verified`, `Partial`, `Pending`, or `N/A` in audits. A successful build
proves only the paths covered by that build. It does not prove visual layout,
ownership, or remote runtime behavior without corresponding evidence.
