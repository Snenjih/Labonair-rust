# Documentation Reform Plan

**Status:** In progress — reform infrastructure implemented; native R07 evidence and deeper capability evidence remain pending
**Version:** 3
**Date:** 2026-10-09
**Authority:** Planning document; it does not override normative documents or the active rework queue.
**Execution:** Every implementation item must be moved into `tasks/rework/` in dependency order.

## 1. Goal

Build an agent-discoverable, mechanically verifiable documentation and
delivery system for Labonair.

The desired result is that a human or AI agent can answer, from the repository
without relying on chat history:

- What is Labonair and what is explicitly out of scope?
- Which module and canonical crate own this capability?
- What is the public contract and which dependencies are allowed?
- What is the canonical user entry point, command, menu, panel, or status item?
- Which state, settings, persistence, events, notifications, and registries are involved?
- What is implemented today versus only planned or partially migrated?
- Which tests, screenshots, security checks, and performance measurements prove it?
- What is the next safe implementation task and which gates must pass?

The goal is not to promise that the application will never have problems. The
goal is to make ambiguity, architectural drift, missing evidence, stale
documentation, and incomplete migrations visible and rejectable as early as
possible.

## 2. Authority and scope

This plan extends the existing Labonair documentation system. It does not
replace or weaken:

- `AGENTS.md` and the repository instructions;
- the authority order in `docs/README.md`;
- the normative contracts in `docs/`;
- accepted ADRs;
- the single active queue in `tasks/rework/`;
- the ownership rules in `docs/modules.md` and `docs/capabilities.md`;
- the boundary-first workflow in `docs/feature-lifecycle.md`.

The normative repository documentation remains in English, as required by the
repository instructions. Conversation, review, and planning may use German,
but repository contracts, code-facing terminology, task records, and generated
documentation must remain consistent and searchable in English.

The reform applies to:

- AI-agent onboarding and prompting;
- product and user-workflow documentation;
- architecture, ownership, dependency, and lifecycle documentation;
- capability, command, menu, panel, dock, and status-surface inventories;
- settings, persistence, data lifecycle, and migration records;
- MCP, automation, host access, and security documentation;
- test, visual, performance, release, and recovery evidence;
- CI and local verification infrastructure;
- documentation governance and freshness checks.

It does not authorize a product-scope expansion, a new shell surface, a
second feature owner, a second active task queue, or a rewrite of the
application architecture by itself.

## 3. Current baseline

### 3.1 Existing Labonair strengths

Labonair already has the most important architectural foundations:

- a normative documentation index and authority hierarchy;
- an explicit distinction between target architecture and current-state audits;
- one-owner-per-capability and one-canonical-crate rules;
- typed contracts, registries, events, and composition-root boundaries;
- a capability matrix with current implementation dispositions;
- an ordered and guarded `tasks/rework/` queue;
- settings, workspace, registry, visual-verification, and lifecycle contracts;
- native GPUI and UI-kit rules;
- a visual verification process tied to the exact native binary and window PID;
- documentation, rework-queue, and dependency-verification scripts;
- an existing command-palette, keymap, panel, status-item, theme, transfer, and notification registry model;
- an implemented MCP server with session, host, grant, and visible-action concepts.

These foundations should be strengthened rather than replaced.

### 3.2 Current gaps to close

The following gaps were the reason for this plan. Their current state is
recorded explicitly so this document does not preserve obsolete findings:

1. The root rules needed a shorter five-minute reading path and a change-type
   decision matrix. **Resolved:** `docs/agents/` now provides routing,
   contracts, finish gates, handoff, and anti-pattern guidance.
2. Most capability rows are still maintained as a human matrix. **Controlled:**
   three pilot descriptors are structured and all remaining rows have explicit
   migration records in `docs/capabilities/coverage.toml`, each with a
   repository-relative `next_task`; expansion remains a tracked follow-up
   rather than an undocumented gap.
3. The owner/layer/edge model was not one source. **Resolved:**
   `docs/architecture/graph.toml` is checked against Cargo metadata and feeds a
   generated projection.
4. Commands and surfaces lacked a complete catalog. **Resolved:** all 101
   stable `CommandId` variants, 11 canonical surfaces, 5 menu branches, and 15
   end-to-end workflows are cataloged and checked.
