//! Step 1 — colour scheme, interface material, pane layout. Applied live.

use gpui::{
    div, AnyView, App, AppContext, Context, Entity, IntoElement, ParentElement, Render,
    SharedString, Styled, Window,
};
use labonair_settings::content::appearance::PaneLayout;
use labonair_settings::content::general::ThemePref;
use labonair_settings::{GeneralSettings, Settings, ThemeSettings};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{
    preview_tile, preview_tile_group, ColorSchemeChoice, InterfaceStyleChoice, Palette,
    PaneLayoutStyle, PreviewKind,
};

use crate::services::{write_user_setting, ApplyAppearance, OnboardingServices};
use crate::step::{OnboardingStep, StepInstance};

pub(super) struct LookStep {
    theme: Entity<ThemeStore>,
    apply: ApplyAppearance,
}

impl LookStep {
    pub(super) fn new(services: &OnboardingServices) -> Self {
        Self {
            theme: services.theme.clone(),
            apply: services.apply_appearance.clone(),
        }
    }
}

impl OnboardingStep for LookStep {
    fn id(&self) -> &'static str {
        "onboarding.look"
    }

    fn order(&self) -> u32 {
        100
    }

    fn title(&self) -> SharedString {
        "Choose your look".into()
    }

    fn hint(&self) -> SharedString {
        "Changes apply right away.".into()
    }

    fn build(&self, _window: &mut Window, cx: &mut App) -> StepInstance {
        let view = cx.new(|_| LookView {
            theme: self.theme.clone(),
            apply: self.apply.clone(),
        });
        StepInstance {
            view: AnyView::from(view),
            on_finish: None,
        }
    }
}

struct LookView {
    theme: Entity<ThemeStore>,
    apply: ApplyAppearance,
}

impl Render for LookView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let scheme = GeneralSettings::try_get(cx)
            .map(|s| s.theme_pref())
            .unwrap_or_default();
        let (glassy, layout) = ThemeSettings::try_get(cx)
            .map(|s| (s.translucent_surfaces(), s.pane_layout()))
            .unwrap_or_default();

        let apply = self.apply.clone();
        let scheme_group = preview_tile_group(
            "onboarding-scheme",
            c,
            match scheme {
                ThemePref::System => "system",
                ThemePref::Light => "light",
                ThemePref::Dark => "dark",
            },
        )
        .tile(preview_tile(
            "system",
            "Auto",
            PreviewKind::ColorScheme(ColorSchemeChoice::Auto),
        ))
        .tile(preview_tile(
            "light",
            "Light",
            PreviewKind::ColorScheme(ColorSchemeChoice::Light),
        ))
        .tile(preview_tile(
            "dark",
            "Dark",
            PreviewKind::ColorScheme(ColorSchemeChoice::Dark),
        ))
        .on_select(move |key, _window, cx| {
            let pref = match key.as_ref() {
                "light" => ThemePref::Light,
                "dark" => ThemePref::Dark,
                _ => ThemePref::System,
            };
            write_user_setting(cx, move |s| s.general.theme = Some(pref));
            apply(cx);
        });

        let apply = self.apply.clone();
        let interface_group = preview_tile_group(
            "onboarding-interface",
            c,
            if glassy { "glassy" } else { "classic" },
        )
        .tile(preview_tile(
            "classic",
            "Classic",
            PreviewKind::InterfaceStyle(InterfaceStyleChoice::Classic),
        ))
        .tile(preview_tile(
            "glassy",
            "Glassy",
            PreviewKind::InterfaceStyle(InterfaceStyleChoice::Glassy),
        ))
        .on_select(move |key, _window, cx| {
            let glassy = key.as_ref() == "glassy";
            write_user_setting(cx, move |s| {
                s.appearance.translucent_surfaces = Some(glassy)
            });
            apply(cx);
        });

        let apply = self.apply.clone();
        let layout_group = preview_tile_group(
            "onboarding-layout",
            c,
            match layout {
                PaneLayout::Comfortable => "comfortable",
                PaneLayout::Compact => "compact",
            },
        )
        .tile(preview_tile(
            "comfortable",
            "Comfortable",
            PreviewKind::PaneLayout(PaneLayoutStyle::Comfortable),
        ))
        .tile(preview_tile(
            "compact",
            "Compact",
            PreviewKind::PaneLayout(PaneLayoutStyle::Compact),
        ))
        .on_select(move |key, _window, cx| {
            let layout = if key.as_ref() == "compact" {
                PaneLayout::Compact
            } else {
                PaneLayout::Comfortable
            };
            write_user_setting(cx, move |s| s.appearance.pane_layout = Some(layout));
            apply(cx);
        });

        let section = |label: &'static str, group: gpui::AnyElement| {
            div()
                .flex()
                .flex_col()
                .gap(c.space(8.0))
                .child(div().text_sm().text_color(c.muted).child(label))
                .child(group)
        };
        div()
            .flex()
            .flex_col()
            .gap(c.space(16.0))
            .child(section("Color scheme", scheme_group.into_any_element()))
            .child(section("Interface", interface_group.into_any_element()))
            .child(section("Pane layout", layout_group.into_any_element()))
    }
}
