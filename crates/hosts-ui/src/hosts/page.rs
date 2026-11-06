//! The Hosts page of Settings.
//!
//! One view, two levels (ADR 0007): a list page (search, Hosts | Credentials
//! switch, filters) and a detail sub-page per host or credential. Everything is
//! built from the same `settings_group` / `settings_row` cards as the other
//! Settings pages, and every edit autosaves with a status badge in the header.

use gpui::{AnyElement, Div, Stateful};

use super::*;
use labonair_ui_kit::{
    button, button_disabled, h_stack, icon_button_builder, modal_overlay, segmented_control,
    select_popover, select_trigger, settings_group, settings_row, switch, text_area_surface,
    v_stack, ButtonSize, ButtonVariant, IconButtonShape, MotionExt, SegmentSize, SegmentVariant,
    SelectPopoverAnchor, SettingsRow,
};

const PAGE_PAD: f32 = 24.0;
const FIELD_WIDTH: f32 = 280.0;

/// How a lazily created text input behaves.
#[derive(Clone, Copy, PartialEq, Eq)]
enum InputKind {
    Single,
    Masked,
    Multi,
}

/// A port is valid when it is a number in `1..=65535`.
fn valid_port(text: &str) -> bool {
    text.trim().parse::<u16>().is_ok_and(|port| port > 0)
}

/// Empty or a non-negative number (keep-alive fields are optional).
fn valid_optional_number(text: &str) -> bool {
    text.trim().is_empty() || text.trim().parse::<u32>().is_ok()
}

/// Whether routing `editing` through `candidate` would close a jump-host loop
/// (`candidate` already routes, directly or not, through `editing`).
fn jump_creates_cycle(hosts: &[Host], editing: &str, candidate: &str) -> bool {
    let mut seen = HashSet::new();
    let mut current = Some(candidate);
    while let Some(id) = current {
        if id == editing || !seen.insert(id) {
            return true;
        }
        current = hosts
            .iter()
            .find(|host| host.id == id)
            .and_then(|host| host.jump_host_id.as_deref());
    }
    false
}

fn credential_label(credential: &Credential) -> String {
    format!(
        "{} ({})",
        credential.name,
        if credential.cred_type == "key" {
            "SSH Key"
        } else {
            "Password"
        }
    )
}

fn credential_icon(credential: &Credential) -> IconName {
    if credential.cred_type == "key" {
        IconName::FileLock
    } else {
        IconName::Lock
    }
}

fn credential_subtitle(credential: &Credential) -> String {
    if credential.cred_type == "key" {
        match credential.key_type.as_deref() {
            Some(kind) => format!("SSH key \u{00b7} {kind}"),
            None => "SSH key".to_string(),
        }
    } else if credential.has_secret {
        "Password \u{00b7} secret set".to_string()
    } else {
        "Password".to_string()
    }
}

/// Where a select list opens for a click inside its trigger: just under the
/// control instead of covering it (the click lands near the trigger's middle).
fn below_trigger(click: Point<Pixels>) -> Point<Pixels> {
    Point {
        x: click.x,
        y: click.y + px(18.0),
    }
}

/// Round the outer corners of a row that touches the card edge, so its hover
/// fill follows the card's rounding (the card clips to a rectangle only).
fn round_edges(row: Stateful<Div>, first: bool, last: bool, p: &Palette) -> Stateful<Div> {
    let radius = px((p.radius.lg - 1.0).max(0.0));
    row.when(first, |row| row.rounded_tl(radius).rounded_tr(radius))
        .when(last, |row| row.rounded_bl(radius).rounded_br(radius))
}

/// A rounded icon tile (host / credential avatar).
fn avatar(p: &Palette, icon: IconName, size: f32) -> Div {
    div()
        .flex()
        .flex_shrink_0()
        .items_center()
        .justify_center()
        .size(p.control_space(size))
        .rounded(px(p.radius.md))
        .bg(p.muted_bg)
        .border_1()
        .border_color(p.line_subtle)
        .child(icon.svg(p.fg).size(px(size * 0.45)))
}

impl Render for HostManagerView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.wire_blink_focus(window, cx);
        self.ensure_search_input(window, cx);
        let p = self.palette(cx);
        let in_detail = self.form.is_some() || self.cred_form.is_some();

        let header = if in_detail {
            self.render_detail_header(&p, cx)
        } else {
            self.render_list_header(&p, window, cx)
        };
        let body = if self.form.is_some() {
            self.render_host_detail(&p, window, cx)
        } else if self.cred_form.is_some() {
            self.render_cred_detail(&p, window, cx)
        } else if self.mode == PageMode::Hosts {
            self.render_host_list(&p, cx)
        } else {
            self.render_cred_list(&p, cx)
        };
        let detail_key = self
            .form
            .as_ref()
            .and_then(|f| f.editing_id.clone())
            .or_else(|| self.cred_form.as_ref().and_then(|f| f.editing_id.clone()))
            .unwrap_or_else(|| "new".to_string());

        div()
            .track_focus(&self.focus_handle)
            .key_context("HostsPage")
            .on_key_down(cx.listener(|this, ev: &KeyDownEvent, _w, cx| {
                let ks = &ev.keystroke;
                if ks.key == "escape" {
                    if this.dropdown.is_some()
                        || this.icon_picker_open
                        || this.confirm.is_some()
                        || this.more_menu.is_some()
                    {
                        this.dropdown = None;
                        this.icon_picker_open = false;
                        this.confirm = None;
                        this.more_menu = None;
                    } else if this.form.is_some() || this.cred_form.is_some() {
                        this.close_detail(cx);
                    } else {
                        // Settings handles Escape (clear search, close window).
                        return;
                    }
                    cx.stop_propagation();
                    cx.notify();
                }
            }))
            .relative()
            .size_full()
            .min_w_0()
            .overflow_hidden()
            .bg(p.bg)
            // The content is absolutely positioned so its unwrapped text does
            // not widen the Settings window (GPUI reports text min-content as
            // its single-line width); the root's size comes from its parent.
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .flex_col()
                    .child(header)
                    .child(
                        div()
                            .id("hosts-scroll")
                            .flex_1()
                            .min_h_0()
                            .overflow_y_scroll()
                            .px(p.space(PAGE_PAD))
                            .pb(p.space(PAGE_PAD))
                            .child(div().w_full().child(body).motion_fade_in(
                                SharedString::from(format!("hosts-page-{detail_key}")),
                                p,
                            )),
                    ),
            )
            .children(self.render_dropdown(&p, cx))
            .children(self.render_icon_picker(&p, cx))
            .children(self.render_more_menu(&p, cx))
            .children(self.render_confirm(&p, cx))
            .children(
                self.import
                    .is_some()
                    .then(|| self.render_import(&p, cx).into_any_element()),
            )
            .children(
                self.export
                    .is_some()
                    .then(|| self.render_export(&p, cx).into_any_element()),
            )
            .children(self.render_host_menu(&p, cx))
            .children(self.render_group_menu(&p, cx))
            .children(self.render_group_rename(&p, cx))
    }
}

// ── shared plumbing ────────────────────────────────────────────────────────

impl HostManagerView {
    /// The group-name draft and rename fields are hand-rolled; their caret
    /// blink is driven by focus listeners that need a `Window`, so they are
    /// wired on the first render.
    fn wire_blink_focus(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.blink_focus_wired {
            return;
        }
        self.blink_focus_wired = true;
        for (handle, region) in [
            (self.group_focus.clone(), FocusRegion::Group),
            (self.group_rename_focus.clone(), FocusRegion::GroupRename),
        ] {
            self._blink_focus_subs
                .push(cx.on_focus(&handle, window, move |this, _w, cx| {
                    this.focused_region = Some(region);
                    this.blink.update(cx, |b, cx| b.start(cx));
                }));
            self._blink_focus_subs
                .push(cx.on_blur(&handle, window, move |this, _w, cx| {
                    if this.focused_region == Some(region) {
                        this.focused_region = None;
                    }
                    this.blink.update(cx, |b, cx| b.stop(cx));
                }));
        }
    }

    fn field_value(&self, key: FieldKey) -> String {
        match key {
            FieldKey::Host(field) => self
                .form
                .as_ref()
                .map(|form| form.field(field).to_string())
                .unwrap_or_default(),
            FieldKey::Cred(field) => self
                .cred_form
                .as_ref()
                .map(|form| form.field(field).to_string())
                .unwrap_or_default(),
        }
    }

