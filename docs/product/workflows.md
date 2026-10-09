# Product Workflow Catalog

**Status:** Normative
**Version:** 1
**Related:** [`../product.md`](../product.md), [`../capabilities.md`](../capabilities.md), [`catalog-schema.md`](catalog-schema.md), [`../testing/evidence-model.md`](../testing/evidence-model.md)

The structured source is [`workflows.toml`](workflows.toml), with one record
per end-to-end user workflow. A workflow is broader than a crate: it crosses
entry point, state, commands, persistence, notifications, failure/recovery,
security, and evidence.

Every workflow record states preconditions, canonical entry point, state
transitions, command IDs, persistence, notifications, failure modes, security
boundary, tests, visual evidence, and current status. `implemented` is not a
substitute for `integration-tested` or `visually-verified`.

Every record also names a repository-relative `next_task`. This is the bounded
queue item that owns the next evidence or implementation step; prose in
`next_action` must not be the only pointer to unfinished work.
