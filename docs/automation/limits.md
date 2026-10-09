# Automation Limits

**Status:** Normative
**Version:** 1
**Related:** [`overview.md`](overview.md), [`tools.toml`](tools.toml), [`../settings.md`](../settings.md)

The following limits are part of the current MCP contract and must be updated
in the tool catalog and tests if they change:

| Limit | Current value | Purpose |
|---|---:|---|
| Loopback port | `47823` by default | Avoid exposing the bridge on a network interface. |
| Maximum `run_command` timeout | `300000 ms` by default | Bound waiting for terminal output. |
| Default `run_command` timeout | `30000 ms` | Keep ordinary calls responsive. |
| Default `read_output` wait | `1000 ms` | Poll live output without scrollback ownership. |
| `open_tab` wait | `15 s` | Bound visible tab/session creation. |
| `close_tab` wait | `10 s` | Bound tab operation acknowledgement. |
| Notification history | 100 records | Bound retained UI state. |
| Notification dedupe window | 2 s | Suppress equivalent bursts. |

Timeouts return partial/typed failure information where the operation may still
be running. They do not silently detach a command from the visible terminal.
