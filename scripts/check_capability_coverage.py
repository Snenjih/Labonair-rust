#!/usr/bin/env python3
"""Validate that every capability-matrix row has structured coverage."""

from __future__ import annotations

from datetime import date
import re
import sys
import tomllib

from architecture_model import ROOT, load_manifest


SOURCE = ROOT / "docs" / "capabilities" / "coverage.toml"
MATRIX = ROOT / "docs" / "capabilities.md"
STATUSES = {"matrix-tracked", "descriptor-pilot", "descriptor-defined", "deferred"}


def matrix_names() -> set[str]:
    names: set[str] = set()
    in_table = False
    for line in MATRIX.read_text(encoding="utf-8").splitlines():
        if line.startswith("| Capability |"):
            in_table = True
            continue
        if in_table and not line.startswith("|"):
            break
        if in_table and not line.startswith("|---"):
            match = re.match(r"\|\s*([^|]+?)\s*\|", line)
            if match:
                names.add(match.group(1))
    return names


def main() -> int:
    errors: list[str] = []
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
        manifest = load_manifest()
        expected = matrix_names()
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"capability coverage check failed to load inputs: {error}", file=sys.stderr)
        return 1
    owners = {crate["name"]: crate["owner"] for crate in manifest.get("crate", [])}
    entries = data.get("coverage", {}).get("capability", [])
    seen_names: set[str] = set()
    seen_ids: set[str] = set()
    for entry in entries:
        identifier = entry.get("id", "<unnamed>")
        display = entry.get("display_name", "<unnamed>")
        if display in seen_names:
            errors.append(f"{identifier}: duplicate display name {display}")
        seen_names.add(display)
        if identifier in seen_ids:
            errors.append(f"{identifier}: duplicate coverage id")
        seen_ids.add(identifier)
        for field in ("id", "display_name", "owner_module", "architecture_owner", "canonical_crate", "descriptor_file", "coverage_status", "canonical_entry_point", "next_action", "last_verified"):
            if field not in entry:
                errors.append(f"{identifier}: missing {field}")
        if entry.get("canonical_crate") not in owners:
            errors.append(f"{identifier}: unknown canonical crate {entry.get('canonical_crate')}")
        elif owners[entry["canonical_crate"]] != entry.get("architecture_owner"):
            errors.append(f"{identifier}: architecture_owner does not match canonical crate owner")
        if entry.get("coverage_status") not in STATUSES:
            errors.append(f"{identifier}: unsupported coverage status {entry.get('coverage_status')!r}")
        try:
            date.fromisoformat(entry.get("last_verified"))
        except (TypeError, ValueError):
            errors.append(f"{identifier}: last_verified must be an ISO date")
        descriptor = entry.get("descriptor_file")
        if descriptor != "none" and not (SOURCE.parent / descriptor).is_file():
            errors.append(f"{identifier}: missing descriptor file {descriptor}")

    if seen_names != expected:
        errors.append(f"coverage/matrix mismatch; missing={sorted(expected - seen_names)}, extra={sorted(seen_names - expected)}")
    if errors:
        print("capability coverage check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"capability coverage check OK — {len(entries)} matrix rows tracked")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
