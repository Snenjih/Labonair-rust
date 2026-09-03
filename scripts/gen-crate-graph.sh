#!/usr/bin/env bash
#
# gen-crate-graph.sh — regenerate the "Ist-Graph" artefacts for
# docs/architecture.md §9 from the live workspace:
#
#   docs/assets/crate-graph.dot   Graphviz source
#   docs/assets/crate-graph.svg   self-contained tiered SVG (committed)
#
# and print the Markdown adjacency list that goes into docs/architecture.md §9.
#
# If Graphviz (`dot`) is installed the SVG is rendered from the .dot for a
# nicer layout; otherwise the built-in tiered renderer is used (no external
# dependency — CI only runs scripts/check-crate-deps.sh, not this).
#
set -euo pipefail

cd "$(dirname "$0")/.."

if ! command -v cargo >/dev/null 2>&1; then
	echo "gen-crate-graph: cargo not found on PATH" >&2
	exit 2
fi

META="$(cargo metadata --format-version 1 --no-deps)"

echo "$META" | python3 "$(dirname "$0")/gen_crate_graph.py"

if command -v dot >/dev/null 2>&1; then
	dot -Tsvg docs/assets/crate-graph.dot -o docs/assets/crate-graph.svg
	echo "# SVG rendered with Graphviz."
else
	echo "# Graphviz not installed — kept the built-in tiered SVG."
fi
