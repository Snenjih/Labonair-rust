# Settings and Data Lifecycle

**Status:** Normative
**Version:** 1
**Related:** [`../settings.md`](../settings.md), [`../settings-inventory.md`](../settings-inventory.md), [`../architecture/runtime-lifecycle.md`](../architecture/runtime-lifecycle.md)

The lifecycle for a persisted value is:

```text
input → validation → layered in-memory state → persistence → reload
       → migration/unknown-key handling → deletion or retention decision
```

## Ownership

| Data | Owner | Rules |
|---|---|---|
| Settings values and schema | `labonair-settings` / settings content | Values only; typed scopes and defaults. |
| Settings UI/navigation | `labonair-settings-ui` | Renders values and validation; no feature ownership. |
| Host metadata | Hosts | SQLite/domain ownership; no raw secret payload. |
| Credentials/key material | Credentials + Secrets | References and keychain/secret boundary. |
| Workspace/session/layout | Workspace | Versioned snapshots, migration, and recovery. |
| Notifications | Notifications | In-memory retained lifecycle; no settings projection. |
| Themes/keymap/transfers | Their owning modules | No parallel Settings management category. |

## Change requirements

Every new or changed value records:

- stable key and scope (`app`, `workspace`, `host`, or `session`);
- default and validation rule;
- storage file/table and serialization format;
- load/save/reload and malformed-input behavior;
- migration and unknown-key decision;
- live-reload versus restart behavior;
- secret classification and redaction rule;
- owning consumer, UI entry point, tests, and evidence.

Delete dead fields or attach a named removal task. A parser accepting a field
is not evidence that the field still belongs in the product.
