//! Step 1 — what opens on launch. Applied live.

use gpui::{AnyView, App, AppContext, Context, Entity, IntoElement, Render, SharedString, Window};
use labonair_settings::content::general::StartupTab;
use labonair_settings::{GeneralSettings, Settings};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{segmented_control, settings_group, settings_row, switch, Palette};

use crate::services::{write_user_setting, OnboardingServices};
use crate::step::{OnboardingStep, StepInstance};

pub(super) struct StartStep {
    theme: Entity<ThemeStore>,
}

impl StartStep {
    pub(super) fn new(services: &OnboardingServices) -> Self {
        Self {
            theme: services.theme.clone(),
        }
    }
}

impl OnboardingStep for StartStep {
    fn id(&self) -> &'static str {
        "onboarding.start"
    }

    fn order(&self) -> u32 {
        100
    }

    fn title(&self) -> SharedString {
        "How should Labonair start?".into()
    }

    fn hint(&self) -> SharedString {
        "Applies from the next launch.".into()
    }

    fn build(&self, _window: &mut Window, cx: &mut App) -> StepInstance {
        let theme = self.theme.clone();
        StepInstance {
            view: AnyView::from(cx.new(|_| StartView { theme })),
            on_finish: None,
        }
    }
}

struct StartView {
    theme: Entity<ThemeStore>,
}

impl Render for StartView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let (tab, restore) = GeneralSettings::try_get(cx)
            .map(|s| (s.default_startup_tab(), s.session_restore()))
            .unwrap_or_default();

        let tab_control = segmented_control(
            "onboarding-startup-tab",
            c,
            match tab {
                StartupTab::Terminal => "terminal",
                StartupTab::Empty => "empty",
            },
        )
        .segment("terminal", "Open a terminal")
        .segment("empty", "Start empty")
        .on_select(|key, _window, cx| {
            let tab = if key.as_ref() == "terminal" {
                StartupTab::Terminal
            } else {
                StartupTab::Empty
            };
            write_user_setting(cx, move |s| s.general.default_startup_tab = Some(tab));
        });

        let restore_control =
            switch("onboarding-session-restore", c, restore).on_click(|on, _window, cx| {
                let on = *on;
                write_user_setting(cx, move |s| s.general.session_restore = Some(on));
            });

        settings_group("onboarding-start", c)
            .row(
                settings_row("onboarding-startup-tab-row", c, "On launch")
                    .description("What you see when Labonair opens.")
                    .control(tab_control),
            )
            .row(
                settings_row(
                    "onboarding-session-restore-row",
                    c,
                    "Restore previous session",
                )
                .description("Reopen your tabs and splits from last time.")
                .control(restore_control),
            )
    }
}
