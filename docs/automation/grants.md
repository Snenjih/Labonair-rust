# Automation Grants

**Status:** Normative
**Version:** 1
**Related:** [`overview.md`](overview.md), [`tools.toml`](tools.toml), [`../security/trust-boundaries.md`](../security/trust-boundaries.md)

MCP authorization is explicit and session-scoped. A tool must not infer access
from the existence of a host, an SSH session, or a remembered previous action.

## Grant rules

- `list_sessions` returns only currently granted tabs.
- `run_command`, `read_output`, `send_keys`, and `close_tab` require a current
  grant for the requested session.
- `open_tab` requires a saved host with non-interactive stored credentials and
  `block_agent_access == false`; it creates a grant only after the visible tab
  operation succeeds.
- A host policy change revokes affected grants and emits `mcp_grant_expired`.
- Auto-revoke is configurable through the MCP capability preference; zero means
  no inactivity sweep.
- A grant is never a secret. Passwords, private keys, bearer tokens, and raw
  credential material must not be returned to the agent.

The workspace keeps a local mirror for UI state, while the MCP capability is
the authoritative grant owner. The bridge rechecks authorization at execution
time rather than trusting a stale UI snapshot.
