#!/usr/bin/env python3
"""Validate current documentation authority and local Markdown links."""

from pathlib import Path
import re
import sys


ROOT = Path(__file__).resolve().parents[1]

NORMATIVE_DOCS = (
    "docs/architecture.md",
    "docs/architecture/composition-root.md",
    "docs/architecture/dependency-rules.md",
    "docs/architecture/layers.md",
    "docs/architecture/runtime-lifecycle.md",
    "docs/agents/routes-schema.md",
    "docs/automation/grants.md",
    "docs/automation/limits.md",
    "docs/automation/overview.md",
    "docs/automation/test-matrix.md",
    "docs/capabilities.md",
    "docs/capabilities/schema.md",
    "docs/design-system.md",
    "docs/documentation-governance.md",
    "docs/feature-lifecycle.md",
    "docs/modules.md",
    "docs/registries.md",
    "docs/repository-layout.md",
    "docs/settings-guidelines.md",
    "docs/settings-inventory.md",
    "docs/settings.md",
    "docs/product/catalog-schema.md",
    "docs/product/workflows.md",
    "docs/performance/budgets.md",
    "docs/performance/catalog-schema.md",
    "docs/release/packaging.md",
    "docs/release/platforms.md",
    "docs/release/rollback.md",
    "docs/release/support-policy.md",
    "docs/release/checklist-schema.md",
    "docs/security/remote-access.md",
    "docs/security/secrets.md",
    "docs/security/threat-model.md",
    "docs/security/trust-boundaries.md",
    "docs/settings/persistence.md",
    "docs/settings/catalog-schema.md",
    "docs/testing/evidence-model.md",
    "docs/testing/visual-matrix.md",
    "docs/testing/visual-evidence-schema.md",
    "docs/visual-verification.md",
    "docs/workspace-model.md",
    "docs/reform/README.md",
)

MARKDOWN_FILES = (
    "AGENTS.md",
    "CLAUDE.md",
    "README.md",
    "docs-reform-plan.md",
    "docs",
    "ideas",
    "tasks/rework",
)

CONTROL_FILES = (".github", ".vscode")

INDEXED_PATHS = (
    "agents/routes.toml",
    "architecture/graph.toml",
    "capabilities/coverage.toml",
    "product/commands.toml",
    "product/surfaces.toml",
    "product/menu.toml",
    "product/workflows.toml",
    "settings/catalog.toml",
    "performance/budgets.toml",
    "release/checklist.toml",
    "automation/tools.toml",
    "evidence/scorecard.toml",
    "testing/visual-evidence.toml",
    "reform/requirements.toml",
)

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


def check_canonical_index(errors: list[str]) -> None:
    index = (ROOT / "docs" / "README.md").read_text(encoding="utf-8")
    required = [relative.removeprefix("docs/") for relative in NORMATIVE_DOCS]
    required.extend(INDEXED_PATHS)
    for target in required:
        if f"]({target})" not in index and f"](<{target}>)" not in index:
            errors.append(f"docs/README.md does not index canonical document/source: {target}")


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
    check_canonical_index(errors)
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
