# R11-001 — Onboarding module

Decision record: [`ADR 0008`](../../docs/adr/0008-onboarding-module.md).

## Status

`✅ Done`

Implemented directly, outside the task queue, at the owner's request.

## Owner

- Module: `onboarding`
- Capability-matrix row: [`../../docs/capabilities.md`](../../docs/capabilities.md)
- Composition entry point: `labonair-shell` (`crates/shell/src/onboarding.rs`, wired in `bootstrap`)

## What exists

- `crates/onboarding` (`labonair-onboarding`): `OnboardingStep` + `OnboardingRegistry`,
  `OnboardingProgress` (`<data_dir>/onboarding.json`), `OnboardingView` (page),
  built-in steps Look / Start / Hosts, `host_rows` (redaction boundary),
  `command_provider` (`Onboarding: Open`).
- `labonair-workspace`: `TabKind::Onboarding` / `PaneKind::Onboarding` singleton
  hosting an `AnyView` (`open_onboarding_tab`, `focus_onboarding_tab`,
  `close_onboarding_tab`).
- `labonair-command-palette-core`: `CommandId::OpenOnboarding` (`app::OpenOnboarding`).
- `labonair-shell`: builds `OnboardingServices`, opens the tab once at startup
  when steps are pending, closes it on `OnboardingEvent::Closed`, reloads the
  host manager after an import.

## Verified

- Unit tests: registry order and duplicates, progress round trip and corrupt
  file, host rows (key path never exposed, masking, tags, default selection,
  alias-only search).
- Native run with an isolated `HOME`: the tab opens on first launch; Look, Start,
  and Hosts sections render; Hosts discovers 3 hosts and skips `Host *`; the
  expanded Advanced list shows aliases, tags, and masked details only.

## Not verified

- Click paths (Finish / Skip, toggles, accordion, checkboxes, Show details,
  actual import, progress persisted across a relaunch): synthetic mouse events
  did not reach the window in the capture setup. Exercise by hand.
- Capability-matrix row, architecture manifest, coverage entry, and dependency
  verifier entries for the new crate are still to be added.

## Follow-ups

- Align the Hosts import dialog in `labonair-hosts-ui` with the masking rule.
- Future steps register from their owners: keymap preset (when `BaseKeymap`
  lands), AI provider (when the AI frontend returns), font size.
