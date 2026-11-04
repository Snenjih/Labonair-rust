//! Component gallery — a debug-only window that renders every `ui-kit`
//! primitive across its variants, sizes and states (T20-004).
//!
//! It is a hand-maintained page, not a `Component`-trait registry (Zed's
//! `crates/component_preview/` is the full version; a one-person project does
//! not need the registry machinery). Opened from the command palette's
//! debug-only "Open Component Gallery" row, which calls
//! [`open_gallery_window`].
//!
//! ## What is real and what is simulated
//!
//! GPUI has no way to force an element into its `:hover` / `:active` visual
//! state from code, so **hover and press states are not shown** — move the
//! mouse over a control in the running window to see them. Everything else is
//! real: `disabled`, `selected`, `pressed` (toggles), `checked`, the severity
//! tints, the min/max clamp of `NumberField`, the open/closed chevron of
//! `Disclosure`. The theme switch at the top flips the live
//! [`labonair_theme::ThemeStore`] preference, so every primitive below
//! re-renders in Light or Dark. Moved from `labonair-ui-kit` to
//! `labonair-shell` in R08-008 (it holds an `Entity<ThemeStore>`, a runtime
//! type the foundation crate no longer sees).

use std::time::Instant;

use gpui::{
    div, point, prelude::FluentBuilder, px, size, App, AppContext, Bounds, ClickEvent, Context,
    Entity, FontWeight, Global, InteractiveElement, IntoElement, ParentElement, Pixels, Point,
    Render, SharedString, StatefulInteractiveElement, Styled, TitlebarOptions, Window,
    WindowBounds, WindowHandle, WindowKind, WindowOptions,
};
use gpui_component::Root;

use labonair_theme::{theme_store, ThemePreference, ThemeStore};

use labonair_ui_kit::{
    banner, button, category_chip, checkbox, context_menu_with_exit, dialog_surface, disclosure,
    divider, floating_tab_strip, h_stack, icon_button, icon_toggle_button, indicator, kbd_row,
    keybinding_hint, list_header, menu_card_preview, modal_overlay_with_exit, number_field,
    pane_bar, pane_frame, preview_tile, preview_tile_group, segmented_control, select_trigger,
    settings_group, settings_row, slider, slider_row, tab_drop_marker, tab_group_gap, tab_item,
    toggle_base, Axis, ButtonSize, ButtonVariant, ChipHue, ColorSchemeChoice, IconButtonShape,
    IconName, IndicatorSize, InterfaceStyleChoice, ListItem, MenuItem, MotionExt, MotionLevel,
    MotionPrefs, Palette, PaneAction, PaneLayoutStyle, Presence, PresenceStep, PreviewKind,
    SegmentSize, SegmentVariant, Severity, SliderState, TabDropSide, TabLayout, TabStyle, TileMock,
    ToggleSize, ToggleVariant,
};

fn noop_tab_activate(_: &mut Window, _: &mut App) {}

fn noop_tab_navigate(_: bool, _: &mut Window, _: &mut App) {}

fn noop_tab_close(_: &mut Window, _: &mut App) {}

/// The gallery view. Holds only the handful of caller-owned flags the
/// interactive primitives need (`Disclosure`, `SegmentedControl`,
/// `NumberField`, `Checkbox`) plus the theme handle it re-renders off.
pub struct Gallery {
    theme: Entity<ThemeStore>,
    disclosure_open: bool,
    segment: SharedString,
    number: f64,
    checkbox_on: bool,
    slider: Entity<SliderState>,
    /// Overrides the palette's motion level for the whole page (Motion demo).
    motion_level: MotionLevel,
    tile_scheme: SharedString,
    tile_style: SharedString,
    tile_layout: SharedString,
    tile_theme: SharedString,
    pane_zoomed: bool,
    /// Bumped by the Motion demo's replay buttons to restart animations.
    motion_nonce: u32,
    reveal_open: bool,
    state_on: bool,
    menu_anchor: Point<Pixels>,
    menu_presence: Presence,
    dialog_presence: Presence,
}

impl Gallery {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        let slider = cx.new(|_| {
            SliderState::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(42.0)
        });
        Self {
            theme,
            disclosure_open: true,
            segment: "outline".into(),
            number: 12.0,
            checkbox_on: true,
            slider,
            motion_level: MotionLevel::Full,
            tile_scheme: "auto".into(),
            tile_style: "classic".into(),
            tile_layout: "comfortable".into(),
            tile_theme: "labonair".into(),
            pane_zoomed: false,
            motion_nonce: 0,
            reveal_open: true,
            state_on: false,
            menu_anchor: Point::default(),
            menu_presence: Presence::new(),
            dialog_presence: Presence::new(),
        }
    }
}

/// A titled card wrapping one primitive's demo rows.
fn section(title: &str, c: Palette, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .w_full()
        .child(
            div()
                .text_size(px(13.0))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(c.fg)
                .child(title.to_string()),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .w_full()
                .p_3()
                .rounded_md()
                .bg(c.card)
                .border_1()
                .border_color(c.border)
                .child(body),
        )
}

