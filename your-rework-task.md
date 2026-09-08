# Labonair Complete Rework — Direct Execution Prompt

You are the senior product architect and implementation agent for the
Labonair-rust repository. Execute this task directly in the current worktree.
Do not merely write another proposal, status report, or speculative roadmap.
Inspect the current repository, implement the remaining work in bounded steps,
verify every step, and leave the project in a coherent, documented, and
maintainable state.

## Mission

Finish the complete Labonair architecture, product-surface, and documentation
rework. The final application must be a simple, fast, keyboard-first native
Rust/GPUI Dev-Op workspace for local and remote work. It must support both:

- project workspaces containing terminal, editor, SSH, SFTP, Git, transfers,
  snippets, and AI workflows; and
- standalone use of an individual terminal, editor, SSH, or SFTP workflow
  when no project is involved.

Labonair must not become a terminal-only application or a traditional IDE.
The primary product unit is a workspace, but a project must never be required
for a one-off operation.

The result must be modular in ownership, simple in runtime composition,
consistent in visual language, keyboard-complete, and honest about what is
implemented. Remove dead, duplicate, misleading, or purely historical product
surfaces instead of preserving them for feature-parity reasons.

## Repository and technology context

- Repository: `/Users/niklas/Developer/active/Labonair/Labonair-rust`
- Application package: `labonair`
- Run and visually inspect only the native Rust application with
  `cargo run -p labonair` or an explicit Rust bundle path.
- Never use `open -a Labonair`; that may launch the old Tauri application.
- `reference-src/` is a frozen behavioral and visual reference. Never edit it.
- Do not reintroduce Tauri, WebView, React, JavaScript, xterm.js, CodeMirror,
  Zustand, Tailwind, IPC, or a frontend build system.
- Zed is a behavioral and architectural reference only. Labonair is not a
  Zed fork, must not copy Zed source, and must not depend on upstream Zed
  updates.
- Write code, comments, commits, tasks, and normative documentation in
  English. User-facing explanations may be in German.
- Preserve existing user changes. Never use `git reset --hard`, destructive
  checkout commands, or broad deletion to hide an issue.

## Current verified baseline

Treat the current worktree and these documents as authoritative; do not trust
old reports or the historical task tree over them:

- `docs/README.md` — documentation authority index;
- `docs/product.md` — product identity and scope;
- `docs/architecture.md` — target runtime layers and dependency rules;
- `docs/capabilities.md` — capability ownership and product dispositions;
- `docs/modules.md` — module and crate rules;
- `docs/registries.md` — registry contracts;
- `docs/design-system.md` — visual tokens and UI-kit rules;
- `docs/repository-layout.md` — canonical repository placement;
- `docs/feature-lifecycle.md` — boundary-first implementation workflow;
- `docs/documentation-governance.md` — authority and archival rules;
- `docs/rework-roadmap.md` — current implementation sequence;
- `docs/audits/architecture-inventory.md` — current source evidence;
- `docs/audits/remaining-boundaries.md` — remaining dependency-boundary
  backlog;
- `docs/audits/product-surface-acceptance.md` — product and visual evidence;
- `tasks/rework/README.md` — the only active implementation queue;
- `handshake.md` — session continuity, not architecture authority.

The repository currently contains approximately 52 workspace crates and an
acyclic graph of approximately 219 internal edges. The former broad backend
facade has been removed. The shell is already largely composition-only, and
many capability owners and registries have already been migrated.

There is exactly one active rework task at a time:
`tasks/rework/R07-001-product-surface-acceptance.md` is active.
`tasks/rework/R07-004-explorer-host-contract.md` is planned and must not be
activated until R07-001 is genuinely complete. Historical files under
`tasks/archive/` are not an implementation queue.

The current worktree may contain an unfinished documentation-governance slice
including `scripts/check_documentation.py`, CI wiring, and updates to the
project instructions. Inspect the worktree first. Preserve, finish, verify,
and commit that slice before mixing it with later feature work. Do not discard
it merely because it is not a Rust source change.

## Non-negotiable product contract

The permanent product surface has exactly these zones:

1. **Titlebar:** tabs and one global menu button.
2. **Workspace:** active tab content and split panes.
3. **Docks:** registered panels attached to supported docks.
4. **Statusbar:** panel controls on the left and global information/status
   items on the right.
5. **Overlay layer:** command palette, dialogs, focused pickers, and other
   explicitly modal surfaces.

Do not add permanent global toolbars, duplicate badge rows, feature-specific
strips, or decorative shell chrome without an accepted ADR and a demonstrated
workflow that cannot fit an existing surface.