    /// Store `value` in the form; `true` when it changed.
    fn set_field_value(&mut self, key: FieldKey, value: String) -> bool {
        let slot = match key {
            FieldKey::Host(field) => self.form.as_mut().map(|form| form.field_mut(field)),
            FieldKey::Cred(field) => self.cred_form.as_mut().map(|form| form.field_mut(field)),
        };
        match slot {
            Some(slot) if *slot != value => {
                *slot = value;
                true
            }
            _ => false,
        }
    }

    /// The text input for `key`, created on first use from the form's value.
    fn input_for(
        &mut self,
        key: FieldKey,
        kind: InputKind,
        placeholder: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Entity<InputState> {
        if let Some((input, _)) = self.inputs.get(&key) {
            return input.clone();
        }
        let value = self.field_value(key);
        let input = cx.new(|cx| {
            let mut state = text_field(window, cx)
                .masked(kind == InputKind::Masked)
                .placeholder(placeholder);
            if kind == InputKind::Multi {
                state = state.multi_line(true).rows(5);
            }
            if !value.is_empty() {
                state.set_value(value, window, cx);
            }
            state
        });
        let subscription =
            cx.subscribe(
                &input,
                move |this, input, event: &InputEvent, cx| match event {
                    InputEvent::Change => {
                        let value = input.read(cx).value().to_string();
                        if this.set_field_value(key, value) {
                            this.schedule_autosave(cx);
                        }
                    }
                    InputEvent::Focus => {
                        this.focused_input = Some(key);
                        cx.notify();
                    }
                    InputEvent::Blur => {
                        if this.focused_input == Some(key) {
                            this.focused_input = None;
                        }
                        cx.notify();
                    }
                    InputEvent::PressEnter { .. } => {
                        if kind != InputKind::Multi {
                            this.flush_open_form(cx);
                        }
                    }
                },
            );
        self.inputs.insert(key, (input.clone(), subscription));
        input
    }

    /// A framed single-line (or masked) field for a settings row.
    #[allow(clippy::too_many_arguments)]
    fn text_control(
        &mut self,
        key: FieldKey,
        kind: InputKind,
        placeholder: &'static str,
        invalid: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
        p: &Palette,
    ) -> Stateful<Div> {
        let input = self.input_for(key, kind, placeholder, window, cx);
        let state = if invalid {
            TextFieldState::Invalid
        } else if self.focused_input == Some(key) {
            TextFieldState::Focused
        } else {
            TextFieldState::Normal
        };
        let focus_input = input.clone();
        text_field_surface(
            SharedString::from(format!("hf-{key:?}")),
            *p,
            state,
            text_input(&input, *p),
        )
        .w(p.control_space(FIELD_WIDTH))
        .cursor_text()
        .on_click(move |_: &ClickEvent, window, cx| {
            focus_input.update(cx, |state, cx| state.focus(window, cx));
        })
    }

    /// A select trigger that opens the shared dropdown on click.
    #[allow(clippy::too_many_arguments)]
    fn select_control(
        &self,
        id: &'static str,
        label: String,
        kind: DropdownKind,
        options: Vec<SelectOption>,
        selected: String,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let open = self.dropdown.as_ref().is_some_and(|d| d.kind == kind);
        select_trigger(id, *p, label, open)
            .w(p.control_space(FIELD_WIDTH))
            .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                this.dropdown_list.reset(options.len());
                this.dropdown = Some(Dropdown {
                    kind,
                    at: below_trigger(ev.position()),
                    options: options.clone(),
                    selected: selected.clone(),
                });
                cx.notify();
            }))
    }

    fn render_dropdown(&mut self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let menu = self.dropdown.as_ref()?;
        let (kind, at) = (menu.kind, menu.at);
        let options = menu.options.clone();
        let selected = menu.selected.clone();
        let view = cx.entity();
        let dismiss_view = view.clone();
        Some(select_popover(
            SelectPopoverAnchor::new("hosts-dropdown", at, self.dropdown_list.clone()),
            *p,
            &options,
            &selected,
            move |_w, cx| {
                dismiss_view.update(cx, |this, cx| {
                    this.dropdown = None;
                    cx.notify();
                })
            },
            move |token, _w, cx| {
                let token = token.to_string();
                view.update(cx, |this, cx| this.apply_dropdown(kind, token, cx));
            },
        ))
    }

    fn apply_dropdown(&mut self, kind: DropdownKind, token: String, cx: &mut Context<Self>) {
        self.dropdown = None;
        let id = (!token.is_empty()).then_some(token.clone());
        match kind {
            DropdownKind::Sort => {
                self.sort = match token.as_str() {
                    "asc" => HostSort::NameAsc,
                    "desc" => HostSort::NameDesc,
                    _ => HostSort::LastConnected,
                };
                cx.notify();
                return;
            }
            DropdownKind::Group => {
                if let Some(form) = self.form.as_mut() {
                    form.group = id;
                }
            }
            DropdownKind::Credential => {
                if let Some(form) = self.form.as_mut() {
                    form.credential = id;
                }
            }
            DropdownKind::JumpHost => {
                if let Some(form) = self.form.as_mut() {
                    form.jump_host = id;
                }
            }
            DropdownKind::Snippet => {
                if let Some(form) = self.form.as_mut() {
                    form.snippet_id = id;
                }
            }
        }
        self.schedule_autosave(cx);
    }

    /// A settings row with a switch.
    #[allow(clippy::too_many_arguments)]
    fn switch_row(
        &self,
        id: &'static str,
        title: &'static str,
        description: &'static str,
        on: bool,
        p: &Palette,
        cx: &mut Context<Self>,
        toggle: impl Fn(&mut Self, bool) + Clone + 'static,
    ) -> SettingsRow {
        let view = cx.entity();
        settings_row(id, *p, title)
            .description(description)
            .control(
                switch(SharedString::from(format!("sw-{id}")), *p, on).on_click(
                    move |next: &bool, _w, cx| {
                        let (next, toggle) = (*next, toggle.clone());
                        view.update(cx, |this, cx| {
                            toggle(this, next);
                            this.schedule_autosave(cx);
                        });
                    },
                ),
            )
    }

    fn segmented<const N: usize>(
        &self,
        id: &'static str,
        selected: &'static str,
        segments: [(&'static str, &'static str); N],
        p: &Palette,
        on_select: impl Fn(&mut Self, &str, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity();
        let mut control = segmented_control(id, *p, selected)
            .variant(SegmentVariant::Solid)
            .size(SegmentSize::Sm);
        for (key, label) in segments {
            control = control.segment(key, label);
        }
        control.on_select(move |key, _w, cx| {
            view.update(cx, |this, cx| on_select(this, key.as_ref(), cx));
        })
    }

    /// Reachability dot colour of a host row.
    fn status_dot(&self, host_id: &str, p: &Palette) -> gpui::Hsla {
        match self.ping.get(host_id).copied().unwrap_or_default() {
            Ping::Online => p.success,
            Ping::Offline => p.muted,
            Ping::Checking => p.line_strong,
        }
    }
}

// ── headers ────────────────────────────────────────────────────────────────

