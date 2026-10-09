# Automation and MCP Overview

**Status:** Normative
**Version:** 1
**Related:** [`tools.toml`](tools.toml), [`grants.md`](grants.md), [`limits.md`](limits.md), [`test-matrix.md`](test-matrix.md), [`../security/threat-model.md`](../security/threat-model.md)

Labonair's automation boundary is the local MCP Streamable HTTP bridge. It is
disabled by default, binds to loopback, and requires bearer authentication.
Tools operate on terminal tabs that the user explicitly granted to the agent.

## Trust model

1. The user enables the bridge and supplies/retains the local bearer token.
2. The user grants a specific local or SSH tab; a grant is keyed by tab identity
   and revalidated against the current host policy on every tool call.
3. Tools act visibly through the real terminal pane or explicit tab operation.
4. Host `block_agent_access` and missing/non-interactive credentials deny access.
5. Activity and expiry events return through typed MCP contracts and the shared
   notification/event infrastructure.

The exact tools, grants, limits, side effects, and negative tests are the
structured source in [`tools.toml`](tools.toml). Generated documentation is
[`../generated/tools.md`](../generated/tools.md).
