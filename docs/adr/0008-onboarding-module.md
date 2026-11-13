# ADR 0008 — Onboarding is one self-contained module, shown as a workspace tab

* **Status:** Accepted
* **Date:** 2026-11-13
* **Deciders:** project owner and implementation team
* **Related:** [`0005-universal-panes.md`](0005-universal-panes.md), [`0007-hosts-settings-page.md`](0007-hosts-settings-page.md), [`../registries.md`](../registries.md), [`../capabilities.md`](../capabilities.md), [`../../tasks/rework/R11-001-onboarding-module.md`](../../tasks/rework/R11-001-onboarding-module.md)

## Context

A first launch used to drop the user on the empty workspace with every default
applied and no pointer to the choices that matter most (look, startup
behaviour, existing SSH hosts). The product wants a short, minimal, fast
first-run flow that can grow: later features must be able to add a step
without editing onboarding code. The reference (Zed) shows onboarding as a
page in the workspace's centre pane, not as a popup.

## Decision

1. **One new capability crate, `labonair-onboarding` (`crates/onboarding`).**
   It contains everything onboarding-specific: the step contract, the step
   registry, persisted progress, the page view, the built-in steps, the
   `Onboarding: Open` command, and the redaction helpers. There is no
   `onboarding-ui` sibling because there is no real dependency or lifecycle
   boundary to justify one.
2. **The surface is a singleton workspace tab, not a modal.**
   `TabKind::Onboarding` / `PaneKind::Onboarding` is a singleton like Keymap
   (opens once, focuses on reopen, cannot be split, is never session-restored).
   The workspace hosts an `AnyView` plus its focus handle supplied by the shell
   (`Workspace::open_onboarding_tab`, `focus_onboarding_tab`,
   `close_onboarding_tab`), so `labonair-workspace` does not depend on the
   onboarding crate. The page is a scrollable centred column (max 720 px): a
   header with `Skip` and `Finish setup`, then one section per step (title,
   hint, body). It is not a wizard; there is no next/back state.
3. **Steps are registry contributions.** Each step implements
   `OnboardingStep` (stable namespaced `id`, `order`, `title`, `hint`,
   `is_applicable`, async `prepare`, `build`). A built step returns its view
   plus an optional `on_finish` for deferred work. Duplicate IDs are an error.
   The crate registers four built-in steps through `register_builtin_steps`
   (`onboarding.start` 100, `onboarding.look` 200, `onboarding.sessions` 250,
   `onboarding.hosts` 300; `sessions` is unix-only and writes
   `terminal.sessionHost`);
   other modules may register more through the same typed API. The shell never
   keeps a step list.
4. **Onboarding writes through existing owners, it does not own their data.**
   Look, Start and Sessions write ordinary settings values (`general.theme`,
   `appearance.translucentSurfaces`, `appearance.paneLayout`,
   `general.defaultStartupTab`, `general.sessionRestore`,
   `terminal.sessionHost`) through
   `SettingsStore::update_user_settings_deferred`, the route the Settings
   window uses. The composition root injects `OnboardingServices`, including an
   `apply_appearance` callback that re-derives the theme store, so the crate
   needs neither `labonair-theme-ui` nor the shell. The Hosts step goes through
   the existing `labonair_ssh::SshConfigService` (`parse` / `import`).
5. **Progress is per-installation state, not a setting.** It lives in
   `<data_dir>/onboarding.json` (`completedSteps` only), owned by the crate. It
   is not in `settings.json` because that file is synced between machines via
   dotfiles and a new machine must run onboarding again.
6. **Pending-step rule.** On startup the page opens iff at least one
   registered step is applicable and not in `completedSteps`. A fresh install
   sees all steps; an existing user after an update sees only steps added since.
   `Finish setup` runs the steps' deferred work; `Skip` does not; both mark
   every shown step completed and close the tab. Onboarding never nags. The
   command `Onboarding: Open` (`CommandId::OpenOnboarding`) replays every
   applicable step, or focuses the tab if it is already open.
7. **Secret-safe by construction.** The Hosts step renders `HostImportRow`
   view models built by a pure function from `SshConfigEntry`. A row has no
   field for the identity-file path or any credential. Address, port, and user
   are masked (`••••••`) behind a "Show details" toggle that is off whenever
   the page opens. Search matches the alias only. A test asserts that nothing a
   row exposes contains the identity-file path.
8. **Hosts step shape.** Shown only when `~/.ssh/config` has hosts. A switch
   "Import hosts from ~/.ssh/config" (default on). Off: a hint pointing at
   Settings › Hosts. On: "N of M selected" and a collapsed **Advanced**
   disclosure holding search, select all/none, the Show-details toggle, and the
   host list (checkbox, alias, tags `Key` / `Asks for password` / `Jump host` /
   `Already added`). Existing hosts start unselected; the conflict policy is
   fixed to `Skip`. The import runs on `Finish setup` on the tokio runtime and
   reports through the notification center; afterwards the shell reloads the
   host manager.

## Consequences

* Adding an onboarding step is a registration call and needs no change to
  `labonair-onboarding`, the workspace, or the shell.
* `labonair-onboarding` is a leaf feature crate depending on `labonair-ui-kit`,
  `labonair-theme`, `labonair-settings`, `labonair-ssh`,
  `labonair-notifications`, and the command-palette contract crates. The shell
  is its only consumer; it must not depend on the shell, the workspace, or
  other feature UI crates.
* The workspace gained one tab/pane kind and an `AnyView` host; nothing in it
  knows what the view is.
* AGENTS.md rule 9 is unaffected: Settings gains no key and no category.
* The existing Hosts import dialog in `labonair-hosts-ui` is unchanged. It still
  shows address, port, and user unmasked; aligning it with the masking rule is
  a separate follow-up.
