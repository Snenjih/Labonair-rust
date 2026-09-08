# Labonair Remaining Work Audit — Direct Follow-up Task

**Audit date:** 2026-09-08  
**Audit type:** Independent architecture, module-boundary, product-surface, and documentation review  
**Status:** Open — not a completion declaration  
**Repository:** `Labonair-rust` native Rust/GPUI application

## Direct instruction

Use this document as the verified follow-up list for the remaining Labonair
rework. Inspect the current source before every change. Implement the items in
bounded tasks and dependency order; do not merely update this checklist to
make it look complete. A finding is complete only when its source, contract,
consumer migration, tests, documentation, dependency graph, and visual
evidence agree.

Two independent read-only audits were performed: one focused on architecture
and module boundaries, the other on product surfaces, UX, and documentation.
The findings below were then compared against the current Cargo metadata,
source tree, active queue, and validators. Historical documents under
`tasks/archive/` and `docs/archive/` are not evidence of current completion.

## Current verified state

- The application is the native Rust/GPUI application in this repository.
  Never launch the old Tauri application with `open -a Labonair`.
- The broad `labonair-backend` workspace crate is absent.
- The workspace currently contains 54 crates and 222 internal dependency
  edges. The graph is acyclic and the current dependency verifier reports no
  untracked edges, but a green verifier does not prove that every allowed
  edge is architecturally desirable.
- `panel-explorer → workspace` is structurally removed through
  `labonair-explorer-host` in commit `44f30a2`.
- `workspace → background` is structurally removed through
  `labonair-background-host` in commit `a8a2ddb`.
- The active queue has 24 entries. The only active task is
  `R07-001-product-surface-acceptance.md`. R07-004 and R07-005 contain
  structural migrations that landed early but remain planned until their
  acceptance/evidence state is reconciled.
- `python3 scripts/check_documentation.py`,
  `python3 scripts/check_rework_queue.py`, and
  `bash scripts/check-crate-deps.sh` pass at audit time.
- The full workspace test command is not currently reliable inside the
  restricted sandbox: the AI loopback listener test can receive
  `Operation not permitted`, and the Settings rename-watcher test can miss a
  filesystem event. These are environmental failures already recorded in
  `memory/bugs_and_fixes.md`; they must be rerun in an authorized environment
  before final sign-off.

## Priority 0 — acceptance and truthfulness

### P0.1 Complete the native visual acceptance gate

**Status:** Confirmed open.

Evidence:

- `tasks/rework/R07-001-product-surface-acceptance.md` remains
  `🔄 In Progress`.
- `docs/audits/product-surface-acceptance.md` still marks the visual matrix
  pending.
- `docs/visual-verification.md` requires native-bundle evidence.
- macOS Screen Recording permission previously prevented capture.

Required work:

- Run and inspect only the native Rust application.
- Complete the matrix for titlebar/global menu, workspace/empty state, docks,
  statusbar, command palette, keymap, Settings, Hosts, Themes, Icon Themes,
  Notifications, Transfers, Search, Updater, and relevant standalone/project
  workflows.
- Cover normal, narrow, focused, empty, loading, error, long-list/scroll,
  and overlay states where applicable.
- Explicitly inspect titlebar menu placement, statusbar dropdown anchoring,
  palette preview/cancel/confirm, Host Enter/Shift+Enter, notification
  expansion, transfer actions, and keyboard navigation.
- Record native evidence and link it from the audit matrix. Mark a cell N/A
  only with a reason.

Acceptance criteria:

- No applicable visual cell remains falsely marked verified or silently
  pending.
- R07-001 is marked done only when every acceptance criterion is actually
  satisfied.
- If Screen Recording is still unavailable, record the exact external
  blocker and do not close the task.

### P0.2 Reconcile task/documentation truth

**Status:** Confirmed contradiction requiring audit.

The independent review found completed task files whose acceptance checkboxes
do not match their `Done` status, claims in the roadmap that are stronger
than the source evidence, and conflicting Settings-inventory statements about
consumer tests. R07-004 and R07-005 also need a deliberate status decision
because their structural work landed early while their visual evidence belongs
to R07-001.