The following product decisions are binding:

- Settings stores configuration values only. It does not manage hosts, themes,
  icon themes, keymap editing, transfers, notifications, or command discovery.
- Keymap, Themes, Icon Themes, and Hosts are exposed from the titlebar global
  menu and/or command palette through their owning modules.
- Themes and icon themes use deterministic built-in registries initially.
  Selection previews immediately while navigating; Enter confirms and
  persists. Remote downloads, marketplace behavior, and extensions are
  deferred.
- Hosts have one dedicated management surface and one command-palette picker.
  Enter opens SSH; Shift+Enter opens SFTP. Jump hosts remain part of SSH
  connection configuration and execution, not a separate primary badge/menu.
- Notifications are persistent structured records in one registry and are
  displayed only in the statusbar notification dropdown. There is no toast
  renderer, timer-based passive popup, or duplicate passive inline error.
- Transfers are a first-class statusbar item with progress, history, and
  resolution actions.
- The command palette has one registry and one UI. Feature modules contribute
  commands and dynamic submenu snapshots; no static shell-wide feature table
  may return.
- Every meaningful action has a stable command identity and can receive a
  keybinding. Frequent workflows should ship defaults; rare actions remain
  palette-reachable even without a default binding.
- Every reusable button, input, list, menu, dropdown, badge, disclosure,
  dialog, tab, and similar control comes from `labonair-ui-kit`.
- Passive user messages go to Notifications. Inline UI may remain only when
  the user must make an immediate decision or correct input to complete the
  current operation.

## Non-negotiable architecture contract

### Ownership

Every product capability is one owning module with one canonical capability
crate. The owner owns its domain models, state, lifecycle, backend operations,
UI, persistence, migration, commands, keybindings, notifications, and tests.

Allowed sibling crates belong to the same module and require a real boundary:

```text
crates/<capability>/             # canonical contract and core behavior
crates/<capability>-ui/          # optional GPUI presentation boundary
crates/<capability>-storage/     # optional substantial persistence boundary
crates/<capability>-integration/ # optional external/platform adapter
```

Do not create a crate merely for symmetry, naming, or theoretical layering.
Create one only when core/UI, persistence, an external adapter, compilation
isolation, or a real multi-consumer contract justifies it. Do not create
empty facades or general-purpose feature crates.

### Dependencies

- `labonair` and `labonair-shell` are composition roots only.
- The composition root constructs concrete services, injects them, and calls
  registration functions. It does not own feature state, feature behavior,
  feature views, or aggregate backend state.
- Feature crates must not depend on `labonair-shell` or access another
  feature's private state.
- Foundation crates, especially `labonair-ui-kit`, must not depend on product
  modules.
- Use a direct typed contract for one caller/provider. Use a registry only
  when multiple providers or consumers genuinely need discovery.
- A public contract must expose stable domain values, IDs, commands, events,
  traits, or registration functions—not another feature's GPUI entities,
  storage schema, or private widget implementation.
- Every temporary compatibility path needs a named consumer and explicit
  removal condition. Delete it once the consumer moves.
- The dependency graph must remain acyclic and every non-trivial cross-module
  edge must be documented and checked by `scripts/check-crate-deps.sh`.

### Boundary-first implementation order

For every remaining migration, follow this exact order:

1. inventory current consumers and behavior;
2. define the typed target contract in the owning module;
3. add focused contract tests;
4. move owning state and behavior;
5. inject platform/storage implementations at composition;
6. migrate consumers to the public contract;
7. remove old registrations, imports, dependencies, and compatibility paths;
8. update capability matrix, architecture inventory, roadmap/task, and
   dependency rules;
9. verify code, behavior, and visual states;
10. commit the bounded task before activating the next one.

## What is already complete — do not redo it blindly

The current audit says these areas are structurally implemented and should be
verified or refined rather than rebuilt without evidence:

- documentation reset, product contract, module rules, registry rules,
  design system, settings contract, workspace model, and active queue;
- removal of the broad backend facade and migration of concrete backend
  responsibilities to capability/integration crates;
- owner-registered command metadata and typed command-palette runtime action
  handlers;
- command-palette provider registry and dynamic action forwarding;
- keymap core, JSONC document model, contexts, conflict handling, persistence,
  and dedicated keymap UI;
- Settings reduction to typed values, migration ownership, project/global
  scope handling, and removal of management categories;
- static color and icon-theme registries with transactional preview;
- dedicated Hosts management and typed SSH/SFTP picker actions;
- Notifications registry and statusbar dropdown with structured details,
  actions, read state, history, scrolling, and deduplication;
