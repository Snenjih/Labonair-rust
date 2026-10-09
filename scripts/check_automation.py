#!/usr/bin/env python3
"""Validate the structured MCP automation catalog against tool source."""

from __future__ import annotations

import re
import sys
import tomllib

from architecture_model import ROOT


SOURCE = ROOT / "docs" / "automation" / "tools.toml"
SERVER = ROOT / "crates" / "mcp-server" / "src" / "server.rs"
REQUIRED = {
    "id", "owner", "purpose", "required_grant", "allowed_targets",
    "path_restrictions", "secret_behavior", "timeouts_limits", "side_effects",
    "confirmation", "failure_behavior", "audit_behavior", "source_files",
    "positive_tests", "negative_tests",
}


def main() -> int:
    try:
        data = tomllib.loads(SOURCE.read_text(encoding="utf-8"))
        server = SERVER.read_text(encoding="utf-8")
    except (OSError, tomllib.TOMLDecodeError) as error:
        print(f"automation catalog check failed to load inputs: {error}", file=sys.stderr)
        return 1
    actual = set()
    for block in re.split(r"(?=\#\[tool\()", server)[1:]:
        match = re.search(r"async fn ([a-z][a-z0-9_]*)\s*\(", block)
        if match:
            actual.add(match.group(1))
    errors: list[str] = []
    if data.get("version") != 1:
        errors.append("automation catalog must declare version = 1")
    seen: set[str] = set()
    for tool in data.get("tool", []):
        identifier = tool.get("id", "<unnamed>")
        if identifier in seen:
            errors.append(f"duplicate automation tool id: {identifier}")
        seen.add(identifier)
        for field in REQUIRED - set(tool):
            errors.append(f"{identifier}: missing {field}")
        if identifier not in actual:
            errors.append(f"{identifier}: not found as an MCP server tool")
        for field in ("source_files", "positive_tests", "negative_tests"):
            values = tool.get(field)
            if not isinstance(values, list) or not values or not all(isinstance(value, str) for value in values):
                errors.append(f"{identifier}: {field} must be a non-empty string array")
        for field in REQUIRED - {"id", "source_files", "positive_tests", "negative_tests"}:
            if not isinstance(tool.get(field), str) or not tool[field].strip():
                errors.append(f"{identifier}: {field} must be a non-empty string")
        for path in tool.get("source_files", []):
            if not (ROOT / path).exists():
                errors.append(f"{identifier}: missing source reference {path}")
    missing = sorted(actual - seen)
    extra = sorted(seen - actual)
    if missing:
        errors.append(f"MCP tool functions missing catalog entries: {missing}")
    if extra:
        errors.append(f"catalog entries are not MCP tool functions: {extra}")
    if errors:
        print("automation catalog check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print(f"automation catalog check OK — {len(seen)} tools with positive and negative test decisions")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
