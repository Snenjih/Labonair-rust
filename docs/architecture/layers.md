# Architecture Layers

**Status:** Normative
**Version:** 1
**Related:** [`../architecture.md`](../architecture.md), [`graph.toml`](graph.toml), [`dependency-rules.md`](dependency-rules.md)

This document defines the allowed direction of the Labonair crate graph. The
machine-readable edge and ownership source is [`graph.toml`](graph.toml); this
document explains how agents should interpret it.

## Layers

| Layer | Responsibility | May depend on |
|---|---|---|
| `application` | Process startup, native shell composition, window actions, and service wiring. | Product, integration, contract, and foundation crates through explicit composition edges. |
| `product` | User-facing capabilities, workspace orchestration, panels, and feature presentation. | Foundation contracts/services and other capabilities only through typed contracts, registries, or events. |
| `integration` | Concrete protocol, process, filesystem, or external-service adapters. | UI-free capability contracts and foundation services. |
| `foundation` | Reusable values, contracts, UI primitives, persistence, errors, and platform services. | External libraries and lower-level foundation crates; never product modules. |

The layer label describes the crate's architectural role, not the ownership
module. A product owner may therefore have a foundation contract sibling and
an integration sibling while retaining one canonical capability owner.

## Direction rules

1. Dependencies point from application to product/integration/foundation and
   from product or integration toward foundation contracts.
2. Foundation crates never depend on product crates, the shell, or the binary.
3. Product crates never depend on `labonair-shell` or reach into another
   feature's private state.
4. Integration crates implement typed contracts; they do not become a second
   owner of product state.
5. `labonair-ui-kit` is reusable foundation infrastructure. It may consume
   design tokens, but it must not know product modules.
6. A dependency that cannot be explained by the owning capability, a typed
   contract, a registry, or composition wiring is a boundary defect.

## Current graph

The generated projection at
[`../generated/architecture.md`](../generated/architecture.md) lists every
workspace crate and direct internal edge. Run
`python3 scripts/verify.py --scope architecture` after changing a Cargo
dependency or ownership record.