/// A labelled row of demo elements.
fn row(label: &str, c: Palette, body: impl IntoElement) -> impl IntoElement {
    h_stack()
        .gap_3()
        .w_full()
        .child(
            div()
                .w(px(120.0))
                .flex_shrink_0()
                .text_size(px(11.0))
                .text_color(c.muted)
                .child(label.to_string()),
        )
        .child(h_stack().flex_wrap().gap_2().child(body))
}

impl Render for Gallery {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let mut c = Palette::from_theme(self.theme.read(cx));
        c.motion.level = self.motion_level;

        // Exit-state demos: advance the presence machines once per frame.
        let now = Instant::now();
        // (Focus return would key off `PresenceStep::JustClosed` here.)
        let _: PresenceStep = self.menu_presence.tick(now, &c.motion);
        let _: PresenceStep = self.dialog_presence.tick(now, &c.motion);
        let view = cx.entity();
        let close_menu = {
            let view = view.clone();
            move |_: &mut Window, cx: &mut App| {
                view.update(cx, |this, cx| this.close_menu(cx));
            }
        };

        div()
            .id("gallery-root")
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
            .when(self.menu_presence.is_mounted(), |root| {
                let item = |id: &'static str, label: &'static str| {
                    let view = view.clone();
                    MenuItem::new(id, label).on_click(move |_, _, cx| {
                        view.update(cx, |this, cx| this.close_menu(cx));
                    })
                };
                root.child(context_menu_with_exit(
                    self.menu_anchor,
                    c,
                    self.menu_presence.is_closing(),
                    close_menu,
                    vec![
                        item("gm-a", "Fade out on pick"),
                        item("gm-b", "Second item"),
                    ],
                ))
            })
            .when(self.dialog_presence.is_mounted(), |root| {
                let view = view.clone();
                root.child(modal_overlay_with_exit(
                    "gallery-dialog",
                    c,
                    self.dialog_presence.is_closing(),
                    dialog_surface("gallery-dialog-card", c)
                        .w(px(320.0))
                        .child(
                            div()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("Exit animation"),
                        )
                        .child(
                            div()
                                .text_color(c.muted)
                                .child("The scrim and card fade out over 120 ms, then unmount."),
                        )
                        .child(
                            button(
                                "gallery-dialog-close",
                                c,
                                ButtonVariant::Outlined,
                                ButtonSize::Sm,
                            )
                            .child("Close")
                            .on_click(move |_, _, cx| {
                                view.update(cx, |this, cx| this.close_dialog(cx));
                            }),
                        ),
                ))
            })
            .child(
                div()
                    .id("gallery-scroll")
                    .size_full()
                    .overflow_y_scroll()
                    .p_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_4()
                            .child(self.render_header(c, cx))
                            .child(self.render_buttons(c))
                            .child(self.render_toggles(c, cx))
                            .child(self.render_checkbox(c, cx))
                            .child(self.render_list(c))
                            .child(self.render_disclosure(c, cx))
                            .child(self.render_segmented(c, cx))
                            .child(self.render_tabs(c))
                            .child(self.render_floating_tabs(c))
                            .child(self.render_panes(c, cx))
                            .child(self.render_settings_widgets(c, cx))
                            .child(self.render_motion(c, cx))
                            .child(self.render_number(c, cx))
                            .child(self.render_slider(c))
                            .child(self.render_select(c))
                            .child(self.render_banner(c))
                            .child(self.render_kbd(c))
                            .child(self.render_context_menu(c))
                            .child(self.render_misc(c)),
                    ),
            )
    }
}

impl Gallery {
    fn render_slider(&self, c: Palette) -> impl IntoElement {
        section(
            "Slider",
            c,
            row("0..100", c, slider(&self.slider, c).w(px(220.0))),
        )
    }

