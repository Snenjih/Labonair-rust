//! Step 3 — import hosts from `~/.ssh/config`.
//!
//! Shown only when the config has at least one host. The import runs when the
//! whole flow is finished, through the existing `SshConfigService`. The list
//! renders [`HostImportRow`]s only, so no key path or credential can appear.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    div, px, AnyView, App, AppContext, ClickEvent, Context, Entity, Focusable, InteractiveElement,
    IntoElement, ParentElement, Render, SharedString, StatefulInteractiveElement, Styled,
    Subscription, Task, Window,
};
use labonair_notifications::{notification_center, notify_err, Notification};
use labonair_ssh::{ImportConflict, SshConfigEntry, SshConfigService};
use labonair_theme::store::ThemeStore;
use labonair_ui_kit::{
    button, checkbox, disclosure, search_field, search_input, settings_group, settings_row, switch,
    text_field, ButtonSize, ButtonVariant, InputEvent, InputState, ListItem, Palette,
    ScrollableElement,
};

use crate::host_rows::{build_rows, default_selection, filter, HostImportRow};
use crate::services::{ExistingHostNames, OnboardingServices};
use crate::step::{OnboardingStep, StepInstance};

pub(super) struct HostsStep {
    theme: Entity<ThemeStore>,
    tokio: tokio::runtime::Handle,
    ssh_config: Arc<dyn SshConfigService>,
    existing_host_names: ExistingHostNames,
    on_hosts_imported: Rc<dyn Fn(&mut App)>,
    /// Filled by [`Self::prepare`]; `is_applicable` is "the config has hosts".
    discovered: Rc<RefCell<Vec<SshConfigEntry>>>,
}

impl HostsStep {
    pub(super) fn new(services: &OnboardingServices) -> Self {
        Self {
            theme: services.theme.clone(),
            tokio: services.tokio.clone(),
            ssh_config: services.ssh_config.clone(),
            existing_host_names: services.existing_host_names.clone(),
            on_hosts_imported: services.on_hosts_imported.clone(),
            discovered: Rc::default(),
        }
    }
}

impl OnboardingStep for HostsStep {
    fn id(&self) -> &'static str {
        "onboarding.hosts"
    }

    fn order(&self) -> u32 {
        300
    }

    fn title(&self) -> SharedString {
        "Bring your SSH hosts".into()
    }

    fn hint(&self) -> SharedString {
        "Found in ~/.ssh/config. Nothing is imported until you finish.".into()
    }

    fn is_applicable(&self, _cx: &App) -> bool {
        !self.discovered.borrow().is_empty()
    }

    fn prepare(&self, cx: &mut App) -> Task<()> {
        let config = self.ssh_config.clone();
        let parsing = self.tokio.spawn(async move { config.parse().await });
        let discovered = self.discovered.clone();
        cx.spawn(async move |_| {
            let entries = match parsing.await {
                Ok(Ok(entries)) => entries,
                Ok(Err(error)) => {
                    tracing::warn!(%error, "labonair-onboarding: cannot read ~/.ssh/config");
                    Vec::new()
                }
                Err(error) => {
                    tracing::warn!(%error, "labonair-onboarding: ssh config parse task failed");
                    Vec::new()
                }
            };
            *discovered.borrow_mut() = entries;
        })
    }

    fn build(&self, window: &mut Window, cx: &mut App) -> StepInstance {
        let entries = self.discovered.borrow().clone();
        let existing: HashSet<String> = (self.existing_host_names)(cx).into_iter().collect();
        let theme = self.theme.clone();
        let view = cx.new(|cx| HostsView::new(entries, &existing, theme, window, cx));

        let finish_view = view.clone();
        let config = self.ssh_config.clone();
        let tokio = self.tokio.clone();
        let imported = self.on_hosts_imported.clone();
        StepInstance {
            view: AnyView::from(view),
            on_finish: Some(Box::new(move |cx| {
                let entries = finish_view.read(cx).entries_to_import();
                if !entries.is_empty() {
                    run_import(entries, config, tokio, imported, cx);
                }
            })),
        }
    }
}

/// Imports on the tokio runtime and reports through the notification center.
/// Existing hosts are skipped, never overwritten.
fn run_import(
    entries: Vec<SshConfigEntry>,
    config: Arc<dyn SshConfigService>,
    tokio: tokio::runtime::Handle,
    on_imported: Rc<dyn Fn(&mut App)>,
    cx: &mut App,
) {
    let total = entries.len();
    let importing = tokio.spawn(async move { config.import(entries, ImportConflict::Skip).await });
    cx.spawn(async move |cx| {
        let result = match importing.await {
            Ok(result) => result,
            Err(error) => Err(error.to_string()),
        };
        let _ = cx.update(|cx| match result {
            Ok(ids) => {
                notification_center(cx).update(cx, |center, cx| {
                    center.push(
                        Notification::success(
                            "SSH hosts imported",
                            format!("{} of {total} host(s) imported.", ids.len()),
                        ),
                        cx,
                    );
                });
                on_imported(cx);
            }
            Err(error) => {
                notify_err("Could not import SSH hosts", Err::<(), _>(error), cx);
            }
        });
    })
    .detach();
}

struct HostsView {
    theme: Entity<ThemeStore>,
    entries: Vec<SshConfigEntry>,
    rows: Vec<HostImportRow>,
    selected: HashSet<String>,
    import_enabled: bool,
    advanced_open: bool,
    reveal_details: bool,
    search: Entity<InputState>,
    query: String,
    _search_subscription: Subscription,
}

