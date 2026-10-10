# Packaging Contract

**Status:** Normative
**Version:** 1
**Related:** [`README.md`](README.md), [`../RELEASE.md`](../RELEASE.md), [`../testing/evidence-model.md`](../testing/evidence-model.md)

The package source of truth is `crates/app/Cargo.toml` for the version and
`scripts/package-macos.sh` for the native bundle. The release path is:

```text
cargo build --release -p labonair
→ Labonair.app (binary, Info.plist, icon, compiled resources)
→ optional signing/notarization
→ optional DMG and signed updater artifact
→ smoke test and artifact verification
```

Never commit signing keys, notarization profiles, updater private keys, or
bearer tokens. Package evidence records the exact target, artifact digest,
smoke command, signing result, and any intentional unsigned state.

The macOS bundle registers the `labonair` URL scheme for Settings field links;
keep that declaration in `packaging/macos/Info.plist` aligned with the native
open-URL handler in `crates/app` and the route contract in `docs/settings.md`.
