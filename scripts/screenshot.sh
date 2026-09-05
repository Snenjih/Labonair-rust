#!/usr/bin/env bash
# Capture the running Labonair window to a PNG so an AI agent can examine the
# actual rendered UI (closes the "headless, user visual check open" gap).
#
# Usage: scripts/screenshot.sh [out.png]
#   out.png defaults to shots/labonair.png (dir auto-created).
#
# Depends on macOS Screen Recording permission for the calling terminal app.

set -euo pipefail

OUT="${1:-shots/labonair.png}"
mkdir -p "$(dirname "$OUT")"

# Bring Labonair's main window to the front.
osascript -e 'tell application "System Events" to set frontmost of (first process whose name is "labonair") to true' 2>/dev/null \
  || osascript -e 'tell application "Labonair" to activate' 2>/dev/null || true
sleep 0.5

# Find the CGWindowID of the frontmost on-screen Labonair window (layer 0).
WID=$(swift - <<'EOF' 2>/dev/null | head -1
import CoreGraphics
import Foundation
let opts = CGWindowListOption([.optionOnScreenOnly, .excludeDesktopElements])
if let list = CGWindowListCopyWindowInfo(opts, kCGNullWindowID) as? [[String: Any]] {
    for w in list {
        let owner = w[kCGWindowOwnerName as String] as? String ?? ""
        if owner.lowercased().contains("labonair") {
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
    screencapture -x -l "$WID" -t png "$OUT"
else
    echo "window id not found; falling back to main-display capture" >&2
    screencapture -x -m -t png "$OUT"
fi

echo "saved: $OUT"