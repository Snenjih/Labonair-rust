# Automation and Remote Threat Model

**Status:** Normative
**Version:** 1
**Related:** [`trust-boundaries.md`](trust-boundaries.md), [`secrets.md`](secrets.md), [`remote-access.md`](remote-access.md), [`../automation/tools.toml`](../automation/tools.toml)

This threat model covers the local MCP bridge, AI/agent access, SSH/SFTP
sessions, credentials, and filesystem-adjacent feature integrations.

## Assets

- bearer tokens and stored credentials;
- private keys, passwords, host metadata, and session identifiers;
- local filesystem content and visible terminal input/output;
- remote hosts, sessions, tunnels, and transfer targets;
- user trust that agent actions are visible, scoped, and revocable.

## Actors and boundaries

| Actor | Trust level | Boundary |
|---|---|---|
| User | Decision authority | Grants/revokes tab access and chooses hosts/actions. |
| Labonair UI | Trusted application process | Owns capability state and presents visible actions. |
| MCP client/agent | Untrusted-by-default caller | Must authenticate and hold explicit session grants. |
| Local bridge | Narrow privileged adapter | Loopback bearer endpoint; rechecks policy per tool call. |
| SSH/SFTP remote | External/untrusted system | Reached only through typed transport contracts and host policy. |
| Secret store | Sensitive platform boundary | Returns credential material only to the narrow authentication path. |

## Threats and controls

| Threat | Required control | Residual evidence |
|---|---|---|
| Unauthenticated tool call | Disabled-by-default bridge, loopback binding, bearer token. | Negative authentication test required. |
| Stale tab grant | Recheck current grant and host block flag at execution time. | Grant expiry and blocked-host tests. |
| Agent reaches blocked host | `block_agent_access` denies open and active calls; revokes existing grants. | Host/MCP integration test. |
| Secret exfiltration | No raw secrets in tool schema/output; secret files blocked by AI path guards. | Secret redaction and path-negative tests. |
| Hidden command execution | Commands are typed into the visible granted terminal. | Visible-terminal integration evidence. |
| Runaway command | Per-session lock and bounded timeout cap; partial result is explicit. | Timeout/cancellation tests. |
| Remote path or host confusion | Saved-host identity and typed SSH/SFTP contracts; no arbitrary tool host input. | Host/transport contract tests. |
| Log/notification leakage | Structured details and source; never record credentials/tokens. | Review and redaction tests. |

The bridge is not a general sandbox. The user grant, host policy, secret
boundary, visible terminal, and explicit denial behavior are the safety model;
any stronger isolation claim requires a separate architecture decision.
