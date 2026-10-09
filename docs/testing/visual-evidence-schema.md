# Visual Evidence Registry

**Status:** Normative
**Version:** 1
**Owner:** R07 acceptance and surface owners

The structured source at [`visual-evidence.toml`](visual-evidence.toml) is the
machine-readable screenshot index for the canonical surfaces in
[`../product/surfaces.toml`](../product/surfaces.toml). The generated view at
[`../generated/visual-evidence.md`](../generated/visual-evidence.md) is the
human-readable index. Neither source replaces the native-bundle rules in
[`../visual-verification.md`](../visual-verification.md) or the acceptance
interpretation in [`visual-matrix.md`](visual-matrix.md).

## Required contract

The registry must declare:

- the canonical surface source and visual-matrix source;
- a repository-relative artifact root;
- a deterministic screenshot filename pattern containing surface, state,
  viewport, platform, and commit placeholders;
- the supported capture viewports, including a distinct `narrow` viewport;
- the exact native-binary rule and capture helper;
- the artifact-path helper that resolves an exact catalog state and commit to
  the deterministic filename;
- the current environment blocker, when native evidence is unavailable;
- one state-status map for every surface ID in `surfaces.toml`.

State labels are copied from `surfaces.toml`; agents must not invent aliases
such as `ready` or `normal` when the catalog uses a more specific label. The
checker rejects missing, extra, or duplicate surface/state records.

## Evidence states

The registry uses `Verified`, `Partial`, `Pending`, and `N/A`.

- `Verified` requires an exact native executable, PID/window identity,
  platform, viewport, capture date, and an existing artifact under the
  declared artifact root.
- `Partial` also requires a durable capture record and existing artifact; it
  means the retained evidence does not cover the complete state requirement.
- `Partial` identifies evidence that exists but does not cover the complete
  state requirement.
- `Pending` records missing evidence and keeps the blocker visible.
- `N/A` requires a state-specific reason in the source record.

An optional `capture` record attaches durable evidence to a state. Its
artifact filename must match the deterministic pattern and contain the exact
lowercase commit identifier. A `narrow window` state always uses the `narrow`
viewport; other states use the declared default unless a future source rule
explicitly says otherwise. A pending state does not receive a placeholder
screenshot; the generated index prints the exact path that a future capture
must use.

## Capture rule

The exact executable path and PID must be resolved before capture. A process
name, a generic `Labonair` window title, a reference rendering, or a
successful compile is not visual evidence. If the supported native host or
capture permission is unavailable, keep the state `Pending`, record the
blocker, and leave the next action in the active task or handoff.