- removal of passive toasts and migration of passive operation errors to
  Notifications, while retaining only genuinely actionable dialogs/validation;
- Transfers registry, SFTP worker boundary, and statusbar transfer UI;
- removal of the dedicated Jump Hosts statusbar/menu surface;
- Workspace identity, project/standalone transitions, persisted session
  identity, and explicit return-to-standalone behavior;
- owner-registered status items and command contributions for Workspace,
  Terminal, Settings, Hosts, Themes, Updater, Git, Snippets, and related
  capabilities;
- updater core/UI separation and native predecessor-compatible manifest
  format without a Tauri runtime dependency.

Do not claim a feature is complete solely because a registry or crate exists.
Check real consumers, runtime behavior, and evidence in the audit.

## Remaining work and required execution order

### Step 0 — Finish the current documentation-control slice

Inspect any existing changes related to `scripts/check_documentation.py`.
Finish them if necessary. The check must:

- require Status and Version metadata on current normative documents;
- validate local Markdown links in current instructions, docs, ideas, and
  active tasks without treating historical archives as current authority;
- reject stale predecessor control markers in `.github` and `.vscode`;
- run in CI beside the active rework-queue check;
- be documented in `AGENTS.md`, `docs/documentation-governance.md`,
  `docs/feature-lifecycle.md`, and the pull-request template.

Do not make the checker reject qualified values such as `Normative target
architecture`; validate the `Normative` prefix. Run it and commit this slice
as an independent documentation change.

### Step 1 — Complete R07-001 product-surface acceptance

Use `docs/audits/product-surface-acceptance.md` as the working matrix. Verify
the actual native Rust application, not the old app. Cover every applicable
permanent surface in these states:

- normal;
- narrow window;
- focused/keyboard-navigation state;
- empty state;
- loading state;
- error state;
- long-list/scroll state;
- overlay/open-dropdown state.

At minimum inspect titlebar/global menu, tabs and empty workspace, docks,
statusbar, command palette, keymap, Settings, Hosts management and picker,
Themes, Icon Themes, Notifications, Transfers, Search, and Updater.

Specifically verify that the titlebar menu popover is anchored to the clicked
button's actual bounds and opens below that button, not at the opposite side of
the application. Verify keyboard navigation, Enter/Shift+Enter semantics,
scrolling, preview/confirmation, notification expansion, transfer actions, and
standalone/project transitions.

Record evidence in the matrix. If macOS Screen Recording permission prevents
capture, record the exact blocker and keep the visual criteria pending. Do
not mark the task complete based only on compilation. Ask for the permission
or use an authorized capture environment before closing the task.

R07-001 is complete only when every applicable visual cell is either verified
with native evidence or explicitly N/A with a documented reason.

### Step 2 — R07-004: remove the Explorer → Workspace view dependency

Activate `tasks/rework/R07-004-explorer-host-contract.md` only after Step 1.
The current evidence identifies `crates/panel-explorer/src/panel_explorer.rs`
as storing `Entity<Workspace>` and using Workspace callbacks for opening files,
terminals, previews, and drag/preview shims.

Implement the migration as follows:

1. inventory every Explorer-to-Workspace call, type re-export, and consumer;
2. define a narrow typed Explorer host/intents contract owned at the lowest
   sensible boundary;
3. move drag, preview, and other shared value types below both modules when
   they are truly shared;
4. make Explorer emit typed intents such as open-file, open-terminal, or
   open-preview without holding a Workspace GPUI entity;
5. inject the host implementation at composition time;
6. migrate local and remote Explorer consumers and preserve all required
   workflows;
7. remove the direct `panel-explorer → workspace` dependency and obsolete
   shims/re-exports;
8. update Cargo metadata, dependency allow-lists, capabilities, inventory,
   remaining-boundaries audit, and the task record;
9. add focused contract and behavior tests plus the required visual states.

Do not replace one direct Workspace entity with a new broad “workspace
service” facade. The contract must contain only Explorer's real intents.

### Step 3 — B02: remove Workspace → BackgroundStore coupling

After R07-004 is complete, create and activate the next bounded task for the
`workspace → background` edge. The current Workspace views directly hold an
`Entity<BackgroundStore>` to mount the background layer.

Define the smallest typed presentation capability needed by the Workspace
view, inject it at composition, and remove Workspace's dependency on the
Background entity/store. Backgrounds must continue to own image storage,
import/delete behavior, persistence, decoding, and rendering policy. Do not
move Background state into Workspace and do not create a generic visual
facade. Update the same contracts, dependency checks, tests, and visual
matrix as in Step 2.

