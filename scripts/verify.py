#!/usr/bin/env python3
"""Run Labonair's canonical local verification scopes in deterministic order."""

from __future__ import annotations

import argparse
from pathlib import Path
import subprocess
import sys


ROOT = Path(__file__).resolve().parents[1]

SCOPES: dict[str, tuple[tuple[str, ...], ...]] = {
    "docs": (
        ("python3", "scripts/check_documentation.py"),
    ),
    "agents": (
        ("python3", "scripts/check_agent_routes.py"),
        ("python3", "scripts/gen_agent_index.py", "--check"),
    ),
    "queue": (
        ("python3", "scripts/check_rework_queue.py"),
    ),
    "architecture": (
        ("python3", "scripts/check_architecture.py"),
        ("scripts/check-crate-deps.sh",),
        ("python3", "scripts/gen_architecture.py", "--check"),
    ),
    "capabilities": (
        ("python3", "scripts/check_capabilities.py"),
        ("python3", "scripts/check_capability_coverage.py"),
        ("python3", "scripts/gen_capabilities.py", "--check"),
        ("python3", "scripts/gen_capability_coverage.py", "--check"),
    ),
    "surfaces": (
        ("python3", "scripts/check_product_catalog.py"),
        ("python3", "scripts/gen_product_catalog.py", "--check"),
    ),
    "workflows": (
        ("python3", "scripts/check_workflows.py"),
        ("python3", "scripts/gen_workflows.py", "--check"),
    ),
    "settings": (
        ("python3", "scripts/check_settings.py"),
        ("python3", "scripts/gen_settings.py", "--check"),
    ),
    "performance": (
        ("python3", "scripts/check_performance.py"),
        ("python3", "scripts/gen_performance.py", "--check"),
    ),
    "release": (
        ("python3", "scripts/check_release.py"),
        ("python3", "scripts/gen_release.py", "--check"),
    ),
    "visual": (
        ("python3", "scripts/check_visual_evidence.py"),
        ("python3", "scripts/gen_visual_evidence.py", "--check"),
    ),
    "evidence": (
        ("python3", "scripts/check_scorecard.py"),
        ("python3", "scripts/gen_scorecard.py", "--check"),
    ),
    "automation": (
        ("python3", "scripts/check_automation.py"),
        ("python3", "scripts/gen_automation.py", "--check"),
    ),
    "rust": (
        ("cargo", "fmt", "--all", "--", "--check"),
        ("cargo", "check", "--workspace", "--all-targets"),
        ("cargo", "clippy", "--workspace", "--all-targets", "--", "-D", "warnings"),
        ("cargo", "test", "--workspace"),
    ),
}
SCOPES["knowledge"] = (
    *SCOPES["docs"],
    *SCOPES["agents"],
    *SCOPES["queue"],
    *SCOPES["architecture"],
    *SCOPES["capabilities"],
    *SCOPES["surfaces"],
    *SCOPES["workflows"],
    *SCOPES["settings"],
    *SCOPES["performance"],
    *SCOPES["release"],
    *SCOPES["visual"],
    *SCOPES["evidence"],
    *SCOPES["automation"],
)


def run_scope(scope: str) -> int:
    commands = SCOPES[scope]
    for command in commands:
        rendered = " ".join(command)
        print(f"==> {scope}: {rendered}", flush=True)
        result = subprocess.run(command, cwd=ROOT, check=False)
        if result.returncode != 0:
            print(f"FAIL {scope}: {rendered} (exit {result.returncode})", file=sys.stderr)
            return result.returncode
    print(f"PASS {scope}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--scope",
        choices=("docs", "agents", "queue", "architecture", "capabilities", "surfaces", "workflows", "settings", "performance", "release", "visual", "evidence", "automation", "knowledge", "rust", "all"),
        default="all",
        help="verification scope to run (default: all)",
    )
    args = parser.parse_args()

    scopes = ("knowledge", "rust") if args.scope == "all" else (args.scope,)
    for scope in scopes:
        status = run_scope(scope)
        if status != 0:
            return status
    print(f"PASS verify --scope {args.scope}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