Required work:

- Inspect every `tasks/rework/R*.md` status and every acceptance checkbox.
- Reconcile each status with current code, tests, dependency output, and
  visual evidence.
- Update `docs/rework-roadmap.md`, `docs/capabilities.md`,
  `docs/audits/architecture-inventory.md`,
  `docs/audits/remaining-boundaries.md`, and `handshake.md` wherever claims
  are contradicted.
- Do not mark a task done solely because its structural portion landed in an
  earlier commit.

Acceptance criteria:

- A `Done` task has no unchecked applicable acceptance criterion.
- Visual claims link to accepted native evidence.
- The Settings inventory does not simultaneously claim missing and complete
  consumer evidence.
- The active queue has one unambiguous next task.

## Priority 1 — confirmed ownership and boundary violations

### P1.1 Remove Theme knowledge from `labonair-ui-kit`

**Status:** Confirmed violation.

Evidence:

- `crates/ui-kit/Cargo.toml` depends on `labonair-theme`.
- `crates/ui-kit/src/theme.rs` imports concrete `ThemeStore`, `Theme`, and
  `RadiusScale` types.
- `crates/ui-kit/src/palette.rs` and `src/icon.rs` use theme-module types.
- `crates/ui-kit/src/gallery.rs` stores `Entity<ThemeStore>`.
- `scripts/check_crate_deps.py` currently permits the product dependency even
  though `docs/architecture.md` prohibits foundation-to-product coupling.

Required work:

- Keep the UI kit limited to reusable controls and foundation-owned semantic
  tokens/values.
- Move concrete Theme adapters and the theme-dependent gallery to the Theme
  module or to an explicitly shell-owned debug integration.
- If a semantic token contract is needed, place it below product modules and
  keep it free of ThemeStore/product state.
- Tighten the dependency verifier so `labonair-ui-kit` cannot depend on a
  product module.

Acceptance criteria:

- `crates/ui-kit/Cargo.toml` has no `labonair-theme` dependency.
- `rg "labonair_theme|ThemeStore|IconThemeContent" crates/ui-kit/src` finds
  no production product references.
- Reusable components still render using an explicit injected/foundation
  token contract.
- The component gallery remains available only through a documented Theme or
  debug integration.
- A dependency-verifier regression test rejects product dependencies from
  UI-kit.

### P1.2 Remove Workspace ownership of Hosts UI entities

**Status:** Confirmed violation.

Evidence:

- `crates/workspace/Cargo.toml` depends on `labonair-hosts-ui`.
- `crates/workspace/src/workspace.rs` stores or exposes
  `Entity<HostManagerView>` and `Entity<ConnectionStatusStore>` and calls
  Hosts UI mutation methods.
- `crates/hosts-ui/src/hosts.rs` owns the concrete UI entities and their
  `set_status`/`set_active_tunnels` operations.

Required work:

- Keep `HostManagerView`, `ConnectionStatusStore`, `ConnectionEntry`, and
  `HostStatus` UI behavior inside Hosts UI.
- Define a UI-free host/SSH status snapshot or event contract at the lowest
  sensible boundary.
- Inject only the narrow catalog/status/event capability required by
  Workspace. Do not replace the entity with a broad host service facade.
- Keep host management and picker semantics owned by Hosts.

Acceptance criteria:

- `labonair-workspace` no longer depends on `labonair-hosts-ui`.
- Workspace contains no `Entity<HostManagerView>` or
  `Entity<ConnectionStatusStore>` and cannot mutate Hosts UI state directly.
- Host lookup, connection state, and active-tunnel rendering still work via
  typed values/events.
- Focused tests cover status updates and host action routing.

### P1.3 Remove Workspace entity coupling from Snippets UI

**Status:** Confirmed violation and missing backlog item.

Evidence:

- `crates/panel-snippets/Cargo.toml` depends on `labonair-workspace`.
- `crates/panel-snippets/src/panel_snippets.rs` stores `Entity<Workspace>`.
- The panel directly calls Workspace for local execution, SSH execution,
  terminal injection, and active-session lookup.

