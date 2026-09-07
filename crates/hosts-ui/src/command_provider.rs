//! Hosts-UI-owned command execution contributions.

use std::rc::Rc;

use gpui::{App, Entity, Window};
use labonair_command_palette_core::CommandId;
use labonair_command_palette_runtime::CommandHandlerRegistry;

use crate::HostManagerView;

/// Host operation used to open the canonical command-palette host picker.
pub type HostPickerHandler = Rc<dyn Fn(&mut Window, &mut App)>;

/// Register the canonical Hosts management-window entry point.
pub fn register_handlers(registry: &mut CommandHandlerRegistry, hosts: &Entity<HostManagerView>) {
    let hosts = hosts.clone();
    registry
        .register(CommandId::OpenHosts, move |_window, cx: &mut App| {
            super::open_hosts_window(hosts.clone(), cx);
        })
        .expect("hosts UI command handler must have a unique id");
}

/// Register connection commands that all enter through the Hosts picker.
/// `Enter` and `Shift+Enter` then select SSH or SFTP on the picker row.
pub fn register_picker_handlers(
    registry: &mut CommandHandlerRegistry,
    open_picker: HostPickerHandler,
) {
    for id in [
        CommandId::NewSshTab,
        CommandId::NewSftpTab,
        CommandId::NewQuickSsh,
        CommandId::NewSshConnection,
    ] {
        let open_picker = open_picker.clone();
        registry
            .register(id, move |window, cx| open_picker(window, cx))
            .expect("hosts picker command handler must have a unique id");
    }
}
