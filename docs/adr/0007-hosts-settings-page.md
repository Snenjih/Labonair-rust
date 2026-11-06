# ADR 0007 — Hosts management is a full-page Settings surface

* **Status:** Accepted
* **Date:** 2026-11-06
* **Deciders:** project owner and implementation team
* **Related:** [`0006-settings-hosts-owner-surfaces-inline.md`](0006-settings-hosts-owner-surfaces-inline.md), [`../settings.md`](../settings.md), [`../settings-guidelines.md`](../settings-guidelines.md), [`../capabilities.md`](../capabilities.md)

## Context

Hosts management was a single workspace tab. Hosts and credentials are
configuration that is edited occasionally (an IP changes, a key is rotated),
not a working surface, and the product wants as few tabs as possible. ADR 0006
already reserved a `Hosts` owner page below `Keymap` and allowed owners to
embed their views in Settings.

## Decision

1. Hosts and credentials management lives on a `Hosts` Settings page, placed
   directly under `Keymap` in the owner-page group. The `Hosts` workspace tab
   (`TabKind::Hosts`, `PaneKind::Hosts`, `Workspace::open_hosts_tab`) is removed.
2. The Hosts owner registers the page through the Settings surface contract
   (`labonair_hosts_ui::register_settings_surfaces`). The surface is inline and
   `full_page`: Settings gives the owner's view the whole content area instead
   of wrapping it in a section card, and the view scrolls itself.
3. Ownership does not move. `labonair-hosts` still owns the host model and
   store, `labonair-credentials` owns credentials, and `labonair-hosts-ui` owns
   `HostManagerView` and its presentation state. Settings stores no host or
   credential data and only hosts the view.
4. The page has two levels: a list (search, `Hosts | Credentials` switch,
   group/type filters, sort, `New …`) and a detail sub-page per host or
   credential. Detail pages use `settings_group` / `settings_row` cards and
   autosave with a status badge (unsaved → saving → saved). A new host is
   created once it has an address, a new credential once it has a name.
5. Every entry point that used to open the tab (`OpenHosts` command, titlebar
   global menu, "Edit Host" in the SSH loading screen, "All hosts…" in the new
   tab menu) opens Settings on the `hosts` page through
   `WorkspaceEvent::OpenHosts`. Connecting from the page emits the existing
   typed `HostManagerEvent::Open`; the shell brings the main window forward.
6. Host selection for connecting stays in the command-palette Hosts submenu.

## Consequences

* AGENTS.md rule 9 and `docs/settings-guidelines.md` rule 3 read: Settings do
  not own non-settings state; hosting the Hosts owner surface inline is allowed,
  as with the Themes and Background surfaces. No other module may add a
  parallel Settings category.
* One `HostManagerView` entity exists. `Workspace` keeps it as the live
  host-data hub (picker rows, connection status, tunnels) and Settings embeds
  the same entity, so no state is duplicated.
* `SettingsSurface::full_page` is a new, generic flag. A full-page page owns the
  keyboard: the Settings window does not treat typing there as search input.
* Deleting a credential resets hosts that authenticated through it to password
  authentication (owned by `labonair-credentials`).
