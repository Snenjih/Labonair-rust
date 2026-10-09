# Secret Handling

**Status:** Normative
**Version:** 1
**Related:** [`threat-model.md`](threat-model.md), [`remote-access.md`](remote-access.md), [`../automation/tools.toml`](../automation/tools.toml)

Secrets include passwords, private keys, bearer tokens, keychain references,
credential payloads, and sensitive host/session values.

## Rules

- Store secret material only through `labonair-secrets` and the credential
  owner; host records may store references and metadata, not raw secrets.
- Never place secrets in source, logs, notifications, SQLite fixtures, tests,
  generated documentation, screenshots, or commits.
- MCP tools may check that a credential exists, but must not return its value.
- Tool schemas must use host/session IDs and typed references instead of raw
  password/key parameters.
- AI path guards reject obvious secret filenames/directories and protected
  system paths; symlink-aware checks are required where a path is resolved.
- Error details must be useful without echoing command input or credential
  material.
- Test fixtures use synthetic credentials and must be redacted before sharing
  artifacts.

The bearer token and update-signing private key are operator/CI secrets and do
not belong in repository documentation. Only their storage contract and
redaction behavior are documented here.
