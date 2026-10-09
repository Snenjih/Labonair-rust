#!/usr/bin/env python3
"""Validate structured capability descriptors and their source references."""

from __future__ import annotations

from datetime import date
from pathlib import Path
import sys
import tomllib

from architecture_model import ROOT, load_manifest


DESCRIPTOR_DIR = ROOT / "docs" / "capabilities"
STATUS_VALUES = {
    "planned", "contract-defined", "implemented", "integration-tested",
    "visually-verified", "security-reviewed", "performance-baselined",
    "regression-locked", "partial", "deferred", "removed",
}
REQUIRED_FIELDS = {
    "id", "display_name", "owner_module", "canonical_crate", "sibling_crates",
    "public_contract", "canonical_entry_points", "alternate_entry_points",
    "commands", "default_keymap", "registries", "events", "state_owner",
    "persistence_owner", "settings", "notifications", "ui_kit_components",
    "automation_tools", "security_boundaries", "source_files", "tests",
    "visual_states", "performance_budget", "current_status", "target_status",
    "known_limitations", "removal_condition", "last_verified",
}
LIST_FIELDS = {
    "sibling_crates", "public_contract", "canonical_entry_points",
    "alternate_entry_points", "commands", "default_keymap", "registries",
    "events", "settings", "notifications", "ui_kit_components",
    "automation_tools", "security_boundaries", "source_files", "tests",
    "visual_states", "known_limitations",
}
EVIDENCE_FIELDS = {
    "contract", "unit", "integration", "command", "surface", "persistence",
    "notification", "visual", "security", "performance", "regression",
}


def main() -> int:
    errors: list[str] = []
    try:
        manifest = load_manifest()
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"capability check failed to load architecture manifest: {error}", file=sys.stderr)
        return 1

    crate_owners = {
        crate["name"]: crate["owner"] for crate in manifest.get("crate", [])
    }
    descriptor_paths = sorted(DESCRIPTOR_DIR.glob("*.toml"))
    descriptors = []
    for path in descriptor_paths:
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError):
            descriptors.append(path)
            continue
        if "capability" in data:
            descriptors.append(path)
    if not descriptors:
        errors.append("no capability descriptors found under docs/capabilities")

    seen_ids: set[str] = set()
    for path in descriptors:
        relative = path.relative_to(ROOT)
        try:
            data = tomllib.loads(path.read_text(encoding="utf-8"))
        except (OSError, tomllib.TOMLDecodeError) as error:
            errors.append(f"{relative}: cannot parse TOML: {error}")
            continue
        if "coverage" in data:
            continue
        capability = data.get("capability")
        if not isinstance(capability, dict):
            errors.append(f"{relative}: missing [capability] table")
            continue

        missing = sorted(REQUIRED_FIELDS - set(capability))
        for field in missing:
            errors.append(f"{relative}: missing required field {field}")
        identifier = capability.get("id")
        if not isinstance(identifier, str) or not identifier.strip():
            errors.append(f"{relative}: id must be a non-empty string")
            identifier = relative.stem
        elif identifier in seen_ids:
            errors.append(f"{relative}: duplicate capability id {identifier}")
        else:
            seen_ids.add(identifier)
        if identifier != relative.stem:
            errors.append(f"{relative}: file name must match capability id {identifier}")

        for field in LIST_FIELDS:
            value = capability.get(field)
            if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
                errors.append(f"{relative}: {field} must be an array of strings")
        for field in ("display_name", "owner_module", "canonical_crate", "state_owner",
                      "persistence_owner", "performance_budget", "removal_condition"):
            if not isinstance(capability.get(field), str) or not capability[field].strip():
                errors.append(f"{relative}: {field} must be a non-empty string")

        canonical = capability.get("canonical_crate")
        if canonical not in crate_owners:
            errors.append(f"{relative}: canonical crate is not in architecture graph: {canonical}")
        elif crate_owners[canonical] != capability.get("owner_module"):
            errors.append(
                f"{relative}: canonical crate owner {crate_owners[canonical]!r} does not "
                f"match owner_module {capability.get('owner_module')!r}"
            )
        for sibling in capability.get("sibling_crates", []):
            if sibling not in crate_owners:
                errors.append(f"{relative}: sibling crate is not in architecture graph: {sibling}")
            elif crate_owners[sibling] != capability.get("owner_module"):
                errors.append(
                    f"{relative}: sibling crate {sibling} belongs to {crate_owners[sibling]!r}, "
                    f"not {capability.get('owner_module')!r}"
                )

        for source in capability.get("source_files", []):
            if not (ROOT / source).exists():
                errors.append(f"{relative}: source reference does not exist: {source}")

        for field in ("current_status", "target_status"):
            if capability.get(field) not in STATUS_VALUES:
                errors.append(f"{relative}: unsupported {field}: {capability.get(field)!r}")
        verified = capability.get("last_verified")
        try:
            date.fromisoformat(verified)
        except (TypeError, ValueError):
            errors.append(f"{relative}: last_verified must be an ISO date")

        evidence = data.get("evidence")
        if not isinstance(evidence, dict):
            errors.append(f"{relative}: missing [evidence] table")
        else:
            for field in sorted(EVIDENCE_FIELDS):
                value = evidence.get(field)
                if not isinstance(value, list) or not all(isinstance(item, str) for item in value):
                    errors.append(f"{relative}: evidence.{field} must be an array of strings")

    if errors:
        print("capability descriptor check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"capability descriptor check OK — {len(descriptors)} descriptors, {len(seen_ids)} unique ids")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
