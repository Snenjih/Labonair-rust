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
//! re-renders in Light or Dark.

use gpui::{
    div, point, prelude::FluentBuilder, px, size, App, AppContext, Bounds, Context, Entity,
    FontWeight, Global, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, TitlebarOptions, Window, WindowBounds, WindowHandle,
    WindowKind, WindowOptions,
};
use gpui_component::Root;

use labonair_theme::{theme_store, ThemePreference, ThemeStore};

use crate::context_menu::menu_card_preview;
use crate::{
    banner, button, checkbox, disclosure, divider, h_stack, icon_toggle_button, indicator, kbd_row,
    keybinding_hint, list_header, number_field, segmented_control, select_trigger, toggle_base,
    Axis, ButtonSize, ButtonVariant, IconName, IndicatorSize, ListItem, MenuItem, Palette,
    SegmentSize, SegmentVariant, Severity, ToggleSize, ToggleVariant,
};

/// The gallery view. Holds only the handful of caller-owned flags the
/// interactive primitives need (`Disclosure`, `SegmentedControl`,
/// `NumberField`, `Checkbox`) plus the theme handle it re-renders off.
pub struct Gallery {
    theme: Entity<ThemeStore>,
    disclosure_open: bool,
    segment: SharedString,
    number: f64,
    checkbox_on: bool,
}

impl Gallery {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            disclosure_open: true,
            segment: "outline".into(),
            number: 12.0,
            checkbox_on: true,
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
        let c = Palette::from_theme(self.theme.read(cx));

        div()
            .id("gallery-root")
            .size_full()
            .bg(c.bg)
            .text_color(c.fg)
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
                            .child(self.render_number(c, cx))
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
    fn render_header(&self, c: Palette, cx: &mut Context<Self>) -> impl IntoElement {
        let pref = self.theme.read(cx).preference();
        let switch = |label: &'static str, id: &'static str, p: ThemePreference| {
            let active = pref == p;
            button(id, c, ButtonVariant::Outline, ButtonSize::Sm)
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
            (ButtonVariant::Default, "Default"),
            (ButtonVariant::Outline, "Outline"),
            (ButtonVariant::Secondary, "Secondary"),
            (ButtonVariant::Ghost, "Ghost"),
            (ButtonVariant::Destructive, "Destructive"),
            (ButtonVariant::Link, "Link"),
        ];
        let sizes = [
            (ButtonSize::Xs, "Xs"),
            (ButtonSize::Sm, "Sm"),
            (ButtonSize::Default, "Md"),
            (ButtonSize::Lg, "Lg"),
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
        // Icon sizes + disabled-look (Link handler dropped by opacity only in
        // the real primitive on `disabled`, which `button` does not model — so
        // the disabled column here is a manual dim to show intent).
        let icons = h_stack()
            .flex_wrap()
            .gap_2()
            .child(
                button("btn-icon-xs", c, ButtonVariant::Outline, ButtonSize::IconXs)
                    .child(IconName::Plus.svg(c.fg).size(px(12.0))),
            )
            .child(
                button("btn-icon-sm", c, ButtonVariant::Outline, ButtonSize::IconSm)
                    .child(IconName::Plus.svg(c.fg).size(px(14.0))),
            )
            .child(
                button("btn-icon-md", c, ButtonVariant::Outline, ButtonSize::Icon)
                    .child(IconName::Plus.svg(c.fg).size(px(16.0))),
            )
            .child(
                button("btn-icon-lg", c, ButtonVariant::Outline, ButtonSize::IconLg)
                    .child(IconName::Plus.svg(c.fg).size(px(18.0))),
            )
            .child(
                button("btn-disabled", c, ButtonVariant::Default, ButtonSize::Sm)
                    .opacity(crate::DISABLED_OPACITY)
                    .child("Disabled"),
            );
        body = body.child(row("Icon / disabled", c, icons));
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
            .child(ListItem::new("li-plain", c.fg, c.muted, c.accent).child("Plain row"))
            .child(
                ListItem::new("li-icon", c.fg, c.muted, c.accent)
                    .icon(IconName::File)
                    .child("With leading icon"),
            )
            .child(
                ListItem::new("li-sel", c.fg, c.muted, c.accent)
                    .icon(IconName::Sparkles)
                    .selected(true)
                    .trailing(kbd_row(["\u{2318}", "K"], c))
                    .child("Selected + trailing kbd"),
            )
            .child(
                ListItem::new("li-dis", c.fg, c.muted, c.accent)
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
