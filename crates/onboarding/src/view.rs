//! The onboarding page: a scrollable column with one section per registered
//! step and a Finish / Skip pair in the header, in the shape of Zed's
//! onboarding page. The shell hosts it as a workspace tab.
//!
//! Finishing runs the steps' deferred work (the host import); skipping does
//! not. Either way every shown step counts as seen and the page asks its host
//! to close it with [`OnboardingEvent::Closed`].

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{
    div, px, AnyView, App, Context, Entity, EventEmitter, FocusHandle, Focusable, FontWeight,
    InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{
    button, divider, Axis, ButtonSize, ButtonVariant, Palette, ScrollableElement,
};

use crate::progress::OnboardingProgress;
use crate::step::{OnFinish, OnboardingStep};

/// Maximum width of the page column, as in Zed's onboarding page.
const PAGE_WIDTH: f32 = 720.0;

pub enum OnboardingEvent {
    /// The user finished or skipped; progress is saved and deferred work ran.
    Closed,
}

struct BuiltStep {
    id: &'static str,
    title: SharedString,
    hint: SharedString,
    view: AnyView,
    on_finish: Option<OnFinish>,
}

pub struct OnboardingView {
    focus: FocusHandle,
    theme: Entity<ThemeStore>,
    steps: Vec<BuiltStep>,
    progress: OnboardingProgress,
    progress_path: PathBuf,
}

impl OnboardingView {
    /// `steps` are the steps to show, already filtered and ordered (see
    /// [`crate::OnboardingRegistry::pending`]).
    pub fn new(
        steps: &[Rc<dyn OnboardingStep>],
        theme: Entity<ThemeStore>,
        progress: OnboardingProgress,
        progress_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let steps = steps
            .iter()
            .map(|step| {
                let instance = step.build(window, cx);
                BuiltStep {
                    id: step.id(),
                    title: step.title(),
                    hint: step.hint(),
                    view: instance.view,
                    on_finish: instance.on_finish,
                }
            })
            .collect();
        Self {
            focus: cx.focus_handle(),
            theme,
            steps,
            progress,
            progress_path,
        }
    }

    /// `apply` decides whether the steps' deferred choices run.
    fn close(&mut self, apply: bool, cx: &mut Context<Self>) {
        self.progress
            .complete(self.steps.iter().map(|step| step.id));
        if let Err(error) = self.progress.save(&self.progress_path) {
            tracing::warn!(%error, "labonair-onboarding: cannot save progress");
        }
        for step in &mut self.steps {
            if let Some(on_finish) = step.on_finish.take() {
                if apply {
                    on_finish(cx);
                }
            }
        }
        cx.emit(OnboardingEvent::Closed);
    }
}

impl EventEmitter<OnboardingEvent> for OnboardingView {}

impl Focusable for OnboardingView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for OnboardingView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));

        let skip = button("onboarding-skip", c, ButtonVariant::Subtle, ButtonSize::Sm)
            .child("Skip")
            .on_click(cx.listener(|this, _, _window, cx| this.close(false, cx)));
        let finish = button(
            "onboarding-finish",
            c,
            ButtonVariant::Action,
            ButtonSize::Sm,
        )
        .child("Finish setup")
        .on_click(cx.listener(|this, _, _window, cx| this.close(true, cx)));

        let header =
            div()
                .flex()
                .items_center()
                .justify_between()
                .gap(c.space(16.0))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(c.space(4.0))
                        .child(
                            div()
                                .text_size(px(22.0))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Welcome to Labonair"),
                        )
                        .child(div().text_sm().text_color(c.muted).child(
                            "Set up the basics. You can change all of this later in Settings.",
                        )),
                )
                .child(div().flex().gap(c.space(8.0)).child(skip).child(finish));

        let mut column = div()
            .w_full()
            .max_w(px(PAGE_WIDTH))
            .mx_auto()
            .p(c.space(48.0))
            .flex()
            .flex_col()
            .gap(c.space(24.0))
            .child(header);
        for step in &self.steps {
            column = column
                .child(divider(Axis::Horizontal, c.line_subtle))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(c.space(12.0))
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .gap(c.space(2.0))
                                .child(
                                    div()
                                        .text_base()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(step.title.clone()),
                                )
                                .child(
                                    div().text_sm().text_color(c.muted).child(step.hint.clone()),
                                ),
                        )
                        .child(step.view.clone()),
                );
        }

        div()
            .id("onboarding-page")
            .track_focus(&self.focus)
            .key_context("Onboarding")
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
            .overflow_y_scrollbar()
            .child(column)
    }
}