impl HostManagerView {
    fn render_list_header(
        &mut self,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hosts_mode = self.mode == PageMode::Hosts;
        if self.search_hint_mode != self.mode {
            self.search_hint_mode = self.mode;
            if let Some(input) = self.search_input.clone() {
                input.update(cx, |state, cx| {
                    state.set_placeholder(
                        if hosts_mode {
                            "Find a host or type user@hostname\u{2026}"
                        } else {
                            "Find a credential\u{2026}"
                        },
                        window,
                        cx,
                    )
                });
            }
        }
        let search_focused = self.search_input_focused;
        let clear_button = (!self.search.is_empty()).then(|| {
            search_clear_button(
                "host-search-clear",
                *p,
                cx.listener(|this, _: &ClickEvent, window, cx| {
                    this.search.clear();
                    if let Some(input) = this.search_input.clone() {
                        input.update(cx, |state, cx| state.set_value(String::new(), window, cx));
                    }
                    cx.notify();
                }),
            )
            .into_any_element()
        });
        let search = search_field(
            "host-search",
            *p,
            search_focused,
            search_input(
                self.search_input
                    .as_ref()
                    .expect("search input initialized"),
                *p,
            ),
            clear_button,
        )
        .flex_1()
        .min_w_0()
        .key_context("HostSearch")
        .on_key_down(cx.listener(Self::on_search_key));

        let mode = self.segmented(
            "hosts-mode",
            if hosts_mode { "hosts" } else { "credentials" },
            [("hosts", "Hosts"), ("credentials", "Credentials")],
            p,
            |this, key, cx| {
                this.mode = if key == "credentials" {
                    PageMode::Credentials
                } else {
                    PageMode::Hosts
                };
                cx.notify();
            },
            cx,
        );

        let primary = icon_button_builder("hosts-new", *p, IconName::Plus)
            .variant(ButtonVariant::Filled)
            .size(ButtonSize::Sm)
            .label(if hosts_mode {
                "New Host"
            } else {
                "New Credential"
            })
            .render()
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                if hosts_mode {
                    this.new_host(cx);
                } else {
                    this.new_credential(None, cx);
                }
            }));

        let more = hosts_mode.then(|| {
            icon_button_builder("hosts-more", *p, IconName::Ellipsis)
                .variant(ButtonVariant::Subtle)
                .size(ButtonSize::IconSm)
                .shape(IconButtonShape::Square)
                .tooltip("Import / Export")
                .render()
                .on_click(cx.listener(|this, ev: &ClickEvent, _w, cx| {
                    this.more_menu = Some(ev.position());
                    cx.notify();
                }))
        });

        let filters = if hosts_mode {
            let group_chips = self.render_group_chips(p, cx);
            let new_group = match self.group_draft.as_ref() {
                Some(draft) => div()
                    .track_focus(&self.group_focus)
                    .key_context("GroupDraft")
                    .on_key_down(cx.listener(Self::on_group_key))
                    .px_2()
                    .py_0p5()
                    .rounded(px(p.radius.md))
                    .bg(p.field_bg)
                    .border_1()
                    .border_color(p.field_ring)
                    .text_xs()
                    .text_color(p.fg)
                    .flex()
                    .items_center()
                    .child(SharedString::from(draft.clone()))
                    .when(
                        self.focused_region == Some(FocusRegion::Group)
                            && self.blink.read(cx).visible(),
                        |el| el.child(caret(p.fg, 12.0)),
                    )
                    .into_any_element(),
                None => icon_button_builder("new-group", *p, IconName::Plus)
                    .size(ButtonSize::Xs)
                    .label("Group")
                    .render()
                    .on_click(cx.listener(|this, _: &ClickEvent, w, cx| {
                        this.group_draft = Some(String::new());
                        w.focus(&this.group_focus);
                        cx.notify();
                    }))
                    .into_any_element(),
            };
            let sort_options: Vec<SelectOption> = vec![
                ("last".into(), "Last connected".into()),
                ("asc".into(), "Name A \u{2192} Z".into()),
                ("desc".into(), "Name Z \u{2192} A".into()),
            ];
            let sort_selected = match self.sort {
                HostSort::LastConnected => "last",
                HostSort::NameAsc => "asc",
                HostSort::NameDesc => "desc",
            };
            let sort = icon_button_builder("sort-cycle", *p, IconName::ChevronUpDown)
                .size(ButtonSize::Xs)
                .label(format!("Sort: {}", self.sort.label()))
                .render()
                .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                    this.dropdown_list.reset(sort_options.len());
                    this.dropdown = Some(Dropdown {
                        kind: DropdownKind::Sort,
                        at: below_trigger(ev.position()),
                        options: sort_options.clone(),
                        selected: sort_selected.to_string(),
                    });
                    cx.notify();
                }));
            h_stack()
                .items_center()
                .gap(p.space(8.0))
                .child(div().flex_1().min_w_0().child(group_chips))
                .child(new_group)
                .child(sort)
                .into_any_element()
        } else {
            let selected = match self.cred_filter {
                None => "all",
                Some(true) => "key",
                Some(false) => "password",
            };
            self.segmented(
                "cred-filter",
                selected,
                [
                    ("all", "All"),
                    ("key", "SSH Keys"),
                    ("password", "Passwords"),
                ],
                p,
                |this, key, cx| {
                    this.cred_filter = match key {
                        "key" => Some(true),
                        "password" => Some(false),
                        _ => None,
                    };
                    cx.notify();
                },
                cx,
            )
            .into_any_element()
        };

        v_stack()
            .flex_shrink_0()
            .gap(p.space(10.0))
            .px(p.space(PAGE_PAD))
            .pt(p.space(8.0))
            .pb(p.space(14.0))
            .child(
                h_stack()
                    .items_center()
                    .gap(p.space(8.0))
                    .child(search)
                    .child(mode)
                    .children(more)
                    .child(primary),
            )
            .child(filters)
            .into_any_element()
    }

    /// Back arrow, avatar, title, save badge and host actions.
    fn render_detail_header(&mut self, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let back = icon_button_builder("hosts-back", *p, IconName::ArrowLeft)
            .variant(ButtonVariant::Subtle)
            .size(ButtonSize::IconSm)
            .shape(IconButtonShape::Square)
            .tooltip(if self.mode == PageMode::Hosts {
                "Back to hosts"
            } else {
                "Back to credentials"
            })
            .render()
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.close_detail(cx)));

        let (icon, title, caption, host_id): (IconName, String, String, Option<String>) =
            if let Some(form) = self.form.as_ref() {
                let title = if form.name.trim().is_empty() {
                    if form.address.trim().is_empty() {
                        "New host".to_string()
                    } else {
                        form.address.trim().to_string()
                    }
                } else {
                    form.name.trim().to_string()
                };
                let caption = if form.editing_id.is_none() {
                    "Saved once it has an address".to_string()
                } else {
                    format!(
                        "{}@{}:{}",
                        form.username.trim(),
                        form.address.trim(),
                        form.port.trim()
                    )
                };
                (
                    host_icon(form.icon.as_deref()),
                    title,
                    caption,
                    form.editing_id.clone(),
                )
            } else if let Some(form) = self.cred_form.as_ref() {
                let title = if form.name.trim().is_empty() {
                    "New credential".to_string()
                } else {
                    form.name.trim().to_string()
                };
                let caption = if form.editing_id.is_none() {
                    "Saved once it has a name".to_string()
                } else if form.is_key {
                    "SSH key credential".to_string()
                } else {
                    "Password credential".to_string()
                };
                (
                    if form.is_key {
                        IconName::FileLock
                    } else {
                        IconName::Lock
                    },
                    title,
                    caption,
                    None,
                )
            } else {
                return div().into_any_element();
            };

        let is_host = self.form.is_some();
        let avatar_el = if is_host {
            let tile = avatar(p, icon, 36.0);
            div()
                .id("host-icon")
                .cursor_pointer()
                .child(tile)
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    this.icon_picker_open = !this.icon_picker_open;
                    cx.notify();
                }))
                .into_any_element()
        } else {
            avatar(p, icon, 36.0).into_any_element()
        };

        let actions = host_id.map(|id| {
            let (id_ssh, id_sftp, id_menu) = (id.clone(), id.clone(), id);
            h_stack()
                .items_center()
                .gap(p.space(6.0))
                .child(
                    button("hd-test", *p, ButtonVariant::Outlined, ButtonSize::Xs)
                        .child("Test")
                        .on_click(
                            cx.listener(|this, _: &ClickEvent, _w, cx| this.test_connection(cx)),
                        ),
                )
                .child(
                    button("hd-sftp", *p, ButtonVariant::Outlined, ButtonSize::Xs)
                        .child("SFTP")
                        .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                            cx.emit(HostManagerEvent::Open(HostOpenRequest::sftp(
                                id_sftp.clone(),
                            )));
                        })),
                )
                .child(
                    button("hd-connect", *p, ButtonVariant::Filled, ButtonSize::Xs)
                        .child("Connect")
                        .on_click(cx.listener(move |_this, _: &ClickEvent, _w, cx| {
                            cx.emit(HostManagerEvent::Open(HostOpenRequest::ssh(id_ssh.clone())));
                        })),
                )
                .child(
                    icon_button_builder("hd-more", *p, IconName::Ellipsis)
                        .variant(ButtonVariant::Subtle)
                        .size(ButtonSize::IconSm)
                        .shape(IconButtonShape::Square)
                        .tooltip("More")
                        .render()
                        .on_click(cx.listener(move |this, ev: &ClickEvent, _w, cx| {
                            this.host_menu = Some((id_menu.clone(), ev.position()));
                            cx.notify();
                        })),
                )
        });

        h_stack()
            .flex_shrink_0()
            .items_center()
            .gap(p.space(12.0))
            .px(p.space(PAGE_PAD))
            .pt(p.space(8.0))
            .pb(p.space(14.0))
            .child(back)
            .child(avatar_el)
            .child(
                v_stack()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .text_size(p.control_space(15.0))
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(p.fg)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(title)),
                    )
                    .child(
                        div()
                            .text_size(p.control_space(12.0))
                            .text_color(p.muted)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(caption)),
                    ),
            )
            .children(self.render_save_badge(p))
            .children(actions)
            .into_any_element()
    }

    /// The autosave status: quiet while pending, a gentle fade-in on "Saved".
    fn render_save_badge(&self, p: &Palette) -> Option<AnyElement> {
        let (icon, color, label) = match self.save_state {
            SaveState::Idle => return None,
            SaveState::Pending => (IconName::Circle, p.muted, "Unsaved changes"),
            SaveState::Saving => (IconName::LoadCircle, p.muted, "Saving\u{2026}"),
            SaveState::Saved => (IconName::Check, p.success, "Saved"),
            SaveState::Error => (IconName::Warning, p.error, "Save failed"),
        };
        let badge = h_stack()
            .items_center()
            .gap(p.space(5.0))
            .text_size(p.control_space(12.0))
            .text_color(color)
            .child(icon.svg(color).size(px(12.0)))
            .child(label);
        Some(
            badge
                .motion_fade_in(
                    SharedString::from(format!("save-{:?}-{}", self.save_state, self.saved_gen)),
                    *p,
                )
                .into_any_element(),
        )
    }
}

