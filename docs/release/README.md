# Release Documentation

**Status:** Normative index
**Version:** 1

The machine-readable release source is [`checklist.toml`](checklist.toml), its
contract is [`checklist-schema.md`](checklist-schema.md), and the generated
operational view is [`../generated/release.md`](../generated/release.md).
Run `python3 scripts/verify.py --scope release` before a release handoff.

The current release authority is [`../RELEASE.md`](../RELEASE.md). This
directory organizes the operational contract without creating a second version
source:

- [`platforms.md`](platforms.md) — supported/prepared platform policy;
- [`packaging.md`](packaging.md) — native bundle and artifact flow;
- [`rollback.md`](rollback.md) — updater rollback and recovery;
- [`support-policy.md`](support-policy.md) — release/support evidence and limits.
- [`checklist-schema.md`](checklist-schema.md) — structured release-check contract.
- [`checklist.toml`](checklist.toml) — package, signing, updater, rollback, and support checks.
