#!/usr/bin/env python3
"""Validate the structured performance budgets and measurement evidence."""

from __future__ import annotations

from datetime import date
import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "performance" / "budgets.toml"
ALLOWED = {"Verified", "Partial", "Pending", "N/A"}
REQUIRED = {
    "id", "area", "owner", "unit", "target", "measurement", "environment",
    "status", "evidence", "source_refs", "limitation", "next_action", "last_verified",
}


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"performance catalog check failed to load source: {error}", file=sys.stderr)
        return 1
    errors: list[str] = []
    if data.get("version") != 1:
        errors.append("performance catalog must declare version = 1")
    if set(data.get("status_vocabulary", [])) != ALLOWED:
        errors.append("performance status_vocabulary must be exactly Verified/Partial/Pending/N/A")
    seen: set[str] = set()
    budgets = data.get("budget", [])
    for budget in budgets:
        identifier = budget.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate performance budget id: {identifier}")
        seen.add(identifier)
        errors.extend(f"{identifier}: missing {field}" for field in sorted(REQUIRED - set(budget)))
        if budget.get("status") not in ALLOWED:
            errors.append(f"{identifier}: unsupported status {budget.get('status')!r}")
        try:
            date.fromisoformat(budget.get("last_verified"))
        except (TypeError, ValueError):
            errors.append(f"{identifier}: last_verified must be an ISO date")
        for field in ("evidence", "source_refs"):
            values = budget.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
                errors.append(f"{identifier}: {field} must be a non-empty string array")
        for field in ("area", "owner", "unit", "target", "measurement", "environment", "limitation", "next_action"):
            if not isinstance(budget.get(field), str) or not budget[field].strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        for reference in budget.get("source_refs", []):
            if not (ROOT / reference).exists():
                errors.append(f"{identifier}: missing source reference {reference}")
        if budget.get("status") == "Verified" and any("Pending" in value for value in budget.get("evidence", [])):
            errors.append(f"{identifier}: Verified budget cannot cite Pending evidence")
    if not budgets:
        errors.append("performance catalog must contain at least one budget")
    if errors:
        print("performance catalog check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"performance catalog check OK — {len(budgets)} budgets, {sum(b['status'] == 'Pending' for b in budgets)} pending")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