// ── list pages ─────────────────────────────────────────────────────────────

impl HostManagerView {
    fn render_host_list(&mut self, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let visible = self.visible_hosts();
        let mut page = v_stack().gap(p.space(20.0));

        if let Some((user, host, port)) = self.quick_connect_target() {
            let label = format!("{user}@{host}:{port}");
            page = page.child(
                settings_group("hosts-quick", *p).row(
                    settings_row("quick-connect", *p, "Quick connect")
                        .description(SharedString::from(label))
                        .control(
                            icon_button_builder("quick-connect-go", *p, IconName::Zap)
                                .variant(ButtonVariant::Filled)
                                .size(ButtonSize::Xs)
                                .label("Connect")
                                .render()
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    this.quick_connect(user.clone(), host.clone(), port, cx)
                                })),
                        ),
                ),
            );
        }

        if !self.active_tunnels.is_empty() {
            let mut group = settings_group("hosts-tunnels", *p).title("Active tunnels");
            for tunnel in &self.active_tunnels {
                group = group.row(
                    settings_row(
                        SharedString::from(format!("tunnel-{}", tunnel.local_port)),
                        *p,
                        tunnel.host_label.clone(),
                    )
                    .description(SharedString::from(format!(
                        "localhost:{} \u{2192} {}:{}",
                        tunnel.local_port, tunnel.remote_host, tunnel.remote_port
                    ))),
                );
            }
            page = page.child(group);
        }

        if visible.is_empty() {
            let (message, hint) = if self.hosts.is_empty() {
                (
                    "No hosts yet",
                    "Add a host, or import the ones from your ~/.ssh/config.",
                )
            } else {
                ("No hosts match", "Try another search or group filter.")
            };
            let mut empty = v_stack()
                .items_center()
                .gap(p.space(6.0))
                .py(p.space(48.0))
                .child(
                    div()
                        .text_size(p.control_space(14.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(p.fg)
                        .child(message),
                )
                .child(
                    div()
                        .text_size(p.control_space(12.0))
                        .text_color(p.muted)
                        .child(hint),
                );
            if self.hosts.is_empty() {
                empty = empty.child(
                    div().pt(p.space(8.0)).child(
                        button("hosts-import", *p, ButtonVariant::Outlined, ButtonSize::Xs)
                            .child("Import from ~/.ssh/config")
                            .on_click(
                                cx.listener(|this, _: &ClickEvent, _w, cx| this.open_import(cx)),
                            ),
                    ),
                );
            }
            return page.child(empty).into_any_element();
        }

        let mut group = settings_group("hosts-list", *p);
        let count = visible.len();
        for (index, host) in visible.iter().enumerate() {
            group = group.row(self.host_row(host, index == 0, index + 1 == count, p, cx));
        }
        page.child(group).into_any_element()
    }

    fn host_row(
        &self,
        host: &Host,
        first: bool,
        last: bool,
        p: &Palette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let data = self.host_row_data(host);
        let dot = self.status_dot(&host.id, p);
        let (id_click, id_menu, id_drop) = (host.id.clone(), host.id.clone(), host.id.clone());
        let accent = p.accent;
        div()
            .id(SharedString::from(format!("host-item-{}", host.id)))
            .flex()
            .items_center()
            .gap(p.space(12.0))
            .w_full()
            .min_h(p.control_space(56.0))
            .px(p.control_space(16.0))
            .py(p.control_space(8.0))
            .cursor_pointer()
            .hover(move |s| s.bg(accent))
            .map(|row| round_edges(row, first, last, p))
            .child(avatar(p, data.icon, 32.0))
            .child(
                v_stack()
                    .flex_1()
                    .min_w_0()
                    .child(
                        h_stack()
                            .items_center()
                            .gap(p.space(6.0))
                            .child(
                                div()
                                    .text_size(p.control_space(13.0))
                                    .text_color(p.fg)
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(SharedString::from(data.name.clone())),
                            )
                            .when(data.pin_to_top, |d| {
                                d.child(IconName::Pin.svg(p.muted).size(px(11.0)))
                            }),
                    )
                    .child(
                        div()
                            .text_size(p.control_space(12.0))
                            .text_color(p.muted)
                            .overflow_hidden()
                            .whitespace_nowrap()
                            .child(SharedString::from(data.subtitle.clone())),
                    ),
            )
            .child(
                h_stack()
                    .items_center()
                    .gap(p.space(8.0))
                    .flex_shrink_0()
                    .when(data.status != HostStatus::Disconnected, |d| {
                        d.child(
                            div()
                                .text_size(p.control_space(11.0))
                                .text_color(if data.status == HostStatus::Failed {
                                    p.error
                                } else {
                                    p.success
                                })
                                .child(data.status.label()),
                        )
                    })
                    .child(indicator(IndicatorSize::Sm, dot))
                    .child(IconName::ChevronRight.svg(p.muted).size(px(14.0))),
            )
            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                if let Some(host) = this.hosts.iter().find(|h| h.id == id_click).cloned() {
                    this.select_host(&host, cx);
                }
            }))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, ev: &MouseDownEvent, _w, cx| {
                    this.host_menu = Some((id_menu.clone(), ev.position));
                    cx.notify();
                }),
            )
            .on_drag(
                DraggedHost {
                    id: host.id.clone(),
                },
                |_, _, _, cx| cx.new(|_| HostDragGhost),
            )
            .drag_over::<DraggedHost>(move |style, _, _, _| style.bg(accent))
            .on_drop(cx.listener(move |this, dragged: &DraggedHost, _w, cx| {
                this.reorder_hosts(&dragged.id, &id_drop, cx);
            }))
            .into_any_element()
    }

    fn visible_credentials(&self) -> Vec<Credential> {
        let q = self.search.trim().to_lowercase();
        self.credentials
            .iter()
            .filter(|c| match self.cred_filter {
                None => true,
                Some(key) => (c.cred_type == "key") == key,
            })
            .filter(|c| q.is_empty() || c.name.to_lowercase().contains(&q))
            .cloned()
            .collect()
    }

    fn render_cred_list(&mut self, p: &Palette, cx: &mut Context<Self>) -> AnyElement {
        let visible = self.visible_credentials();
        if visible.is_empty() {
            let (message, hint) = if self.credentials.is_empty() {
                (
                    "No credentials yet",
                    "A credential is a saved password or SSH key you can reuse across hosts.",
                )
            } else {
                ("No credentials match", "Try another search or filter.")
            };
            return v_stack()
                .items_center()
                .gap(p.space(6.0))
                .py(p.space(48.0))
                .child(
                    div()
                        .text_size(p.control_space(14.0))
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(p.fg)
                        .child(message),
                )
                .child(
                    div()
                        .text_size(p.control_space(12.0))
                        .text_color(p.muted)
                        .child(hint),
                )
                .into_any_element();
        }
        let accent = p.accent;
        let mut group = settings_group("credentials-list", *p);
        let count = visible.len();
        for (index, credential) in visible.into_iter().enumerate() {
            let (first, last) = (index == 0, index + 1 == count);
            let used = self
                .hosts
                .iter()
                .filter(|h| h.credential_id.as_deref() == Some(credential.id.as_str()))
                .count();
            let id = credential.id.clone();
            group = group.row(
                div()
                    .id(SharedString::from(format!("cred-item-{}", credential.id)))
                    .flex()
                    .items_center()
                    .gap(p.space(12.0))
                    .w_full()
                    .min_h(p.control_space(56.0))
                    .px(p.control_space(16.0))
                    .py(p.control_space(8.0))
                    .cursor_pointer()
                    .hover(move |s| s.bg(accent))
                    .map(|row| round_edges(row, first, last, p))
                    .child(avatar(p, credential_icon(&credential), 32.0))
                    .child(
                        v_stack()
                            .flex_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_size(p.control_space(13.0))
                                    .text_color(p.fg)
                                    .overflow_hidden()
                                    .whitespace_nowrap()
                                    .child(SharedString::from(credential.name.clone())),
                            )
                            .child(
                                div()
                                    .text_size(p.control_space(12.0))
                                    .text_color(p.muted)
                                    .child(SharedString::from(credential_subtitle(&credential))),
                            ),
                    )
                    .child(
                        h_stack()
                            .items_center()
                            .gap(p.space(8.0))
                            .flex_shrink_0()
                            .child(
                                div()
                                    .text_size(p.control_space(12.0))
                                    .text_color(p.muted)
                                    .child(SharedString::from(match used {
                                        0 => "Unused".to_string(),
                                        1 => "1 host".to_string(),
                                        n => format!("{n} hosts"),
                                    })),
                            )
                            .child(IconName::ChevronRight.svg(p.muted).size(px(14.0))),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                        if let Some(c) = this.credentials.iter().find(|c| c.id == id).cloned() {
                            this.select_credential(&c, cx);
                        }
                    })),
            );
        }
        group.into_any_element()
    }
}

