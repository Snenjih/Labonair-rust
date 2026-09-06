# R01-007 — Remove the Hosts UI dependency on the backend facade

## Status

`✅ Done`

## Owner

- Module: `hosts`
- Canonical crates: `labonair-hosts`, `labonair-hosts-ui`
- Related matrix: [`../../docs/capabilities.md`](../../docs/capabilities.md)

## Goal

Make the Hosts UI a presentation consumer of capability-owned host,
credential, and snippet contracts. The UI must not receive the broad backend
facade merely to perform persistence.

## Scope

- Replace backend host/credential/snippet calls with the canonical capability
  crates.
- Inject the shared database, secret state, data directory, and a narrow host
  side-effect callback from the composition boundary.
- Preserve the existing host event handoff to Workspace for SSH/SFTP opening.
- Preserve MCP agent-access revocation when a host is blocked.
- Do not expand this task into the larger Workspace→backend event migration.

## Acceptance criteria

- [x] `labonair-hosts-ui` has no `labonair-backend` dependency, import, or
  `Backend` field.
- [x] Host, group, credential, and snippet reads/writes use canonical
  capability APIs or injected typed contracts.
- [x] Blocking a host still triggers the existing agent-access revocation
  side effect through a narrow callback.
- [x] Existing host-manager behavior and focused tests remain intact.
- [x] Capability matrix, inventory, and dependency verifier describe the
  removed edge.
- [x] Full workspace verification gates pass.

## Removal condition

The temporary backend host wrappers may be removed once all remaining
workspace, SSH, migration, and MCP consumers use the canonical capability
contracts directly.

## Outcome

`labonair-hosts-ui` now receives the shared database, secret state, data
directory, and an optional typed host-event callback from the composition
boundary. It calls the canonical host, credential, and snippet stores directly.
The backend retains compatibility adapters for remaining non-UI consumers, and
the narrow MCP agent-access revocation adapter remains at the Workspace
composition boundary.
