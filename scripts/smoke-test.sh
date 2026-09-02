#!/usr/bin/env bash
#
# End-to-end release smoke test (T15-004):
#   1. build the macOS .app bundle,
#   2. structurally verify it (binary, Info.plist, icon, version),
#   3. run the core-functionality smoke test against the release code
#      (`cargo test -p labonair --test smoke`: backend init, PTY shell round-trip,
#       update-manifest check).
#
# Note: launching the GUI itself needs a logged-in window server, so the
# executable-launch check is opt-in via LABONAIR_SMOKE_LAUNCH=1 (it opens the
# app for 5s then quits it).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

APP="target/release/bundle/macos/Labonair.app"

echo "### 1. build bundle"
scripts/package-macos.sh

echo "### 2. verify bundle structure"
fail=0
check() { if eval "$2"; then echo "  ok   $1"; else echo "  FAIL $1"; fail=1; fi; }
check "bundle dir exists"        "[[ -d '$APP' ]]"
check "executable present"       "[[ -x '$APP/Contents/MacOS/labonair' ]]"
check "Info.plist present"       "[[ -f '$APP/Contents/Info.plist' ]]"
check "Info.plist lints"         "plutil -lint '$APP/Contents/Info.plist' >/dev/null"
check "icon present"             "[[ -f '$APP/Contents/Resources/AppIcon.icns' ]]"
check "PkgInfo present"          "[[ -f '$APP/Contents/PkgInfo' ]]"
VERS="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist")"
check "version substituted"      "[[ '$VERS' != '__VERSION__' && -n '$VERS' ]]"
check "identifier correct"       "[[ \"\$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' '$APP/Contents/Info.plist')\" == 'com.labonair.app' ]]"
if [[ -n "${LABONAIR_SIGN_IDENTITY:-}" ]]; then
	check "signature valid"          "codesign --verify --deep --strict '$APP'"
fi
[[ $fail -eq 0 ]] || { echo "bundle verification failed"; exit 1; }
echo "  bundle: $APP ($VERS)"

echo "### 3. core functionality smoke test"
cargo test -p labonair --test smoke

if [[ "${LABONAIR_SMOKE_LAUNCH:-0}" == "1" ]]; then
	echo "### 4. launch bundle for 5s"
	open -W --new -a "$APP" &
	OPEN_PID=$!
	sleep 5
	osascript -e 'tell application "Labonair" to quit' || killall labonair || true
	wait "$OPEN_PID" 2>/dev/null || true
	echo "  launched and quit cleanly"
fi

echo "### smoke test passed"