// ── host detail ────────────────────────────────────────────────────────────

impl HostManagerView {
    fn render_host_detail(
        &mut self,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(form) = self.form.clone() else {
            return div().into_any_element();
        };
        let tab_token = match form.tab {
            FormTab::General => "general",
            FormTab::Ssh => "ssh",
            FormTab::Sftp => "sftp",
            FormTab::Tunnels => "tunnels",
        };
        let tabs = self.segmented(
            "host-tabs",
            tab_token,
            [
                ("general", "General"),
                ("ssh", "SSH"),
                ("sftp", "SFTP"),
                ("tunnels", "Tunnels"),
            ],
            p,
            |this, key, cx| {
                if let Some(form) = this.form.as_mut() {
                    form.tab = match key {
                        "ssh" => FormTab::Ssh,
                        "sftp" => FormTab::Sftp,
                        "tunnels" => FormTab::Tunnels,
                        _ => FormTab::General,
                    };
                }
                this.dropdown = None;
                cx.notify();
            },
            cx,
        );
        let content = match form.tab {
            FormTab::General => self.render_general_tab(&form, p, window, cx),
            FormTab::Ssh => self.render_ssh_tab(&form, p, window, cx),
            FormTab::Sftp => self.render_sftp_tab(p, window, cx),
            FormTab::Tunnels => self.render_tunnels_tab(&form, p, window, cx),
        };
        v_stack()
            .gap(p.space(20.0))
            .child(h_stack().child(tabs))
            .child(content)
            .into_any_element()
    }

