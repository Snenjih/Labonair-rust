#!/usr/bin/env bash
# Launch the exact packaged Rust executable and capture one native window.
#
# Usage:
#   scripts/capture-macos-native.sh artifacts/visual/<state>.png
#
# The wrapper deliberately does not infer a surface or state. The caller must
# inspect the PNG and add a durable capture record to visual-evidence.toml only
# when the rendered state is truthful.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP="$REPO_ROOT/target/release/bundle/macos/Labonair.app"
BUNDLE_BINARY="$APP/Contents/MacOS/labonair"
OUT="${1:?usage: scripts/capture-macos-native.sh <output.png>}"

[[ -x "$BUNDLE_BINARY" ]] || {
	echo "native bundle executable not found: $BUNDLE_BINARY" >&2
	exit 1
}

mkdir -p "$(dirname "$OUT")"

native_process_pids() {
	ps -axo pid=,command= | awk -v target="$BUNDLE_BINARY" '$2 == target {print $1}'
}

before_pids="$(native_process_pids)"
open -n "$APP" >/tmp/labonair-rust-native-capture-open.log 2>&1 &
open_pid=$!
rust_pid=""

for _ in {1..80}; do
	while IFS= read -r pid; do
		[[ -n "$pid" ]] || continue
		if printf '%s\n' "$before_pids" | grep -Fqx "$pid"; then
			continue
		fi
		rust_pid="$pid"
		break 2
	done < <(native_process_pids)
	sleep 0.25
done

cleanup() {
	if [[ -n "$rust_pid" ]]; then
		kill "$rust_pid" 2>/dev/null || true
		wait "$rust_pid" 2>/dev/null || true
	fi
	wait "$open_pid" 2>/dev/null || true
}
trap cleanup EXIT INT TERM

if [[ -z "$rust_pid" ]]; then
	echo "the packaged Rust Labonair process did not appear" >&2
	sed -n '1,120p' /tmp/labonair-rust-native-capture-open.log >&2 || true
	exit 1
fi

# Give GPUI and the window server time to create the layer-0 window before the
# fail-closed screenshot helper resolves the exact native PID.
sleep 2
"$REPO_ROOT/scripts/screenshot.sh" "$OUT" "$rust_pid"

printf 'native_binary=%s\npid=%s\nartifact=%s\n' "$BUNDLE_BINARY" "$rust_pid" "$OUT"