5. Evidence was not consistently indexed. **Improved:** capability pilots,
   workflows, visual matrix, scorecard, performance budgets, and release
   checklist now use explicit status/evidence records; native visual cells
   remain honestly pending.
6. There was no product scorecard. **Resolved:** the generated scorecard covers
   architecture, agents, docs, capabilities, commands, surfaces, settings,
   remote safety, notifications, Rust, visual, security, performance, and
   release readiness.
7. MCP and remote behavior lacked one discoverable security route. **Resolved:**
   tool, grant, limit, threat-model, secret, remote-access, and positive/
   negative-test documents are indexed and the six MCP tools are checked.
8. The documentation checker did not cover all generated/indexed sources.
   **Improved:** metadata, links, canonical index coverage, structured source
   presence, and generated freshness are checked; semantic orphan/claim drift
   still requires human review and explicit evidence.
9. The Explorer dependency gate was broken. **Resolved structurally:** direct
   panel-to-SSH/SFTP edges now cross the injected `RemoteExplorerService`;
   native R07 visual acceptance evidence is still pending.
10. Scoped verification did not cover all reform domains. **Resolved:** the
    canonical verifier now exposes docs, queue, architecture, capabilities,
    surfaces, workflows, settings, performance, evidence, automation,
    release, knowledge, Rust, and all scopes.

The detailed comparison is preserved in
[`docs/reports/vergleichsbericht-labonair-vs-photocraft.md`](docs/reports/vergleichsbericht-labonair-vs-photocraft.md).

### 3.3 Implementation progress

The following reform foundations are implemented and verified in the current
tree:

- the five-minute agent route, change matrix, task template, finish gates,
  handoff template, anti-pattern guide, and structured request route source
  under `docs/agents/`, with a generated agent index;
- `python3 scripts/verify.py --scope {docs,queue,architecture,visual,rust,all}` as
  the canonical local verification entry point;
- `docs/architecture/graph.toml` as the declarative ownership/layer/edge
  source for all 57 workspace crates and 227 internal edges;
- `scripts/check_architecture.py` and the manifest-backed
  `scripts/check_crate_deps.py` with acyclicity and boundary invariants;
- the generated architecture projection at
  `docs/generated/architecture.md`;
- decomposed layer, composition-root, lifecycle, dependency, and capability
  documentation linked from the canonical documentation index;
- the Explorer remote-directory contract and shell-owned adapter, removing
  the panel's direct SSH/SFTP dependencies.
- structured pilot descriptors for Command Palette, Hosts, and Notifications
  under `docs/capabilities/`, with schema validation and a generated index;
- freshness checks for the generated architecture and capability projections;
  every capability coverage record has a bounded `next_task` in the active
  queue.
- command, menu, and surface catalogs covering all 101 stable `CommandId`
  variants, with duplicate/orphan checks and generated views;
- an evidence scorecard with 16 current product/engineering items;
- an MCP tool catalog covering all six annotated tools with grant, limit,
  secret, positive-test, and negative-test decisions;
- organized Settings, security, performance, and release contracts, including
  machine-readable Settings, performance-budget, and release-check catalogs;
  generated views and scoped validators cover each one.
- a machine-readable visual evidence registry derived from all 11 canonical
  surfaces, with 63 exact catalog states, deterministic artifact naming, and a
  generated per-surface screenshot index; all states remain explicitly
  `Pending` until supported-host captures exist.
- a commit-bound visual capture path resolver and manual macOS workflow that
  accept only canonical surface/state labels, enforce the catalog viewport
  rule, and upload a reviewer-inspectable native capture without claiming a
  state automatically.
- a 15-workflow end-to-end catalog connecting entry points, state,
  persistence, notifications, failures, security, tests, and visual evidence.
- a definition-of-done traceability catalog covering all 17 reform requirements
  with source references, status, limitations, and bounded queue tasks.

The current environment cannot provide the native macOS visual captures
required by R07. Those cells remain `Pending` in the acceptance matrix rather
than being inferred from a successful build.

## 4. Design principles

### 4.1 One source of truth per claim

Every important fact has one canonical owner:

- product scope → product contract;
- capability ownership → capability registry/matrix;
- architecture edges → architecture manifest;
- commands and entry points → command/surface descriptors;
- settings → settings inventory;
- security boundaries → threat model and automation contract;
- current implementation → audits and generated evidence;
- active work → `tasks/rework/`.

