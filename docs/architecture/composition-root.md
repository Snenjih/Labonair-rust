# Composition Root Contract

**Status:** Normative
**Version:** 1
**Related:** [`../architecture.md`](../architecture.md), [`runtime-lifecycle.md`](runtime-lifecycle.md), [`../feature-lifecycle.md`](../feature-lifecycle.md)

`labonair` and `labonair-shell` are the only all-feature composition roots.
They assemble concrete services and connect typed contracts; they do not own
feature behavior.

## Responsibilities

- construct the GPUI application and native window shell;
- load settings, themes, keymap, persistence, and platform services;
- construct integration adapters such as SSH, SFTP, Git, transfers, and MCP;
- inject those adapters into owner-defined contracts;
- register panels, commands, status items, notifications, and palette
  providers through their canonical registries;
- connect typed events between independently owned modules;
- select the initial workspace and recoverable startup state.

## Prohibited responsibilities

The composition roots must not:

- store feature state or duplicate feature persistence;
- render feature-specific views or implement capability business logic;
- contain a feature-name switch for command, panel, status, or notification
  behavior;
- expose a backend facade that every feature calls instead of its owner;
- add a second settings, notification, command, or surface registry;
- reach into private module state when a typed contract or event is required.

## Construction rule

For every injected service, the owning task and capability record must identify
the contract, implementation adapter, construction site, and failure behavior.
The adapter belongs in the integration or composition boundary; the contract
belongs to the owning capability or a deliberately shared foundation crate.

The preferred flow is:

```text
configuration and platform services
        ↓
typed capability contracts
        ↓
owner registration and feature views
        ↓
typed events back through the composition root
```

If construction requires a new permanent shell surface, first create or
update an ADR and prove that an existing product surface cannot host it.
