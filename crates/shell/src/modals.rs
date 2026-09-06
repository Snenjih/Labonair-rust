//! Shell-local [`ModalView`] wrapper newtypes (T17-005, relocated in T17-006).
//!
//! The command palette and updater dialog predate the
//! [`ModalLayer`](labonair_workspace::modal_layer::ModalLayer) and still paint
//! their own scrim + centered card, so each sets `ModalView::render_bare`.
//! These thin wrappers only give the layer the `ModalView` identity it needs —
//! a distinct type to toggle on and a `DismissEvent` to observe. The wrappers
//! live here, not in each view's crate, because `labonair-command-palette`
//! cannot depend on `labonair-workspace` (where `ModalView` lives) without a
//! cycle, and the orphan rule bars `impl ModalView for CommandPalette`
//! anywhere else.

use gpui::{
    App, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render,
    Subscription, Window,
};
use labonair_command_palette::CommandPalette;
use labonair_settings_ui::PreferencesStore;

use crate::theme::ThemeStore;
use crate::updater::UpdaterView;
use crate::workspace::Workspace;

/// The concrete command-palette instantiation used throughout the shell.
pub(crate) type ShellPalette = CommandPalette<PreferencesStore, Workspace, ThemeStore>;

pub(crate) struct CommandPaletteModal {
    inner: Entity<ShellPalette>,
    focus: FocusHandle,
    _dismiss: Subscription,
}

impl CommandPaletteModal {
    pub(crate) fn new(inner: Entity<ShellPalette>, cx: &mut Context<Self>) -> Self {
        let focus = inner.read(cx).focus_handle(cx);
        let dismiss = cx.subscribe(&inner, |_, _, _: &DismissEvent, cx| cx.emit(DismissEvent));
        Self {
            inner,
            focus,
            _dismiss: dismiss,
        }
    }
}

impl EventEmitter<DismissEvent> for CommandPaletteModal {}

impl Focusable for CommandPaletteModal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for CommandPaletteModal {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.inner.clone()
    }
}

impl labonair_workspace::modal_layer::ModalView for CommandPaletteModal {
    fn on_dismiss(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.inner.update(cx, |p, cx| p.close(cx));
    }

    fn render_bare(&self) -> bool {
        true
    }
}

pub(crate) struct UpdaterModal {
    inner: Entity<UpdaterView>,
    focus: FocusHandle,
}

impl UpdaterModal {
    pub(crate) fn new(inner: Entity<UpdaterView>, cx: &mut Context<Self>) -> Self {
        Self {
            inner,
            focus: cx.focus_handle(),
        }
    }
}

impl EventEmitter<DismissEvent> for UpdaterModal {}

impl Focusable for UpdaterModal {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for UpdaterModal {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.inner.clone()
    }
}

impl labonair_workspace::modal_layer::ModalView for UpdaterModal {
    fn on_dismiss(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.inner.update(cx, |u, cx| u.close_dialog(cx));
    }

    fn render_bare(&self) -> bool {
        true
    }
}
