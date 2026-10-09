#!/usr/bin/env python3
"""Validate command, surface, and menu catalogs against Rust contracts."""

from __future__ import annotations

import re
from pathlib import Path
import sys
import tomllib

from architecture_model import ROOT, load_manifest


PRODUCT_DIR = ROOT / "docs" / "product"
COMMANDS = PRODUCT_DIR / "commands.toml"
SURFACES = PRODUCT_DIR / "surfaces.toml"
MENU = PRODUCT_DIR / "menu.toml"
COMMAND_SOURCE = ROOT / "crates" / "command-palette-core" / "src" / "lib.rs"


def load(path: Path) -> dict:
    return tomllib.loads(path.read_text(encoding="utf-8"))


def rust_command_ids() -> set[str]:
    text = COMMAND_SOURCE.read_text(encoding="utf-8")
    start = text.index("pub enum CommandId {") + len("pub enum CommandId {")
    end = text.index("\n}", start)
    return set(re.findall(r"^    ([A-Z][A-Za-z0-9_]*)[,]?$", text[start:end], re.MULTILINE))


def check_existing_paths(errors: list[str], relative: str, paths: list[str]) -> None:
    for path in paths:
        if not (ROOT / path).exists():
            errors.append(f"{relative}: missing source/test path: {path}")


def main() -> int:
    errors: list[str] = []
    try:
        architecture = load_manifest()
        commands = load(COMMANDS)
        surfaces = load(SURFACES)
        menu = load(MENU)
    except (OSError, tomllib.TOMLDecodeError, ValueError) as error:
        print(f"product catalog check failed to load inputs: {error}", file=sys.stderr)
        return 1

    owners = {crate["name"]: crate["owner"] for crate in architecture.get("crate", [])}
    command_groups = commands.get("group", [])
    command_ids: list[str] = []
    group_ids: set[str] = set()
    command_required = {
        "id", "owner_module", "canonical_crate", "menu_path", "context",
        "availability", "default_keymap", "palette_visibility", "execution_handler",
        "success_behavior", "failure_behavior", "notification_behavior", "undo_redo",
        "tests", "visual_evidence", "source_files", "ids",
    }
    for group in command_groups:
        group_id = group.get("id", "<unnamed>")
        relative = f"docs/product/commands.toml::{group_id}"
        if group_id in group_ids:
            errors.append(f"{relative}: duplicate group id")
        group_ids.add(group_id)
        for field in command_required:
            if field not in group:
                errors.append(f"{relative}: missing {field}")
        if group.get("canonical_crate") not in owners:
            errors.append(f"{relative}: unknown canonical crate {group.get('canonical_crate')}")
        elif owners[group["canonical_crate"]] != group.get("owner_module"):
            errors.append(f"{relative}: owner module does not match architecture owner")
        if not isinstance(group.get("ids"), list):
            errors.append(f"{relative}: ids must be an array")
        else:
            command_ids.extend(group["ids"])
        check_existing_paths(errors, relative, group.get("source_files", []))
        check_existing_paths(errors, relative, group.get("tests", []))

    actual_ids = rust_command_ids()
    declared_ids = set(command_ids)
    if len(command_ids) != len(declared_ids):
        duplicates = sorted({identifier for identifier in command_ids if command_ids.count(identifier) > 1})
        errors.append(f"commands.toml: duplicate command ids: {duplicates}")
    if declared_ids != actual_ids:
        missing = sorted(actual_ids - declared_ids)
        extra = sorted(declared_ids - actual_ids)
        if missing:
            errors.append(f"commands.toml: undocumented CommandId variants: {missing}")
        if extra:
            errors.append(f"commands.toml: unknown command ids: {extra}")

    surface_records = surfaces.get("surface", [])
    surface_ids: set[str] = set()
    surface_required = {
        "id", "display_name", "owner_module", "canonical_crate", "kind",
        "canonical_entry_points", "alternate_entry_points", "context", "command_ids",
        "settings", "notifications", "visual_states", "source_files", "tests",
        "evidence", "status",
    }
    for surface in surface_records:
        surface_id = surface.get("id", "<unnamed>")
        relative = f"docs/product/surfaces.toml::{surface_id}"
        if surface_id in surface_ids:
            errors.append(f"{relative}: duplicate surface id")
        surface_ids.add(surface_id)
        for field in surface_required:
            if field not in surface:
                errors.append(f"{relative}: missing {field}")
        if surface.get("canonical_crate") not in owners:
            errors.append(f"{relative}: unknown canonical crate {surface.get('canonical_crate')}")
        elif owners[surface["canonical_crate"]] != surface.get("owner_module"):
            errors.append(f"{relative}: owner module does not match architecture owner")
        for command_id in surface.get("command_ids", []):
            if command_id != "none" and command_id not in declared_ids:
                errors.append(f"{relative}: unknown command id {command_id}")
        check_existing_paths(errors, relative, surface.get("source_files", []))
        check_existing_paths(errors, relative, surface.get("tests", []))

    menu_records = menu.get("menu", [])
    menu_ids: set[str] = set()
    for entry in menu_records:
        menu_id = entry.get("id", "<unnamed>")
        relative = f"docs/product/menu.toml::{menu_id}"
        if menu_id in menu_ids:
            errors.append(f"{relative}: duplicate menu id")
        menu_ids.add(menu_id)
        for field in ("id", "label", "owner_module", "canonical_surface", "command_ids", "availability", "evidence"):
            if field not in entry:
                errors.append(f"{relative}: missing {field}")
        if entry.get("canonical_surface") not in surface_ids:
            errors.append(f"{relative}: unknown canonical surface {entry.get('canonical_surface')}")
        for command_id in entry.get("command_ids", []):
            if command_id not in declared_ids:
                errors.append(f"{relative}: unknown command id {command_id}")

    if "shell.global-menu" not in surface_ids:
        errors.append("surfaces.toml: shell.global-menu is required as the canonical global menu surface")
    if not menu_records:
        errors.append("menu.toml: at least one menu branch is required")

    if errors:
        print("product catalog check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(
        "product catalog check OK — "
        f"{len(declared_ids)} commands, {len(surface_records)} surfaces, "
        f"{len(menu_records)} menu branches"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
