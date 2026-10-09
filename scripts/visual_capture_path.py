#!/usr/bin/env python3
"""Resolve one canonical visual-capture artifact path."""

from __future__ import annotations

import argparse
from pathlib import Path
import re
import sys
import tomllib


ROOT = Path(__file__).resolve().parents[1]
SURFACES = ROOT / "docs" / "product" / "surfaces.toml"
EVIDENCE = ROOT / "docs" / "testing" / "visual-evidence.toml"
COMMIT_RE = re.compile(r"^[0-9a-f]{7,64}$")


def slug(value: str) -> str:
    result = re.sub(r"[^a-z0-9]+", "-", value.lower()).strip("-")
    return result or "state"


def fail(message: str) -> int:
    print(f"visual capture path: {message}", file=sys.stderr)
    return 2


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--surface", required=True, help="canonical surface ID")
    parser.add_argument("--state", required=True, help="exact catalog state label")
    parser.add_argument("--commit", required=True, help="lowercase commit identifier")
    parser.add_argument("--platform", default="macos")
    parser.add_argument("--viewport", help="optional viewport; must match the catalog state")
    args = parser.parse_args()

    try:
        surface_data = tomllib.loads(SURFACES.read_text(encoding="utf-8"))
        evidence_data = tomllib.loads(EVIDENCE.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        return fail(f"cannot load canonical sources: {error}")

    surface = next((item for item in surface_data.get("surface", []) if item.get("id") == args.surface), None)
    if surface is None:
        return fail(f"unknown surface {args.surface!r}")
    states = surface.get("visual_states", [])
    if args.state not in states:
        return fail(f"state {args.state!r} is not declared for {args.surface!r}")

    if args.platform != evidence_data.get("default_platform"):
        return fail(f"platform must be {evidence_data.get('default_platform')!r}")
    if not COMMIT_RE.fullmatch(args.commit):
        return fail("commit must be a lowercase hexadecimal identifier")

    expected_viewport = "narrow" if args.state.strip().lower() == "narrow window" else evidence_data.get("default_viewport")
    if args.viewport is not None and args.viewport != expected_viewport:
        return fail(f"state {args.state!r} requires viewport {expected_viewport!r}")

    pattern = evidence_data.get("artifact_name_pattern")
    artifact_root = evidence_data.get("artifact_root")
    if not isinstance(pattern, str) or not isinstance(artifact_root, str):
        return fail("visual-evidence.toml has no usable artifact pattern")
    artifact = pattern.format(
        surface_id=slug(args.surface),
        state_slug=slug(args.state),
        viewport=expected_viewport,
        platform=args.platform,
        commit=args.commit,
    )
    print(f"{artifact_root}/{artifact}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
