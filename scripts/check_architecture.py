#!/usr/bin/env python3
"""Validate the declarative crate ownership and dependency policy."""

from __future__ import annotations

from datetime import date, timedelta
from pathlib import Path
import sys

from architecture_model import load_manifest, load_workspace_metadata, manifest_graph, workspace_graph


VALID_ROLES = {
    "build-support",
    "capability",
    "composition",
    "contract",
    "foundation",
    "integration",
    "presentation",
    "service",
}
VALID_LAYERS = {"application", "foundation", "integration", "product"}


def check_acyclic(graph: dict[str, set[str]], errors: list[str]) -> None:
    visiting: set[str] = set()
    visited: set[str] = set()

    def visit(name: str, path: tuple[str, ...]) -> None:
        if name in visiting:
            cycle = " -> ".join((*path, name))
            errors.append(f"dependency cycle detected: {cycle}")
            return
        if name in visited:
            return
        visiting.add(name)
        for dependency in sorted(graph.get(name, set())):
            visit(dependency, (*path, name))
        visiting.remove(name)
        visited.add(name)

    for name in sorted(graph):
        visit(name, ())


def main() -> int:
    errors: list[str] = []
    try:
        manifest = load_manifest()
        metadata = load_workspace_metadata()
    except (OSError, RuntimeError, ValueError) as error:
        print(f"architecture check failed to load inputs: {error}", file=sys.stderr)
        return 1

    if manifest.get("version") != 1:
        errors.append("docs/architecture/graph.toml must declare version = 1")
    if manifest.get("status") != "normative policy source":
        errors.append("architecture manifest must remain a normative policy source")
    if manifest.get("source_of_truth") != "docs/architecture.md":
        errors.append("architecture manifest must point to docs/architecture.md")

    workspace = workspace_graph(metadata)
    declared = manifest.get("crate", [])
    declared_names = [crate.get("name") for crate in declared]
    if len(declared_names) != len(set(declared_names)):
        errors.append("architecture manifest contains duplicate crate records")
    declared_set = set(declared_names)
    workspace_set = set(workspace)
    if declared_set != workspace_set:
        missing = sorted(workspace_set - declared_set)
        extra = sorted(declared_set - workspace_set)
        if missing:
            errors.append(f"manifest is missing workspace crates: {missing}")
        if extra:
            errors.append(f"manifest contains non-workspace crates: {extra}")

    for crate in declared:
        name = crate.get("name", "<unnamed>")
        for field in ("owner", "role", "layer"):
            if not isinstance(crate.get(field), str) or not crate[field].strip():
                errors.append(f"{name}: missing architecture field {field}")
        if crate.get("role") not in VALID_ROLES:
            errors.append(f"{name}: unsupported architecture role {crate.get('role')!r}")
        if crate.get("layer") not in VALID_LAYERS:
            errors.append(f"{name}: unsupported architecture layer {crate.get('layer')!r}")

    manifest_edges = manifest_graph(manifest)
    if set(manifest_edges) != workspace_set:
        missing = sorted(workspace_set - set(manifest_edges))
        extra = sorted(set(manifest_edges) - workspace_set)
        if missing:
            errors.append(f"manifest is missing edge records: {missing}")
        if extra:
            errors.append(f"manifest contains edge records for unknown crates: {extra}")
    for name in sorted(workspace_set & set(manifest_edges)):
        unexpected = sorted(manifest_edges[name] - workspace_set)
        if unexpected:
            errors.append(f"{name}: manifest has unknown dependencies {unexpected}")
        if manifest_edges[name] != workspace[name]:
            errors.append(
                f"{name}: manifest edges differ from cargo metadata; "
                f"declared={sorted(manifest_edges[name])}, actual={sorted(workspace[name])}"
            )

    policy = manifest.get("policy", {})
    for root in policy.get("composition_roots", []):
        if root not in workspace_set:
            errors.append(f"composition root is not a workspace crate: {root}")
    ui_kit = policy.get("ui_kit")
    if ui_kit not in workspace_set:
        errors.append(f"configured UI kit is not a workspace crate: {ui_kit}")
    if policy.get("acyclic") is not True:
        errors.append("architecture policy must require an acyclic dependency graph")

    for transition in manifest.get("transitional_edge", []):
        source = transition.get("from")
        target = transition.get("to")
        if source not in workspace_set or target not in workspace_set:
            errors.append(f"transitional edge references unknown crate: {source} -> {target}")
        if target not in manifest_edges.get(source, set()):
            errors.append(f"transitional edge is not a real dependency: {source} -> {target}")
        for field in ("status", "removal_condition", "removal_task", "review_trigger"):
            if not isinstance(transition.get(field), str) or not transition[field].strip():
                errors.append(f"transitional edge {source} -> {target} has no {field}")
        removal_task = transition.get("removal_task")
        if isinstance(removal_task, str) and not (Path(__file__).resolve().parents[1] / removal_task).is_file():
            errors.append(f"transitional edge {source} -> {target} has missing removal task: {removal_task}")
        try:
            reviewed = date.fromisoformat(transition.get("last_reviewed"))
        except (TypeError, ValueError):
            errors.append(f"transitional edge {source} -> {target} has invalid last_reviewed")
        else:
            interval = transition.get("review_interval_days")
            if not isinstance(interval, int) or interval <= 0:
                errors.append(f"transitional edge {source} -> {target} has invalid review_interval_days")
            elif date.today() > reviewed + timedelta(days=interval):
                errors.append(
                    f"transitional edge {source} -> {target} review expired on "
                    f"{reviewed + timedelta(days=interval)}; re-review the edge"
                )

    check_acyclic(workspace, errors)
    if errors:
        print("architecture policy check FAILED:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    edge_count = sum(len(dependencies) for dependencies in workspace.values())
    print(
        "architecture policy check OK — "
        f"{len(workspace)} crates, {edge_count} internal edges, "
        f"{len(manifest.get('transitional_edge', []))} transitional records, acyclic"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