    fn render_general_tab(
        &mut self,
        form: &HostForm,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let editing = form.editing_id.clone();
        let port_invalid = !valid_port(&form.port);

        let connection = settings_group("hf-connection", *p)
            .title("Connection")
            .row(
                settings_row("hf-name", *p, "Display name")
                    .description("The label shown in lists and tab titles.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::Name),
                        InputKind::Single,
                        "My server",
                        false,
                        window,
                        cx,
                        p,
                    )),
            )
            .row(
                settings_row("hf-address", *p, "Host / IP address")
                    .description("The host name or address to connect to.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::Address),
                        InputKind::Single,
                        "example.com",
                        false,
                        window,
                        cx,
                        p,
                    )),
            )
            .row(
                settings_row("hf-port", *p, "Port")
                    .description(if port_invalid {
                        "Enter a port between 1 and 65535 (22 is used otherwise)."
                    } else {
                        "The SSH port of the host."
                    })
                    .control(self.text_control(
                        FieldKey::Host(HostField::Port),
                        InputKind::Single,
                        "22",
                        port_invalid,
                        window,
                        cx,
                        p,
                    )),
            );

        let mut group_options: Vec<SelectOption> = vec![("".into(), "None".into())];
        group_options.extend(self.groups.iter().map(|g| {
            (
                SharedString::from(g.id.clone()),
                SharedString::from(g.name.clone()),
            )
        }));
        let group_label = form
            .group
            .as_deref()
            .and_then(|id| self.groups.iter().find(|g| g.id == id))
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "None".to_string());
        let general = settings_group("hf-general", *p)
            .title("General")
            .row(
                settings_row("hf-group", *p, "Group")
                    .description("Groups appear as filters above the host list.")
                    .control(self.select_control(
                        "hf-group-select",
                        group_label,
                        DropdownKind::Group,
                        group_options,
                        form.group.clone().unwrap_or_default(),
                        p,
                        cx,
                    )),
            )
            .row(self.switch_row(
                "hf-pin",
                "Pin to top",
                "Always show this host first.",
                form.pin_to_top,
                p,
                cx,
                |this, on| {
                    if let Some(form) = this.form.as_mut() {
                        form.pin_to_top = on;
                    }
                },
            ));

        let auth = form.auth;
        let auth_token = auth.as_str();
        let auth_picker = self.segmented(
            "hf-auth-method",
            auth_token,
            [
                ("password", "Password"),
                ("key", "SSH Key"),
                ("credential", "Credential"),
                ("none", "None"),
            ],
            p,
            |this, key, cx| {
                if let Some(form) = this.form.as_mut() {
                    form.auth = AuthMethod::from_str(key);
                }
                this.dropdown = None;
                this.schedule_autosave(cx);
            },
            cx,
        );
        let mut authentication = settings_group("hf-auth", *p)
            .title("Authentication")
            .row(
                settings_row("hf-username", *p, "Username")
                    .description("The account to log in as.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::Username),
                        InputKind::Single,
                        "root",
                        false,
                        window,
                        cx,
                        p,
                    )),
            )
            .row(
                settings_row("hf-auth-row", *p, "Auth method")
                    .description("How Labonair proves who you are to this host.")
                    .extra(div().pt(p.space(8.0)).child(auth_picker)),
            );
        authentication = match auth {
            AuthMethod::Password => authentication.row(
                settings_row("hf-password", *p, "Password")
                    .description("Stored in the secret store, never in the database.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::Password),
                        InputKind::Masked,
                        "Password",
                        false,
                        window,
                        cx,
                        p,
                    )),
            ),
            AuthMethod::Key => authentication.row(
                settings_row("hf-key-path", *p, "Private key file")
                    .description("Path to the private key, for example ~/.ssh/id_ed25519.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::KeyPath),
                        InputKind::Single,
                        "~/.ssh/id_ed25519",
                        false,
                        window,
                        cx,
                        p,
                    )),
            ),
            AuthMethod::Credential => {
                let mut options: Vec<SelectOption> = vec![("".into(), "None".into())];
                options.extend(self.credentials.iter().map(|c| {
                    (
                        SharedString::from(c.id.clone()),
                        SharedString::from(credential_label(c)),
                    )
                }));
                let label = form
                    .credential
                    .as_deref()
                    .and_then(|id| self.credentials.iter().find(|c| c.id == id))
                    .map(credential_label)
                    .unwrap_or_else(|| "None".to_string());
                let origin = editing.clone();
                authentication.row(
                    settings_row("hf-credential", *p, "Credential")
                        .description("Reuse a saved password or SSH key.")
                        .extra(
                            div().pt(p.space(4.0)).child(
                                button(
                                    "hf-new-credential",
                                    *p,
                                    ButtonVariant::Link,
                                    ButtonSize::Xs,
                                )
                                .child("+ Create new credential")
                                .on_click(cx.listener(
                                    move |this, _: &ClickEvent, _w, cx| {
                                        // The host must exist so the credential page
                                        // can send the user back to it.
                                        this.flush_open_form(cx);
                                        this.mode = PageMode::Credentials;
                                        this.new_credential(origin.clone(), cx);
                                    },
                                )),
                            ),
                        )
                        .control(self.select_control(
                            "hf-credential-select",
                            label,
                            DropdownKind::Credential,
                            options,
                            form.credential.clone().unwrap_or_default(),
                            p,
                            cx,
                        )),
                )
            }
            AuthMethod::None => authentication,
        };

        let mut jump_options: Vec<SelectOption> =
            vec![("".into(), "None (direct connection)".into())];
        for host in &self.hosts {
            let blocked = editing
                .as_deref()
                .is_some_and(|id| jump_creates_cycle(&self.hosts, id, &host.id));
            if !blocked {
                jump_options.push((
                    SharedString::from(host.id.clone()),
                    SharedString::from(host.name.clone()),
                ));
            }
        }
        let jump_label = form
            .jump_host
            .as_deref()
            .and_then(|id| self.hosts.iter().find(|h| h.id == id))
            .map(|h| h.name.clone())
            .unwrap_or_else(|| "None (direct connection)".to_string());
        let jump = settings_group("hf-jump", *p)
            .title("Jump host (ProxyJump)")
            .row(
                settings_row("hf-jump-row", *p, "Via jump host")
                    .description("Connect through another host as a bastion / jump server.")
                    .control(self.select_control(
                        "hf-jump-select",
                        jump_label,
                        DropdownKind::JumpHost,
                        jump_options,
                        form.jump_host.clone().unwrap_or_default(),
                        p,
                        cx,
                    )),
            );

        let notes_input = self.input_for(
            FieldKey::Host(HostField::Notes),
            InputKind::Multi,
            "Notes, runbook, reminders\u{2026}",
            window,
            cx,
        );
        let notes = settings_group("hf-notes", *p).title("Notes / runbook").row(
            div().p(p.control_space(12.0)).child(text_area_surface(
                "hf-notes-field",
                *p,
                if self.focused_input == Some(FieldKey::Host(HostField::Notes)) {
                    TextFieldState::Focused
                } else {
                    TextFieldState::Normal
                },
                text_input(&notes_input, *p),
            )),
        );

        v_stack()
            .gap(p.space(20.0))
            .child(connection)
            .child(general)
            .child(authentication)
            .child(jump)
            .child(notes)
            .into_any_element()
    }

    fn render_ssh_tab(
        &mut self,
        form: &HostForm,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let interval_invalid = !valid_optional_number(&form.keep_alive_interval);
        let tries_invalid = !valid_optional_number(&form.keep_alive_tries);
        let connection = settings_group("hs-connection", *p)
            .title("Connection")
            .row(
                settings_row("hs-path", *p, "Default path")
                    .description("Runs `cd <path>` automatically after connecting.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::DefaultPath),
                        InputKind::Single,
                        "/home/user/projects",
                        false,
                        window,
                        cx,
                        p,
                    )),
            )
            .row(
                settings_row("hs-keepalive", *p, "Keep-alive interval (s)")
                    .description("Seconds between keep-alive packets. Empty uses the default.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::KeepAliveInterval),
                        InputKind::Single,
                        "25",
                        interval_invalid,
                        window,
                        cx,
                        p,
                    )),
            )
            .row(
                settings_row("hs-tries", *p, "Max tries")
                    .description("Missed keep-alives before the connection is dropped.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::KeepAliveTries),
                        InputKind::Single,
                        "3",
                        tries_invalid,
                        window,
                        cx,
                        p,
                    )),
            );

        let sudo_set = form.sudo_password_set && !form.sudo_clear;
        let mut sudo_control = h_stack().items_center().gap(p.space(8.0));
        if sudo_set {
            sudo_control = sudo_control.child(
                button(
                    "hs-sudo-remove",
                    *p,
                    ButtonVariant::Outlined,
                    ButtonSize::Xs,
                )
                .child("Remove")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    if let Some(form) = this.form.as_mut() {
                        form.sudo_clear = true;
                        form.sudo_password.clear();
                    }
                    // Recreate the field without the "set" placeholder.
                    this.inputs.remove(&FieldKey::Host(HostField::SudoPassword));
                    this.schedule_autosave(cx);
                })),
            );
        }
        sudo_control = sudo_control.child(self.text_control(
            FieldKey::Host(HostField::SudoPassword),
            InputKind::Masked,
            if sudo_set {
                "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022} (set)"
            } else {
                "Sudo password"
            },
            false,
            window,
            cx,
            p,
        ));
        let sudo = settings_group("hs-sudo", *p).title("Sudo").row(
            settings_row("hs-sudo-row", *p, "Sudo password autofill")
                .description(
                    "Automatically fills sudo password prompts. Stored in the system keychain.",
                )
                .control(sudo_control),
        );

        let agent = settings_group("hs-agent", *p).title("Agent access").row(
            self.switch_row(
                "hs-block-agent",
                "Block AI agent access",
                "Prevents this host's SSH tabs from ever being granted or used via the AI agent bridge, even if enabled elsewhere.",
                form.block_agent_access,
                p,
                cx,
                |this, on| {
                    if let Some(form) = this.form.as_mut() {
                        form.block_agent_access = on;
                    }
                },
            ),
        );

        let mut snippet_options: Vec<SelectOption> = vec![("".into(), "None (disabled)".into())];
        snippet_options.extend(self.snippets.iter().map(|(id, name)| {
            (
                SharedString::from(id.clone()),
                SharedString::from(name.clone()),
            )
        }));
        let snippet_label = form
            .snippet_id
            .as_deref()
            .and_then(|sid| self.snippets.iter().find(|(id, _)| id == sid))
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| "None (disabled)".to_string());
        let mut startup = settings_group("hs-startup", *p)
            .title("Startup snippet")
            .row(
                settings_row("hs-snippet", *p, "Snippet")
                    .description("Runs automatically when this host connects via SSH.")
                    .control(self.select_control(
                        "hs-snippet-select",
                        snippet_label,
                        DropdownKind::Snippet,
                        snippet_options,
                        form.snippet_id.clone().unwrap_or_default(),
                        p,
                        cx,
                    )),
            );
        if form.snippet_id.is_some() {
            let mode_token = if form.snippet_mode == "inject" {
                "inject"
            } else {
                "execute"
            };
            startup = startup.row(
                settings_row("hs-snippet-mode", *p, "Run mode")
                    .description("Execute presses enter; Inject only types the snippet.")
                    .control(self.segmented(
                        "hs-snippet-mode-seg",
                        mode_token,
                        [("execute", "Execute"), ("inject", "Inject")],
                        p,
                        |this, key, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.snippet_mode = key.to_string();
                            }
                            this.schedule_autosave(cx);
                        },
                        cx,
                    )),
            );
        }

        v_stack()
            .gap(p.space(20.0))
            .child(connection)
            .child(sudo)
            .child(agent)
            .child(startup)
            .into_any_element()
    }

    fn render_sftp_tab(
        &mut self,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        settings_group("hp-connection", *p)
            .title("Connection")
            .row(
                settings_row("hp-path", *p, "Default path")
                    .description("Opens this directory in the SFTP file browser on connect.")
                    .control(self.text_control(
                        FieldKey::Host(HostField::DefaultPathSftp),
                        InputKind::Single,
                        "/var/www",
                        false,
                        window,
                        cx,
                        p,
                    )),
            )
            .into_any_element()
    }

    fn render_tunnels_tab(
        &mut self,
        form: &HostForm,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let caption = |text: &'static str| {
            div()
                .text_size(p.control_space(11.0))
                .text_color(p.muted)
                .child(text)
        };
        let mut group = settings_group("ht-tunnels", *p).title("Local port forwards");
        for (index, tunnel) in form.tunnels.iter().enumerate() {
            let local_invalid = !tunnel.local_port.is_empty() && !valid_port(&tunnel.local_port);
            let remote_invalid = !tunnel.remote_port.is_empty() && !valid_port(&tunnel.remote_port);
            let local = self
                .text_control(
                    FieldKey::Host(HostField::TunnelLocalPort(index)),
                    InputKind::Single,
                    "8080",
                    local_invalid,
                    window,
                    cx,
                    p,
                )
                .w(p.control_space(96.0));
            let remote_host = self
                .text_control(
                    FieldKey::Host(HostField::TunnelRemoteHost(index)),
                    InputKind::Single,
                    "127.0.0.1",
                    false,
                    window,
                    cx,
                    p,
                )
                .w_full();
            let remote_port = self
                .text_control(
                    FieldKey::Host(HostField::TunnelRemotePort(index)),
                    InputKind::Single,
                    "8080",
                    remote_invalid,
                    window,
                    cx,
                    p,
                )
                .w(p.control_space(96.0));
            group = group.row(
                h_stack()
                    .items_end()
                    .gap(p.space(10.0))
                    .px(p.control_space(16.0))
                    .py(p.control_space(12.0))
                    .child(
                        v_stack()
                            .gap(p.space(4.0))
                            .child(caption("Local port"))
                            .child(local),
                    )
                    .child(
                        div()
                            .pb(p.space(9.0))
                            .child(IconName::ArrowRight.svg(p.muted).size(px(14.0))),
                    )
                    .child(
                        v_stack()
                            .flex_1()
                            .min_w_0()
                            .gap(p.space(4.0))
                            .child(caption("Remote host"))
                            .child(remote_host),
                    )
                    .child(
                        v_stack()
                            .gap(p.space(4.0))
                            .child(caption("Remote port"))
                            .child(remote_port),
                    )
                    .child(
                        icon_button_builder(
                            SharedString::from(format!("tunnel-del-{index}")),
                            *p,
                            IconName::Trash,
                        )
                        .variant(ButtonVariant::Subtle)
                        .size(ButtonSize::IconSm)
                        .shape(IconButtonShape::Square)
                        .tooltip("Remove tunnel")
                        .render()
                        .on_click(cx.listener(
                            move |this, _: &ClickEvent, _w, cx| {
                                if let Some(form) = this.form.as_mut() {
                                    if index < form.tunnels.len() {
                                        form.tunnels.remove(index);
                                    }
                                }
                                // Row indices shifted: rebuild the tunnel inputs.
                                this.inputs.retain(|key, _| {
                                    !matches!(
                                        key,
                                        FieldKey::Host(
                                            HostField::TunnelLocalPort(_)
                                                | HostField::TunnelRemoteHost(_)
                                                | HostField::TunnelRemotePort(_)
                                        )
                                    )
                                });
                                this.schedule_autosave(cx);
                            },
                        )),
                    ),
            );
        }
        let add = icon_button_builder("ht-add", *p, IconName::Plus)
            .variant(ButtonVariant::Outlined)
            .size(ButtonSize::Sm)
            .label("Add tunnel")
            .render()
            .w_full()
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                if let Some(form) = this.form.as_mut() {
                    form.tunnels.push(TunnelDraft::new());
                }
                this.schedule_autosave(cx);
            }));
        let mut tab = v_stack().gap(p.space(12.0));
        if form.tunnels.is_empty() {
            tab = tab.child(
                div()
                    .text_size(p.control_space(12.0))
                    .text_color(p.muted)
                    .child("Forward a local port to a port on (or behind) this host. Tunnels start when you connect."),
            );
        } else {
            tab = tab.child(group);
        }
        tab.child(add).into_any_element()
    }
}

