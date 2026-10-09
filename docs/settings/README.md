# Settings Documentation

**Status:** Normative index
**Version:** 1

Settings are values and schemas. Capability behavior, hosts, themes, keymaps,
notifications, transfers, and command registration remain owned by their
modules.

- [`../settings.md`](../settings.md) — boundary and value ownership.
- [`../settings-guidelines.md`](../settings-guidelines.md) — UI navigation and field rules.
- [`../settings-inventory.md`](../settings-inventory.md) — current field decisions.
- [`persistence.md`](persistence.md) — data lifecycle and migration contract.
- [`catalog-schema.md`](catalog-schema.md) — machine-readable lifecycle contract.
- [`catalog.toml`](catalog.toml) — structured area ownership and evidence source.

The generated field/lifecycle view is [`../generated/settings.md`](../generated/settings.md).
Validate it with `python3 scripts/verify.py --scope settings`.

New setting records must identify scope, default, storage, migration, live
reload/restart behavior, consumer, and secret classification.