Required work:

- Define an owner-neutral `SnippetExecutionHost`-style contract or equivalent
  narrow typed callbacks for only those intents.
- Inject the host from composition.
- Keep snippet state, persistence, and execution semantics in the Snippets
  module; keep Workspace orchestration outside the panel.

Acceptance criteria:

- `panel-snippets` has no `labonair-workspace` dependency.
- `SnippetsView` contains no `Entity<Workspace>`.
- Local execution, remote execution, terminal injection, and active-session
  lookup retain their behavior.
- Focused contract tests exercise execution without constructing Workspace
  entities.

### P1.4 Remove Workspace entity coupling from Transfers UI

**Status:** Confirmed violation.

Evidence:

- `crates/transfers-ui/Cargo.toml` depends on `labonair-workspace`.
- `crates/transfers-ui/src/status_item.rs` stores `Entity<Workspace>` and calls
  `Workspace::refresh_sftp_after_transfer`.
- `docs/registries.md` says Transfers UI should emit a typed completion signal
  rather than own Workspace refresh wiring.

Required work:

- Make Transfers UI emit a typed completion signal or accept one narrow
  injected callback.
- Move SFTP-pane refresh wiring to composition/Workspace orchestration.
- Keep transfer lifecycle, progress, history, and resolution actions in
  Transfers.

Acceptance criteria:

- `transfers-ui` has no Workspace dependency or Workspace entity field.
- Transfer completion still refreshes the affected SFTP pane.
- A focused test proves completion dispatch without constructing Workspace.
- Registry documentation and Cargo graph describe the same boundary.

### P1.5 Move Theme management out of Settings UI

**Status:** Confirmed ownership leak.

Evidence:

- `crates/settings-ui/src/command_provider.rs` receives `Entity<ThemeStore>`
  and registers theme-related handling.
- `crates/settings-ui/src/apply.rs` contains theme metrics application,
  preview, activation, and persistence behavior.
- `crates/settings-ui/src/view.rs` re-exports/uses `ThemeStore`.
- Current contracts say Theme owns theme selection and Settings edits values
  only.

Required work:

- Move theme preview, activation, cancel/revert, persistence, and theme
  preference application into `labonair-theme` or a Theme-owned UI sibling.
- Let Settings UI edit only typed settings values.
- Keep a narrow value/configuration contract if Theme needs settings values;
  do not let Settings UI own ThemeStore policy.
- Update command and palette registration so Theme registers Theme actions.

Acceptance criteria:

- Settings UI has no theme preview/activation/persistence policy and does not
  mutate `ThemeStore`.
- Theme-owned tests cover preview, cancel/revert, confirmation, and
  persistence.
- Theme command handlers are registered by the Theme owner.
- Settings remains a values-only management surface.

## Priority 1 — command and keymap correctness

### P1.6 Eliminate active legacy keymap tables

**Status:** Confirmed partial migration and documentation contradiction.

Evidence:

- `crates/keymap/src/lib.rs` still defines and uses `SHORTCUTS`,
  `ShortcutId`, shortcut lookup, conflict lookup, and effective bindings.
- `crates/keymap/src/command_provider.rs` still publishes `ShortcutId`.
- `crates/workspace/src/command_provider.rs` duplicates default-binding
  information through `ShortcutId`.
- `crates/shell/src/commands.rs` reads legacy shortcut information.
- `crates/command-palette/src/palette.rs` imports `ShortcutId`.
- Default keymap assets still describe the old table as a source.
- Current roadmap/docs claim the legacy table is no longer authoritative.

Required work:

- Make `CommandId` and owner `CommandDescriptor` metadata the sole source
  for new command rows, defaults, and keymap-management data.
- Keep legacy `ShortcutId`/`SHORTCUTS` only inside an explicitly isolated
  migration adapter for existing user files, or remove it after migration.
- Remove duplicated default-binding literals and shell/palette reads.
- Add a source/contract test that prevents non-migration use of the legacy
  table.

Acceptance criteria:

