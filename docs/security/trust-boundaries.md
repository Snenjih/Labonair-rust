# Trust Boundaries

**Status:** Normative
**Version:** 1
**Related:** [`threat-model.md`](threat-model.md), [`../automation/grants.md`](../automation/grants.md), [`../architecture/composition-root.md`](../architecture/composition-root.md)

The following boundaries must remain explicit in code and documentation:

```text
user decision
    ↓ explicit tab/host grant
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

An implementation change crossing one of these boundaries requires an owner,
typed contract, negative-test decision, and scorecard evidence update.
