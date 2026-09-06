#!/usr/bin/env python3
"""Validate the active architecture-rework task queue."""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]
QUEUE = ROOT / "tasks" / "rework"
README = QUEUE / "README.md"
ALLOWED = {"✅ Done", "🔄 In Progress", "⏳ Planned", "⬜ Todo"}


def task_status(path: Path) -> str:
    match = re.search(r"^## Status\s*\n+\s*`?([^`\n]+)`?\s*$", path.read_text(), re.MULTILINE)
    if not match:
        raise ValueError(f"{path.relative_to(ROOT)} has no parseable Status")
    status = match.group(1).strip()
    if status not in ALLOWED:
        raise ValueError(f"{path.relative_to(ROOT)} has unsupported status: {status}")
    return status


def main() -> int:
    task_files = sorted(
        path
        for path in QUEUE.glob("R*.md")
        if re.fullmatch(r"R\d{2}-\d{3}-.+\.md", path.name)
    )
    if not task_files:
        print("rework queue check failed: no active task files found", file=sys.stderr)
        return 1

    try:
        statuses = {path.name: task_status(path) for path in task_files}
    except ValueError as error:
        print(f"rework queue check failed: {error}", file=sys.stderr)
        return 1

    active = [name for name, status in statuses.items() if status == "🔄 In Progress"]
    if len(active) != 1:
        print(
            "rework queue check failed: expected exactly one in-progress task, "
            f"found {active or 'none'}",
            file=sys.stderr,
        )
        return 1

    incomplete = [
        name for name, status in statuses.items() if status != "✅ Done"
    ]
    if incomplete[0] != active[0]:
        print(
            "rework queue check failed: the earliest incomplete task is "
            f"{incomplete[0]}, but {active[0]} is active",
            file=sys.stderr,
        )
        return 1

    listed = re.findall(r"`(R\d{2}-\d{3}-[^`]+\.md)`", README.read_text())
    expected = [path.name for path in task_files]
    if listed != expected:
        print("rework queue check failed: README sequence does not match task files", file=sys.stderr)
        print(f"  listed:   {listed}", file=sys.stderr)
        print(f"  expected: {expected}", file=sys.stderr)
        return 1

    print(
        "rework queue check OK — "
        f"{len(task_files)} tasks, active: {active[0]}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