    fn render_header(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let pref = self.theme.read(cx).preference();
        let switch = |label: &'static str, id: &'static str, p: ThemePreference| {
            let active = pref == p;
            button(id, c, ButtonVariant::Outlined, ButtonSize::Sm)
                .when(active, |b| b.bg(c.accent).text_color(c.accent_fg))
                .child(label)
                .on_click(cx.listener(move |this: &mut Gallery, _, _w, cx| {
                    this.theme.update(cx, |s, cx| s.set_preference(p, cx));
                }))
        };
        let system = switch("System", "gallery-theme-system", ThemePreference::System);
        let light = switch("Light", "gallery-theme-light", ThemePreference::Light);
        let dark = switch("Dark", "gallery-theme-dark", ThemePreference::Dark);
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(
                div()
                    .text_size(px(18.0))
                    .font_weight(FontWeight::BOLD)
                    .child("Component Gallery"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child("Every labonair-ui-kit primitive. Hover/press states are live in the window, not pre-rendered here."),
            )
            .child(
                h_stack()
                    .gap_2()
                    .pt_2()
                    .child(
                        div()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child("Theme"),
                    )
                    .child(system)
                    .child(light)
                    .child(dark),
            )
    }

    fn render_buttons(&self, c: Palette) -> impl IntoElement {
        let variants = [
            (ButtonVariant::Subtle, "Subtle"),
            (ButtonVariant::Filled, "Filled"),
            (ButtonVariant::Outlined, "Outlined"),
            (ButtonVariant::OutlinedGhost, "Outlined Ghost"),
            (ButtonVariant::TintedError, "Tinted Error"),
            (ButtonVariant::Link, "Link"),
            (ButtonVariant::Transparent, "Transparent"),
        ];
        let sizes = [
            (ButtonSize::None, "None"),
            (ButtonSize::Xs, "Compact"),
            (ButtonSize::Default, "Default"),
            (ButtonSize::Sm, "Medium"),
            (ButtonSize::Lg, "Large"),
        ];
        let mut body = div().flex().flex_col().gap_2();
        for (v, vname) in variants {
            let mut r = h_stack().flex_wrap().gap_2();
            for (s, sname) in sizes {
                r = r.child(
                    button(SharedString::from(format!("btn-{vname}-{sname}")), c, v, s)
                        .child(SharedString::from(format!("{vname} {sname}"))),
                );
            }
            body = body.child(row(vname, c, r));
        }
        // Icon sizes, shapes, selected state, indicator, and inert state.
        let icons = h_stack()
            .flex_wrap()
            .gap_2()
            .child(icon_button(
                "btn-icon-xs",
                c,
                IconName::Plus,
                ButtonVariant::Outlined,
                ButtonSize::IconXs,
            ))
            .child(icon_button(
                "btn-icon-sm",
                c,
                IconName::Plus,
                ButtonVariant::Outlined,
                ButtonSize::IconSm,
            ))
            .child(icon_button(
                "btn-icon-md",
                c,
                IconName::Plus,
                ButtonVariant::Outlined,
                ButtonSize::Icon,
            ))
            .child(icon_button(
                "btn-icon-lg",
                c,
                IconName::Plus,
                ButtonVariant::Outlined,
                ButtonSize::IconLg,
            ))
            .child(icon_button(
                "btn-icon-none",
                c,
                IconName::Plus,
                ButtonVariant::Outlined,
                ButtonSize::IconNone,
            ))
            .child(
                labonair_ui_kit::icon_button_builder("btn-icon-wide", c, IconName::Plus)
                    .variant(ButtonVariant::Outlined)
                    .size(ButtonSize::Sm)
                    .shape(IconButtonShape::Wide)
                    .tooltip("Wide icon button")
                    .render(),
            )
            .child(
                labonair_ui_kit::icon_button_builder("btn-icon-selected", c, IconName::PanelLeft)
                    .selected(true)
                    .tooltip("Selected panel")
                    .render(),
            )
            .child(
                labonair_ui_kit::icon_button_builder("btn-icon-indicator", c, IconName::Bell)
                    .indicator(labonair_ui_kit::IndicatorSize::Xs, c.warning)
                    .indicator_border_color(c.card)
                    .tooltip("Notifications")
                    .render(),
            )
            .child(
                labonair_ui_kit::button_disabled(
                    "btn-disabled",
                    c,
                    ButtonVariant::Subtle,
                    ButtonSize::Sm,
                )
                .child("Disabled"),
            );
        body = body.child(row("Icon states and shapes", c, icons));
        section("Button", c, body)
    }

    fn render_toggles(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let icon_row = h_stack()
            .flex_wrap()
            .gap_2()
            .child(icon_toggle_button(
                "tg-icon-off",
                c,
                IconName::PanelLeft,
                false,
            ))
            .child(icon_toggle_button(
                "tg-icon-on",
                c,
                IconName::PanelLeft,
                true,
            ))
            .child(icon_toggle_button(
                "tg-icon-b-off",
                c,
                IconName::PanelBottom,
                false,
            ))
            .child(icon_toggle_button(
                "tg-icon-b-on",
                c,
                IconName::PanelBottom,
                true,
            ));
        let _ = cx;

        let mut labelled = h_stack().flex_wrap().gap_2();
        for (v, vname) in [
            (ToggleVariant::Default, "Default"),
            (ToggleVariant::Outline, "Outline"),
        ] {
            for (s, sname) in [
                (ToggleSize::Xs, "Xs"),
                (ToggleSize::Sm, "Sm"),
                (ToggleSize::Md, "Md"),
            ] {
                labelled = labelled.child(
                    toggle_base(
                        SharedString::from(format!("tg-{vname}-{sname}")),
                        c,
                        v,
                        s,
                        sname == "Sm",
                        false,
                    )
                    .child(SharedString::from(format!("{vname}/{sname}"))),
                );
            }
        }
        labelled = labelled.child(
            toggle_base(
                "tg-disabled",
                c,
                ToggleVariant::Outline,
                ToggleSize::Sm,
                true,
                true,
            )
            .child("Disabled"),
        );

        section(
            "ToggleButton",
            c,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(row("icon (off / on)", c, icon_row))
                .child(row("labelled", c, labelled)),
        )
    }

    fn render_checkbox(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let interactive = checkbox("cb-interactive", c, self.checkbox_on)
            .label("Interactive (click me)")
            .on_click(cx.listener(|this: &mut Gallery, v: &bool, _w, cx| {
                this.checkbox_on = *v;
                cx.notify();
            }));
        section(
            "Checkbox",
            c,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(row(
                    "unchecked",
                    c,
                    checkbox("cb-off", c, false).label("Off"),
                ))
                .child(row("checked", c, checkbox("cb-on", c, true).label("On")))
                .child(row(
                    "disabled",
                    c,
                    checkbox("cb-dis", c, true).label("Disabled").disabled(true),
                ))
                .child(row("stateful", c, interactive)),
        )
    }

    fn render_list(&self, c: Palette) -> impl IntoElement {
        let group = div()
            .flex()
            .flex_col()
            .w(px(320.0))
            .child(list_header("Section", c.muted))
            .child(ListItem::new("li-plain", c).child("Plain row"))
            .child(
                ListItem::new("li-icon", c)
                    .icon(IconName::File)
                    .child("With leading icon"),
            )
            .child(
                ListItem::new("li-sel", c)
                    .icon(IconName::Sparkles)
                    .selected(true)
                    .trailing(kbd_row(["\u{2318}", "K"], c))
                    .child("Selected + trailing kbd"),
            )
            .child(
                ListItem::new("li-dis", c)
                    .icon(IconName::Lock)
                    .disabled(true)
                    .child("Disabled row"),
            );
        section("ListItem", c, group)
    }

    fn render_disclosure(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let d = disclosure(
            "gallery-disc",
            "Toggle me",
            !self.disclosure_open,
            c.muted,
            c.fg,
        )
        .on_click(cx.listener(|this: &mut Gallery, _, _w, cx| {
            this.disclosure_open = !this.disclosure_open;
            cx.notify();
        }));
        section(
            "Disclosure",
            c,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(d)
                .when(self.disclosure_open, |el| {
                    el.child(
                        div()
                            .pl_4()
                            .text_size(px(11.0))
                            .text_color(c.muted)
                            .child("…expanded content (state is caller-owned)"),
                    )
                }),
        )
    }

    fn render_segmented(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let live = segmented_control("seg-live", c, self.segment.clone())
            .segments([
                ("outline", "Outline"),
                ("solid", "Solid"),
                ("third", "Third"),
            ])
            .on_select(cx.listener(|this: &mut Gallery, k: &SharedString, _w, cx| {
                this.segment = k.clone();
                cx.notify();
            }));
        let mut variants = div().flex().flex_col().gap_2();
        for (v, vname) in [
            (SegmentVariant::Outline, "outline"),
            (SegmentVariant::Solid, "solid"),
        ] {
            let mut r = h_stack().flex_wrap().gap_3();
            for (s, sname) in [
                (SegmentSize::Xs, "Xs"),
                (SegmentSize::Sm, "Sm"),
                (SegmentSize::Md, "Md"),
            ] {
                r = r.child(
                    segmented_control(SharedString::from(format!("seg-{vname}-{sname}")), c, "a")
                        .variant(v)
                        .size(s)
                        .segments([("a", "One"), ("b", "Two")]),
                );
            }
            variants = variants.child(row(vname, c, r));
        }
        section(
            "SegmentedControl",
            c,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(row("stateful", c, live))
                .child(variants),
        )
    }

    fn render_tabs(&self, c: Palette) -> impl IntoElement {
        let horizontal = h_stack()
            .gap_1()
            .child(
                tab_item("gallery-tab-editor", c, "src/main.rs")
                    .leading(IconName::SquarePen)
                    .selected(true)
                    .dirty(true)
                    .on_activate(noop_tab_activate)
                    .on_navigate(noop_tab_navigate)
                    .on_close(noop_tab_close)
                    .render(),
            )
            .child(
                tab_item("gallery-tab-preview", c, "README.md")
                    .leading(IconName::FileText)
                    .busy(true)
                    .peek(true)
                    .on_activate(noop_tab_activate)
                    .on_navigate(noop_tab_navigate)
                    .on_close(noop_tab_close)
                    .render(),
            )
            .child(
                tab_item("gallery-tab-disabled", c, "Unavailable")
                    .leading(IconName::File)
                    .disabled(true)
                    .render(),
            );
        let vertical = div()
            .flex()
            .flex_col()
            .gap_1()
            .w(px(280.0))
            .child(
                tab_item("gallery-tab-sidebar-editor", c, "src/main.rs")
                    .layout(TabLayout::Vertical)
                    .leading(IconName::SquarePen)
                    .selected(true)
                    .dirty(true)
                    .on_activate(noop_tab_activate)
                    .on_navigate(noop_tab_navigate)
                    .on_close(noop_tab_close)
                    .render(),
            )
            .child(
                tab_item("gallery-tab-sidebar-terminal", c, "Terminal")
                    .layout(TabLayout::Vertical)
                    .leading(IconName::ToolTerminal)
                    .on_activate(noop_tab_activate)
                    .on_navigate(noop_tab_navigate)
                    .on_close(noop_tab_close)
                    .render(),
            );
        section(
            "TabItem",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row("horizontal", c, horizontal))
                .child(row("vertical", c, vertical)),
        )
    }

