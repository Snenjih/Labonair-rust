#!/usr/bin/env python3
"""Validate the structured native visual-evidence registry."""

from __future__ import annotations

from datetime import date
from pathlib import Path
import re
import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "testing" / "visual-evidence.toml"
SURFACES = ROOT / "docs" / "product" / "surfaces.toml"
ALLOWED = {"Verified", "Partial", "Pending", "N/A"}
REQUIRED_ROOT = {
    "version",
    "surface_source",
    "matrix_source",
    "artifact_root",
    "artifact_name_pattern",
    "native_binary_rule",
    "capture_helper",
    "default_platform",
    "default_viewport",
    "environment_status",
    "environment_blocker",
    "last_verified",
}
REQUIRED_SURFACE = {"id", "state_status", "next_action"}
REQUIRED_CAPTURE = {
    "surface_id",
    "state",
    "status",
    "artifact",
    "platform",
    "viewport",
    "binary",
    "pid",
    "window",
    "captured_at",
}
PLACEHOLDER_RE = re.compile(r"\{([a-z_]+)\}")
SAFE_RELATIVE = re.compile(r"^[^/][^\n]*$")


def load(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def derived_status(statuses: set[str]) -> str:
    if statuses == {"N/A"}:
        return "N/A"
    if statuses and statuses <= {"Verified", "N/A"} and "Verified" in statuses:
        return "Verified"
    if "Partial" in statuses or "Verified" in statuses or "N/A" in statuses:
        return "Partial"
    return "Pending"


def artifact_is_safe(relative: str, artifact_root: str) -> bool:
    path = Path(relative)
    root = Path(artifact_root)
    return (
        not path.is_absolute()
        and SAFE_RELATIVE.fullmatch(relative) is not None
        and ".." not in path.parts
        and path.parts[: len(root.parts)] == root.parts
    )


def main() -> int:
    errors: list[str] = []
    try:
        data = load(SOURCE)
        catalog = load(SURFACES)
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"visual evidence check failed to load input: {error}", file=sys.stderr)
        return 1

    missing_root = sorted(REQUIRED_ROOT - set(data))
    errors.extend(f"visual-evidence.toml: missing {field}" for field in missing_root)
    if data.get("version") != 1:
        errors.append("visual-evidence.toml: version must be 1")
    if data.get("surface_source") != "docs/product/surfaces.toml":
        errors.append("visual-evidence.toml: surface_source must point to docs/product/surfaces.toml")
    if data.get("matrix_source") != "docs/testing/visual-matrix.md":
        errors.append("visual-evidence.toml: matrix_source must point to docs/testing/visual-matrix.md")
    if data.get("environment_status") not in ALLOWED:
        errors.append("visual-evidence.toml: unsupported environment_status")
    try:
        date.fromisoformat(data.get("last_verified"))
    except (TypeError, ValueError):
        errors.append("visual-evidence.toml: last_verified must be an ISO date")
    for key in ("surface_source", "matrix_source", "capture_helper"):
        value = data.get(key)
        if not isinstance(value, str) or not (ROOT / value).exists():
            errors.append(f"visual-evidence.toml: missing path for {key}: {value!r}")
    artifact_root = data.get("artifact_root")
    if not isinstance(artifact_root, str) or Path(artifact_root).is_absolute() or ".." in Path(artifact_root).parts:
        errors.append("visual-evidence.toml: artifact_root must be repository-relative")
    pattern = data.get("artifact_name_pattern")
    placeholders = set(PLACEHOLDER_RE.findall(pattern)) if isinstance(pattern, str) else set()
    required_placeholders = {"surface_id", "state_slug", "viewport", "platform", "commit"}
    if placeholders != required_placeholders:
        errors.append(
            "visual-evidence.toml: artifact_name_pattern must contain exactly "
            "surface_id/state_slug/viewport/platform/commit placeholders"
        )

    catalog_surfaces = catalog.get("surface", [])
    catalog_by_id = {surface.get("id"): surface for surface in catalog_surfaces}
    if len(catalog_by_id) != len(catalog_surfaces):
        errors.append("surfaces.toml contains duplicate IDs")
    records = data.get("surface", [])
    records_by_id: dict[str, dict] = {}
    for record in records:
        identifier = record.get("id", "<unnamed>")
        relative = f"docs/testing/visual-evidence.toml::{identifier}"
        if identifier in records_by_id:
            errors.append(f"{relative}: duplicate surface record")
        records_by_id[identifier] = record
        errors.extend(f"{relative}: missing {field}" for field in sorted(REQUIRED_SURFACE - set(record)))
        if identifier not in catalog_by_id:
            errors.append(f"{relative}: unknown surface ID")
            continue
        statuses = record.get("state_status")
        expected_states = set(catalog_by_id[identifier].get("visual_states", []))
        if not isinstance(statuses, dict):
            errors.append(f"{relative}: state_status must be a table")
            continue
        actual_states = set(statuses)
        if actual_states != expected_states:
            errors.append(
                f"{relative}: state labels must exactly match surfaces.toml "
                f"(missing={sorted(expected_states - actual_states)}, extra={sorted(actual_states - expected_states)})"
            )
        for state, status in statuses.items():
            if status not in ALLOWED:
                errors.append(f"{relative}: unsupported status for {state!r}: {status!r}")
        expected_overall = derived_status(set(statuses.values()))
        if expected_overall == "N/A":
            errors.append(f"{relative}: a surface cannot be entirely N/A")
        if not isinstance(record.get("next_action"), str) or not record["next_action"].strip():
            errors.append(f"{relative}: next_action must be a non-empty string")

    missing_surfaces = sorted(set(catalog_by_id) - set(records_by_id))
    extra_surfaces = sorted(set(records_by_id) - set(catalog_by_id))
    errors.extend(f"visual evidence missing surface record: {identifier}" for identifier in missing_surfaces)
    errors.extend(f"visual evidence has extra surface record: {identifier}" for identifier in extra_surfaces)

    captures = data.get("capture", [])
    capture_keys: set[tuple[str, str, str, str]] = set()
    for capture in captures:
        identifier = f"capture {capture.get('surface_id', '<unnamed>')}::{capture.get('state', '<unnamed>')}"
        errors.extend(f"{identifier}: missing {field}" for field in sorted(REQUIRED_CAPTURE - set(capture)))
        surface_id = capture.get("surface_id")
        state = capture.get("state")
        if surface_id not in catalog_by_id:
            errors.append(f"{identifier}: unknown surface")
            continue
        if state not in catalog_by_id[surface_id].get("visual_states", []):
            errors.append(f"{identifier}: state is not declared by surfaces.toml")
            continue
        key = (surface_id, state, str(capture.get("platform")), str(capture.get("viewport")))
        if key in capture_keys:
            errors.append(f"{identifier}: duplicate surface/state/platform/viewport capture")
        capture_keys.add(key)
        status = capture.get("status")
        if status not in ALLOWED:
            errors.append(f"{identifier}: unsupported status {status!r}")
        elif records_by_id.get(surface_id, {}).get("state_status", {}).get(state) != status:
            errors.append(f"{identifier}: capture status does not match state_status")
        artifact = capture.get("artifact")
        if not isinstance(artifact, str) or not artifact_is_safe(artifact, artifact_root):
            errors.append(f"{identifier}: artifact must be below {artifact_root}")
        if status == "Verified":
            for field in ("binary", "pid", "window", "platform", "viewport"):
                if not isinstance(capture.get(field), str) or not capture[field].strip():
                    errors.append(f"{identifier}: verified capture needs non-empty {field}")
            try:
                date.fromisoformat(capture.get("captured_at"))
            except (TypeError, ValueError):
                errors.append(f"{identifier}: captured_at must be an ISO date")
            if isinstance(artifact, str) and artifact_is_safe(artifact, artifact_root) and not (ROOT / artifact).is_file():
                errors.append(f"{identifier}: verified artifact does not exist: {artifact}")

    if errors:
        print("visual evidence check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    pending = sum(
        status == "Pending"
        for record in records
        for status in record.get("state_status", {}).values()
    )
    print(
        "visual evidence check OK — "
        f"{len(records)} surfaces, {sum(len(record['state_status']) for record in records)} states, "
        f"{len(captures)} captures, {pending} pending"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