### Step 4 — finish the Keymap platform boundary and action coverage

Audit the remaining `keymap-ui`/shell platform installation and file-watch
adapter. The final shape must keep parsing, validation, default layers,
contexts, conflict handling, persistence, and management UI in the Keymap
module. The shell may only provide a minimal platform/composition adapter and
must not reimplement loading, action-name aliases, validation, or registry
resolution.

Audit all user-visible actions across tabs, panes, docks, terminal, editor,
Explorer, SSH/SFTP, transfers, Git, snippets, Settings, Themes, Hosts,
Notifications, AI, and native window actions. Each meaningful action must
have one stable command ID, one owner, optional default binding, and palette
reachability. Remove duplicate legacy keymap tables and keep compatibility
aliases only for existing user files with an explicit removal condition.

Add focused tests proving registration, resolution, context precedence,
conflict reporting, display formatting, persistence, and command execution.

### Step 5 — finish Workspace, Terminal, and Editor ownership boundaries

Audit `crates/workspace`, `crates/terminal`, and `crates/editor` for remaining
feature behavior hidden in Workspace. Workspace should own workspace identity,
tabs, panes, docks, layout, transitions, and orchestration. Terminal should own
terminal state, PTY/session behavior, rendering support, input mapping, and
terminal-specific notifications. Editor should own document state, editing,
language behavior, Vim behavior, search, diff primitives, and editor-specific
notifications.

Extract only real boundaries. Prefer typed view factories, tab descriptors,
events, or service traits over passing broad GPUI entities or introducing
aggregate facades. Keep project/standalone behavior equivalent. Remove shell
conditionals and Workspace feature logic once consumers move. Add tasks before
each bounded migration and do not combine unrelated refactors.

### Step 6 — rebuild the AI UI boundary without a backend facade

AI core/provider/session/tool behavior remains owned by `labonair-ai`; concrete
MCP integration remains in its explicit integration crate. When rebuilding the
AI frontend, create an AI UI sibling only if the GPUI dependency boundary is
real. Define typed contracts for workspace context, streaming state, tool
approval, cancellation, session lifecycle, and user-facing notifications.

Workspace may orchestrate a typed AI bridge, but must not own AI state,
provider-specific logic, tool registries, or an AI error banner. Passive AI
errors go to Notifications; approval or correction dialogs remain only when a
user decision is required. Do not add speculative model marketplaces,
provider abstractions, or settings that have no current workflow.

### Step 7 — review B03–B06 without speculative splitting

Re-evaluate the remaining documented edges:

- Workspace → AI;
- Workspace → Settings;
- Source Control → Editor/Settings;
- Command Palette → Settings/Filesystem.

For each edge, inspect real consumers and decide one of:

- extract a narrow contract now if implementation detail or feature state is
  crossing the boundary;
- retain it if it is already a typed public-value/composition edge;
- defer extraction until a concrete second consumer or module rebuild exists.

Record the decision and evidence in `docs/audits/remaining-boundaries.md`.
Do not create facades or crates merely to make the graph look symmetrical.

### Step 8 — product-wide simplification and consistency pass

Search the active source tree for:

- duplicate command tables or stringly-typed action dispatch;
- toast renderers, timers, passive banners, and duplicate operation errors;
- Settings categories that actually belong to another owner;
- feature state or UI behavior in `labonair-shell` or a broad backend-like
  module;
- feature-local button/list/menu/dropdown/badge styling;
- obsolete Jump Hosts menu/badge code;
- hidden or unreachable Transfers UI;
- remote theme download/marketplace scaffolding without a current workflow;
- stale Tauri/frontend instructions in active controls;
- unowned capabilities or undocumented compatibility paths.

For every finding, remove it, migrate it, or record a bounded task with an
explicit owner and removal condition. Do not silently leave it because it is
old or difficult.

Run the full visual matrix again after all relevant changes. Check spacing,
colors, radii, typography, focus rings, icons, list density, overlays, and
popover anchoring against `docs/design-system.md` and the reference tokens.

## Registry requirements

Keep registries small, typed, and owner-driven:

- command registry: stable ID, title, aliases, category/submenu, contexts,
  default binding, and typed execution/action payload;
- keymap registry/runtime: IDs, contexts, precedence, conflicts, persistence,
  diagnostics, and immutable snapshots;
- theme registries: separate color and icon IDs, metadata, preview state, and
  confirmation persistence;
- panel/status-item registries: identity, placement, supported docks, and
  view factory;
