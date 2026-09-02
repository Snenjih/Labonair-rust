#!/usr/bin/env bash
#
# Build a distributable macOS .app bundle (and optionally a .dmg) for Labonair.
#
# There is no `tauri bundle` equivalent for a GPUI app, so the bundle is
# assembled by hand from a release build of the `labonair` binary + the static
# assets in packaging/macos/. Fonts and Tree-sitter grammars are compiled into
# the binary (see crates/theme/src/fonts.rs and crates/editor/Cargo.toml), so
# the bundle is fully self-contained — nothing extra to copy.
#
# Usage:
#   scripts/package-macos.sh [--dmg] [--target <triple>]
#
# Environment:
#   LABONAIR_SIGN_IDENTITY   codesign identity ("Developer ID Application: …" or
#                            "-" for ad-hoc). Unset → no signing.
#   LABONAIR_NOTARY_PROFILE  `xcrun notarytool` keychain profile name. Set (with
#                            a Developer ID signature) → notarize + staple.
#
# See docs/RELEASE.md for the full release procedure.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

DMG=0
CARGO_TARGET_ARGS=()
TARGET_TRIPLE=""
while [[ $# -gt 0 ]]; do
	case "$1" in
		--dmg) DMG=1; shift ;;
		--target) TARGET_TRIPLE="$2"; CARGO_TARGET_ARGS=(--target "$2"); shift 2 ;;
		*) echo "unknown argument: $1" >&2; exit 2 ;;
	esac
done

# --- version (single source of truth: crates/app/Cargo.toml) ------------------
VERSION="$(awk -F'"' '/^\[package\]/{p=1} p && /^version[[:space:]]*=/{print $2; exit}' crates/app/Cargo.toml)"
[[ -n "$VERSION" ]] || { echo "could not read version from crates/app/Cargo.toml" >&2; exit 1; }
BUILD="$(git rev-list --count HEAD 2>/dev/null || echo 1)"
echo "==> Labonair $VERSION (build $BUILD)"

# --- release build -----------------------------------------------------------
echo "==> cargo build --release -p labonair ${CARGO_TARGET_ARGS[*]:-}"
cargo build --release -p labonair ${CARGO_TARGET_ARGS[@]+"${CARGO_TARGET_ARGS[@]}"}

if [[ -n "$TARGET_TRIPLE" ]]; then
	BIN="target/$TARGET_TRIPLE/release/labonair"
else
	BIN="target/release/labonair"
fi
[[ -x "$BIN" ]] || { echo "build produced no binary at $BIN" >&2; exit 1; }

# --- assemble the bundle ----------------------------------------------------
APP="target/release/bundle/macos/Labonair.app"
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/labonair"
cp packaging/macos/AppIcon.icns "$APP/Contents/Resources/AppIcon.icns"
sed -e "s/__VERSION__/$VERSION/" -e "s/__BUILD__/$BUILD/" \
	packaging/macos/Info.plist > "$APP/Contents/Info.plist"
printf 'APPL????' > "$APP/Contents/PkgInfo"

plutil -lint "$APP/Contents/Info.plist"
echo "==> bundle assembled: $APP"

# --- optional signing -------------------------------------------------------
if [[ -n "${LABONAIR_SIGN_IDENTITY:-}" ]]; then
	echo "==> codesign ($LABONAIR_SIGN_IDENTITY)"
	codesign --force --deep --options runtime --timestamp \
		--entitlements packaging/macos/Labonair.entitlements \
		--sign "$LABONAIR_SIGN_IDENTITY" "$APP"
	codesign --verify --deep --strict --verbose=2 "$APP"
else
	echo "==> skipping codesign (LABONAIR_SIGN_IDENTITY unset)"
fi

# --- optional dmg ---------------------------------------------------------
DMG_PATH="target/release/bundle/macos/Labonair_${VERSION}_$(uname -m).dmg"
if [[ $DMG -eq 1 ]]; then
	echo "==> building dmg"
	rm -f "$DMG_PATH"
	STAGE="$(mktemp -d)"
	cp -R "$APP" "$STAGE/"
	ln -s /Applications "$STAGE/Applications"
	hdiutil create -volname "Labonair" -srcfolder "$STAGE" -ov -format UDZO "$DMG_PATH"
	rm -rf "$STAGE"
	[[ -n "${LABONAIR_SIGN_IDENTITY:-}" ]] && codesign --force --sign "$LABONAIR_SIGN_IDENTITY" "$DMG_PATH"
	echo "==> dmg: $DMG_PATH"
fi

# --- optional notarization ------------------------------------------------
if [[ -n "${LABONAIR_NOTARY_PROFILE:-}" && -n "${LABONAIR_SIGN_IDENTITY:-}" ]]; then
	TO_NOTARIZE="$APP"
	[[ $DMG -eq 1 ]] && TO_NOTARIZE="$DMG_PATH"
	echo "==> notarize $TO_NOTARIZE"
	if [[ "$TO_NOTARIZE" == *.app ]]; then
		ZIP="target/release/bundle/macos/Labonair_notarize.zip"
		ditto -c -k --keepParent "$APP" "$ZIP"
		xcrun notarytool submit "$ZIP" --keychain-profile "$LABONAIR_NOTARY_PROFILE" --wait
		xcrun stapler staple "$APP"
	else
		xcrun notarytool submit "$TO_NOTARIZE" --keychain-profile "$LABONAIR_NOTARY_PROFILE" --wait
		xcrun stapler staple "$TO_NOTARIZE"
	fi
else
	echo "==> skipping notarization (needs LABONAIR_NOTARY_PROFILE + LABONAIR_SIGN_IDENTITY)"
fi

echo "==> done: $APP"
