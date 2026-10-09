# Agent Route Schema

**Status:** Normative
**Version:** 1
**Related:** [`README.md`](README.md), [`change-matrix.md`](change-matrix.md), [`../documentation-governance.md`](../documentation-governance.md)

The structured route source maps a request class to the minimum canonical
documents and verification scopes an agent must use. It complements the
human-readable change matrix; it does not authorize work outside the active
queue or override normative contracts.

Each route has a stable ID, request kinds, first-read documents, canonical
source documents, required verifier scopes, and an evidence decision. All
repository paths and scopes are checked by `scripts/check_agent_routes.py`.

Run the check with:

```text
python3 scripts/verify.py --scope agents
```
