# Release & Packaging

How Labonair-rust is built into a distributable artifact. macOS is the primary
target; Linux is prepared but not yet a first-class release (see below).

Unlike the original Tauri app there is no `tauri bundle`; the bundle is
assembled by a script from a plain `--release` build.

## Version — single source of truth

The version lives in **`crates/app/Cargo.toml`** (`[package] version`). Every
other place derives from it:

- the binary exposes it as `CARGO_PKG_VERSION`
  (`labonair_backend::CURRENT_VERSION`),
- `scripts/package-macos.sh` reads it into `CFBundleShortVersionString`,
- `CFBundleVersion` is set to the commit count (`git rev-list --count HEAD`).

Bump it there, add a `CHANGELOG.md` section, tag `vX.Y.Z`.

## macOS build

Prerequisites: Xcode CLT + the Metal Toolchain
(`xcodebuild -downloadComponent MetalToolchain`), stable Rust.

```sh
scripts/package-macos.sh            # -> target/release/bundle/macos/Labonair.app
scripts/package-macos.sh --dmg      # also builds Labonair_<version>_<arch>.dmg
```

The script:

1. `cargo build --release -p labonair`,
2. creates `Labonair.app/Contents/{MacOS,Resources}`,
3. copies the binary, `AppIcon.icns`, a version-substituted `Info.plist`
   (from `packaging/macos/Info.plist`) and `PkgInfo`,
4. `plutil -lint`s the plist,
5. optionally signs, builds a dmg, and notarizes (see below).

### What's in the bundle

Just the binary + `Info.plist` + icon. **Fonts** (Inter, JetBrains Mono) and
**Tree-sitter grammars** are compiled *into* the binary
(`crates/theme/src/fonts.rs` via `include_bytes!`, the `tree-sitter-*` crates
are statically linked), so there are no loose resources to lose — the warning
in the task about missing bundled resources does not apply to this
architecture. A release build that is missing a grammar simply would not link.

### Universal binary (optional)

```sh
scripts/package-macos.sh --target aarch64-apple-darwin
scripts/package-macos.sh --target x86_64-apple-darwin
lipo -create -output labonair-universal \
  target/aarch64-apple-darwin/release/labonair \
  target/x86_64-apple-darwin/release/labonair
```

## Code signing & notarization (macOS)

Optional — **never blocks CI**. Without the env vars the script produces an
unsigned bundle (Gatekeeper: right-click → Open, or `xattr -dr
com.apple.quarantine`).

| Env var | Meaning |
|---|---|
| `LABONAIR_SIGN_IDENTITY` | `"Developer ID Application: NAME (TEAMID)"`, or `"-"` for ad-hoc |
| `LABONAIR_NOTARY_PROFILE` | `xcrun notarytool` keychain profile (set up once with `notarytool store-credentials`) |

```sh
xcrun notarytool store-credentials labonair-notary \
  --apple-id you@example.com --team-id TEAMID --password <app-specific-pw>

LABONAIR_SIGN_IDENTITY="Developer ID Application: … (TEAMID)" \
LABONAIR_NOTARY_PROFILE=labonair-notary \
  scripts/package-macos.sh --dmg
```

Signing uses `--options runtime` (hardened runtime) with
`packaging/macos/Labonair.entitlements` (keychain access group
`com.labonair.app`, sandbox disabled — needed for terminal/editor filesystem
access, network client for SSH/SFTP). Notarization zips the `.app` (or signs
the dmg), submits with `--wait`, then staples.

## Linux (prepared, not yet released)

The binary builds on Linux (`cargo build --release -p labonair`, GPUI Vulkan
renderer, needs Vulkan loader + `libxkbcommon`, `wayland`/`xcb`, `fontconfig`,
`openssl` dev libs). Packaging is intentionally not automated yet; when it is,
the intended path is an **AppImage** (bundle the binary + a `.desktop` file +
`packaging/macos/icon.png` re-used as the app icon) and/or a **Flatpak**
manifest. Keep platform packaging behind `scripts/package-<os>.sh` so the
release workflow stays a per-OS switch.

## Auto-update (T15-005)

