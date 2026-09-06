#!/usr/bin/env bash
#
# End-to-end release smoke test (T15-004):
#   1. build the macOS .app bundle,
#   2. structurally verify it (binary, Info.plist, icon, version),
#   3. run the core-functionality smoke test against the release code
#      (`cargo test -p labonair --test smoke`: backend init, PTY shell round-trip,
#       update-manifest check).
#
# Note: launching the GUI itself needs a logged-in window server and a working
# macOS AppKit/LaunchServices session, so the executable-launch check is opt-in
# via LABONAIR_SMOKE_LAUNCH=1 (it opens the exact bundled Rust app for 5s).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

APP="$REPO_ROOT/target/release/bundle/macos/Labonair.app"

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
check "identifier correct"       "[[ \"\$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' '$APP/Contents/Info.plist')\" == 'com.labonair.rust' ]]"
if [[ -n "${LABONAIR_SIGN_IDENTITY:-}" ]]; then
	check "signature valid"          "codesign --verify --deep --strict '$APP'"
fi
[[ $fail -eq 0 ]] || { echo "bundle verification failed"; exit 1; }
echo "  bundle: $APP ($VERS)"

echo "### 3. core functionality smoke test"
cargo test -p labonair --test smoke

if [[ "${LABONAIR_SMOKE_LAUNCH:-0}" == "1" ]]; then
	echo "### 4. launch bundle for 5s"
	# Open the bundle by its absolute path. Do not use `open -a Labonair`: the
	# legacy Tauri app has the same display name and is commonly installed in
	# /Applications.
	BUNDLE_BINARY="$APP/Contents/MacOS/labonair"
	native_process_pids() {
		ps -axo pid=,command= | awk -v target="$BUNDLE_BINARY" '$2 == target {print $1}'
	}
	BEFORE_PIDS="$(native_process_pids)"
	open -n -W "$APP" >/tmp/labonair-rust-smoke-open.log 2>&1 &
	OPEN_PID=$!
	RUST_PID=""
	for _ in {1..60}; do
		for pid in $(native_process_pids); do
			case " $BEFORE_PIDS " in
				*" $pid "*) ;;
				*) RUST_PID="$pid"; break 2 ;;
		esac
		done
		sleep 0.25
	done
	if [[ -z "$RUST_PID" ]]; then
		echo "  FAIL native Rust process was not found after opening the bundle" >&2
		kill "$OPEN_PID" 2>/dev/null || true
		exit 1
	fi
	native_process_alive() {
		local state
		state="$(ps -p "$RUST_PID" -o state= 2>/dev/null | tr -d '[:space:]')"
		[[ -n "$state" && "$state" != Z* ]]
	}
	sleep 1
	if ! native_process_alive; then
		echo "  FAIL native Rust process exited during launch" >&2
		sed -n '1,80p' /tmp/labonair-rust-smoke-open.log >&2 || true
		exit 1
	fi
	sleep 4
	if ! native_process_alive; then
		echo "  FAIL native Rust process did not survive the launch interval" >&2
		sed -n '1,80p' /tmp/labonair-rust-smoke-open.log >&2 || true
		kill "$OPEN_PID" 2>/dev/null || true
		exit 1
	fi
	kill "$RUST_PID" 2>/dev/null || true
	wait "$RUST_PID" 2>/dev/null || true
	wait "$OPEN_PID" 2>/dev/null || true
	echo "  launched and quit cleanly"
fi

echo "### smoke test passed"
