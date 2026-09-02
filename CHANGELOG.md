# Changelog

All notable changes to **Labonair-rust** — the pure-Rust / GPUI hard fork of
Labonair. Format follows [Keep a Changelog](https://keepachangelog.com/);
versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Added
- **Auto-updater — macOS (T15-005).**
  - `labonair_backend::updater` gained `fetch_manifest` / `download_update`
    (streamed, with progress) / `verify_update` (minisign Ed25519, pre-hashed —
    empty key or signature is a hard failure) / `apply_macos_update` (atomic
    `.app` swap with rollback) / `relaunch`, plus a 6 h auto-check backoff.
  - `labonair_ui::updater::UpdaterView` — native GPUI update dialog
    (available / downloading + progress / ready), a startup background check
    gated on the `checkForUpdates` preference, and a **Check for Updates…**
    entry in the app menu and the command palette. Failures surface as toasts.
  - `scripts/package-macos.sh` now emits `Labonair_<version>_<arch>.app.tar.gz`
    + a filled `latest.json`, signing the tarball with minisign when
    `LABONAIR_UPDATER_KEY` is set; `.github/workflows/release.yml` uploads both.
  - Decision (Sparkle vs. custom) and signing setup documented in
    `docs/RELEASE.md`.
- **Packaging & release foundation (T15-004).**
  - `scripts/package-macos.sh` — assembles a self-contained `Labonair.app`
    from a `--release` build (`Info.plist`, `AppIcon.icns`, version from
    `crates/app/Cargo.toml`), with opt-in code signing (hardened runtime +
    entitlements), `.dmg` creation and `notarytool` notarization.
  - `packaging/macos/` — `Info.plist` template, `Labonair.entitlements`,
    app icon.
  - `labonair_backend::updater` — Tauri-compatible `latest.json` manifest
    types, platform target key, and a dependency-free `SemVer` version check
    (`UpdateManifest::available()`). Endpoint:
    `github.com/Snenjih/Labonair-rust/releases/latest/download/latest.json`.
    (Download/verify/apply + UI is T15-005.)
  - `scripts/smoke-test.sh` + `crates/app/tests/smoke.rs` — end-to-end release
    verification: build bundle, structurally validate it, then exercise
    backend init, a real PTY shell round-trip, and the update check.
  - `docs/RELEASE.md` — build/sign/notarize procedure, artifacts, Linux
    perspective, known limitations vs. the original app.
  - `docs/LICENSES.md` — full dependency-tree license audit (result: clear;
    GPUI 0.2.2 is Apache-2.0, no strong-copyleft dependency).

### Known limitations vs. the original app
- No in-app web/URL preview (GPUI has no WebView) — native markdown +
  "open in browser" instead.
- macOS / Linux only, no Windows.
- Auto-update is macOS-only; Linux update path is still TODO.
- No packaged Linux release yet (builds from source).