**Decision — custom minimal updater, not Sparkle.** Sparkle would need an
`objc2`/framework binding or a Swift shim plus its own signed-appcast tooling;
the app already publishes a Tauri-shaped `latest.json` (T15-004) and Tauri's
updater used **minisign** signatures. Reusing that format keeps the release
pipeline unchanged and avoids a second signing system, so the port reimplements
the same four steps natively (this is also the approach Zed's auto-updater
takes).

`labonair_backend::updater` (`crates/backend/src/modules/updater/`):

- **Manifest** — Tauri-compatible `latest.json` at `DEFAULT_UPDATE_ENDPOINT`
  (`…/releases/latest/download/latest.json`). Shape: `{ version, notes,
  pub_date, platforms: { "<os>-<arch>": { url, signature } } }`. `signature` is
  the base64 of the whole `.minisig` file.
- **Check** — `fetch_manifest()` GETs it; `UpdateManifest::available()` returns
  an `AvailableUpdate` only for a strictly-newer `SemVer` *with* an artifact for
  this `UPDATE_TARGET`. Auto-cadence: `CHECK_INTERVAL` (6 h), tracked in
  `~/.config/labonair/updater-last-check`.
- **Download** — `download_update()` streams the artifact with progress
  callbacks.
- **Verify** — `verify_update()` checks the minisign (Ed25519, pre-hashed)
  signature against `UPDATE_PUBLIC_KEY`. An empty key or empty signature is a
  hard failure — the update is **never** applied unverified.
- **Apply** — `apply_macos_update()` unpacks the `<name>.app.tar.gz` and
  atomically swaps the running `.app` bundle (moves the old one aside, rolls
  back on failure); `relaunch()` re-opens it via `open` and exits.

UI: `crates/ui/src/updater.rs` (`UpdaterView`) — the native GPUI dialog
(available / downloading / ready), auto-check at startup when the
`checkForUpdates` preference is on, and a **Check for Updates…** entry in the
app menu and command palette. Failures go through the notification system.

### Signing the update artifact

1. Generate a minisign keypair once: `minisign -G -p updater.pub -s updater.key`.
2. Put the **public** key's second line into
   `crates/backend/src/modules/updater/install.rs::UPDATE_PUBLIC_KEY`.
3. Store the **secret** key + its password as the CI secrets
   `LABONAIR_UPDATER_PRIVATE_KEY` / `LABONAIR_UPDATER_KEY_PASSWORD`.

`scripts/package-macos.sh` then emits `Labonair_<version>_<arch>.app.tar.gz`,
signs it (`<tarball>.minisig`) and writes `latest.json` with the base64
signature inline. Until step 2 is done, `UPDATE_PUBLIC_KEY` is empty and the
in-app updater refuses every update (safe default).

> A universal `latest.json` needs both `darwin-aarch64` and `darwin-x86_64`
> entries; the per-arch runs each produce a single-arch manifest — merge them
> (or run the arm64 build last and hand-add the x86_64 block) before upload.

## Distributed artifacts

| Platform | Artifact | Notes |
|---|---|---|
| macOS (Apple Silicon) | `Labonair_<version>_arm64.dmg` | primary |
| macOS (Intel) | `Labonair_<version>_x86_64.dmg` | via `--target` |
| macOS | `Labonair_<version>_<arch>.app.tar.gz` (+ `.minisig`) | auto-update payload |
| macOS | `latest.json` | update manifest |
| Linux | — | prepared, not released (see above) |

## Known limitations vs. the original app

- **No in-app web/URL preview** — GPUI cannot embed a WebView. Replaced by
  native markdown rendering + "open in system browser".
- **macOS / Linux only** — no Windows.
- Auto-update: macOS only. Linux update (AppImage/Flatpak/repo) is still TODO.
- Linux has no packaged release yet (builds from source).

## Verifying a release

```sh
scripts/smoke-test.sh        # builds the bundle, structurally verifies it,
                             # runs `cargo test -p labonair --test smoke`
LABONAIR_SMOKE_LAUNCH=1 scripts/smoke-test.sh   # also opens the app for 5s
```