// ── credential detail ──────────────────────────────────────────────────────

impl HostManagerView {
    fn render_cred_detail(
        &mut self,
        p: &Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(form) = self.cred_form.clone() else {
            return div().into_any_element();
        };
        let type_picker = self.segmented(
            "cf-type",
            if form.is_key { "key" } else { "password" },
            [("password", "Password"), ("key", "SSH Key")],
            p,
            |this, key, cx| {
                if let Some(form) = this.cred_form.as_mut() {
                    form.is_key = key == "key";
                }
                this.schedule_autosave(cx);
            },
            cx,
        );
        let general = settings_group("cf-general", *p)
            .title("Credential")
            .row(
                settings_row("cf-name", *p, "Name")
                    .description("How this credential is listed when you pick it for a host.")
                    .control(self.text_control(
                        FieldKey::Cred(CredField::Name),
                        InputKind::Single,
                        "Labonair network",
                        false,
                        window,
                        cx,
                        p,
                    )),
            )
            .row(
                settings_row("cf-type-row", *p, "Type")
                    .description("A password, or an SSH private key with an optional passphrase.")
                    .control(type_picker),
            );

        let secret_set = form.has_secret && !form.secret_clear;
        let mut secret_control = h_stack().items_center().gap(p.space(8.0));
        if secret_set {
            secret_control = secret_control.child(
                button(
                    "cf-secret-remove",
                    *p,
                    ButtonVariant::Outlined,
                    ButtonSize::Xs,
                )
                .child("Remove")
                .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                    if let Some(form) = this.cred_form.as_mut() {
                        form.secret_clear = true;
                        form.secret.clear();
                    }
                    this.inputs.remove(&FieldKey::Cred(CredField::Secret));
                    this.schedule_autosave(cx);
                })),
            );
        }
        let secret_label = if form.is_key {
            "Passphrase"
        } else {
            "Password"
        };
        secret_control = secret_control.child(self.text_control(
            FieldKey::Cred(CredField::Secret),
            InputKind::Masked,
            if secret_set {
                "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022} (set)"
            } else {
                secret_label
            },
            false,
            window,
            cx,
            p,
        ));

        let mut page = v_stack().gap(p.space(20.0)).child(general);

        if form.is_key {
            let can_generate = form.editing_id.is_some();
            let generate = if can_generate {
                button("cf-generate", *p, ButtonVariant::Outlined, ButtonSize::Xs)
                    .child("Generate ed25519 key")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.generate_key(cx)))
                    .into_any_element()
            } else {
                button_disabled("cf-generate", *p, ButtonVariant::Outlined, ButtonSize::Xs)
                    .child("Generate ed25519 key")
                    .into_any_element()
            };
            let mut key_group = settings_group("cf-key", *p)
                .title("Key")
                .row(
                    settings_row("cf-key-path", *p, "Private key file")
                        .description("An existing key file, or generate a new key pair below.")
                        .control(self.text_control(
                            FieldKey::Cred(CredField::KeyPath),
                            InputKind::Single,
                            "~/.ssh/id_ed25519",
                            false,
                            window,
                            cx,
                            p,
                        )),
                )
                .row(
                    settings_row("cf-generate-row", *p, "Generate key pair")
                        .description(if can_generate {
                            "Replaces the key file above with a new key in Labonair's key folder. The passphrase below protects it."
                        } else {
                            "Give the credential a name first so it can be saved."
                        })
                        .control(generate),
                )
                .row(
                    settings_row("cf-passphrase", *p, "Passphrase")
                        .description("Optional. Stored in the secret store, never in the database.")
                        .control(secret_control),
                );
            if let Some(public_key) = form.public_key.clone() {
                let copy_value = public_key.clone();
                key_group = key_group.row(
                    settings_row("cf-public-key", *p, "Public key")
                        .description("Add this to the server's authorized_keys.")
                        .extra(
                            div()
                                .mt(p.space(6.0))
                                .px(p.control_space(10.0))
                                .py(p.control_space(8.0))
                                .rounded(px(p.radius.md))
                                .bg(p.field_bg)
                                .border_1()
                                .border_color(p.input)
                                .text_size(p.control_space(11.0))
                                .text_color(p.muted)
                                .font_family("monospace")
                                .child(SharedString::from(public_key)),
                        )
                        .control(
                            icon_button_builder("cf-copy", *p, IconName::Copy)
                                .variant(ButtonVariant::Outlined)
                                .size(ButtonSize::Xs)
                                .label("Copy")
                                .render()
                                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(
                                        copy_value.clone(),
                                    ));
                                    this.notify(
                                        "Public key",
                                        "Copied to the clipboard.".into(),
                                        cx,
                                    );
                                })),
                        ),
                );
            }
            page = page.child(key_group);
        } else {
            page = page.child(
                settings_group("cf-secret", *p).title("Secret").row(
                    settings_row("cf-password", *p, "Password")
                        .description("Stored in the secret store, never in the database.")
                        .control(secret_control),
                ),
            );
        }

        if let Some(id) = form.editing_id.clone() {
            let users: Vec<(String, String)> = self
                .hosts
                .iter()
                .filter(|h| h.credential_id.as_deref() == Some(id.as_str()))
                .map(|h| (h.id.clone(), h.name.clone()))
                .collect();
            let mut used = settings_group("cf-used", *p).title("Used by");
            if users.is_empty() {
                used = used.row(
                    settings_row("cf-unused", *p, "Not used by any host yet")
                        .description("Pick this credential in a host's authentication settings."),
                );
            }
            let accent = p.accent;
            let count = users.len();
            for (index, (host_id, name)) in users.into_iter().enumerate() {
                let (first, last) = (index == 0, index + 1 == count);
                used = used.row(
                    div()
                        .id(SharedString::from(format!("cf-used-{host_id}")))
                        .flex()
                        .items_center()
                        .gap(p.space(10.0))
                        .px(p.control_space(16.0))
                        .min_h(p.control_space(44.0))
                        .cursor_pointer()
                        .hover(move |s| s.bg(accent))
                        .map(|row| round_edges(row, first, last, p))
                        .child(IconName::Server.svg(p.muted).size(px(14.0)))
                        .child(
                            div()
                                .flex_1()
                                .text_size(p.control_space(13.0))
                                .text_color(p.fg)
                                .child(SharedString::from(name)),
                        )
                        .child(IconName::ChevronRight.svg(p.muted).size(px(14.0)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.flush_open_form(cx);
                            if let Some(host) = this.hosts.iter().find(|h| h.id == host_id).cloned()
                            {
                                this.mode = PageMode::Hosts;
                                this.select_host(&host, cx);
                            }
                        })),
                );
            }
            page = page.child(used);

            let delete_id = id;
            page = page.child(
                settings_group("cf-danger", *p).row(
                    settings_row("cf-delete", *p, "Delete credential")
                        .description("Hosts using it fall back to password authentication.")
                        .control(
                            button(
                                "cf-delete-btn",
                                *p,
                                ButtonVariant::TintedError,
                                ButtonSize::Xs,
                            )
                            .child("Delete")
                            .on_click(cx.listener(
                                move |this, _: &ClickEvent, _w, cx| {
                                    this.confirm =
                                        Some(Confirm::DeleteCredential(delete_id.clone()));
                                    cx.notify();
                                },
                            )),
                        ),
                ),
            );
        }
        page.into_any_element()
    }
}