    fn render_number(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let live = number_field("nf-live", c, self.number, 8.0, 32.0, 1.0).on_change(cx.listener(
            |this: &mut Gallery, v: &f64, _w, cx| {
                this.number = *v;
                cx.notify();
            },
        ));
        section(
            "NumberField",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row("8..32, step 1", c, live))
                .child(row(
                    "at min (8)",
                    c,
                    number_field("nf-min", c, 8.0, 8.0, 32.0, 1.0),
                ))
                .child(row(
                    "at max (32)",
                    c,
                    number_field("nf-max", c, 32.0, 8.0, 32.0, 1.0),
                ))
                .child(row(
                    "float, no track",
                    c,
                    number_field("nf-float", c, 0.5, 0.0, 1.0, 0.05)
                        .decimals(2)
                        .track(false),
                ))
                .child(row(
                    "disabled",
                    c,
                    number_field("nf-dis", c, 12.0, 8.0, 32.0, 1.0).disabled(true),
                )),
        )
    }

    fn render_select(&self, c: Palette) -> impl IntoElement {
        // The open list (`select_popover`) is a `deferred`/`anchored` overlay
        // and needs a live anchor + open flag; that interaction is exercised
        // in Settings. Here we show the trigger in both visual states.
        section(
            "Select",
            c,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(row(
                    "closed",
                    c,
                    select_trigger("sel-closed", c, "Block cursor", false),
                ))
                .child(row(
                    "open",
                    c,
                    select_trigger("sel-open", c, "Bar cursor", true),
                )),
        )
    }

    fn render_banner(&self, c: Palette) -> impl IntoElement {
        let mut body = div().flex().flex_col().gap_2().w_full();
        for (sev, label) in [
            (
                Severity::Note,
                "Note — a neutral card-tinted message, no icon.",
            ),
            (Severity::Info, "Info — informational, drawn from --info."),
            (Severity::Success, "Success — drawn from --success."),
            (Severity::Warning, "Warning — drawn from --warning."),
            (Severity::Error, "Error — drawn from --error."),
        ] {
            body = body.child(banner(sev, c).child(label));
        }
        section("Banner", c, body)
    }

    fn render_kbd(&self, c: Palette) -> impl IntoElement {
        section(
            "Kbd / KeybindingHint",
            c,
            div()
                .flex()
                .flex_col()
                .gap_2()
                .child(row("single chord", c, kbd_row(["\u{2318}", "K"], c)))
                .child(row(
                    "multi chord",
                    c,
                    kbd_row(["\u{2303}", "\u{2325}", "\u{21E7}", "F"], c),
                ))
                .child(row(
                    "hint",
                    c,
                    keybinding_hint("Toggle Sidebar", ["\u{2318}", "B"], c),
                )),
        )
    }

    fn render_context_menu(&self, c: Palette) -> impl IntoElement {
        // `menu_card_preview` is the bare card (no backdrop/anchor) so it can
        // sit inline as a "permanently open" example.
        let menu = menu_card_preview(
            c,
            vec![
                MenuItem::label("Section"),
                MenuItem::new("cm-copy", "Copy")
                    .icon(IconName::Copy)
                    .keybind(["\u{2318}", "C"]),
                MenuItem::new("cm-paste", "Paste").icon(IconName::Copy),
                MenuItem::new("cm-checked", "Word Wrap").checked(true),
                MenuItem::separator(),
                MenuItem::submenu(
                    "cm-more",
                    "More",
                    vec![
                        MenuItem::new("cm-more-a", "Nested A"),
                        MenuItem::new("cm-more-b", "Nested B"),
                    ],
                ),
                MenuItem::new("cm-disabled", "Unavailable").disabled(true),
                MenuItem::separator(),
                MenuItem::new("cm-delete", "Delete")
                    .icon(IconName::Trash)
                    .destructive(),
            ],
        );
        section("ContextMenu (permanently open)", c, menu)
    }

    fn render_misc(&self, c: Palette) -> impl IntoElement {
        let dividers = div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(240.0))
            .child(div().child("above"))
            .child(divider(Axis::Horizontal, c.border))
            .child(
                h_stack()
                    .h(px(24.0))
                    .gap_2()
                    .child(div().child("left"))
                    .child(divider(Axis::Vertical, c.border))
                    .child(div().child("right")),
            );
        let dots = h_stack()
            .flex_wrap()
            .gap_3()
            .child(indicator(IndicatorSize::Xs, c.muted))
            .child(indicator(IndicatorSize::Sm, c.success))
            .child(indicator(IndicatorSize::Sm, c.warning))
            .child(indicator(IndicatorSize::Sm, c.error))
            .child(indicator(IndicatorSize::Md, c.info));
        section(
            "Divider / Indicator",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row("divider", c, dividers))
                .child(row("indicator", c, dots)),
        )
    }
}

