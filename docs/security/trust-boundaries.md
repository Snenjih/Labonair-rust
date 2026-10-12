# Trust Boundaries

**Status:** Normative
**Version:** 1
**Related:** [`threat-model.md`](threat-model.md), [`../automation/grants.md`](../automation/grants.md), [`../architecture/composition-root.md`](../architecture/composition-root.md)

The following boundaries must remain explicit in code and documentation:

```text
user decision
    ├─ project/worktree trust decision
    │      ↓ typed Workspace trust state
    │   project settings / Editor language services / MCP integrations
    └─ explicit tab/host grant
           ↓
        MCP loopback + bearer token
           ↓ typed MCP contracts/events
        composition root adapters
           ↓ owner contracts
        workspace / hosts / SSH / SFTP / terminal capabilities
           ↓ narrow secret lookup
        secret store and remote transport
```

- `labonair-mcp-core` owns wire values and narrow service contracts.
- `labonair-mcp-server` owns HTTP tools, grants, timeouts, and event adapters.
- `labonair-shell` composes concrete state into `McpServerAccess`.
- Workspace mirrors grants for visible UI; it does not own the authoritative
  grant map.
- Hosts owns host policy and `block_agent_access`; SSH/SFTP own transport.
- Secrets owns keychain/secret material and is never a general data source.

## Project and worktree trust

- Workspace (`labonair-workspace`) owns the Restricted Mode decision,
  path-scoped trust policy, persisted trust state, reset behavior, and
  user-facing recovery. Settings stores values only; the global auto-trust
  preference defaults to false, and Workspace owns its effect.
- Untrusted project and single-file roots start restricted. Trust requires an
  explicit user decision for the selected path or validated parent scope. Local
  path trust and remote-host trust are isolated identities.
- Project trust does not grant SSH host-key acceptance or MCP tab access. Each
  remains a separate user decision with its existing owner and typed contract.
- Project settings, Editor language services, Agent, Git, and MCP integrations
  query a narrow typed Workspace trust contract before project-controlled
  settings, skills, tools, repository workers, or services take effect.
  Consumers do not read Settings, a shell trust table, or Workspace private
  state directly.
- A failed trust write must not be presented as durable. Report persistence
  failures through the notification center, provide retry or recovery, and
  return an unpersisted path to Restricted Mode when it is reopened.

An implementation change crossing one of these boundaries requires an owner,
typed contract, negative-test decision, and scorecard evidence update.
