#!/usr/bin/env python3
"""Validate the structured documentation reform scorecard."""

from __future__ import annotations

from datetime import date
from pathlib import Path
import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "evidence" / "scorecard.toml"
ALLOWED = {"Verified", "Partial", "Pending", "N/A"}
REQUIRED = {"id", "area", "target", "owner", "status", "evidence", "source_refs", "last_verified", "limitation", "next_action"}


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"scorecard check failed to load source: {error}", file=sys.stderr)
        return 1
    errors: list[str] = []
    if data.get("version") != 1:
        errors.append("scorecard must declare version = 1")
    if set(data.get("status_vocabulary", [])) != ALLOWED:
        errors.append("scorecard status_vocabulary must be exactly Verified/Partial/Pending/N/A")
    seen: set[str] = set()
    items = data.get("item", [])
    for item in items:
        identifier = item.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate scorecard id: {identifier}")
        seen.add(identifier)
        missing = sorted(REQUIRED - set(item))
        errors.extend(f"{identifier}: missing {field}" for field in missing)
        if item.get("status") not in ALLOWED:
            errors.append(f"{identifier}: unsupported status {item.get('status')!r}")
        try:
            date.fromisoformat(item.get("last_verified"))
        except (TypeError, ValueError):
            errors.append(f"{identifier}: last_verified must be an ISO date")
        for field in ("evidence", "source_refs"):
            values = item.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
                errors.append(f"{identifier}: {field} must be a non-empty string array")
        for field in ("area", "target", "owner", "limitation", "next_action"):
            if not isinstance(item.get(field), str) or not item[field].strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        for reference in item.get("source_refs", []):
            if not (ROOT / reference).exists():
                errors.append(f"{identifier}: missing source reference {reference}")
    if not items:
        errors.append("scorecard must contain at least one item")
    if errors:
        print("scorecard check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"scorecard check OK — {len(items)} items, {sum(item['status'] == 'Pending' for item in items)} pending")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