Supporting documents link to the owner instead of copying an independent
version of the claim.

### 4.2 Separate target, current state, and evidence

Every status must distinguish:

```text
Target:       what the architecture or product intends
Current:      what the source tree currently contains
Evidence:     what tests, runtime checks, screenshots, or measurements prove
```

No build pass may be used as evidence that a complete user workflow exists.
No target document may be cited as proof that a migration is complete.

### 4.3 Owner before implementation

No feature change starts before the owner, canonical crate, public contract,
entry point, state owner, persistence owner, notifications, registries, and
UI-kit components are identified.

If no owner can be identified, the agent must stop and create a bounded
architecture decision or task instead of placing behavior in the shell.

### 4.4 Generated views, not duplicated tables

Human-readable indexes, capability pages, command catalogs, architecture
graphs, and scorecards should be generated from structured sources wherever
practical. Generated pages must be marked as generated and must not be edited
manually.

### 4.5 Evidence before completion

`implemented` and `verified` are different states. A capability is only
complete when its required contract, integration, UI, persistence, error,
security, performance, and visual evidence is present or explicitly marked
as not applicable.

### 4.6 The shortest safe agent path

An agent should not need to read the entire repository before making a bounded
change. The repository must provide a short router that leads to the exact
normative documents, owner, task, source files, and verification gates.

### 4.7 One queue and bounded work

The active architecture queue remains `tasks/rework/`. This plan creates no
parallel planning system. Each reform phase becomes a task only after its
dependencies are complete, and only the earliest eligible task may start.

## 5. Target knowledge-system architecture

The desired information flow is:

```text
Normative contracts + structured manifests + code metadata
                         |
                         v
             validators and generators
                         |
       +-----------------+------------------+
       v                                    v
Human-readable generated indexes       CI machine results
       |                                    |
       +-----------------+------------------+
                         v
               evidence and scorecard
                         |
                         v
                agent task and handoff
```

The system should have four layers:

1. **Canonical sources** — normative Markdown, ADRs, capability descriptors,
   architecture manifests, settings records, command metadata, and task files.
2. **Generated views** — architecture graph, capability pages, menu and
   command catalogs, agent index, and scorecard.
3. **Evidence artifacts** — test output, screenshot paths, visual matrices,
   performance baselines, security results, corpus results, and release checks.
4. **Verification tooling** — local scripts and CI jobs that check both the
   source contracts and generated outputs.

The definition-of-done requirements themselves are tracked in the structured
source [`docs/reform/requirements.toml`](docs/reform/requirements.toml) and
the generated projection
[`docs/generated/reform-coverage.md`](docs/generated/reform-coverage.md). This
traceability layer does not replace the evidence scorecard or active task
queue; it ensures that every reform claim has a current status, source
references, a limitation, and a bounded next task.

## 6. Target documentation layout

The existing documents should be retained and expanded incrementally. The
following layout is the target, not a requirement to rename everything in one
change:

```text
AGENTS.md                         # short repository router and hard rules
CLAUDE.md                         # thin adapter; no second authority
docs-reform-plan.md               # this project goal

docs/
  README.md                       # canonical index and authority order

  agents/
    README.md                     # five-minute agent path
    change-matrix.md              # change type -> reading and gate matrix
    task-template.md              # pre-change contract
    finish-gates.md               # required completion evidence
    handoff-template.md           # structured agent handoff
    anti-patterns.md              # known unsafe or obsolete approaches

  product/
    product-contract.md           # scope, direction, and non-goals
    user-workflows.md             # end-to-end workflows
    surfaces.md                   # canonical product surfaces
    menu-model.md                 # global menu and palette model

  architecture/
    graph.toml                    # structured crates, layers, and edges
    layers.md                     # human-readable layer rules
    composition-root.md           # app/shell responsibilities
    runtime-lifecycle.md          # startup, workspace, shutdown, jobs
    dependency-rules.md           # edge classes and transition policy

  capabilities/
    README.md                     # generated/indexed capability overview
    *.toml                        # one structured descriptor per capability

  generated/
    architecture.md
    capabilities.md
    commands.md
    surfaces.md
    menu.md
    agent-index.md
    scorecard.md
    visual-evidence.md

  settings/
    README.md
    catalog-schema.md
    catalog.toml
    persistence.md

  automation/
    overview.md
    tools.toml
    grants.md
    limits.md
    test-matrix.md

  security/
    threat-model.md
    trust-boundaries.md
    secrets.md
    remote-access.md

  testing/
    test-strategy.md
    evidence-model.md
    visual-matrix.md
    visual-evidence-schema.md
    visual-evidence.toml
    native-visual-capture-runbook.md
    integration-matrix.md

  performance/
    catalog-schema.md
    budgets.md
    budgets.toml

  release/
    README.md
    checklist-schema.md
    checklist.toml
    platforms.md
    packaging.md
    rollback.md
    support-policy.md

  adr/
  audits/
  reports/
  archive/

tasks/rework/                      # only active implementation queue
memory/                            # continuity and non-obvious fixes only
scripts/                           # deterministic checks and generators
```

