//! Hosts-UI-owned command execution contributions.

use gpui::{App, Entity};
use labonair_command_palette_core::CommandId;
use labonair_command_palette_runtime::CommandHandlerRegistry;

use crate::HostManagerView;

/// Register the canonical Hosts management-window entry point.
pub fn register_handlers(registry: &mut CommandHandlerRegistry, hosts: &Entity<HostManagerView>) {
    let hosts = hosts.clone();
    registry
        .register(CommandId::OpenHostSettings, move |_window, cx: &mut App| {
            super::open_hosts_window(hosts.clone(), cx);
        })
        .expect("hosts UI command handler must have a unique id");
}
