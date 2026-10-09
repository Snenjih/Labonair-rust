#!/usr/bin/env python3
"""Validate the structured AI-agent request router."""

from __future__ import annotations

import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "agents" / "routes.toml"
SCOPES = {
    "docs", "queue", "architecture", "capabilities", "surfaces", "workflows",
    "settings", "performance", "visual", "evidence", "automation", "release", "knowledge", "rust", "all",
}
REQUIRED = {"id", "display_name", "request_kinds", "first_reads", "canonical_sources", "required_scopes", "evidence_decision"}


def paths_exist(errors: list[str], label: str, values: object) -> None:
    if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
        errors.append(f"{label} must be a non-empty string array")
        return
    for value in values:
        if not (ROOT / value).exists():
            errors.append(f"{label}: missing repository path {value}")


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"agent route check failed to load source: {error}", file=sys.stderr)
        return 1
    errors: list[str] = []
    if data.get("version") != 1:
        errors.append("agent route source must declare version = 1")
    for key in ("guide", "matrix"):
        value = data.get(key)
        if not isinstance(value, str) or not (ROOT / value).exists():
            errors.append(f"agent route source has invalid {key}: {value!r}")
    if not isinstance(data.get("generated_view"), str):
        errors.append("agent route source must declare generated_view")
    routes = data.get("route", [])
    seen: set[str] = set()
    for route in routes:
        identifier = route.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate agent route id: {identifier}")
        seen.add(identifier)
        errors.extend(f"{identifier}: missing {field}" for field in sorted(REQUIRED - set(route)))
        if not isinstance(route.get("request_kinds"), list) or not route["request_kinds"] or not all(isinstance(value, str) for value in route["request_kinds"]):
            errors.append(f"{identifier}.request_kinds must be a non-empty string array")
        paths_exist(errors, f"{identifier}.first_reads", route.get("first_reads"))
        paths_exist(errors, f"{identifier}.canonical_sources", route.get("canonical_sources"))
        scopes = route.get("required_scopes")
        if not isinstance(scopes, list) or not scopes or not all(scope in SCOPES for scope in scopes):
            errors.append(f"{identifier}.required_scopes must contain only known verifier scopes")
        for field in ("display_name", "evidence_decision"):
            if not isinstance(route.get(field), str) or not route[field].strip():
                errors.append(f"{identifier}.{field} must be a non-empty string")
    if len(routes) < 8:
        errors.append("agent route source must cover at least eight request classes")
    if errors:
        print("agent route check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"agent route check OK — {len(routes)} request routes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
