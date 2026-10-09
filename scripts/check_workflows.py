#!/usr/bin/env python3
"""Validate the structured end-to-end workflow catalog."""

from __future__ import annotations

import sys
import tomllib

from architecture_model import ROOT, load_manifest


SOURCE = ROOT / "docs" / "product" / "workflows.toml"
COMMANDS = ROOT / "docs" / "product" / "commands.toml"
STATUSES = {"planned", "partial", "implemented", "integration-tested", "visually-verified", "deferred"}
REQUIRED = {
    "id", "display_name", "owner_module", "canonical_crate", "preconditions",
    "entry_point", "state_transitions", "commands", "persistence", "notifications",
    "failure_modes", "security_boundary", "tests", "visual_evidence", "status", "next_action", "next_task",
}
LIST_FIELDS = {"preconditions", "state_transitions", "commands", "persistence", "notifications", "failure_modes", "tests", "visual_evidence"}


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
        command_data = tomllib.loads(COMMANDS.read_text(encoding="utf-8"))
        manifest = load_manifest()
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"workflow check failed to load inputs: {error}", file=sys.stderr)
        return 1
    owners = {crate["name"]: crate["owner"] for crate in manifest.get("crate", [])}
    command_ids = {identifier for group in command_data.get("group", []) for identifier in group["ids"]}
    errors: list[str] = []
    seen: set[str] = set()
    workflows = data.get("workflow", [])
    for workflow in workflows:
        identifier = workflow.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"{identifier}: duplicate workflow id")
        seen.add(identifier)
        for field in REQUIRED - set(workflow):
            errors.append(f"{identifier}: missing {field}")
        crate = workflow.get("canonical_crate")
        if crate not in owners:
            errors.append(f"{identifier}: unknown canonical crate {crate}")
        elif owners[crate] != workflow.get("owner_module"):
            errors.append(f"{identifier}: owner module does not match canonical crate owner")
        for field in LIST_FIELDS:
            values = workflow.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
                errors.append(f"{identifier}: {field} must be a non-empty string array")
        for command in workflow.get("commands", []):
            if command != "none" and command not in command_ids:
                errors.append(f"{identifier}: unknown command id {command}")
        for path in workflow.get("tests", []):
            if not (ROOT / path).exists():
                errors.append(f"{identifier}: missing test/evidence reference {path}")
        next_task = workflow.get("next_task")
        if not isinstance(next_task, str) or not (ROOT / next_task).is_file():
            errors.append(f"{identifier}: missing bounded next task {next_task!r}")
        for field in ("display_name", "entry_point", "security_boundary", "status", "next_action"):
            if not isinstance(workflow.get(field), str) or not workflow[field].strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        if workflow.get("status") not in STATUSES:
            errors.append(f"{identifier}: unsupported status {workflow.get('status')!r}")
    if len(workflows) < 10:
        errors.append("workflow catalog must cover at least ten end-to-end workflows")
    if errors:
        print("workflow catalog check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"workflow catalog check OK — {len(workflows)} workflows")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