// ── overlays ───────────────────────────────────────────────────────────────

impl HostManagerView {
    /// The host-avatar picker, anchored under the detail header.
    fn render_icon_picker(&mut self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.icon_picker_open || self.form.is_none() {
            return None;
        }
        let current = self.form.as_ref().and_then(|f| f.icon.clone());
        let tiles = HOST_ICONS.iter().map(|(key, icon)| {
            let key = key.to_string();
            let active = current.as_deref() == Some(key.as_str());
            icon_button_builder(SharedString::from(format!("host-icon-{key}")), *p, *icon)
                .size(ButtonSize::IconLg)
                .selected(active)
                .render()
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    if let Some(form) = this.form.as_mut() {
                        form.icon = Some(key.clone());
                    }
                    this.icon_picker_open = false;
                    this.schedule_autosave(cx);
                }))
        });
        Some(
            div()
                .id("host-icon-picker")
                .absolute()
                .top(p.space(64.0))
                .left(p.space(PAGE_PAD + 44.0))
                .occlude()
                .flex()
                .flex_col()
                .gap(p.space(6.0))
                .p(p.space(8.0))
                .w(p.control_space(204.0))
                .rounded(px(p.radius.lg))
                .bg(p.popover)
                .border_1()
                .border_color(p.border)
                .shadow(p.popover_shadow())
                .on_mouse_down_out(cx.listener(|this, _, _w, cx| {
                    this.icon_picker_open = false;
                    cx.notify();
                }))
                .child(h_stack().flex_wrap().gap(p.space(2.0)).children(tiles))
                .child(
                    button("host-icon-reset", *p, ButtonVariant::Subtle, ButtonSize::Xs)
                        .child("Reset to default")
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            if let Some(form) = this.form.as_mut() {
                                form.icon = None;
                            }
                            this.icon_picker_open = false;
                            this.schedule_autosave(cx);
                        })),
                )
                .into_any_element(),
        )
    }

    /// The Import / Export overflow menu of the list header.
    fn render_more_menu(&mut self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let at = self.more_menu?;
        let view = cx.entity();
        let close = {
            let view = view.clone();
            move |_w: &mut Window, cx: &mut App| {
                view.update(cx, |this, cx| {
                    this.more_menu = None;
                    cx.notify();
                })
            }
        };
        let items = vec![
            MenuItem::new("more-import", "Import from ~/.ssh/config\u{2026}")
                .icon(IconName::Download)
                .on_click({
                    let view = view.clone();
                    move |_, _w, cx| {
                        view.update(cx, |this, cx| {
                            this.more_menu = None;
                            this.open_import(cx);
                        })
                    }
                }),
            MenuItem::new("more-export", "Export to ~/.ssh/config\u{2026}")
                .icon(IconName::Share)
                .on_click({
                    let view = view.clone();
                    move |_, _w, cx| {
                        view.update(cx, |this, cx| {
                            this.more_menu = None;
                            this.open_export(cx);
                        })
                    }
                }),
        ];
        Some(context_menu(at, *p, close, items))
    }

    /// Confirmation for the destructive actions (host / credential delete).
    fn render_confirm(&mut self, p: &Palette, cx: &mut Context<Self>) -> Option<AnyElement> {
        let confirm = self.confirm.clone()?;
        let (title, body, action): (&str, String, Confirm) = match &confirm {
            Confirm::DeleteHost(id) => {
                let name = self
                    .hosts
                    .iter()
                    .find(|h| h.id == *id)
                    .map(|h| h.name.clone())
                    .unwrap_or_default();
                (
                    "Delete host?",
                    format!("\u{201c}{name}\u{201d} and its stored passwords will be removed. This can't be undone."),
                    confirm.clone(),
                )
            }
            Confirm::DeleteCredential(id) => {
                let used = self
                    .hosts
                    .iter()
                    .filter(|h| h.credential_id.as_deref() == Some(id.as_str()))
                    .count();
                let generated = self
                    .credentials
                    .iter()
                    .find(|c| c.id == *id)
                    .and_then(|c| c.key_path.as_deref())
                    .is_some_and(|path| {
                        std::path::Path::new(path).starts_with(self.data_dir.join("keys"))
                    });
                let mut body =
                    String::from("The credential and its stored secret will be removed.");
                if used > 0 {
                    body.push_str(&format!(
                        " {used} host{} using it will fall back to password authentication.",
                        if used == 1 { "" } else { "s" }
                    ));
                }
                if generated {
                    body.push_str(" The generated key file will be deleted too.");
                }
                ("Delete credential?", body, confirm.clone())
            }
        };
        let cancel = cx.listener(|this, _: &ClickEvent, _w, cx| {
            this.confirm = None;
            cx.notify();
        });
        let accept = cx.listener(move |this, _: &ClickEvent, _w, cx| {
            this.confirm = None;
            match &action {
                Confirm::DeleteHost(id) => this.delete_host(id.clone(), cx),
                Confirm::DeleteCredential(id) => this.delete_credential(id.clone(), cx),
            }
            cx.notify();
        });
        Some(
            modal_overlay("hosts-confirm")
                .child(
                    self.modal_shell(420.0, p)
                        .child(
                            div()
                                .text_size(p.control_space(14.0))
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(p.fg)
                                .child(title),
                        )
                        .child(
                            div()
                                .text_size(p.control_space(13.0))
                                .text_color(p.muted)
                                .child(SharedString::from(body)),
                        )
                        .child(
                            h_stack()
                                .justify_end()
                                .gap(p.space(8.0))
                                .pt(p.space(8.0))
                                .child(
                                    button(
                                        "confirm-cancel",
                                        *p,
                                        ButtonVariant::Outlined,
                                        ButtonSize::Xs,
                                    )
                                    .child("Cancel")
                                    .on_click(cancel),
                                )
                                .child(
                                    button(
                                        "confirm-ok",
                                        *p,
                                        ButtonVariant::TintedError,
                                        ButtonSize::Xs,
                                    )
                                    .child("Delete")
                                    .on_click(accept),
                                ),
                        ),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host(id: &str, jump: Option<&str>) -> Host {
        Host {
            jump_host_id: jump.map(str::to_string),
            ..super::super::tests::host_stub(id, id)
        }
    }

    #[test]
    fn ports_must_be_numbers_in_range() {
        assert!(valid_port("22"));
        assert!(valid_port(" 65535 "));
        assert!(!valid_port("0"));
        assert!(!valid_port("65536"));
        assert!(!valid_port("ssh"));
        assert!(valid_optional_number(""));
        assert!(valid_optional_number("30"));
        assert!(!valid_optional_number("-1"));
    }

    #[test]
    fn jump_host_choices_never_close_a_loop() {
        // a -> b -> c: routing c through a (or b) would loop; routing it through
        // an unrelated host is fine; a host can never route through itself.
        let hosts = vec![
            host("a", Some("b")),
            host("b", Some("c")),
            host("c", None),
            host("d", None),
        ];
        assert!(jump_creates_cycle(&hosts, "c", "a"));
        assert!(jump_creates_cycle(&hosts, "c", "b"));
        assert!(jump_creates_cycle(&hosts, "c", "c"));
        assert!(!jump_creates_cycle(&hosts, "c", "d"));
        assert!(!jump_creates_cycle(&hosts, "d", "a"));
    }
}
