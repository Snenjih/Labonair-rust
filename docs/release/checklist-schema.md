# Release Checklist Schema

**Status:** Normative
**Version:** 1
**Related:** [`README.md`](README.md), [`platforms.md`](platforms.md), [`packaging.md`](packaging.md), [`rollback.md`](rollback.md), [`support-policy.md`](support-policy.md), [`../RELEASE.md`](../RELEASE.md)

The release checklist is the machine-readable operational view of the native
release contract. It records which checks are reproducible in the repository,
which require a supported macOS host or configured signing secrets, and which
evidence is still pending.

`Verified` means the repository has current, repeatable evidence for the
record. `Conditional` means the procedure exists but depends on a platform,
credential, or release configuration. `Pending` means a required artifact has
not yet been captured. A successful build alone never verifies packaging,
signing, rollback, or support readiness.

Run the check with:

```text
python3 scripts/verify.py --scope release
```