impl HostsView {
    fn new(
        entries: Vec<SshConfigEntry>,
        existing: &HashSet<String>,
        theme: Entity<ThemeStore>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let rows = build_rows(&entries, existing);
        let selected = default_selection(&rows);
        let search = cx.new(|cx| text_field(window, cx).placeholder("Search hosts"));
        let subscription = cx.subscribe(&search, |this, input, event: &InputEvent, cx| {
            if let InputEvent::Change = event {
                this.query = input.read(cx).value().to_string();
                cx.notify();
            }
        });
        Self {
            theme,
            entries,
            rows,
            selected,
            import_enabled: true,
            advanced_open: false,
            reveal_details: false,
            search,
            query: String::new(),
            _search_subscription: subscription,
        }
    }

    fn entries_to_import(&self) -> Vec<SshConfigEntry> {
        if !self.import_enabled {
            return Vec::new();
        }
        self.entries
            .iter()
            .filter(|entry| self.selected.contains(&entry.alias))
            .cloned()
            .collect()
    }

    fn toggle_selected(&mut self, alias: &str, cx: &mut Context<Self>) {
        if !self.selected.remove(alias) {
            self.selected.insert(alias.to_owned());
        }
        cx.notify();
    }
}

impl Render for HostsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let total = self.rows.len();

        let toggle = switch("onboarding-hosts-import", c, self.import_enabled).on_click(
            cx.listener(|this, on: &bool, _window, cx| {
                this.import_enabled = *on;
                cx.notify();
            }),
        );
        let toggle_row = settings_group("onboarding-hosts", c).row(
            settings_row(
                "onboarding-hosts-import-row",
                c,
                "Import hosts from ~/.ssh/config",
            )
            .description(format!("{total} host(s) found."))
            .control(toggle),
        );

        let mut body = div().flex().flex_col().gap(c.space(12.0)).child(toggle_row);

        if !self.import_enabled {
            return body
                .child(
                    div()
                        .text_sm()
                        .text_color(c.muted)
                        .child("You can import hosts later in Settings \u{203a} Hosts."),
                )
                .into_any_element();
        }

        let advanced = disclosure(
            "onboarding-hosts-advanced",
            "Advanced",
            !self.advanced_open,
            c.muted,
            c.fg,
        )
        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
            this.advanced_open = !this.advanced_open;
            cx.notify();
        }));
        body = body.child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(advanced)
                .child(
                    div()
                        .text_sm()
                        .text_color(c.muted)
                        .child(format!("{} of {total} selected", self.selected.len())),
                ),
        );

        if self.advanced_open {
            body = body.child(self.render_advanced(c, window, cx));
        }
        body.into_any_element()
    }
}

impl HostsView {
    fn render_advanced(
        &self,
        c: Palette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let focused = self.search.focus_handle(cx).is_focused(window);
        let search = search_field(
            "onboarding-hosts-search",
            c,
            focused,
            search_input(&self.search, c),
            None,
        );

        let reveal = self.reveal_details;
        let reveal_button = button(
            "onboarding-hosts-reveal",
            c,
            ButtonVariant::Subtle,
            ButtonSize::Xs,
        )
        .child(if reveal {
            "Hide details"
        } else {
            "Show details"
        })
        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
            this.reveal_details = !this.reveal_details;
            cx.notify();
        }));
        let select_all = button(
            "onboarding-hosts-all",
            c,
            ButtonVariant::Subtle,
            ButtonSize::Xs,
        )
        .child("Select all")
        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
            this.selected = this.rows.iter().map(|row| row.alias.clone()).collect();
            cx.notify();
        }));
        let select_none = button(
            "onboarding-hosts-none",
            c,
            ButtonVariant::Subtle,
            ButtonSize::Xs,
        )
        .child("Select none")
        .on_click(cx.listener(|this, _: &ClickEvent, _window, cx| {
            this.selected.clear();
            cx.notify();
        }));

        let visible = filter(&self.rows, &self.query);
        let items: Vec<_> = visible
            .into_iter()
            .map(|index| {
                let row = &self.rows[index];
                let alias = row.alias.clone();
                let checked = self.selected.contains(&alias);
                let subtitle = format!(
                    "{}  \u{00b7}  {}",
                    row.tags().join("  \u{00b7}  "),
                    row.details(reveal)
                );
                ListItem::new(SharedString::from(format!("onboarding-host-{alias}")), c)
                    .child(checkbox("", c, checked).box_only())
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(div().text_sm().text_color(c.fg).child(alias.clone()))
                            .child(div().text_xs().text_color(c.muted).child(subtitle)),
                    )
                    .on_click(cx.listener(move |this, _: &ClickEvent, _window, cx| {
                        this.toggle_selected(&alias, cx);
                    }))
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(c.space(8.0))
            .child(search)
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(c.space(4.0))
                    .child(select_all)
                    .child(select_none)
                    .child(div().flex_1())
                    .child(reveal_button),
            )
            .child(
                // `overflow_y_scrollbar` moves the element's own style onto its
                // wrapper and resets it, so the column lives on the child.
                div()
                    .id("onboarding-hosts-list")
                    .max_h(px(220.0))
                    .overflow_y_scrollbar()
                    .child(div().w_full().flex().flex_col().children(items)),
            )
    }
}
