//! [`ModalLayer`] + [`ModalView`] — the workspace's single modal overlay slot
//! (T17-005).
//!
//! Ported from `zed-refrence/zed/crates/workspace/src/modal_layer.rs`, trimmed
//! to what Labonair needs: **one** active, focus-trapping modal (no modal
//! stack, no reopenable-picker stash, no `DismissDecision`).
//!
//! A [`ModalView`] is any `Render + Focusable + EventEmitter<DismissEvent>`
//! view ([`gpui::ManagedView`]). [`ModalLayer::toggle_modal`] /
//! [`ModalLayer::open_modal`] build it, move focus into it, and drop it again
//! when it emits [`DismissEvent`] (Esc / overlay click / programmatic close)
//! or — when `dismiss_on_focus_lost` is set — when focus leaves it.
//!
//! The two existing overlays (command palette, updater dialog) still paint
//! their own full-screen scrim + centered card and handle their own Esc /
//! overlay click, so they set [`ModalView::render_bare`] to
//! `true`: the layer hosts them for lifecycle + focus and renders them as-is.
//! New modals (e.g. the `Cmd+F` search overlay, T18-002) can rely on the
//! default backdrop path this module provides.

use gpui::{
    div, AnyView, App, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle,
    InteractiveElement, IntoElement, ManagedView, MouseButton, MouseDownEvent, ParentElement,
    Render, Styled, Subscription, Window,
};

use crate::theme::modal_scrim;

/// A view that can be hosted by the [`ModalLayer`].
pub trait ModalView: ManagedView {
    /// Called right before the layer drops this modal. Views that keep their
    /// own `open` flag (e.g. the palette) reset it here so the two states
    /// can never drift apart.
    fn on_dismiss(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    /// When `true`, the layer renders the view directly with **no** backdrop
    /// and **no** centering wrapper — the view already paints its own
    /// full-screen overlay (scrim, positioning, Esc, overlay-click). Defaults
    /// to `false` (the layer supplies a scrim + a centered panel).
    fn render_bare(&self) -> bool {
        false
    }
}

/// Object-safe view over an `Entity<V: ModalView>` so [`ModalLayer`] can hold
/// the active modal without being generic.
trait ModalViewHandle {
    fn on_dismiss(&self, window: &mut Window, cx: &mut App);
    fn view(&self) -> AnyView;
    fn focus_handle(&self, cx: &App) -> FocusHandle;
    fn subscribe_dismiss(&self, window: &mut Window, cx: &mut Context<ModalLayer>) -> Subscription;
    fn render_bare(&self, cx: &App) -> bool;
}

impl<V: ModalView> ModalViewHandle for Entity<V> {
    fn on_dismiss(&self, window: &mut Window, cx: &mut App) {
        self.update(cx, |this, cx| this.on_dismiss(window, cx));
    }

    fn view(&self) -> AnyView {
        self.clone().into()
    }

    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.read(cx).focus_handle(cx)
    }

    fn subscribe_dismiss(&self, window: &mut Window, cx: &mut Context<ModalLayer>) -> Subscription {
        cx.subscribe_in(self, window, |this, _, _: &DismissEvent, window, cx| {
            this.hide_modal(window, cx);
        })
    }

    fn render_bare(&self, cx: &App) -> bool {
        self.read(cx).render_bare()
    }
}

struct ActiveModal {
    modal: Box<dyn ModalViewHandle>,
    _subscriptions: [Subscription; 2],
    previous_focus_handle: Option<FocusHandle>,
    focus_handle: FocusHandle,
}

/// The single modal-overlay slot. Held as an [`Entity`] by the shell and
/// rendered as one of the workspace composition's two overlay children (the
/// other being [`crate::toast_layer::ToastLayer`]).
#[derive(Default)]
pub struct ModalLayer {
    active_modal: Option<ActiveModal>,
    dismiss_on_focus_lost: bool,
}

