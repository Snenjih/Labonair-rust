//! Shell hosting for the first-run onboarding (ADR 0008).
//!
//! The `labonair-onboarding` crate owns steps, progress and the page; the
//! shell only builds its services, opens the page as a workspace tab, closes
//! the tab when the page reports it is done, and triggers it once at startup.

use std::path::PathBuf;
use std::rc::Rc;

use gpui::{AppContext, Context, Focusable, Window};
use labonair_onboarding::{
    register_builtin_steps, OnboardingEvent, OnboardingProgress, OnboardingRegistry,
    OnboardingServices, OnboardingView,
};

use crate::app_shell::AppShell;

pub(crate) struct Onboarding {
    registry: Rc<OnboardingRegistry>,
    progress_path: PathBuf,
}

impl Onboarding {
    pub(crate) fn new(services: OnboardingServices, progress_path: PathBuf) -> Self {
        let mut registry = OnboardingRegistry::new();
        register_builtin_steps(&mut registry, &services)
            .expect("built-in onboarding step ids are unique");
        Self {
            registry: Rc::new(registry),
            progress_path,
        }
    }
}

impl AppShell {
    /// Opens the page on first launch (or when new steps shipped), once every
    /// step has finished its discovery. A no-op when nothing is pending.
    pub(crate) fn start_onboarding(&self, window: &mut Window, cx: &mut Context<Self>) {
        let prepare = self.onboarding.registry.prepare(cx);
        cx.spawn_in(window, async move |this, cx| {
            prepare.await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.open_onboarding(false, window, cx)
            });
        })
        .detach();
    }

    /// `replay` shows every applicable step, completed or not.
    pub(crate) fn open_onboarding(
        &mut self,
        replay: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .workspace
            .update(cx, |workspace, cx| workspace.focus_onboarding_tab(cx))
        {
            return;
        }
        let progress = OnboardingProgress::load(&self.onboarding.progress_path);
        let steps = self.onboarding.registry.pending(&progress, replay, cx);
        if steps.is_empty() {
            return;
        }
        let theme = self.theme.clone();
        let path = self.onboarding.progress_path.clone();
        let view = cx.new(|cx| OnboardingView::new(&steps, theme, progress, path, window, cx));
        let focus = view.focus_handle(cx);
        cx.subscribe_in(&view, window, |this, _, _: &OnboardingEvent, window, cx| {
            this.workspace.update(cx, |workspace, cx| {
                workspace.close_onboarding_tab(window, cx)
            });
        })
        .detach();
        self.workspace.update(cx, |workspace, cx| {
            workspace.open_onboarding_tab(view.into(), focus.clone(), cx);
        });
        window.focus(&focus);
    }
}