The documentation index must state which files are normative, generated,
descriptive, advisory, historical, or active execution records.

## 7. Agent onboarding and prompting

### 7.1 Root `AGENTS.md` responsibilities

The root file should remain concise and act as a router. It should contain:

1. Mission, product boundaries, and non-goals.
2. Five-minute orientation commands and reading order.
3. Authority hierarchy and current active queue.
4. Change classification and link to the change matrix.
5. Owner-first and boundary-first rules.
6. Command, menu, settings, persistence, and notification rules.
7. Security and secret-handling rules.
8. Required verification gates.
9. Stop conditions for missing owner, conflicting authority, or unsafe scope.
10. Structured handoff requirements.

Detailed explanations belong in `docs/agents/`, not in a giant root prompt.

### 7.2 Five-minute agent path

Every agent should follow this sequence:

```text
1. Read AGENTS.md.
2. Read docs/README.md.
3. Read docs/agents/README.md.
4. Classify the request.
5. Find the capability owner and canonical crate.
6. Find the active task or create a bounded task record.
7. Read only the relevant normative documents.
8. Write the change contract.
9. Implement contract -> owner -> adapter -> consumer.
10. Run the scoped and mandatory gates.
11. Update evidence and canonical documentation.
12. Write a structured handoff.
```

### 7.3 Change contract required before editing

Every feature, migration, UI, infrastructure, or documentation change must
record:

```text
Request type:
Owner module:
Canonical capability crate:
Canonical user entry point:
Public typed contract:
State owner:
Persistence owner:
Commands and keymap entries:
Events and registry contributions:
Settings:
Notifications:
UI-kit components:
Allowed dependency changes:
Tests:
Visual evidence:
Security impact:
Performance impact:
Removal condition for compatibility code:
```

### 7.4 Structured handoff required after editing

The agent handoff must state:

```text
Changed:
Not changed:
Owner verified:
Contracts updated:
Consumers/adapters updated:
Tests run:
Visual evidence:
Documentation updated:
Known limitations:
Remaining migration edges:
Failed or unavailable gates:
Recommended next task:
```

### 7.5 Prompt rules for AI agents

The repository prompt should explicitly instruct agents to:

- search canonical documents before searching implementation details;
- never infer ownership from a convenient call site;
- never use a target document as current-state evidence;
- never add a second registry or shell-wide static table;
- never create a new compatibility path without a named removal condition;
- never mark a feature complete without evidence;
- never skip the active queue because a later task appears easier;
- stop when a required owner, contract, security boundary, or product decision
  is missing;
- preserve unrelated worktree changes;
- report failed gates instead of hiding or bypassing them.

`CLAUDE.md` and future agent-specific files must point back to `AGENTS.md`
and must not create competing instructions.

## 8. Architecture manifest and dependency infrastructure

### 8.1 Declarative architecture source

Create an architecture manifest, preferably under
`docs/architecture/graph.toml`, containing:

```text
crate identity
capability owner
crate role
layer
public/private boundary
allowed dependency edges
edge reason
edge status
removal task
expiry condition
```

This item is implemented for the current workspace. The manifest is checked
against `cargo metadata`, and the readable view is generated by
`scripts/gen_architecture.py`. Capability descriptors and command/surface
metadata are intentionally separate follow-up sources; they must not be
invented by copying the crate graph.

### 8.2 Required architecture checks

The verifier must reject:

