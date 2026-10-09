#!/usr/bin/env python3
"""Validate traceability for every documentation-reform definition-of-done item."""

from __future__ import annotations

from datetime import date
from pathlib import Path
import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "reform" / "requirements.toml"
ALLOWED = {"Verified", "Partial", "Pending", "N/A"}
REQUIRED = {
    "id",
    "plan_section",
    "requirement",
    "owner",
    "status",
    "evidence",
    "source_refs",
    "last_verified",
    "limitation",
    "next_task",
}
EXPECTED_IDS = {
    "dod.agent-route",
    "dod.canonical-index",
    "dod.capability-ownership",
    "dod.capability-descriptors",
    "dod.architecture-graph",
    "dod.visible-action-contract",
    "dod.menu-surface-checks",
    "dod.settings-persistence",
    "dod.automation-security",
    "dod.status-separation",
    "dod.workflow-evidence",
    "dod.generated-freshness",
    "dod.canonical-verifier",
    "dod.ci-gates",
    "dod.queue-order",
    "dod.compatibility-removal",
    "dod.no-duplicate-surface",
}


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"reform coverage check failed to load source: {error}", file=sys.stderr)
        return 1

    errors: list[str] = []
    if data.get("version") != 1:
        errors.append("reform coverage must declare version = 1")
    if set(data.get("status_vocabulary", [])) != ALLOWED:
        errors.append("reform coverage status_vocabulary must be exactly Verified/Partial/Pending/N/A")

    seen: set[str] = set()
    requirements = data.get("requirement", [])
    for record in requirements:
        identifier = record.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate reform requirement id: {identifier}")
        seen.add(identifier)
        errors.extend(f"{identifier}: missing {field}" for field in sorted(REQUIRED - set(record)))
        if record.get("status") not in ALLOWED:
            errors.append(f"{identifier}: unsupported status {record.get('status')!r}")
        try:
            date.fromisoformat(record.get("last_verified"))
        except (TypeError, ValueError):
            errors.append(f"{identifier}: last_verified must be an ISO date")
        for field in ("evidence", "source_refs"):
            values = record.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) and value.strip() for value in values):
                errors.append(f"{identifier}: {field} must be a non-empty string array")
        for field in ("plan_section", "requirement", "owner", "limitation", "next_task"):
            if not isinstance(record.get(field), str) or not record[field].strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        for reference in record.get("source_refs", []):
            if not (ROOT / reference).exists():
                errors.append(f"{identifier}: missing source reference {reference}")
        next_task = record.get("next_task")
        if not isinstance(next_task, str) or not (ROOT / next_task).is_file():
            errors.append(f"{identifier}: missing bounded next task {next_task!r}")

    missing = sorted(EXPECTED_IDS - seen)
    extra = sorted(seen - EXPECTED_IDS)
    if missing or extra:
        errors.append(f"definition-of-done coverage mismatch; missing={missing}, extra={extra}")
    if len(requirements) != len(EXPECTED_IDS):
        errors.append(f"definition-of-done coverage must contain exactly {len(EXPECTED_IDS)} records")

    if errors:
        print("reform coverage check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(
        "reform coverage check OK — "
        f"{len(requirements)} definition-of-done records, "
        f"{sum(record['status'] == 'Verified' for record in requirements)} verified, "
        f"{sum(record['status'] in {'Partial', 'Pending'} for record in requirements)} open"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
