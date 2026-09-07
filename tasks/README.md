# Implementation Tasks

The historical task tree was written for the previous architecture and is
retained for traceability under `tasks/archive/`. It is not the active source
of product direction.

The active sequence is [`../docs/rework-roadmap.md`](../docs/rework-roadmap.md). New tasks must be created from that roadmap and must identify the owning module, public contracts, affected registries, UI-kit components, migrations, and verification gates.

The current active task queue is [`rework/`](rework/). R00 through R06 are
complete. The current task is
`rework/R07-001-product-surface-acceptance.md`, which audits the remaining
surface-ownership and visual-evidence gaps.

Do not start a historical task merely because it is marked `Todo` or `In Progress`.

Run `python3 scripts/check_rework_queue.py` after changing the active queue.
It verifies that the queue lists every rework task exactly once and that the
earliest incomplete task is the only task marked in progress.