- New palette rows and keymap UI do not read `SHORTCUTS`.
- `ShortcutId` is referenced only by the documented compatibility adapter.
- No default binding is defined twice across keymap, shell, and providers.
- Current documentation accurately describes the remaining migration path.

### P1.7 Remove visible no-op command affordances

**Status:** Confirmed missing execution coverage.

Evidence:

- `crates/workspace/src/command_provider.rs` exposes Zoom In, Zoom Out, and
  Zoom Reset.
- `crates/shell/src/commands.rs` explicitly calls Zoom and Format Document
  actions not-yet-wired.
- Shell tests currently assert that at least `ZoomIn` can resolve without a
  handler.
- `FormatDocument` is exposed by `crates/editor/src/command_provider.rs` but
  has no editor execution handler.

Required work:

- Prefer implementing Workspace-owned Zoom In/Out/Reset for the focused
  supported surface and an Editor-owned Format Document action if semantics
  are defined.
- If an action cannot be implemented yet, remove it from the normal menu,
  palette, and default keymap instead of presenting a no-op.
- Classify navigation-only actions explicitly so they are not treated as
  executable keymap actions.

Acceptance criteria:

- Every visible actionable command has an owner handler and a focused
  execution test, or is absent from the visible surface.
- Zoom changes and reset the documented focused-surface scale.
- Format Document either formats a supported document with notification-based
  failure handling or is removed until supported.
- No normal palette/menu/keymap route silently does nothing.

## Priority 2 — UI consistency and narrower boundaries

### P2.1 Standardize popover anchoring on trigger bounds

**Status:** Confirmed UX defect plus systemic partial compliance.

Evidence:

- `crates/shell/src/titlebar.rs` uses a hard-coded `HEADER_H` y-coordinate and
  click-derived x-coordinate instead of the rendered trigger bounds.
- `crates/ui-kit/src/context_menu.rs` provides a bounds-based positioning
  contract using `bounds.bottom_left()`.
- Notifications (`crates/notifications/src/status_item.rs`), Agent Access
  (`crates/workspace/src/status_items.rs`), and Settings selects
  (`crates/settings-ui/src/panes/generic.rs`) also have click-coordinate
  callers.

Required work:

- Measure and pass the actual trigger bounds to the shared popover API.
- Audit all reusable dropdown/select/context-menu callers and standardize on
  bounds-based anchoring.
- Verify right-edge and narrow-window collision behavior.

Acceptance criteria:

- Clicking any point inside the titlebar trigger produces one stable menu
  anchor below that trigger.
- The menu remains visible within the native window at normal, narrow, and
  right-edge placements.
- Statusbar dropdowns and Settings selects use trigger bounds or have a
  documented owner-specific exception.
- Focused tests and native visual evidence cover these states.

### P2.2 Close UI-kit input compliance gaps

**Status:** Confirmed partial.

Evidence:

- Hosts search is manually rendered in `crates/hosts-ui/src/hosts.rs`.
- Hosts form fields use custom `labelled_field`/`tunnel_field` editing.
- Settings search, Snippet editor fields, SFTP rename, and tab rename use
  local text-entry behavior rather than the shared UI-kit primitive.

Required work:

- Use the shared UI-kit input/text-field control for generic search and text
  entry where its semantics fit.
- For domain-specific custom fields, either migrate them or document a
  formal exception with owner, reason, tests, and removal condition.
- Preserve quick-connect parsing, validation, paste, selection, Escape, and
  keyboard behavior.

Acceptance criteria:

- Every reusable search/text-entry field uses UI-kit controls or has a
  documented approved exception.
- Focused, invalid, empty, paste, deletion, and keyboard states are tested.
- No feature introduces a second generic input visual style.

### P2.3 Narrow raw persistence and secret handles in feature UI

**Status:** Partial boundary leakage; not the old backend facade.

Evidence:

- `HostManagerView` stores raw `Database` and `SecretsState` and performs
  CRUD/credential persistence directly in `crates/hosts-ui/src/hosts.rs`.
- `SnippetsView` stores raw `Database`.

