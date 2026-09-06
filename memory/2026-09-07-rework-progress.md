# Rework progress — 2026-09-07

## Native visual verification

The exact Rust bundle was opened through its absolute `.app` path. Core
Graphics confirmed a visible layer-0 window for the exact bundled Rust PID;
the macOS screenshot API then denied capture because Screen Recording access
is unavailable in the runner. The legacy Tauri app was not used.

`scripts/screenshot.sh` now validates caller-provided PIDs against the exact
Rust executable before window lookup and distinguishes an invalid process from
macOS Screen Recording denial. This keeps visual evidence fail-closed.
