//! Editor-owned statusbar contribution.
//!
//! The Workspace supplies the currently active `EditorView` through a typed
//! provider. The statusbar item only hosts the editor's controls; language,
//! selection, diagnostics, file-format, and modal-mode behavior stays in
//! `EditorView`.

use std::rc::Rc;
use std::sync::Arc;

use gpui::{div, AnyElement, App, AppContext, Context, Entity, IntoElement, Render, Window};
use labonair_panel::{AnyStatusItemHandle, StatusItem, StatusItemRegistration, StatusSide};

use crate::EditorView;

/// Typed lookup for the active editor supplied by the Workspace composition
/// boundary.
pub type ActiveEditorProvider = Rc<dyn Fn(&App) -> Option<Entity<EditorView>>>;

struct EditorStatusItem {
    active_editor: ActiveEditorProvider,
}

impl Render for EditorStatusItem {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_status(window, cx)
    }
}

impl StatusItem for EditorStatusItem {
    fn id(&self) -> &'static str {
        "editor-status"
    }

    fn default_side(&self) -> StatusSide {
        StatusSide::Right
    }

    fn order(&self) -> i32 {
        11
    }

    fn is_empty(&self, cx: &App) -> bool {
        (self.active_editor)(cx).is_none()
    }

    fn render_status(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(editor) = (self.active_editor)(cx) else {
            return div().into_any_element();
        };
        editor.update(cx, |editor, cx| editor.render_statusbar(window, cx))
    }

    fn on_workspace_changed(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }
}

/// Build the Editor capability's composite statusbar contribution.
pub fn status_item_registration(
    active_editor: ActiveEditorProvider,
    cx: &mut App,
) -> StatusItemRegistration {
    let item = cx.new(|_| EditorStatusItem { active_editor });
    let handle = item.clone();
    StatusItemRegistration {
        id: item.read(cx).id(),
        default_side: item.read(cx).default_side(),
        order: item.read(cx).order(),
        group: item.read(cx).group(),
        build: Arc::new(move |_window, _cx| Arc::new(handle.clone()) as AnyStatusItemHandle),
    }
}
