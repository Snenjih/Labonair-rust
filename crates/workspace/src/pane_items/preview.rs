//! Preview pane adapter: a native preview view; a split opens a second
//! instance at the same location.

use gpui::{AnyElement, App, AppContext, Context, IntoElement, SharedString, Styled};

use super::{PaneSpec, SplitCloneError};
use crate::pane::PaneId;
use crate::views::preview::PreviewView;
use crate::Workspace;

impl Workspace {
    pub(super) fn preview_pane_spec(&self, pane: PaneId, cx: &App, spec: &mut PaneSpec) {
        let Some(view) = self.previews.get(&pane) else {
            return;
        };
        let url = view.read(cx).url().to_string();
        if !url.is_empty() {
            spec.title = SharedString::from(
                std::path::Path::new(&url)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .map(str::to_string)
                    .unwrap_or_else(|| url.clone()),
            );
            spec.trailing = Some(SharedString::from(url));
        }
    }

    pub(super) fn preview_pane_body(
        &mut self,
        pane: PaneId,
        _cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.previews.get(&pane) {
            Some(view) => view.clone().into_any_element(),
            None => gpui::div().size_full().into_any_element(),
        }
    }

    pub(super) fn clone_preview_pane(
        &mut self,
        source: PaneId,
        new_id: PaneId,
        cx: &mut Context<Self>,
    ) -> Result<(), SplitCloneError> {
        let url = self
            .previews
            .get(&source)
            .map(|v| v.read(cx).url().to_string())
            .unwrap_or_default();
        let theme = self.theme.clone();
        let view = cx.new(|cx| PreviewView::new(theme, cx));
        if !url.is_empty() {
            view.update(cx, |p, cx| p.set_url(url, cx));
        }
        self.previews.insert(new_id, view);
        Ok(())
    }

    pub(super) fn retire_preview_pane(&mut self, pane: PaneId) {
        self.previews.remove(&pane);
        self.editor_side_previews
            .retain(|_, (preview, _)| *preview != pane);
        if self.following_markdown_preview == Some(pane) {
            self.following_markdown_preview = None;
            self.following_markdown_editor = None;
        }
        if self.following_svg_preview == Some(pane) {
            self.following_svg_preview = None;
            self.following_svg_editor = None;
        }
        self.markdown_previews.remove(&pane);
        self.svg_previews.remove(&pane);
    }
}