/// R10-002 foundation demos: floating tabs, pane chrome, Settings widgets and
/// the motion engine.
impl Gallery {
    fn motion(&self, cx: &Context<Self>) -> MotionPrefs {
        let mut motion = Palette::from_theme(self.theme.read(cx)).motion;
        motion.level = self.motion_level;
        motion
    }

    fn close_menu(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.menu_presence.request_close(Instant::now(), &motion);
        cx.notify();
    }

    fn close_dialog(&mut self, cx: &mut Context<Self>) {
        let motion = self.motion(cx);
        self.dialog_presence.request_close(Instant::now(), &motion);
        cx.notify();
    }

    fn render_floating_tabs(&self, c: Palette) -> impl IntoElement {
        let block = |id: &'static str, label: &'static str, icon: IconName, layout: TabLayout| {
            tab_item(id, c, label)
                .style(TabStyle::Floating)
                .layout(layout)
                .leading(icon)
                .on_activate(noop_tab_activate)
                .on_navigate(noop_tab_navigate)
                .on_close(noop_tab_close)
        };
        let horizontal = floating_tab_strip("gf-strip-h", c, TabLayout::Horizontal)
            .bg(c.toolbar_bg)
            .rounded(px(c.radius.md))
            .child(
                block("gf-pinned", "Pinned", IconName::Pin, TabLayout::Horizontal)
                    .pinned(true)
                    .render(),
            )
            .child(tab_group_gap(c, TabLayout::Horizontal))
            .child(
                block(
                    "gf-selected",
                    "src/main.rs",
                    IconName::SquarePen,
                    TabLayout::Horizontal,
                )
                .selected(true)
                .dirty(true)
                .render(),
            )
            .child(
                block(
                    "gf-drop",
                    "drop target",
                    IconName::File,
                    TabLayout::Horizontal,
                )
                .render()
                .child(tab_drop_marker(
                    c,
                    TabLayout::Horizontal,
                    TabDropSide::Before,
                )),
            )
            .child(
                block(
                    "gf-terminal",
                    "Terminal",
                    IconName::ToolTerminal,
                    TabLayout::Horizontal,
                )
                .busy(true)
                .render(),
            )
            .child(
                block(
                    "gf-disabled",
                    "Unavailable",
                    IconName::File,
                    TabLayout::Horizontal,
                )
                .disabled(true)
                .render(),
            );
        let vertical = floating_tab_strip("gf-strip-v", c, TabLayout::Vertical)
            .w(px(280.0))
            .bg(c.sidebar)
            .rounded(px(c.radius.md))
            .child(
                block(
                    "gf-v-selected",
                    "src/main.rs",
                    IconName::SquarePen,
                    TabLayout::Vertical,
                )
                .selected(true)
                .dirty(true)
                .render(),
            )
            .child(
                block(
                    "gf-v-terminal",
                    "Terminal",
                    IconName::ToolTerminal,
                    TabLayout::Vertical,
                )
                .render()
                .child(tab_drop_marker(c, TabLayout::Vertical, TabDropSide::After)),
            )
            .child(
                block(
                    "gf-v-readme",
                    "README.md",
                    IconName::FileText,
                    TabLayout::Vertical,
                )
                .render(),
            );
        section(
            "Floating tabs",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row("horizontal", c, div().w_full().child(horizontal)))
                .child(row("vertical", c, vertical)),
        )
    }

    fn render_panes(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let zoomed = self.pane_zoomed;
        let view = cx.entity();
        let demo = |style: PaneLayoutStyle, tag: &'static str| {
            let gap = style.gap(&c);
            let bar = |id: String, icon: IconName, title: &'static str, active: bool| {
                pane_bar(id, c, icon, title).layout(style).active(active)
            };
            let toggle_zoom = {
                let view = view.clone();
                move |_: &mut Window, cx: &mut App| {
                    view.update(cx, |this, cx| {
                        this.pane_zoomed = !this.pane_zoomed;
                        cx.notify();
                    });
                }
            };
            let focused = bar(format!("gp-{tag}-a"), IconName::ToolTerminal, "zsh", true)
                .trailing("~/Developer/Labonair")
                .split_horizontal(PaneAction::new("Split right").on_click(|_, _| {}))
                .split_vertical(PaneAction::new("Split down").on_click(|_, _| {}))
                .zoom(
                    PaneAction::new(if zoomed { "Restore pane" } else { "Zoom pane" })
                        .on_click(toggle_zoom.clone()),
                )
                .zoomed(zoomed)
                .close(PaneAction::new("Close pane").on_click(|_, _| {}))
                .on_double_click(toggle_zoom)
                .render();
            let singleton = bar(
                format!("gp-{tag}-b"),
                IconName::GitGraph,
                "Git Graph",
                false,
            )
            .leading(indicator(IndicatorSize::Xs, c.warning))
            .can_split(false)
            .split_horizontal(PaneAction::new("Split right").on_click(|_, _| {}))
            .split_vertical(PaneAction::new("Split down").on_click(|_, _| {}))
            .close(PaneAction::new("Close pane").on_click(|_, _| {}))
            .render();
            div()
                .flex()
                .flex_row()
                .gap(gap)
                .p(gap)
                .w_full()
                .h(px(120.0))
                .bg(c.toolbar_bg)
                .child(
                    pane_frame(SharedString::from(format!("gp-{tag}-fa")), c, style, true)
                        .flex_1()
                        .child(focused)
                        .child(div().p_2().text_color(c.muted).child("Focused pane")),
                )
                .child(
                    pane_frame(SharedString::from(format!("gp-{tag}-fb")), c, style, false)
                        .flex_1()
                        .child(singleton)
                        .child(
                            div()
                                .p_2()
                                .text_color(c.muted)
                                .child("Singleton (no split)"),
                        ),
                )
        };
        section(
            "Pane bar + frame",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row(
                    "comfortable",
                    c,
                    div()
                        .w_full()
                        .child(demo(PaneLayoutStyle::Comfortable, "comfortable")),
                ))
                .child(row(
                    "compact",
                    c,
                    div()
                        .w_full()
                        .child(demo(PaneLayoutStyle::Compact, "compact")),
                )),
        )
    }

    fn render_settings_widgets(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();
        let select = |field: fn(&mut Gallery) -> &mut SharedString| {
            let view = view.clone();
            move |key: &SharedString, _: &mut Window, cx: &mut App| {
                let key = key.clone();
                view.update(cx, |this, cx| {
                    *field(this) = key;
                    cx.notify();
                });
            }
        };
        let schemes = preview_tile_group("gs-scheme", c, self.tile_scheme.clone())
            .tile(preview_tile(
                "auto",
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
            .on_select(select(|g| &mut g.tile_scheme));
        let styles = preview_tile_group("gs-style", c, self.tile_style.clone())
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
            .on_select(select(|g| &mut g.tile_style));
        let layouts = preview_tile_group("gs-layout", c, self.tile_layout.clone())
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
            .on_select(select(|g| &mut g.tile_layout));
        let active = TileMock::from_palette(&c);
        let themes = preview_tile_group("gs-theme", c, self.tile_theme.clone())
            .tile(preview_tile("labonair", "Labonair", PreviewKind::Theme))
            .tile(preview_tile("active", "Current", PreviewKind::Theme).colors(active, active))
            .on_select(select(|g| &mut g.tile_theme));

        let chips = h_stack().gap_2().flex_wrap().children(
            [
                ChipHue::RED,
                ChipHue::ORANGE,
                ChipHue::AMBER,
                ChipHue::GREEN,
                ChipHue::TEAL,
                ChipHue::BLUE,
                ChipHue::INDIGO,
                ChipHue::PURPLE,
                ChipHue::PINK,
                ChipHue::GRAY,
            ]
            .map(|hue| category_chip(c, hue, IconName::Settings)),
        );

        let group = settings_group("gs-group", c)
            .title("Appearance")
            .row(
                settings_row("gs-row-a", c, "Interface density")
                    .description("Spacing of rows, tabs and controls.")
                    .control(
                        segmented_control("gs-density", c, "default")
                            .segments([("compact", "Compact"), ("default", "Default")])
                            .variant(SegmentVariant::Solid)
                            .size(SegmentSize::Sm),
                    ),
            )
            .row(slider_row(
                "gs-row-b",
                c,
                "Corner radius",
                &self.slider,
                "42 px",
            ))
            .row(
                settings_row("gs-row-c", c, "Background opacity")
                    .reset(
                        button("gs-reset", c, ButtonVariant::Subtle, ButtonSize::Xs).child("Reset"),
                    )
                    .control(number_field("gs-opacity", c, 30.0, 0.0, 100.0, 1.0).track(false)),
            )
            .row(settings_row("gs-row-d", c, "Row without a control"));

        section(
            "Settings widgets",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row("group + rows", c, div().w_full().child(group)))
                .child(row("color scheme", c, schemes))
                .child(row("interface style", c, styles))
                .child(row("pane layout", c, layouts))
                .child(row("theme", c, themes))
                .child(row("category chips", c, chips)),
        )
    }

    fn render_motion(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let nonce = self.motion_nonce;
        let level = segmented_control(
            "gm-level",
            c,
            match self.motion_level {
                MotionLevel::Full => "full",
                MotionLevel::Reduced => "reduced",
                MotionLevel::Off => "off",
            },
        )
        .segments([("full", "Full"), ("reduced", "Reduced"), ("off", "Off")])
        .variant(SegmentVariant::Solid)
        .size(SegmentSize::Sm)
        .on_select(
            cx.listener(|this: &mut Gallery, key: &SharedString, _w, cx| {
                this.motion_level = match key.as_ref() {
                    "reduced" => MotionLevel::Reduced,
                    "off" => MotionLevel::Off,
                    _ => MotionLevel::Full,
                };
                cx.notify();
            }),
        );

        let replay = button("gm-replay", c, ButtonVariant::Outlined, ButtonSize::Sm)
            .child("Replay fade + slide")
            .on_click(cx.listener(|this: &mut Gallery, _: &ClickEvent, _w, cx| {
                this.motion_nonce = this.motion_nonce.wrapping_add(1);
                cx.notify();
            }));
        let sample = |label: &'static str| {
            div()
                .px_3()
                .py_2()
                .rounded(px(c.radius.md))
                .bg(c.sunken)
                .border_1()
                .border_color(c.line_subtle)
                .child(label)
        };

        let reveal_open = self.reveal_open;
        let reveal = button("gm-reveal", c, ButtonVariant::Outlined, ButtonSize::Sm)
            .child(if reveal_open { "Collapse" } else { "Expand" })
            .on_click(cx.listener(|this: &mut Gallery, _: &ClickEvent, _w, cx| {
                this.reveal_open = !this.reveal_open;
                cx.notify();
            }));
        let (from, to) = if reveal_open {
            (px(0.0), px(200.0))
        } else {
            (px(200.0), px(0.0))
        };

        let on = self.state_on;
        let state_toggle = button("gm-state", c, ButtonVariant::Outlined, ButtonSize::Sm)
            .child("Flip state")
            .on_click(cx.listener(|this: &mut Gallery, _: &ClickEvent, _w, cx| {
                this.state_on = !this.state_on;
                cx.notify();
            }));
        let glyph = |icon: IconName, id: &'static str, show: bool| {
            div()
                .absolute()
                .top_0()
                .left_0()
                .child(icon.svg(c.fg).size(px(16.0)))
                .motion_state_fade(id, c, show)
        };

        let open_menu = button("gm-menu", c, ButtonVariant::Outlined, ButtonSize::Sm)
            .child("Open menu")
            .on_click(cx.listener(|this: &mut Gallery, ev: &ClickEvent, _w, cx| {
                this.menu_anchor = ev.position();
                this.menu_presence.open();
                cx.notify();
            }));
        let open_dialog = button("gm-dialog", c, ButtonVariant::Outlined, ButtonSize::Sm)
            .child("Open dialog")
            .on_click(cx.listener(|this: &mut Gallery, _: &ClickEvent, _w, cx| {
                this.dialog_presence.open();
                cx.notify();
            }));

        section(
            "Motion",
            c,
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(row("level (whole page)", c, level))
                .child(row(
                    "fade / slide",
                    c,
                    h_stack()
                        .gap_3()
                        .child(replay)
                        .child(sample("fade in").motion_fade_in(("gm-fade", nonce), c))
                        .child(sample("slide in").motion_slide_in_y(
                            ("gm-slide", nonce),
                            c,
                            px(10.0),
                        )),
                ))
                .child(row(
                    "reveal size",
                    c,
                    h_stack().gap_3().child(reveal).child(
                        div()
                            .h(px(28.0))
                            .rounded(px(c.radius.md))
                            .bg(c.sunken)
                            .motion_reveal(
                                ("gm-reveal-box", reveal_open as u32),
                                c,
                                Axis::Horizontal,
                                from,
                                to,
                            ),
                    ),
                ))
                .child(row(
                    "state fade",
                    c,
                    h_stack().gap_3().child(state_toggle).child(
                        div()
                            .relative()
                            .size(px(16.0))
                            .child(glyph(IconName::Check, "gm-glyph-on", on))
                            .child(glyph(IconName::Close, "gm-glyph-off", !on)),
                    ),
                ))
                .child(row(
                    "exit (closing)",
                    c,
                    h_stack().gap_3().child(open_menu).child(open_dialog),
                )),
        )
    }
}

