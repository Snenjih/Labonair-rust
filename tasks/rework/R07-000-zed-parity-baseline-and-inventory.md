# R07-000 — Specify clean-room parity from a pinned baseline

## Status

`🔄 In Progress`

## Owner

- Module: product architecture with all capability owners
- Capability-matrix row: full Zed feature and UI/UX parity
- Canonical contract: [`../../docs/zed-parity.md`](../../docs/zed-parity.md)
- User entry point: N/A — specification and planning only

## Dependencies

- `R06-001-backend-adapter-eradication` (complete)

## Goal

Produce the first complete, source-independent Labonair specification for the
pinned Zed product surface. Settings is the first detailed pilot because four
previous attempts did not reach the visual target. This task defines and
measures the target; implementation starts only in a follow-up task built
from the accepted specification.

## Scope

- In scope: reproducible reference acquisition, source-boundary protocol,
  capability and surface inventory, glossary, universal UI checklist,
  command/keymap and Settings crosswalk, current-state ownership map,
  architecture decisions, macOS visual baseline, and dependency-ordered
  implementation tasks.
- In scope: identify and compare the four reported attempts at Settings using
  their commits and visual artifacts; record verified reasons for each miss.
- First deliverable: a paired Settings baseline for the pinned Zed build and
  current Labonair build, with matching macOS window bounds, theme, scale,
  fonts, and capture conditions. Record navigation and one representative
  category down to labels, controls, dimensions, focus/keyboard behavior, and
  states.
- Initial history leads to verify against the four reported attempts:
  `c9075e9` (2026-10-07), `4cca14a`, `f197d32`, and `3aa2327` (2026-09-25).
  These are candidates from the Settings UI history, not yet confirmed as the
  exact four attempts. The only tracked image under `shots/` is
  `shots/labonair.png`, which shows the SFTP surface and cannot serve as
  Settings comparison evidence.
- Next deliverable: reconcile all four reported Settings attempts against
  their exact commits and available captures; label causes as verified or
  unknown, then produce the complete Settings field crosswalk and measurable
  view-state checklist.
- Broader deliverable: apply the same inventory record to every surface,
  action, capability, extension point, and workflow in the pinned build.
- Out of scope: product code changes, copying Zed code or assets, declaring
  visual parity without paired native evidence, and silently reusing the
  existing source-level reports as implementation requirements.

## Reference and clean-room boundary

- Pin `zed-refrence/zed` to
  `3569541038dd51524b03998ba4d38d253cb54f80` (`nightly-9`).
- Use macOS as the first paired visual and interaction acceptance platform.
- Compare public documentation with the pinned runtime and record drift.
- Make the local gitlink reproducible on a clean checkout; the parent
  repository was missing its `.gitmodules` entry. The mapping is committed in
  `bf70e41`; a fresh parent clone at that commit successfully acquired the
  pinned submodule.
- Store source-level inspection only with reference analysis. Produce a
  separate implementation packet with independently worded observable
  requirements; do not give the implementation role the Zed checkout or
  source-level comparison reports.

## Required inventory entries

Every pinned-reference capability and every current Labonair capability must
be mapped to an owner, canonical user entry point, contract, relevant
commands/keybindings, settings, persistence, views, visual states, extension
points, external services, and acceptance evidence. Use the feature-record
template and status vocabulary in `docs/zed-parity.md`.

The inventory explicitly covers the universal Tab/Split model, shared
dropdown/menu and control system, Settings, Editor and language servers,
language/theme extensions, Explorer, Terminal, Git/Source Control, Git Graph,
universal DiffView, AI/agent, remote development, collaboration, platform
support, and hosted workflows present in the pinned build.

## Work order

1. Record exact Zed and Labonair revisions, macOS version, window bounds,
   theme, scale, fonts, and capture method. Verify the local reference pin and
   document any build/capture blocker without marking evidence complete.
2. Complete the Settings pilot first: paired surface captures, navigation and
   category inventory, field-by-field crosswalk, interaction states, and a
   review of the four prior attempts with evidence-linked findings.
3. Inventory every pinned-reference capability, action, surface, setting,
   extension point, and workflow; compare public docs to the pinned build.
4. Create Labonair's independent glossary, owner crosswalk, reusable UI
   checklist, and universal tab/split/dock/menu/popup/component contracts.
5. Turn the accepted specification into bounded implementation tasks ordered
   by actual contract and crate dependencies; reconcile the R09 Editor tasks.

## Acceptance criteria

- [ ] The pinned reference and macOS capture environment are reproducible,
      or remaining blockers are named with evidence and an owner.
- [ ] The Settings pilot has paired captures with matching conditions, or
      capture blockers are documented and the text/interaction inventory is
      complete to the extent possible without those captures.
- [ ] All four previous Settings attempts are linked to exact commits and
      artifacts; verified causes and unknowns are stated separately.
- [ ] Every feature and workflow in the pinned baseline has a Labonair
      crosswalk; no current `defer` silently removes a reference capability.
- [ ] The independent glossary, universal checklist, and feature record
      template cover all owners and surface types.