impl ModalLayer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle a modal of type `V`: if a modal of the same type is active it is
    /// hidden; otherwise the current modal (if any) is replaced with the new
    /// one. `build` receives the window so it can e.g. focus an input.
    pub fn toggle_modal<V, B>(&mut self, window: &mut Window, cx: &mut Context<Self>, build: B)
    where
        V: ModalView,
        B: FnOnce(&mut Window, &mut Context<V>) -> V,
    {
        let mut previous_focus_handle = window.focused(cx);
        if let Some(active) = &self.active_modal {
            let is_same_type = active.modal.view().downcast::<V>().is_ok();
            previous_focus_handle = active.previous_focus_handle.clone();
            self.hide_modal(window, cx);
            if is_same_type {
                return;
            }
        }
        let new_modal = cx.new(|cx| build(window, cx));
        self.show_modal(Box::new(new_modal), previous_focus_handle, window, cx);
    }

    /// Show `V` unconditionally, replacing any current modal. Used by modals
    /// whose visibility follows external state rather than a user toggle
    /// (the updater dialog).
    pub fn open_modal<V, B>(&mut self, window: &mut Window, cx: &mut Context<Self>, build: B)
    where
        V: ModalView,
        B: FnOnce(&mut Window, &mut Context<V>) -> V,
    {
        let mut previous_focus_handle = window.focused(cx);
        if let Some(active) = &self.active_modal {
            previous_focus_handle = active.previous_focus_handle.clone();
            self.hide_modal(window, cx);
        }
        let new_modal = cx.new(|cx| build(window, cx));
        self.show_modal(Box::new(new_modal), previous_focus_handle, window, cx);
    }

    fn show_modal(
        &mut self,
        modal: Box<dyn ModalViewHandle>,
        previous_focus_handle: Option<FocusHandle>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focus_handle = cx.focus_handle();
        let modal_focus_handle = modal.focus_handle(cx);
        let dismiss_subscription = modal.subscribe_dismiss(window, cx);
        self.active_modal = Some(ActiveModal {
            modal,
            _subscriptions: [
                dismiss_subscription,
                cx.on_focus_out(&focus_handle, window, |this, _event, window, cx| {
                    if this.dismiss_on_focus_lost {
                        this.hide_modal(window, cx);
                    }
                }),
            ],
            previous_focus_handle,
            focus_handle,
        });
        cx.defer_in(window, move |_, window, _cx| {
            window.focus(&modal_focus_handle);
        });
        cx.notify();
    }

    /// Hide the active modal (if any), restoring focus to whatever held it
    /// before the modal opened. Returns `true` when a modal was hidden.
    pub fn hide_modal(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if self.active_modal.is_none() {
            self.dismiss_on_focus_lost = false;
            return false;
        }
        if let Some(active) = self.active_modal.as_ref() {
            active.modal.on_dismiss(window, cx);
        }
        if let Some(active) = self.active_modal.take() {
            if let Some(previous_focus) = active.previous_focus_handle {
                if active.focus_handle.contains_focused(window, cx) {
                    previous_focus.focus(window);
                }
            }
            cx.notify();
        }
        self.dismiss_on_focus_lost = false;
        true
    }

    /// Whether the layer should drop the modal when focus leaves it.
    pub fn set_dismiss_on_focus_lost(&mut self, value: bool) {
        self.dismiss_on_focus_lost = value;
    }

    /// The active modal, if it is of type `V`.
    pub fn active_modal<V: 'static>(&self) -> Option<Entity<V>> {
        self.active_modal
            .as_ref()?
            .modal
            .view()
            .downcast::<V>()
            .ok()
    }

    pub fn has_active_modal(&self) -> bool {
        self.active_modal.is_some()
    }
}

impl EventEmitter<DismissEvent> for ModalLayer {}

impl Render for ModalLayer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(active) = &self.active_modal else {
            return div().into_any_element();
        };

        // Bare modals paint their own scrim / positioning / key handling.
        if active.modal.render_bare(cx) {
            return active.modal.view().into_any_element();
        }

        div()
            .absolute()
            .inset_0()
            .size_full()
            .flex()
            .justify_center()
            .pt(gpui::px(96.0))
            .bg(modal_scrim())
            .occlude()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _: &MouseDownEvent, window, cx| {
                    this.hide_modal(window, cx);
                }),
            )
            .child(
                div()
                    .occlude()
                    .track_focus(&active.focus_handle)
                    .on_mouse_down(MouseButton::Left, |_: &MouseDownEvent, _, cx| {
                        cx.stop_propagation();
                    })
                    .child(active.modal.view()),
            )
            .into_any_element()
    }
}