- notification registry: kind, stable ID/source, title, summary, optional
  details, timestamp, read state, actions, and deduplication metadata;
- transfer registry: lifecycle state, progress, cancellation, conflicts,
  history, and typed service/event contracts.

Every registry must reject duplicate IDs, expose immutable snapshots, be
testable without launching the whole application, and have one owner. Adding
an entry must not require editing an unrelated central feature table.

## Documentation requirements

For every implementation task:

- update the owning capability row in `docs/capabilities.md`;
- update `docs/audits/architecture-inventory.md` when the source graph or
  ownership changes;
- update `docs/audits/remaining-boundaries.md` when an edge is removed,
  retained, or newly discovered;
- update `docs/rework-roadmap.md` and the active task record;
- update a normative document or ADR when a contract changes;
- document compatibility consumers and removal conditions;
- add evidence, blockers, and current/next-task information to
  `handshake.md`;
- add non-obvious bugs and GPUI/API discoveries to
  `memory/bugs_and_fixes.md`;
- keep historical material under `tasks/archive/` or `docs/archive/` and do
  not present it as current authority.

Do not create duplicate documents that define the same rule. Use the canonical
document map in `docs/documentation-governance.md`.

## Verification gates

Before marking any task done, run and pass all applicable checks:

```text
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
scripts/check-crate-deps.sh
python3 scripts/check_rework_queue.py
python3 scripts/check_documentation.py
git diff --check
```

For UI/layout work, run the native visual verification procedure in
`docs/visual-verification.md` and record native-bundle evidence in the
acceptance matrix. A compile-only result never proves visual completion.

If a test fails because the environment denies loopback listeners,
filesystem events, or Screen Recording, do not change product code to hide
the failure. Rerun in an authorized environment when possible and record the
exact environmental limitation in `memory/bugs_and_fixes.md` and
`handshake.md`.

## Task and collaboration protocol

- Work on exactly one active task at a time, in dependency order.
- Read the complete task before editing code.
- Do not activate a later task while an earlier task is incomplete.
- If subagents are used, give each a separate worktree or strictly isolated
  read-only investigation role. Never let multiple agents edit the same
  worktree concurrently.
- Keep diffs bounded and reviewable. Do not mix unrelated cleanup into a
  migration.
- Send concise progress updates that name the current task, files changed,
  verification state, and blocker; do not report plans as completed work.
- Commit each completed bounded task with a conventional commit message that
  includes the task ID. Do not push unless explicitly asked.

## Final acceptance criteria

The complete rework is accepted only when all of the following are true:

1. Every capability has exactly one documented owner, canonical crate, public
   entry point, persistence story, notification story, and migration status.
2. No broad backend facade or hidden aggregate feature service remains.
3. The application composition root only constructs, injects, and registers;
   it does not own feature state or duplicate feature registries.
4. Explorer no longer depends directly on Workspace, and Workspace no longer
   depends directly on `BackgroundStore` for presentation.
5. Remaining Workspace/AI/Settings/SCM/Palette edges are either narrowed or
   explicitly justified as typed public/composition edges.
6. Commands, dynamic palette submenus, keymap entries, panels, status items,
   themes, notifications, and transfers use owner registries without central
   duplicate tables.
7. Settings contains values only; Hosts, Themes, Icon Themes, Keymap,
   Notifications, and Transfers have their own canonical surfaces.
8. Notifications are the sole passive message surface; no toasts or duplicate
   passive inline errors remain.
9. Themes and icon themes are static, searchable, scrollable, previewable,
   and confirmable; remote downloads remain intentionally deferred.
10. Hosts are searchable, Enter opens SSH, Shift+Enter opens SFTP, and jump
    hosts are not presented as an independent primary feature surface.
11. Transfers are visible and usable from the statusbar.
12. Project and standalone workflows both work for local and remote tools.
13. All reusable controls use the shared UI kit and the visual language is
    consistent across permanent surfaces and overlays.
14. The titlebar menu is correctly anchored to the clicked button.
15. The full visual state matrix is complete or explicitly N/A with evidence;
    no blocked visual claim is marked verified.
16. All verification gates pass in an authorized environment.
17. Current documentation is authoritative, historical documentation is
    clearly archived, active tasks are ordered, and the final worktree is
    clean with a complete handshake and memory record.

When all criteria are proven, mark the relevant task records done, update the
roadmap and acceptance audit, commit the final bounded changes, and report
the exact completed tasks, remaining intentional deferrals, verification
commands, and any environment limitations. Do not declare completion merely
because the code compiles or because the current dependency graph looks clean.
