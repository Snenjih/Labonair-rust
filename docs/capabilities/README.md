# Capability Documentation

This directory contains capability-specific detail that is too operational for
the top-level matrix but still belongs to the owning module's documentation.

The descriptor contract is [`schema.md`](schema.md), the all-row migration
coverage source is [`coverage.toml`](coverage.toml), and the current pilot
projection is [`../generated/capabilities.md`](../generated/capabilities.md).
Descriptors and coverage records are source data; generated views must never
be edited directly.

The authority map remains [`../capabilities.md`](../capabilities.md). A file in
this directory may explain a capability's contract, entry points, persistence,
events, settings, or evidence, but it must not create a second owner or
contradict the matrix. Link new capability documents from the matrix and from
the relevant implementation task. A `matrix-tracked` coverage record is an
explicit migration state, not evidence that a full descriptor already exists.
Every coverage record also names a repository-relative `next_task` in the
active rework queue so the migration has a bounded owner and cannot become an
untracked prose promise.

Use the following minimum structure for a capability-specific document:

1. owner and canonical crate;
2. user entry points and menu/panel/status surfaces;
3. state, persistence, settings, commands, and notifications;
4. public typed contracts and integration adapters;
5. current evidence, gaps, and verification commands.
