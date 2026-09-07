# Repository Layout

**Status:** Normative
**Version:** 1

This document defines where product code, contracts, documentation, and
migration work belong. It is a structural rule, not a description of every
file that currently exists. The current exceptions are recorded in the
[architecture inventory](audits/architecture-inventory.md).

## Top-level layout

```text
Labonair-rust/
├── crates/
│   ├── app/                    # binary and process entry point
│   ├── shell/                  # permanent chrome and composition wiring
│   ├── <capability>/           # canonical product capability crate
│   ├── <capability>-ui/        # optional sibling UI boundary
│   ├── <capability>-storage/   # optional sibling persistence boundary
│   ├── <capability>-integration/ # optional platform/protocol boundary
│   └── foundation crates       # reusable contracts and infrastructure
├── docs/                       # normative contracts, ADRs, audits, reports
├── ideas/                      # non-normative proposals only
├── tasks/rework/               # active migration queue
├── tasks/archive/              # historical task records
├── reference-src/              # frozen predecessor reference; read-only
└── scripts/                    # repository checks and tooling
```

`target/`, generated output, and local runtime data are build or machine
state; they are not product source and must not become architectural
dependencies.

## Crate placement

Every product capability has one canonical crate under `crates/`. The crate
owns the capability even when it is later split into siblings. For example,
`themes` may contain `labonair-theme` and a future
`labonair-<capability>-ui` sibling, but there is still one Themes owner and one
public capability contract.

Create a sibling crate only for a real boundary: UI versus UI-free logic,
independent persistence, platform integration, or a stable contract consumed
by multiple modules. Do not create empty `core`, `api`, or `facade` crates to
make the tree look layered.

Foundation crates may be shared by product modules but may not contain
product-specific state or depend upward on a capability. `labonair-ui-kit`
contains reusable interaction and visual primitives; it does not know about
Hosts, Themes, Transfers, or Workspaces.

`labonair-shell` and `labonair` are the only all-feature composition layers.
They may construct concrete services, invoke registration functions, and
connect typed events. They must not become the home of feature state,
feature-specific rendering, or a second command/panel/status registry.

## Capability crate contents

A small capability may remain a single `src/lib.rs`. A larger capability may
organize private implementation into files such as:

```text
crates/<capability>/src/
├── lib.rs          # public contract and module exports
├── domain.rs       # stable values and identifiers
├── state.rs        # lifecycle and state transitions
├── contract.rs     # traits and typed events for consumers
├── registry.rs     # only when discovery has multiple providers/consumers
├── commands.rs     # command/keymap contributions owned by the capability
├── view.rs         # GPUI view when UI belongs in this crate
├── storage.rs      # capability-owned persistence when needed
└── integration.rs  # platform/protocol adapter when needed
```

These names are optional organizational tools, not a requirement to create
files without a current responsibility. The owner keeps domain state,
behavior, UI, persistence, commands, notifications, and tests together unless
a sibling boundary is justified.

## Cross-module placement

- Stable domain values, traits, and typed events belong in the owning
  capability's public contract or a clearly named foundation contract.
- Feature behavior belongs to the owning capability, never to `shell`,
  `workspace`, or `backend` merely because those crates can reach it.
- Reusable controls belong in `ui-kit`; feature views compose them.
- Platform and storage implementations are injected at the composition root
  through narrow contracts.
- Compatibility adapters stay next to the boundary they bridge and must have
  a documented removal condition.

## Documentation placement

- `docs/*.md`: current normative contracts and implementation roadmap.
- `docs/adr/`: accepted architectural decisions and their consequences.
- `docs/audits/`: evidence about the current, unfinished tree.
- `docs/reports/`: research and comparisons; never the source of authority.
- `docs/archive/`: superseded documents retained for traceability.
- `ideas/`: proposals that are not binding until accepted into `docs/` or an
  ADR.
- `tasks/rework/`: the only active implementation queue for the architecture
  rework.

When a document changes an ownership or dependency rule, update the relevant
normative contract, capability matrix, migration task, and inventory in the
same change.