Required work:

- Keep SQL and secret operations in owner stores or injected adapters.
- Expose narrow domain service traits/snapshots to feature UI when a real
  boundary is needed.
- Do not add a generic repository/backend facade.

Acceptance criteria:

- UI constructors accept domain contracts rather than storage schemas/raw
  secret state where the boundary is extracted.
- SQL and keychain/secret handling remain inside owner storage/adapters.
- No storage schema type leaks through UI-facing APIs.

### P2.4 Remove backend-shaped aggregate compatibility names

**Status:** Partial architectural proof weakness.

Evidence:

- `AppComposition` in `crates/shell/src/composition.rs` aggregates runtime
  capabilities and exposes `Deref`.
- `crates/app/src/main.rs` names the composition bundle `backend` in places.
- SSH transport adapters retain `BackendSsh*` names.
- Active comments still refer to deleted backend paths or legacy event streams.

Required work:

- Keep `AppComposition` as a composition-only construction bundle, but remove
  broad dereference access and misleading backend naming where practical.
- Rename concrete adapters to capability-specific names.
- Remove stale active comments; preserve historical terminology only in
  archives or clearly labeled migration notes.

Acceptance criteria:

- No feature constructor accepts `AppComposition` as a general service.
- Composition exposes explicit capability accessors/injection only.
- No broad backend facade or backend-shaped feature owner reappears.
- Active comments accurately describe the native architecture.

### P2.5 Give Notification callback compatibility a bounded end

**Status:** Partial migration.

Evidence:

- `crates/notifications/src/notifications.rs` still stores callback closures
  for actions.
- `docs/registries.md` permits this only as a temporary adapter, but no
  separate active task/removal condition exists.

Required work:

- Add a bounded follow-up task for stable notification action IDs and typed
  payloads.
- Isolate callback support in a compatibility adapter.
- Route new action dispatch through the owning module or command registry.

Acceptance criteria:

- Persistent notification records contain no closures.
- New notifications use stable action IDs and typed payloads.
- The callback adapter has a named consumer and explicit removal condition.
- Notification history, details, read state, deduplication, and actions stay
  intact.

## Priority 2 — deferred or incomplete product paths

### P2.6 Decide the fate of dormant theme/import/watch infrastructure

**Status:** Confirmed product-scope ambiguity.

Evidence:

- `crates/theme/src/registry.rs` contains optional file-loading paths.
- `crates/theme/src/import.rs` and `crates/theme/src/store.rs` contain
  import/export functionality.
- Settings file-watch support exists even though the current product contract
  emphasizes deterministic built-in theme catalogs and does not expose a
  marketplace/download flow.

Required work:

- Confirm that no current workflow needs these paths.
- Remove unused paths, or quarantine them behind an explicitly deferred
  boundary with no current UI claim and a named activation/removal condition.
- Keep built-in color/icon registries and preview/confirmation behavior.

Acceptance criteria:

- Current product docs clearly say what is supported and what is deferred.
- No active UI promises remote downloads, marketplace, or unsupported import
  behavior.
- Retained dormant code has a named owner and removal/activation condition.

### P2.7 Verify standalone/project UX completeness

**Status:** Partial and visually unverified.

The typed workspace identity, persistence, and transitions exist, but source
tests do not prove complete native keyboard-and-mouse workflows.

Required work:

- Verify empty, standalone, project, local, remote, restored-session, and
  return-to-standalone workflows in the native app.
- Ensure the most important empty-state actions are mouse-complete as well as
  keyboard-reachable.
- Keep project identity separate from incidental terminal cwd.

Acceptance criteria:

- A user can start the primary standalone and project workflows with keyboard
  and mouse.
- Local and remote paths are covered.
- Empty/loading/error/restored states are recorded in the visual matrix.

### P2.8 Keep AI intentionally scoped, then rebuild its UI on owner contracts

**Status:** Core retained; UI explicitly incomplete/deferred.

Required work when AI UI work is started:

- Keep provider/session/tool state in `labonair-ai` and concrete MCP behavior
  in its integration crate.
