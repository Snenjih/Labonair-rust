//! Native Hosts management window.
//!
//! The Hosts module owns this surface and its view state. The shell only
//! supplies the already-composed `HostManagerView` handle and opens/focuses
//! the native window in response to a typed navigation request.

use gpui::{
    point, px, size, App, AppContext, Bounds, Global, TitlebarOptions, WindowBounds, WindowHandle,
    WindowKind, WindowOptions,
};
use gpui_component::Root;

use crate::HostManagerView;

#[derive(Default)]
struct HostsWindowRef {
    handle: Option<WindowHandle<Root>>,
}

impl Global for HostsWindowRef {}

/// Open the canonical Hosts management window or focus its existing instance.
pub fn open_hosts_window(hosts: gpui::Entity<HostManagerView>, cx: &mut App) {
    if let Some(handle) = cx
        .try_global::<HostsWindowRef>()
        .and_then(|reference| reference.handle)
    {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            cx.activate(true);
            return;
        }
        cx.set_global(HostsWindowRef { handle: None });
    }

    let bounds = Bounds::centered(None, size(px(1120.0), px(760.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Hosts".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(19.0), px(15.0))),
            }),
            window_min_size: Some(size(px(760.0), px(520.0))),
            kind: WindowKind::Normal,
            is_movable: true,
            ..Default::default()
        },
        move |window, cx| {
            let view: gpui::AnyView = hosts.clone().into();
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(handle) => {
            cx.set_global(HostsWindowRef {
                handle: Some(handle),
            });
            cx.activate(true);
        }
        Err(error) => tracing::error!("failed to open hosts window: {error}"),
    }
}