/// The open gallery window, if one exists — a second open request focuses it
/// instead of spawning a duplicate.
#[derive(Default)]
struct GalleryWindowRef {
    handle: Option<WindowHandle<Root>>,
}

impl Global for GalleryWindowRef {}

/// Open the component-gallery window (or focus it if already open). Wired to
/// the debug-only `CommandId::OpenComponentGallery` palette row in
/// `labonair-shell`.
pub fn open_gallery_window(cx: &mut App) {
    if let Some(handle) = cx.try_global::<GalleryWindowRef>().and_then(|w| w.handle) {
        if handle
            .update(cx, |_, window, _| window.activate_window())
            .is_ok()
        {
            cx.activate(true);
            return;
        }
        cx.set_global(GalleryWindowRef { handle: None });
    }

    let bounds = Bounds::centered(None, size(px(840.0), px(920.0)), cx);
    let opened = cx.open_window(
        WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some("Component Gallery".into()),
                appears_transparent: true,
                traffic_light_position: Some(point(px(19.0), px((40.0 - 14.0) / 2.0))),
            }),
            window_min_size: Some(size(px(480.0), px(400.0))),
            kind: WindowKind::Normal,
            is_movable: true,
            ..Default::default()
        },
        |window, cx| {
            let theme = theme_store(cx);
            let view = cx.new(|cx| Gallery::new(theme, cx));
            let view: gpui::AnyView = view.into();
            cx.new(|cx| Root::new(view, window, cx))
        },
    );

    match opened {
        Ok(handle) => {
            cx.set_global(GalleryWindowRef {
                handle: Some(handle),
            });
            cx.activate(true);
        }
        Err(e) => eprintln!("failed to open component gallery window: {e}"),
    }
}