- Define typed contracts for workspace context, streaming, approval,
  cancellation, session lifecycle, and notifications.
- Create an AI UI sibling only for a real GPUI boundary.
- Do not put AI state, provider logic, or chat error banners in Workspace or
  shell.

Acceptance criteria:

- AI UI has one owner and no aggregate backend dependency.
- Passive AI failures use Notifications; approval/correction dialogs remain
  only where the user must decide.
- AI work is either implemented against the contracts or explicitly parked
  with a current product decision.

## Explicitly checked and currently accepted as structurally present

These areas must not be rebuilt without new contradictory evidence:

- Settings values-only model and removal of Hosts/Themes/Icon Themes/Keymap
  management categories.
- One command-palette registry with owner contributions and duplicate-ID
  checks, subject to the legacy keymap and no-op command findings above.
- Static built-in color and icon-theme catalogs with preview/confirmation
  structures, subject to Theme ownership and visual evidence above.
- Dedicated Hosts management and palette picker semantics, including Enter for
  SSH and Shift+Enter for SFTP, subject to Workspace/Hosts UI decoupling and
  widget compliance above.
- Notification registry/statusbar dropdown architecture with no active toast
  renderer found; callback compatibility and visual evidence remain open.
- Transfer lifecycle registry and statusbar history UI; Workspace refresh
  coupling remains open.
- Explorer and Background structural host contracts; only their visual
  evidence and task-status reconciliation remain open.
- Project/standalone identity and session persistence model; native workflow
  evidence remains open.
- AI UI is intentionally parked; do not add a speculative permanent AI
  surface.
- No evidence of a separate Jump Hosts primary badge/menu or remote theme
  download UI in the active product surfaces.

## Required execution order

1. Reconcile the current worktree and finish/commit any existing
   documentation-governance changes without discarding user work.
2. Complete or unblock R07-001's native visual acceptance and reconcile all
   task/documentation claims.
3. Create bounded tasks for P1.1–P1.5 and execute one at a time in dependency
   order. Do not reopen already-completed Explorer/Background migrations
   unless new evidence shows regression.
4. Complete keymap/command correctness (P1.6–P1.7).
5. Execute the popover and UI-kit consistency work (P2.1–P2.2).
6. Narrow persistence/secret handles, aggregate naming, and notification
   callbacks only where the real boundary justifies it (P2.3–P2.5).
7. Resolve dormant deferred paths and verify standalone/project UX (P2.6–P2.7).
8. Rebuild AI UI only after the workspace/service contracts are stable (P2.8).
9. Run the complete product-surface and visual audit again, then perform the
   final completion audit against every criterion in `your-rework-task.md`.

## Required task discipline

- Maintain exactly one active task in `tasks/rework/`.
- Every new task must define owner, canonical crate, user flow, public
  contract, registry contributions, settings, persistence, notifications,
  UI-kit components, dependencies, tests, visual evidence, and removal
  conditions.
- Move contracts before adapters and consumers. Delete old paths once no
  consumer needs them.
- Do not solve a boundary violation by introducing a larger service facade.
- Do not use parallel writers in the same worktree. Read-only audits may run
  in parallel, but source changes must be integrated and reviewed centrally.
- Update the capability matrix, architecture inventory, boundary backlog,
  roadmap, task status, handshake, and memory whenever the evidence changes.

## Definition of done for this follow-up

This follow-up is complete only when:

- every P0/P1 item is resolved or explicitly deferred with an owner and
  removal/activation condition;
- every remaining dependency edge is intentional, typed, and documented;
- no UI-kit product dependency, direct feature-UI entity coupling, active
  legacy keymap authority, or visible no-op action remains;
- Settings, Themes, Hosts, Notifications, Transfers, command palette, keymap,
  standalone/project workflows, and AI scope match the product contract;
- the native visual matrix is complete or explicitly N/A with evidence;
- documentation and task statuses agree with current source behavior;
- the complete verification suite passes in an authorized environment:

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

- the final worktree is clean, changes are committed with conventional commit
  messages, and `handshake.md` records the exact next state.