- unregistered product crates;
- crates without an owner or role;
- dependency cycles;
- forbidden upward or sideways dependencies;
- feature crates depending on `labonair-shell`;
- UI-kit depending on product modules;
- shell-owned feature state;
- static duplicate feature registries;
- transitional edges without removal tasks;
- transitional edges past their expiry condition;
- direct remote-service dependencies that bypass the owning contract;
- undocumented public crate boundaries.

The current checker rejects unregistered crates, edge drift, cycles, and the
existing UI/panel boundary violations. Product catalog and scorecard checks
now cover duplicate command/surface IDs and evidence references. Transitional
edges now also require a bounded removal task, review trigger, review date,
and a machine-checked review interval; the execution/removal decision remains
tracked in `R07-007` after the knowledge-system dependency. Deeper
symbol-level ownership remains later work.

### 8.3 Immediate boundary priority

The former `panel-explorer` to `ssh`/`sftp` dependency finding is resolved
structurally through the narrow `ExplorerHost` remote-directory contract and a
shell-owned adapter. The active R07 acceptance task remains the controlling
work item for native Explorer evidence and final acceptance.

## 9. Capability and workflow management

### 9.1 Capability descriptor

Every capability should have a structured descriptor with at least:

```text
id
display_name
owner_module
canonical_crate
sibling_crates
public_contract
canonical_entry_points
alternate_entry_points
commands
default_keymap
registries
events
state_owner
persistence_owner
settings
notifications
ui_kit_components
automation_tools
security_boundaries
tests
visual_states
performance_budget
current_status
target_status
known_limitations
removal_condition
last_verified
```

### 9.2 Status vocabulary

Use one status vocabulary across capabilities, tasks, scorecards, and reports:

```text
planned
contract-defined
implemented
integration-tested
visually-verified
security-reviewed
performance-baselined
regression-locked
partial
deferred
removed
```

`target_status` and `current_status` must remain separate.

### 9.3 Product workflow catalog

Document end-to-end workflows, not only crates. Initial workflows should
include:

- application startup and shutdown;
- project workspace creation and transition;
- standalone terminal use;
- local terminal lifecycle;
- SSH host connection;
- SFTP browsing and transfer;
- editor file lifecycle;
- Git/source-control workflow;
- files and explorer workflow;
- themes and icon themes;
- settings editing and reload;
- keymap editing and conflict handling;
- command palette discovery and execution;
- notification creation, deduplication, and presentation;
- background jobs and progress;
- MCP/AI session and grant lifecycle;
- error, cancellation, retry, and recovery paths.

Each workflow must record preconditions, entry point, state transitions,
commands, persistence, notifications, failure modes, security boundaries,
tests, visual evidence, and a repository-relative `next_task` that owns the
next unfinished implementation or evidence step.

## 10. Commands, menus, and product surfaces

### 10.1 Command contract

Every user-visible action should have a stable command descriptor containing:

```text
command_id
owner
label
menu_path
context
availability/enabled rule
default keymap
palette visibility
execution handler
success behavior
failure behavior
notification behavior
undo/redo behavior, if applicable
tests
visual entry point
```

The command registry remains the canonical owner. The shell must only compose
and host contributions.

### 10.2 Surface catalog

Generate a catalog covering:

- titlebar tabs and global menu;
- workspace tabs and splits;
- registered docks and panels;
- statusbar controls and global information items;
- command palette;
- dialogs and popovers;
- notifications dropdown;
- standalone tool surfaces.

Each surface entry must include owner, stable ID, supported context, commands,
availability, settings, notifications, visual states, and evidence links.

### 10.3 Menu checks

The verifier must detect:

- duplicate command or surface IDs;
- menu items without an owner;
- commands without a canonical entry point;
- UI-only actions that bypass the command contract;
- no-op or dead menu items;
- missing enabled/disabled behavior;
- missing error or notification paths;
- missing tests or visual evidence;
- parallel shell tables that duplicate owner contributions.

The canonical Labonair shell model remains titlebar, workspace, docks,
statusbar, and overlay layer. The reform must not add permanent chrome without
an ADR.

## 11. Settings, persistence, and data lifecycle

For every setting, document:

```text
setting_id
owner
scope: app | workspace | host | session
default
storage location
serialization format
migration behavior
runtime consumer
UI entry point
secret classification
restart/live-reload behavior
tests
```

The settings system continues to contain values only. Hosts, themes, keymaps,
transfers, notifications, command registration, and other feature behavior
remain owned by their respective modules.

