# Bugs, fixes, and non-obvious constraints

## 2026-09-05 — Zed UI source cannot be copied into the Apache-2.0 project

**Context:** A source-level comparison was made for Zed's dock/status bar,
Project Panel, Git Panel, and shared UI primitives.

**Finding:** The inspected Zed crates `workspace`, `project_panel`, `git_ui`,
and `ui` each declare `GPL-3.0-or-later`. Labonair's root license is
Apache-2.0.

**Resolution:** Treat Zed as a behavioral and architectural reference. Specify
observable interaction outcomes and independently implement them over
Labonair's existing panel, dock, workspace, and UI-kit APIs. Do not copy or
closely translate Zed function bodies, type layouts, comments, or algorithms
unless the project first makes an explicit, reviewed licensing decision.

**Reference:** `docs/ui-comparison-zed-sidebar-status-bar.md` section 2.
