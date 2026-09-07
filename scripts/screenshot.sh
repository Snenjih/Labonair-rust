#!/usr/bin/env bash
# Capture the running Labonair window to a PNG so an AI agent can examine the
# actual rendered UI. Missing windows are an error: a screenshot of another
# application is never valid visual evidence for Labonair.
#
# Usage: scripts/screenshot.sh [out.png] [rust_pid]
#   out.png defaults to shots/labonair.png (dir auto-created).
#   rust_pid can also be provided through LABONAIR_RUST_PID.
#
# Depends on macOS Screen Recording permission for the calling terminal app.

set -euo pipefail

OUT="${1:-shots/labonair.png}"
mkdir -p "$(dirname "$OUT")"

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RUST_BINARY="$REPO_ROOT/target/debug/labonair"
RUST_BUNDLE_BINARY="$REPO_ROOT/target/release/bundle/macos/Labonair.app/Contents/MacOS/labonair"
RUST_PID="${2:-${LABONAIR_RUST_PID:-}}"

# Never activate or match by the generic application name: the legacy Tauri
# app is installed as /Applications/Labonair.app and has the same owner name.
if [ -z "$RUST_PID" ]; then
    RUST_PID=$(pgrep -f -- '(^|/)(target/debug/labonair|target/release/bundle/macos/Labonair\.app/Contents/MacOS/labonair)$' | head -1 || true)
fi

if [ -z "$RUST_PID" ]; then
    echo "Rust Labonair process not found: $RUST_BINARY or $RUST_BUNDLE_BINARY" >&2
    exit 1
fi

# A caller-provided PID is still validated. This prevents an accidental
# screenshot of an unrelated process (especially the legacy Tauri app) from
# being accepted as Labonair evidence.
RUST_COMMAND=$(ps -p "$RUST_PID" -o command= 2>/dev/null | \
    sed -e 's/^[[:space:]]*//' -e 's/[[:space:]]*$//' || true)
# `cargo run` can expose the executable relative to its working directory,
# while a packaged launch normally reports an absolute path. Normalize only
# the executable token; arguments are not part of the identity check.
RUST_COMMAND_PATH="${RUST_COMMAND%%[[:space:]]*}"
case "$RUST_COMMAND_PATH" in
    /*) ;;
    *) RUST_COMMAND_PATH="$REPO_ROOT/$RUST_COMMAND_PATH" ;;
esac
case "$RUST_COMMAND_PATH" in
    "$RUST_BINARY"|"$RUST_BUNDLE_BINARY") ;;
    *)
        echo "PID $RUST_PID is not the native Rust Labonair executable: ${RUST_COMMAND:-process unavailable}" >&2
        exit 1
        ;;
esac

# Find the CGWindowID of the frontmost on-screen Labonair window (layer 0).
WID=$(swift - "$RUST_PID" - <<'EOF' 2>/dev/null | head -1
import CoreGraphics
import Foundation
let targetPid = Int(CommandLine.arguments[1]) ?? -1
let opts = CGWindowListOption([.optionOnScreenOnly, .excludeDesktopElements])
if let list = CGWindowListCopyWindowInfo(opts, kCGNullWindowID) as? [[String: Any]] {
    for w in list {
        let ownerPid = w[kCGWindowOwnerPID as String] as? Int ?? -1
        if ownerPid == targetPid {
            let layer = w[kCGWindowLayer as String] as? Int ?? 99
            if layer == 0, let n = w[kCGWindowNumber as String] as? Int {
                print(n)
                break
            }
        }
    }
}
EOF
)

if [ -n "$WID" ]; then
    if ! screencapture -x -l "$WID" -t png "$OUT"; then
        echo "Rust Labonair window $WID was found for PID $RUST_PID, but macOS denied capture." >&2
        echo "Grant Screen Recording permission to the terminal or runner, then retry." >&2
        exit 1
    fi
else
    echo "Labonair window id not found; visual evidence was not captured" >&2
    exit 1
fi

echo "saved: $OUT"
