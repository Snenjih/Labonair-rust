#!/usr/bin/env python3
"""Validate the structured native release checklist."""

from __future__ import annotations

from datetime import date
import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "release" / "checklist.toml"
ALLOWED = {"Verified", "Conditional", "Pending", "N/A"}
REQUIRED = {"id", "area", "owner", "command", "status", "evidence", "source_refs", "limitation", "next_action", "last_verified"}


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"release checklist check failed to load source: {error}", file=sys.stderr)
        return 1
    errors: list[str] = []
    if data.get("version") != 1:
        errors.append("release checklist must declare version = 1")
    if set(data.get("status_vocabulary", [])) != ALLOWED:
        errors.append("release status_vocabulary must be exactly Verified/Conditional/Pending/N/A")
    checks = data.get("check", [])
    seen: set[str] = set()
    for check in checks:
        identifier = check.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate release check id: {identifier}")
        seen.add(identifier)
        errors.extend(f"{identifier}: missing {field}" for field in sorted(REQUIRED - set(check)))
        if check.get("status") not in ALLOWED:
            errors.append(f"{identifier}: unsupported status {check.get('status')!r}")
        try:
            date.fromisoformat(check.get("last_verified"))
        except (TypeError, ValueError):
            errors.append(f"{identifier}: last_verified must be an ISO date")
        for field in ("evidence", "source_refs"):
            values = check.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
                errors.append(f"{identifier}: {field} must be a non-empty string array")
        for field in ("area", "owner", "command", "limitation", "next_action"):
            if not isinstance(check.get(field), str) or not check[field].strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        for reference in check.get("source_refs", []):
            if not (ROOT / reference).exists():
                errors.append(f"{identifier}: missing source reference {reference}")
    if not checks:
        errors.append("release checklist must contain at least one check")
    if errors:
        print("release checklist check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"release checklist check OK — {len(checks)} checks, {sum(c['status'] == 'Pending' for c in checks)} pending")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
