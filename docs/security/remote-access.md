# Remote Access Safety

**Status:** Normative
**Version:** 1
**Related:** [`threat-model.md`](threat-model.md), [`secrets.md`](secrets.md), [`../capabilities.md`](../capabilities.md), [`../audits/remaining-boundaries.md`](../audits/remaining-boundaries.md)

Hosts owns saved-host identity and policy. SSH/SFTP own connection and session
execution. Transfers, snippets, Explorer, Workspace, and MCP consume typed
contracts or composition adapters; they must not reach into transport-private
state.

## Required behavior

- Use stable host IDs and typed `HostOpenRequest`/session contracts.
- Keep jump hosts as SSH connection configuration, not a duplicate product
  surface or status item.
- Keep SFTP browsing and transfer lifecycle separate from SSH transport
  implementation details.
- Reject blocked-agent hosts for MCP open and active tool calls.
- Treat connection, trust, authentication, timeout, and transfer failures as
  structured owner notifications with safe details.
- Make cancellation, reconnect, and stale-session behavior explicit in the
  owning task and tests.
- Never claim remote safety from a compile-only check; include allowed and
  denied integration cases.

The Explorer remote-directory contract is the current example of the desired
boundary: the panel consumes `RemoteExplorerService`, while the shell injects
the SFTP adapter.