The data-lifecycle documentation must show:

```text
input → validation → in-memory state → persistence → reload → migration → deletion
```

It must also identify ownership of secrets, credentials, session data, cache
files, logs, SQLite data, temporary files, and user configuration.

Dead settings and fields must be removed or explicitly marked with a named
removal task. A setting must not remain merely because it can be parsed.

## 12. Evidence and scorecard system

### 12.1 Evidence matrix

Each capability and workflow should have a matrix covering:

| Evidence class | Required question |
|---|---|
| Contract | Is the public boundary typed and tested? |
| Unit | Is domain behavior tested without the full application? |
| Integration | Does the owner work through its real consumer? |
| Command | Can the user-visible action execute through the canonical command? |
| Surface | Is the feature reachable from its canonical UI entry point? |
| Persistence | Are load, save, reload, migration, and failure paths covered? |
| Notification | Are success, failure, progress, and deduplication correct? |
| Visual | Are normal, narrow, focused, empty, loading, and error states checked? |
| Security | Are invalid grants, paths, hosts, and secrets rejected? |
| Performance | Is a budget or baseline defined where relevant? |
| Automation | Can the supported CLI/MCP/agent path perform the workflow safely? |
| Regression | Is the original bug or migration edge permanently covered? |

Evidence records should reference exact test names, source paths, screenshot
paths, artifact names, or command outputs. A free-form statement such as
“tested” is insufficient.

### 12.2 Scorecard

Generate a product-specific `docs/generated/scorecard.md` with these areas:

- architecture and ownership;
- crate and dependency health;
- capability completeness;
- commands and keymap;
- menu and surface coverage;
- workspace lifecycle;
- settings and persistence;
- SSH/SFTP and remote safety;
- notifications and background work;
- MCP and automation;
- visual verification;
- reliability and recovery;
- performance;
- documentation freshness;
- release and distribution.

Each scorecard item needs an ID, target, owner, status, evidence, last
verified date, limitation, and next action.

### 12.3 Visual evidence

Retain the existing exact-native-binary and PID-scoped rules. Add:

- deterministic screenshot naming;
- a screenshot index per surface;
- state labels matching the surface catalog;
- a link from each visual scorecard item to its artifacts;
- explicit missing-evidence status;
- native checks for narrow, empty, loading, and error states.

**Implementation state:** The structured source is now
`docs/testing/visual-evidence.toml`, checked by
`scripts/check_visual_evidence.py` and projected to
`docs/generated/visual-evidence.md`. It derives state labels from the surface
catalog and prints the exact future artifact path for every pending state.
No native capture is inferred from this registry; the R07 states remain
pending until the exact-binary/PID protocol succeeds on the supported host.

## 13. Automation, MCP, and security documentation

Create a single automation and security path that documents:

- available tools and their owners;
- required grants and grant lifecycle;
- host and session restrictions;
- rooted-path rules;
- timeouts, output limits, and cancellation;
- shell and command execution boundaries;
- secret redaction;
- audit and logging behavior;
- error and denial semantics;
- human confirmation requirements;
- supported agent workflows;
- negative security tests.

Every tool record should contain:

```text
tool_id
purpose
owner
required_grant
allowed_targets
path restrictions
secret behavior
timeouts and limits
side effects
confirmation requirements
failure behavior
audit behavior
example
positive tests
negative tests
```

The tool catalog should be generated from Rust metadata or contract tests when
possible. Security documentation must never expose real credentials, tokens,
host secrets, or sensitive test fixtures.

## 14. Verification and CI infrastructure

### 14.1 One canonical local verifier

Create one documented local entry point. Initially it may orchestrate the
existing scripts; a Rust `xtask` can be introduced later only if it provides a
clear maintenance benefit.

The repository currently exposes these scoped and complete modes:

```text
python3 scripts/verify.py --scope docs
python3 scripts/verify.py --scope knowledge
python3 scripts/verify.py --scope visual
python3 scripts/verify.py --scope rust
python3 scripts/verify.py --scope all
```

The complete mode must run, directly or through the existing project commands:

