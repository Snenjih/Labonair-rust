#!/usr/bin/env python3
"""Validate the machine-readable Settings ownership and lifecycle catalog."""

from __future__ import annotations

import re
from pathlib import Path
import sys
import tomllib

from architecture_model import ROOT, source_fingerprint


SOURCE = ROOT / "docs" / "settings" / "catalog.toml"
AREAS_SOURCE = ROOT / "crates" / "settings-content" / "src" / "areas.rs"
CONTENT_SOURCE = ROOT / "crates" / "settings-content" / "src" / "settings_content.rs"
FIELD_SOURCE = ROOT / "crates" / "settings-ui" / "src" / "schema.rs"
STATUS_VALUES = {"planned", "contract-defined", "implemented", "integration-tested", "partial", "deferred"}
REQUIRED_AREA_FIELDS = {
    "id", "display_name", "target_module", "owner_module", "canonical_crate",
    "scopes", "storage", "migration", "reload", "secret_classification",
    "entry_point", "source_files", "tests", "evidence", "status",
}
FIELD_CALL = re.compile(
    r"\bfield(?:_with_details)?!\(\s*([A-Za-z_][A-Za-z0-9_]*)\.([A-Za-z_][A-Za-z0-9_]*),\s*\"([^\"]+)\""
)
AREA_META = re.compile(
    r"key:\s*\"([^\"]+)\".*?target_module:\s*\"([^\"]+)\"", re.DOTALL
)
CONTENT_FIELDS = re.compile(r"^\s*pub\s+([a-z_][a-z0-9_]*)\s*:\s*[A-Za-z_][A-Za-z0-9_]*Content,", re.MULTILINE)


def existing_paths(errors: list[str], label: str, values: object) -> None:
    if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
        errors.append(f"{label} must be a non-empty string array")
        return
    for value in values:
        if not (ROOT / value).exists():
            errors.append(f"{label}: missing repository path {value}")


def main() -> int:
    errors: list[str] = []
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
        areas_text = AREAS_SOURCE.read_text(encoding="utf-8")
        content_text = CONTENT_SOURCE.read_text(encoding="utf-8")
        field_text = FIELD_SOURCE.read_text(encoding="utf-8")
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"settings catalog check failed to load source: {error}", file=sys.stderr)
        return 1

    if data.get("version") != 1:
        errors.append("settings catalog must declare version = 1")
    for key in ("content_source", "field_registry_source", "areas_source", "inventory_source"):
        value = data.get(key)
        if not isinstance(value, str) or not (ROOT / value).exists():
            errors.append(f"settings catalog has invalid {key}: {value!r}")
    if not isinstance(data.get("generated_view"), str):
        errors.append(f"settings catalog has invalid generated_view: {data.get('generated_view')!r}")

    declared_area_meta = dict(AREA_META.findall(areas_text))
    declared_content = set(CONTENT_FIELDS.findall(content_text))
    if set(declared_area_meta) != declared_content:
        errors.append(
            "Settings AREAS and SettingsContent top-level modules differ: "
            f"areas={sorted(declared_area_meta)}, content={sorted(declared_content)}"
        )

    fields: dict[str, str] = {}
    for area, rust_field, json_field in FIELD_CALL.findall(field_text):
        path = f"{area}.{json_field}"
        if path in fields:
            errors.append(f"settings field registry contains duplicate path {path}")
        fields[path] = rust_field
        if area not in declared_area_meta:
            errors.append(f"settings field {path} uses unknown area {area}")
    if not fields:
        errors.append("settings field registry contains no field! entries")

    areas = data.get("area", [])
    seen: set[str] = set()
    for area in areas:
        identifier = area.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate settings area id: {identifier}")
        seen.add(identifier)
        missing = sorted(REQUIRED_AREA_FIELDS - set(area))
        errors.extend(f"{identifier}: missing {field}" for field in missing)
        if identifier not in declared_area_meta:
            errors.append(f"{identifier}: not present in native Settings AREAS")
        elif area.get("target_module") != declared_area_meta[identifier]:
            errors.append(
                f"{identifier}: target_module {area.get('target_module')!r} does not match "
                f"native target {declared_area_meta[identifier]!r}"
            )
        if area.get("owner_module") != "settings" or area.get("canonical_crate") != "labonair-settings":
            errors.append(f"{identifier}: Settings areas must be owned by labonair/labonair-settings")
        if area.get("status") not in STATUS_VALUES:
            errors.append(f"{identifier}: unsupported status {area.get('status')!r}")
        if not isinstance(area.get("scopes"), list) or not area["scopes"]:
            errors.append(f"{identifier}: scopes must be a non-empty array")
        for key in ("display_name", "target_module", "storage", "migration", "reload",
                    "secret_classification", "entry_point"):
            if not isinstance(area.get(key), str) or not area[key].strip():
                errors.append(f"{identifier}: {key} must be a non-empty string")
        existing_paths(errors, f"{identifier}.source_files", area.get("source_files"))
        existing_paths(errors, f"{identifier}.tests", area.get("tests"))
        existing_paths(errors, f"{identifier}.evidence", area.get("evidence"))

    if set(seen) != set(declared_area_meta):
        errors.append(
            "catalog areas must exactly match native Settings AREAS: "
            f"catalog={sorted(seen)}, native={sorted(declared_area_meta)}"
        )
    forbidden = {"hosts", "themes", "icon-themes", "keymap", "notifications", "transfers"}
    if forbidden.intersection(seen):
        errors.append("settings catalog must not create capability-management areas")

    if errors:
        print("settings catalog check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    fingerprint = source_fingerprint([SOURCE, AREAS_SOURCE, CONTENT_SOURCE, FIELD_SOURCE, ROOT / data["inventory_source"]])
    print(f"settings catalog check OK — {len(areas)} areas, {len(fields)} registered UI fields, source {fingerprint}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
