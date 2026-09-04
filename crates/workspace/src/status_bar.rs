//! The status bar (T17-003).
//!
//! Replaces the shell's former `render_bar_item` `match` cascade: this
//! component renders **only** from the
//! [`StatusItemRegistry`](labonair_panel::StatusItemRegistry) that
//! `labonair_shell::register_builtin_status_items` populates on the
//! [`Workspace`]. Each registered item is a self-describing
//! [`StatusItem`](labonair_panel::StatusItem) view: it names its `id`, its
//! `default_side` and its `order`, and renders its own content. Left = panel
//! controls, right = info dropdowns (`docs/architecture.md` §4).
//!
//! Personalization (right-click → left/right/hide, persisted) is T18-005; the
//! literal collapse of the transitional `bar_items` blob (`BarLoc`, the
//! `barItemPlacements` → `statusBarItemPlacements` migrator) also lands there.

use gpui::{
    div, px, AnyElement, AnyView, Context, Entity, IntoElement, ParentElement, Render, Styled,
    Window,
};
use labonair_panel::{AnyStatusItemHandle, StatusItemConstructor, StatusSide};

use crate::theme::ThemeStore;
use crate::Workspace;

/// Status-bar row height — matches the shell's former `STATUS_H`.
const STATUS_H: f32 = 32.0;

/// Renders the registered [`StatusItem`](labonair_panel::StatusItem)s, sorted
/// by `order` within each side.
pub struct StatusBar {
    workspace: Entity<Workspace>,
    theme: Entity<ThemeStore>,
    /// Built once from the registry: `(side, order, group, handle)`.
    items: Vec<(StatusSide, i32, u32, AnyStatusItemHandle)>,
    built: bool,
}

impl StatusBar {
    pub fn new(
        workspace: Entity<Workspace>,
        theme: Entity<ThemeStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            workspace,
            theme,
            items: Vec::new(),
            built: false,
        }
    }

    /// Instantiate every registered item once (the constructors hand back a
    /// clone of a pre-built entity, so this is cheap refcount work — mirrors
    /// `Workspace::init_docks`). Registration happens at startup before the
    /// first render, so a single lazy build is enough.
    fn ensure_built(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.built {
            return;
        }
        let regs: Vec<(StatusSide, i32, u32, StatusItemConstructor)> = self
            .workspace
            .read(cx)
            .status_item_registry()
            .iter()
            .map(|r| (r.default_side, r.order, r.group, r.build.clone()))
            .collect();
        if regs.is_empty() {
            return;
        }
        for (side, order, group, build) in regs {
            let handle = build(window, cx);
            self.items.push((side, order, group, handle));
        }
        self.built = true;
    }

    /// Views for `side`, sorted by `order`, with a divider inserted between
    /// two consecutive items whose `group` differs (T18-004 point 8 — dividers
    /// only between logical groups, never between every item).
    fn cluster(&self, side: StatusSide, cx: &Context<Self>) -> Vec<AnyElement> {
        let mut v: Vec<(i32, u32, AnyView)> = self
            .items
            .iter()
            .filter(|(s, _, _, _)| *s == side)
            .map(|(_, order, group, h)| (*order, *group, h.to_any()))
            .collect();
        v.sort_by_key(|(order, _, _)| *order);

        let border = self.theme.read(cx).border();
        let mut out = Vec::with_capacity(v.len() * 2);
        let mut prev_group: Option<u32> = None;
        for (_, group, view) in v {
            if let Some(pg) = prev_group {
                if pg != group {
                    out.push(
                        div()
                            .flex_shrink_0()
                            .w(px(1.0))
                            .h(px(14.0))
                            .bg(border)
                            .into_any_element(),
                    );
                }
            }
            prev_group = Some(group);
            out.push(view.into_any_element());
        }
        out
    }
}

impl Render for StatusBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ensure_built(window, cx);

        let (status_bar_bg, muted, border) = {
            let t = self.theme.read(cx);
            (t.status_bar(), t.muted_foreground(), t.border())
        };
        let left = self.cluster(StatusSide::Left, cx);
        let right = self.cluster(StatusSide::Right, cx);

        div()
            .flex()
            .items_center()
            .justify_between()
            .gap_3()
            .h(px(STATUS_H))
            .w_full()
            .flex_shrink_0()
            .px_3()
            .bg(status_bar_bg)
            .border_t_1()
            .border_color(border)
            .text_size(px(11.0))
            .text_color(muted)
            .child(
                div()
                    .flex()
                    .min_w_0()
                    .flex_1()
                    .items_center()
                    .gap_1()
                    .overflow_hidden()
                    .children(left),
            )
            .child(
                div()
                    .flex()
                    .flex_shrink_0()
                    .items_center()
                    .gap_1()
                    .children(right),
            )
    }
}
