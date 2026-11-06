//! Singleton pane adapters: Git Graph, Diff, Keymap, Hosts. They cannot be
//! split (reopening focuses the existing pane) but live in any pane group.

use gpui::{AnyElement, App, Context, IntoElement};

use super::PaneSpec;
use crate::pane::PaneId;
use crate::pane_kind::PaneKind;
use crate::Workspace;

impl Workspace {
    pub(super) fn singleton_pane_spec(&self, pane: PaneId, cx: &App, spec: &mut PaneSpec) {
        if let Some(tab) = self.tabs.read(cx).get(pane) {
            if let Some(custom) = &tab.custom_title {
                spec.title = custom.clone().into();
            }
        }
    }

    pub(super) fn singleton_pane_body(
        &mut self,
        _pane: PaneId,
        kind: PaneKind,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let placeholder = |this: &Self, cx: &mut Context<Self>| {
            this.placeholder(kind.label(), cx).into_any_element()
        };
        match kind {
            PaneKind::GitGraph => match &self.git_graph {
                Some(view) => view.clone().into_any_element(),
                None => placeholder(self, cx),
            },
            PaneKind::Diff => match &self.project_diff {
                Some(view) => view.clone().into_any_element(),
                None => placeholder(self, cx),
            },
            PaneKind::Keymap => match &self.keymap {
                Some(view) => view.clone().into_any_element(),
                None => placeholder(self, cx),
            },
            _ => placeholder(self, cx),
        }
    }
}
