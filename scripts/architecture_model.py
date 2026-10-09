#!/usr/bin/env python3
"""Shared loader for Labonair's declarative architecture policy."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import subprocess
import tomllib
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "docs" / "architecture" / "graph.toml"


def load_manifest() -> dict[str, Any]:
    with MANIFEST.open("rb") as stream:
        return tomllib.load(stream)


def load_workspace_metadata() -> dict[str, Any]:
    result = subprocess.run(
        ("cargo", "metadata", "--no-deps", "--format-version", "1"),
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


def workspace_graph(metadata: dict[str, Any]) -> dict[str, set[str]]:
    graph: dict[str, set[str]] = {}
    for package in metadata["packages"]:
        name = package["name"]
        if not name.startswith("labonair"):
            continue
        graph[name] = {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency["name"].startswith("labonair")
            and dependency["name"] != name
        }
    return graph


def manifest_graph(manifest: dict[str, Any]) -> dict[str, set[str]]:
    return {
        name: set(dependencies)
        for name, dependencies in manifest.get("allowed_edges", {}).items()
    }


def source_fingerprint(paths: Iterable[Path]) -> str:
    """Return a stable digest for the exact files used by a generated view."""
    digest = hashlib.sha256()
    for path in sorted(paths):
        digest.update(str(path.relative_to(ROOT)).encode("utf-8"))
        digest.update(b"\0")
        digest.update(path.read_bytes())
        digest.update(b"\0")
    return digest.hexdigest()[:16]
