# Automation Test Matrix

**Status:** Normative
**Version:** 1
**Related:** [`overview.md`](overview.md), [`tools.toml`](tools.toml), [`../testing/evidence-model.md`](../testing/evidence-model.md)

Every automation tool has positive and negative test decisions in the
structured catalog. Negative tests are mandatory for missing grants, revoked
grants, blocked hosts, missing credentials, timeout behavior, and secret/path
exposure.

The matrix distinguishes:

- contract tests for stable MCP wire names and typed events;
- unit tests for grants, expiry, timeout caps, and preference persistence;
- integration tests for visible tab operations and terminal adapters;
- denial tests for unauthorized sessions, blocked hosts, unsupported
  authentication, invalid bearer tokens, and loopback/limit violations;
- evidence that no tool returns raw secret material or hidden command output.

The current repository has source-level and contract tests; the catalog marks
remaining end-to-end and negative-test gaps as `Pending` rather than implying
that MCP safety is complete.
