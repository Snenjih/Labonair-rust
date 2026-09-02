# Changelog

All notable changes to **Labonair-rust** — the pure-Rust / GPUI hard fork of
Labonair. Format follows [Keep a Changelog](https://keepachangelog.com/);
versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Added
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
- Auto-update is check-only until T15-005.
- No packaged Linux release yet (builds from source).
