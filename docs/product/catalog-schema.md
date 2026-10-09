# Product Catalog Schemas

**Status:** Normative
**Version:** 1
**Related:** [`../registries.md`](../registries.md), [`../architecture.md`](../architecture.md), [`../capabilities.md`](../capabilities.md), [`commands.toml`](commands.toml), [`surfaces.toml`](surfaces.toml), [`menu.toml`](menu.toml)

The product catalog makes visible actions and surfaces addressable by stable
IDs. Rust remains the source of stable `CommandId` variants; the catalog adds
ownership, entry points, availability, failure behavior, and evidence.

## Command records

`docs/product/commands.toml` groups commands that share one owner and runtime
contract. Every ID in `CommandId` must appear exactly once. A group records the
same fields required for each contained action: owner module, canonical crate,
menu path, context, availability, default keymap, palette visibility,
execution handler, success/failure behavior, notification behavior, undo/redo
semantics, tests, and visual evidence.

## Surface records

`docs/product/surfaces.toml` contains one record per canonical titlebar,
workspace, dock, statusbar, overlay, dialog, and standalone-tool surface. Each
record has a stable ID, owner, kind, entry points, supported context, command
IDs, settings, notifications, visual states, source files, and evidence.

## Menu records

`docs/product/menu.toml` is the single menu model. A menu branch may reference
only cataloged command IDs and must identify its owner and canonical surface.
The global menu and the command palette are complementary entry points; they
must not create duplicate command identities or parallel feature registries.

Run the scoped catalog check with:

```text
python3 scripts/verify.py --scope surfaces
```

The generated views are under `docs/generated/`. They are derived artifacts
and must not be edited directly.

End-to-end workflows use [`workflows.toml`](workflows.toml) and are checked by
`python3 scripts/verify.py --scope workflows`.
