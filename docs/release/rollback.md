# Release Rollback

**Status:** Normative
**Version:** 1
**Related:** [`README.md`](README.md), [`../RELEASE.md`](../RELEASE.md), [`../automation/limits.md`](../automation/limits.md)

The macOS updater verifies a minisign signature before applying an artifact
and atomically swaps the app bundle with rollback on failure. An empty or
invalid public key/signature must fail closed and never apply an update.

For a failed release:

1. stop distribution of the affected artifact/manifest;
2. retain logs and artifact digests without secrets;
3. restore the previous signed bundle or publish the last known-good manifest;
4. verify launch, settings/session recovery, and updater state;
5. record the incident and next release gate in the release evidence record.
