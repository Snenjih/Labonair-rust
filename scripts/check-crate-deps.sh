#!/usr/bin/env bash
#
# check-crate-deps.sh — mechanical guard for the crate dependency rules in
# docs/architecture.md §3 (and the §8.4 amendments).
#
# It parses `cargo metadata` for the workspace-internal edges only (deps whose
# package name starts with `labonair`) and enforces:
#
#   * a per-crate ALLOW-LIST of workspace deps — any workspace dep that is not
#     on the list for that crate fails the build with a message citing the rule;
#   * the graph is acyclic (rule 8);
#   * transitive "must-not-reach" invariants — e.g. no `labonair-panel-*` may
#     reach `labonair-shell` or another `labonair-panel-*` even indirectly
#     (docs/architecture.md §3 warning), and the engine crates
#     (`backend`/`ai`/`editor`) may not reach any UI crate.
#
# `cargo metadata` only reports *direct* deps, so the transitive checks build
# the graph and traverse it here.
#
# Exit 0 = graph matches the architecture doc. Exit 1 = a forbidden edge.
#
# Used by CI (.github/workflows/ci.yml) and runnable locally:
#     scripts/check-crate-deps.sh
#
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
	echo "check-crate-deps: cargo not found on PATH" >&2
	exit 2
fi

cargo metadata --format-version 1 --no-deps | python3 "$(dirname "$0")/check_crate_deps.py" "$@"
