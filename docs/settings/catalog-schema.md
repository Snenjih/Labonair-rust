# Settings Catalog Schema

**Status:** Normative
**Version:** 1
**Related:** [`persistence.md`](persistence.md), [`../settings-inventory.md`](../settings-inventory.md), [`../../crates/settings-ui/src/schema.rs`](../../crates/settings-ui/src/schema.rs)

The structured settings catalog records ownership and data-lifecycle policy
for every top-level settings area. The native Settings field registry remains
the source of truth for the exact visible field paths; the checker compares
that registry with the catalog so a field cannot silently disappear from the
documentation route.

## Source roles

- `docs/settings/catalog.toml` owns area-level lifecycle, scope, storage,
  migration, security, test, and evidence metadata.
- `crates/settings-ui/src/schema.rs` owns the exact visible field registry and
  type-shaped controls.
- `docs/settings-inventory.md` owns the field-by-field keep/move/remove
  decisions and consumer explanations.
- `crates/settings-content/src/settings_content.rs` owns the typed value
  tree.

The generated view at [`../generated/settings.md`](../generated/settings.md)
combines these sources. It is not an additional authority.

## Required area fields

Every area record has a stable ID, real `SettingsContent` target module,
owner/canonical crate, supported scopes, storage format, migration behavior,
reload behavior, secret classification, entry point, source files, tests,
evidence, and current status.

Run the check with:

```text
python3 scripts/verify.py --scope settings
```
