#!/usr/bin/env python3
"""Exercise the deterministic visual-capture path helper for every catalog state."""

from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
HELPER = ROOT / "scripts" / "visual_capture_path.py"
SURFACES = ROOT / "docs" / "product" / "surfaces.toml"
COMMIT = "0" * 40


def run(*args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(HELPER), *args],
        cwd=ROOT,
        check=False,
        capture_output=True,
        text=True,
    )


def expected(surface: str, state: str, viewport: str) -> str:
    import re

    def slug(value: str) -> str:
        return re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-") or "state"

    return (
        "artifacts/visual/"
        f"{slug(surface)}--{slug(state)}--{viewport}--macos--{COMMIT}.png"
    )


def main() -> int:
    catalog = tomllib.loads(SURFACES.read_text(encoding="utf-8"))
    errors: list[str] = []
    states = 0
    for surface in catalog.get("surface", []):
        surface_id = surface["id"]
        for state in surface.get("visual_states", []):
            states += 1
            viewport = "narrow" if state.lower() == "narrow window" else "standard"
            result = run(
                "--surface",
                surface_id,
                "--state",
                state,
                "--commit",
                COMMIT,
            )
            actual = result.stdout.strip()
            if result.returncode != 0 or actual != expected(surface_id, state, viewport):
                errors.append(
                    f"{surface_id}::{state}: expected {expected(surface_id, state, viewport)!r}, "
                    f"got exit={result.returncode} output={actual!r}"
                )

    invalid_cases = (
        ("--surface", "unknown.surface", "--state", "home", "--commit", COMMIT),
        ("--surface", "workspace.active", "--state", "unknown", "--commit", COMMIT),
        (
            "--surface",
            "workspace.docks",
            "--state",
            "narrow window",
            "--commit",
            COMMIT,
            "--viewport",
            "standard",
        ),
        ("--surface", "workspace.active", "--state", "home", "--commit", "GIT-SHA"),
        (
            "--surface",
            "workspace.active",
            "--state",
            "home",
            "--commit",
            COMMIT,
            "--platform",
            "linux",
        ),
    )
    for args in invalid_cases:
        result = run(*args)
        if result.returncode == 0:
            errors.append(f"invalid input unexpectedly accepted: {args!r}")

    if errors:
        print("visual capture path check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"visual capture path check OK — {states} catalog states and {len(invalid_cases)} rejection cases")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
