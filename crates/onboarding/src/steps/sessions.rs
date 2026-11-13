//! Step — should terminals and connections outlive the app? Unix only.

use gpui::px;
use gpui::{
    div, prelude::*, AnyView, App, AppContext, ClickEvent, Context, Entity, FontWeight,
    IntoElement, Render, SharedString, Window,
};
use labonair_settings::content::terminal::SessionHostMode;
use labonair_settings::{Settings, TerminalSettings};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::Palette;

use crate::services::{write_user_setting, OnboardingServices};
use crate::step::{OnboardingStep, StepInstance};

pub(super) struct SessionsStep {
    theme: Entity<ThemeStore>,
}

impl SessionsStep {
    pub(super) fn new(services: &OnboardingServices) -> Self {
        Self {
            theme: services.theme.clone(),
        }
    }
}

impl OnboardingStep for SessionsStep {
    fn id(&self) -> &'static str {
        "onboarding.sessions"
    }

    fn order(&self) -> u32 {
        250
    }

    fn title(&self) -> SharedString {
        "Keep sessions running?".into()
    }

    fn hint(&self) -> SharedString {
        "Applies to sessions you open from now on.".into()
    }

    /// The background session host is unix-only.
    fn is_applicable(&self, _cx: &App) -> bool {
        cfg!(unix)
    }

    fn build(&self, _window: &mut Window, cx: &mut App) -> StepInstance {
        let theme = self.theme.clone();
        let selected = TerminalSettings::try_get(cx)
            .map(|s| s.session_host())
            .unwrap_or_default();
        StepInstance {
            view: AnyView::from(cx.new(|_| SessionsView { theme, selected })),
            on_finish: None,
        }
    }
}

struct SessionsView {
    theme: Entity<ThemeStore>,
    selected: SessionHostMode,
}

const OPTIONS: [(SessionHostMode, &str, &str); 2] = [
    (
        SessionHostMode::App,
        "Tied to the app",
        "Sessions end when you quit Labonair.",
    ),
    (
        SessionHostMode::Detached,
        "Keep running in the background",
        "Quitting Labonair leaves sessions running; reopen it to pick them up again.",
    ),
];

impl Render for SessionsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let tiles = OPTIONS.into_iter().map(|(mode, title, description)| {
            let on = self.selected == mode;
            div()
                .id(SharedString::from(format!("onboarding-sessions-{title}")))
                .flex_1()
                .flex()
                .flex_col()
                .gap(c.space(4.0))
                .p(c.space(12.0))
                .rounded(px(c.radius.lg))
                .border_2()
                .border_color(if on { c.primary } else { c.line_subtle })
                .bg(if on { c.selected_fill } else { c.card })
                .cursor_pointer()
                .hover(|tile| tile.bg(c.hover))
                .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                    this.selected = mode;
                    write_user_setting(cx, move |s| s.terminal.session_host = Some(mode));
                    cx.notify();
                }))
                .child(
                    div()
                        .text_color(c.fg)
                        .when(on, |t| t.font_weight(FontWeight::BOLD))
                        .child(title),
                )
                .child(
                    div()
                        .text_size(c.control_space(12.0))
                        .text_color(c.muted)
                        .child(description),
                )
        });
        div()
            .flex()
            .flex_col()
            .gap(c.space(16.0))
            .child(div().flex().flex_row().gap(c.space(12.0)).children(tiles))
            .child(div().text_sm().text_color(c.muted).child(
                "In the background, terminals, SSH and SFTP sessions and file transfers \
                     keep running after you quit and can be re-attached when you open \
                     Labonair again. You can change this later in Settings > Terminal.",
            ))
    }
}