- [ ] The universal Tab/Split and DiffView decisions have accepted typed
      contracts, including movement, focus, close, restore, and failure
      behavior.
- [ ] Shared popup/menu and UI control requirements have measurable geometry,
      input, and visual evidence criteria.
- [ ] Every Settings key and visible field in the pinned baseline has a
      mapped owner, type, default, scope, control, effect, and paired evidence.
- [ ] Implementation work packages are ordered by actual contract and crate
      dependencies; R09 Editor work is reconciled and remains unstarted until
      this task completes.
- [ ] License, asset, service, and distribution questions have named review
      items; this task makes no unsupported legal-clearance claim.

## Verification and evidence

- `python3 scripts/check_documentation.py`
- `python3 scripts/check_rework_queue.py`
- `git diff --check`
- Evidence: pinned commit and reproducible acquisition record, complete
  feature crosswalk, Settings pilot and prior-attempt review, paired macOS captures,
  glossary/checklist coverage, and ordered implementation task graph.

## Change contract

```text
Request type: Reference-baseline and product-specification work
Owner module: Product architecture, with each capability owner responsible for its crosswalk
Canonical capability crate: N/A; this task changes contracts and research records, not runtime behavior
Canonical user entry point: N/A; no user-facing implementation in this task
Public typed contract: docs/zed-parity.md and the bounded, source-independent follow-up tasks
State/persistence owner: Unchanged; inventory must identify each reference behavior's Labonair owner
Commands and keymap entries: Inventory all reference actions and map them to current or planned owner contributions
Events and registry contributions: Specify multi-owner contracts only where discovery is required
Settings: Record every pinned setting, scope, default, owner, control, and observable effect
Notifications: Inventory action feedback and assign it to the owning surface/notification contract
UI-kit components: Record reusable control geometry and interaction requirements; do not add product controls here
Allowed dependency changes: Parent .gitmodules mapping for the pinned Zed gitlink only
Tests: Documentation, active-queue, and diff checks; no product tests are added by this specification task
Visual evidence: Paired native Settings captures are required; currently pending because native-window inspection/capture is unavailable in this session
Security impact: Inventory extension, remote, collaboration, agent, and hosted-service permission boundaries; no runtime policy change
Performance impact: Record measurable targets and fixtures in the parity catalog; no runtime performance claim
Removal condition for compatibility code: N/A; no compatibility path is introduced
```

## Progress record — 2026-10-09

- Verified the local Zed checkout at the exact pinned commit and clean tree.
- Built `target/debug/zed` successfully from that checkout with
  `mise exec cmake@4.4.3 -- cargo build --locked -p zed`. The linker emitted
  an `__eh_frame` size warning; the binary has not been opened for native
  visual inspection.
- Added the missing parent `.gitmodules` URL mapping. A fresh-parent-clone
  acquisition check passed after commit `bf70e41`: a fresh parent clone ran
  `git submodule update --init --depth 1 -- zed-refrence/zed` and checked out
  `3569541038dd51524b03998ba4d38d253cb54f80`.
- In that clean clone, the documentation check passed (43 normative documents,
  159 Markdown files) and the queue check passed (46 tasks; R07-000 active).
  The original shared worktree's pre-existing `reference-src/` deletions still
  make its documentation check fail on missing link targets.
- Recorded the local macOS version (`26.7.1`) and exact Labonair parent
  revision (`c42d33fae184b98c90516d747bfbb4844eb21992`). The Zed runtime
  binary, matching window bounds/theme/scale/fonts, and paired captures are
  not yet recorded.
- Confirmed the four Settings-history candidate commits and reviewed their
  code changes. No Settings capture exists for any candidate; visual miss
  causes remain unknown.
- Added a draft coverage crosswalk. It does not yet enumerate every pinned
  action, setting, extension, and workflow and must not be treated as complete.
- Added an independent Labonair glossary, reusable UI acceptance checklist,
  and proposed Tab/Split and DiffView lifecycle contracts. The proposed
  contracts still need owner review and typed API decisions.
- Recorded the pinned Settings navigation and all 70 visible Editor setting
  labels/keys from the research pass, plus representative control types and
  source-declared window dimensions. Full defaults/effects/control mapping and
  rendered interaction states remain pending.
- Named the legal, asset, extension-distribution, hosted-service, remote, and
  release review questions and their proposed review owners.
- Documented the source-blind checkout procedure, but that checkout has not
  been created or audited yet.
- The current CUA session reports no application surfaces (`apps: []`); its
  documented native entry points fail at runtime (`cua.getApp is not a
  function`; `cua.computer` is undefined).
  The prior native acceptance log also records macOS denying
  Screen Recording to the runner. Settings visual evidence remains pending;
  the workspace host operator owns restoring native-window access and the
  macOS Screen Recording grant for the runner.

The task remains `In Progress`; no product code was changed.

## Exit condition

This task is complete only when the full feature inventory has no unclassified
areas, all prior Settings attempts have an evidence-based disposition, the
source-blind implementation packet is ready, and the next implementation task
can begin without inventing a missing product or ownership decision. If native
capture access remains blocked, the blocker and owner must be explicit; visual
parity itself cannot be claimed from incomplete evidence.
