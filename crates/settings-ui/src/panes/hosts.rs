//! Settings › Hosts (T19-010) — the final home of host / credential
//! management, replacing the interim `TabKind::Hosts` tab from T17-009.
//!
//! The list + edit form + jump-host picker + tunnel rows + SSH-config
//! import/export are **one** existing, mature component
//! (`labonair_hosts_ui::HostManagerView`, T16-008/T07-*) — per this task's
//! own Notizen ("nicht neu bauen") it is embedded here verbatim rather than
//! rebuilt. `SettingsView::host_manager` is the *exact same* entity
//! `labonair_workspace::Workspace` already uses for connecting /
//! `known_hosts` (threaded in via `SettingsDeps`, `crate::window`), so an
//! edit made here is live everywhere immediately — no new sync path, no new
//! `settings-ui -> workspace` data flow beyond the `Entity` handle itself.
//! Clicking "Connect"/"Open SFTP" inside the embedded view emits the same
//! `HostManagerEvent` `Workspace` already subscribes to (installed once in
//! `Workspace::new`), so it opens the tab in the main window exactly as it
//! did when this was `TabKind::Hosts` — satisfying the task's
//! `on_open_ssh`/`on_open_sftp` callback requirement without a new
//! `settings-ui -> workspace` dependency edge.
//!
//! The single write path into `SettingsContent.hosts.entries` +
//! `credential_ref` is [`labonair_hosts_ui::apply_host_change`], called by
//! `HostManagerView` itself after every host-list mutation — see that
//! function's doc comment for the full rationale.

use crate::view::*;

impl SettingsView {
    /// `hosts` (no sub-page) — the task's `hosts/list` + `hosts/edit`
    /// deep-links both resolve here: `HostManagerView` already toggles
    /// between its list and its edit form internally (T16-014's
    /// master/detail layout), so there is only one embedding point.
    pub(crate) fn render_hosts_pane(
        &mut self,
        c: &Palette,
        _cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(c.muted)
                    .child("Manage saved hosts, credentials, jump hosts and tunnels. Connecting still happens from the command palette (⌘⇧N)."),
            )
            .child(
                div()
                    .h(px(560.0))
                    .child(self.host_manager.clone().into_any_element()),
            )
            .into_any_element()
    }

    /// `hosts/ssh-config` — SSH-config import (from `~/.ssh/config`) /
    /// export, ported in T07-003 as part of `HostManagerView`'s own
    /// dialogs; this sub-page surfaces dedicated buttons that open them
    /// directly instead of requiring the user to find them inside the host
    /// list toolbar.
    pub(crate) fn render_hosts_ssh_config(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let host_manager = self.host_manager.clone();
        let import_manager = host_manager.clone();
        let export_manager = host_manager.clone();
        div()
            .flex()
            .flex_col()
            .gap_3()
            .child(
                div()
                    .text_size(px(11.5))
                    .text_color(c.muted)
                    .child("Import hosts from ~/.ssh/config, or export the current host list as a config fragment."),
            )
            .child(
                // T20-003: shared `button()` primitive (`Outline`/`Xs`).
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button(
                            "hosts-ssh-config-import",
                            *c,
                            ButtonVariant::Outline,
                            ButtonSize::Xs,
                        )
                        .child("Import from ~/.ssh/config\u{2026}")
                        .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                            import_manager.update(cx, |hm, cx| hm.open_import_dialog(cx));
                        })),
                    )
                    .child(
                        button(
                            "hosts-ssh-config-export",
                            *c,
                            ButtonVariant::Outline,
                            ButtonSize::Xs,
                        )
                        .child("Export to ~/.ssh/config\u{2026}")
                        .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                            export_manager.update(cx, |hm, cx| hm.open_export_dialog(cx));
                        })),
                    ),
            )
            .child(
                div()
                    .h(px(480.0))
                    .child(host_manager.into_any_element()),
            )
            .into_any_element()
    }

    /// `hosts/availability` — host-reachability polling knobs
    /// (`connections.*`), rendered as normal generated `SettingField`s
    /// (task Notizen: a Custom body may still embed generated fields).
    pub(crate) fn render_hosts_availability(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        self.render_field_groups(HOSTS_AVAILABILITY_GROUPS, "connections", c, cx)
    }
}
