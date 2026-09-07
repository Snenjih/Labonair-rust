#!/usr/bin/env python3
"""Validate current documentation authority and local Markdown links."""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]

NORMATIVE_DOCS = (
    "docs/architecture.md",
    "docs/capabilities.md",
    "docs/design-system.md",
    "docs/documentation-governance.md",
    "docs/feature-lifecycle.md",
    "docs/modules.md",
    "docs/registries.md",
    "docs/repository-layout.md",
    "docs/settings-guidelines.md",
    "docs/settings-inventory.md",
    "docs/settings.md",
    "docs/visual-verification.md",
    "docs/workspace-model.md",
)

MARKDOWN_FILES = (
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs",
    "ideas",
    "tasks/rework",
)

CONTROL_FILES = (".github", ".vscode")

MARKDOWN_LINK = re.compile(r"!?\[[^\]]*\]\((<[^>]+>|[^)\s]+)(?:\s+[^)]*)?\)")
STATUS = re.compile(r"^\*\*Status:\*\*\s+(.+)$", re.MULTILINE)
VERSION = re.compile(r"^\*\*Version:\*\*\s+(.+)$", re.MULTILINE)


def markdown_files() -> list[Path]:
    files: list[Path] = []
    for entry in MARKDOWN_FILES:
        path = ROOT / entry
        if path.is_file() and path.suffix == ".md":
            files.append(path)
        elif path.is_dir():
            files.extend(sorted(path.rglob("*.md")))
    return sorted(set(files))


def check_normative_metadata(errors: list[str]) -> None:
    for relative in NORMATIVE_DOCS:
        path = ROOT / relative
        if not path.is_file():
            errors.append(f"missing normative document: {relative}")
            continue
        text = path.read_text(encoding="utf-8")
        status = STATUS.search(text)
        version = VERSION.search(text)
        if status is None:
            errors.append(f"{relative} has no Status metadata")
        elif not status.group(1).strip().startswith("Normative"):
            errors.append(f"{relative} is not marked Normative")
        if version is None or not version.group(1).strip():
            errors.append(f"{relative} has no Version metadata")


def check_links(errors: list[str]) -> None:
    root = ROOT.resolve()
    for path in markdown_files():
        relative = path.relative_to(ROOT)
        text = path.read_text(encoding="utf-8")
        for match in MARKDOWN_LINK.finditer(text):
            target = match.group(1)
            if target.startswith("<"):
                target = target[1:-1]
            if target.startswith(("#", "http://", "https://", "mailto:", "tel:")):
                continue
            target = target.split("#", maxsplit=1)[0]
            if not target:
                continue
            resolved = (path.parent / target).resolve()
            try:
                resolved.relative_to(root)
            except ValueError:
                errors.append(f"{relative}: link escapes repository: {target}")
                continue
            if not resolved.exists():
                errors.append(f"{relative}: missing link target: {target}")


def check_control_files(errors: list[str]) -> None:
    forbidden = (
        "pnpm",
        "src-tauri",
        "tauri-apps.tauri-vscode",
        "crates/panel-ai",
        "DevTools",
    )
    for entry in CONTROL_FILES:
        path = ROOT / entry
        if not path.exists():
            continue
        files = [path] if path.is_file() else sorted(path.rglob("*"))
        for candidate in files:
            if not candidate.is_file() or candidate.suffix not in {".md", ".yml", ".yaml", ".json"}:
                continue
            text = candidate.read_text(encoding="utf-8")
            for marker in forbidden:
                if marker in text:
                    errors.append(
                        f"{candidate.relative_to(ROOT)} contains stale control marker: {marker}"
                    )


def main() -> int:
    errors: list[str] = []
    check_normative_metadata(errors)
    check_links(errors)
    check_control_files(errors)
    if errors:
        print("documentation check failed:", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1
    print(
        "documentation check OK — "
        f"{len(NORMATIVE_DOCS)} normative docs and {len(markdown_files())} Markdown files checked"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