```text
python3 scripts/check_documentation.py
python3 scripts/check_rework_queue.py
scripts/check-crate-deps.sh
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

The exact command names may remain project-specific, but the repository now
has one canonical documented invocation. The knowledge scope includes all
structured reform domains; native visual, platform, and credential-dependent
checks remain conditional and are represented as evidence status rather than
silently simulated.

### 14.2 Required CI checks

Add or strengthen these CI categories:

1. **Documentation** — links, metadata, generated freshness, orphan files,
   duplicate IDs, and canonical-index coverage.
2. **Architecture** — graph, layers, ownership, cycles, forbidden edges, and
   transitional-edge expiry.
3. **Capabilities** — descriptor completeness and status/evidence consistency.
4. **Commands and surfaces** — menu, palette, panel, dock, status-item, and
   command reachability.
5. **Rust quality** — formatting, check, clippy, and workspace tests.
6. **Visual** — exact native binary, surface state matrix, and artifacts.
7. **Security** — remote, MCP, grants, paths, and secret-handling tests.
8. **Performance** — budgets and baselines where a capability declares them.
9. **Release** — package, smoke, signing, support, and recovery checks.

Checks that protect a claimed supported platform should be required. Checks
for optional or unavailable platforms may remain explicitly conditional, but
the support policy must say so.

### 14.3 Generated artifacts

Every generator should support:

- deterministic output;
- stable ordering;
- a source-file header;
- deterministic source fingerprint (and a verification command that resolves it);
- machine-readable output for CI;
- human-readable Markdown output for agents;
- a check-only mode that fails when regeneration changes files.

## 15. Implementation phases

### Phase A — Trust and agent routing (P0)

1. Recheck and resolve the Explorer dependency-gate finding.
2. Complete R07 acceptance work before beginning later architecture reform.
3. Add `docs/agents/README.md` with the five-minute path.
4. Add the change matrix, task template, finish-gate template, and handoff template.
5. Document one canonical local verification command.
6. Extend the documentation checker only with deterministic, high-value checks.

**Exit criteria:** an agent can identify the owner, active task, required
documents, and required gates without reading the whole repository; the
existing dependency gate is green or has a formally accepted bounded task.

**Implementation state:** items 3–6 are complete. Item 1 is complete for the
direct Explorer dependency boundary; item 2 remains open only for native visual
acceptance evidence.

### Phase B — Structured architecture and capabilities (P1)

1. Add the architecture graph manifest.
2. Replace hard-coded architecture assumptions with generated or manifest-backed checks where practical.
3. Add capability descriptors for three pilot capabilities:
   - command palette;
   - hosts;
   - notifications.
4. Generate the first architecture and capability views.
5. Validate the model before expanding it to every capability.

**Exit criteria:** the pilot capabilities have owner, contract, entry point,
state, persistence, commands, notifications, tests, and status/evidence
records that agree with source code.

**Implementation state:** the architecture manifest, generated architecture
view, three-capability descriptor pilot, explicit coverage records for all
matrix rows, command/surface/menu catalogs, workflow catalog, and freshness
checks are complete. Expanding descriptors to all capabilities and adding
deeper per-command availability/evidence remains open.

### Phase C — Commands, surfaces, and menus (P1)

1. Define the command descriptor contract.
2. Define the surface and menu catalog schema.
3. Generate command, menu, and surface views.
4. Add orphan, duplicate, no-op, and missing-owner checks.
5. Link every canonical Labonair product surface to its owner and evidence.

**Exit criteria:** a user-visible action cannot be added without a stable
owner, command, entry point, availability rule, and test/evidence decision.

**Implementation state:** complete for the current `CommandId` enum and
canonical surfaces. The checker covers duplicate/orphan IDs, owner/crate
agreement, source/test paths, menu references, and generated freshness.
Workflow records extend the catalog across persistence, failure, security,
and visual states.

### Phase D — Evidence and scorecard (P1/P2)

1. Define the evidence record and status vocabulary.
2. Add capability/workflow evidence matrices.
3. Generate the Labonair scorecard.
4. Index visual artifacts and surface states.
5. Add performance and security evidence where capabilities require it.

**Exit criteria:** “implemented” and “verified” are mechanically distinct, and
the scorecard exposes missing evidence instead of hiding it.

**Implementation state:** complete as an evidence infrastructure baseline.
The scorecard, visual matrix, workflow records, and generated projections
expose current `Partial`/`Pending` records. Native R07 captures and deeper
per-capability artifacts remain open by design.

### Phase E — Settings, automation, security, and operations (P2)

1. Complete settings and data-lifecycle records.
2. Add the MCP/tool catalog and grant/limit documentation.
3. Add threat-model and negative-test references.
4. Add performance baseline and budget records.
5. Add platform, release, signing, rollback, and support documentation.

**Exit criteria:** an agent can safely understand not only how to implement a
feature, but also how it stores data, exposes automation, handles secrets,
fails, ships, and recovers.

**Implementation state:** structured Settings/data-lifecycle, MCP/security,
performance-budget, and release/rollback sources are implemented and checked.
Credential-dependent release, native performance, and complete visual
acceptance remain conditional evidence work.

## 16. Definition of done for this reform

The documentation reform is complete only when all of the following hold:

- `AGENTS.md` provides a short deterministic entry path;
- `docs/README.md` indexes every canonical documentation area;
- every capability has exactly one owner and one canonical crate;
- every capability has a structured descriptor or an explicit tracked migration;
- architecture layers and dependency edges are checked from a declarative source;
- every visible action has an owner and command/surface decision;
- menu and surface catalogs are generated or mechanically checked;
- settings and persistence ownership are explicit;
- automation and MCP tools have documented grants, limits, and negative tests;
- target, current, and evidence status are separate;
- every completed workflow has linked contract, integration, visual, security,
  performance, or not-applicable evidence;
- generated documentation is checked for freshness;
- one local command reproduces the CI verification path;
- all required CI gates are green;
- the active rework queue remains ordered and has no bypassed dependency;
- no obsolete compatibility path remains without a removal condition;
- no duplicate shell-wide feature table or parallel product surface has been introduced.

## 17. Non-goals and safeguards

This plan must not be used to:

- copy PhotoCraft's image-editor product model;
- add web, WASM, or unsupported platforms to Labonair;
- create a giant central shell or feature registry;
- introduce a new abstraction for a single caller;
- turn every implementation detail into a duplicated Markdown page;
- make `AGENTS.md` an unmaintainable encyclopedia;
- declare success because a build passes;
- bypass R07 or start R09 before the acceptance dependency allows it;
- add a new framework or dependency without a documented reason;
- overwrite unrelated worktree changes;
- record secrets, credentials, tokens, or sensitive host data.

## 18. Immediate next tasks

Completed reform prerequisites are recorded above and are not a second task
queue. The remaining work is evidence closure and descriptor expansion; it
must follow the existing `tasks/rework/` order:

1. **R07 acceptance** — run the documented native macOS capture protocol for
   the 63 catalog states (including normal, narrow, focused, empty, loading,
   error, long-list, and overlay equivalents), add durable `capture` records
   to `docs/testing/visual-evidence.toml`, then close R07-001/R07-004/R07-005
   only when exact-binary/PID evidence exists.
2. **`R07-006` knowledge-system completion** — after R07 acceptance, convert
   matrix-tracked rows to full
   descriptors in owner/task order, without creating a second capability
   authority; the coverage file remains the explicit migration record until
   each descriptor is complete.
3. **`R07-007` transitional-edge review** — after the knowledge-system task,
   review each remaining compatibility edge against its removal condition and
   remove or narrow it only when the owning workflow has a real consumer.
4. **Evidence closure** — attach native visual artifacts, integration tests,
   security denials, performance measurements, and release handoffs to the
   workflow/scorecard records where the environment and product workflow
   support them.
5. **Negative and recovery execution** — run the MCP/remote denial matrix,
   staged updater rollback, and representative performance workloads on their
   supported hosts; keep unavailable evidence marked `Pending`.
6. **Generated drift hardening** — extend semantic claim/orphan checks only
   when they can be deterministic and source-backed; do not replace the
   human authority model with duplicated prose.

Each task must state its owner, allowed files, dependencies, removal
conditions, tests, evidence, and completion gates before implementation.

## 19. Success statement

The reform succeeds when Labonair's repository behaves like a reliable product
knowledge base rather than a collection of documents:

```text
One authority per claim.
One owner per capability.
One canonical entry point per workflow.
One typed contract per boundary.
One evidence record per completion claim.
One verification path for agents and CI.
One active implementation queue.
```

At that point, AI agents will not need to guess the application structure from
the shell or from stale prose. They will be able to navigate from request to
owner, contract, implementation, UI, evidence, and next task through the
repository itself.
