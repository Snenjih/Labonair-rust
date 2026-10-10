//! The `SettingsView` entity: its state struct, construction, lifecycle,
//! keyboard handling, the page-driven navigation (T19-004: disclosure
//! sections + scroll-spy + sub-pages + custom top-level chrome, replacing the
//! old flat `CATEGORIES` sidebar), and the small render helpers + `Palette`.
//! The large per-pane render code lives in the sibling `panes/*` modules
//! (each a separate `impl SettingsView` block).

pub use gpui::prelude::FluentBuilder;
pub use gpui::{
    canvas, div, list, px, App, AppContext, Bounds, ClickEvent, ClipboardItem, Context, Entity,
    FocusHandle, Focusable, InteractiveElement, IntoElement, KeyDownEvent, ListAlignment,
    ListOffset, ListState, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window,
};
pub use serde_json::Value;
pub use tokio::runtime::Handle as TokioHandle;

pub use labonair_notifications::{notification_center, Notification};
pub use labonair_settings::{enqueue_user_settings_write, Settings as _, SettingsStore};
pub use labonair_theme::ThemeStore;
pub use labonair_ui_kit::{
    button, checkbox, h_stack, icon_button_builder, keybinding_hint, number_field,
    search_clear_button, search_field, search_input, segmented_control, select_popover,
    select_trigger, select_trigger_disabled, text_field, text_field_surface, text_input, tree_row,
    v_stack, ButtonSize, ButtonVariant, IconButtonShape, IconName, InputEvent, InputState, Palette,
    SegmentSize, SegmentVariant, SelectOption, SelectPopoverAnchor, Switch, TextFieldState,
    TreeRowState, DISABLED_OPACITY,
};

pub(crate) use crate::apply::*;
pub(crate) use crate::pages::*;
pub(crate) use crate::schema::*;
pub(crate) use crate::search::{SearchIndex, SearchRow, SearchTarget};
pub(crate) use crate::services::{SettingsFileTarget, SettingsServices, SettingsSurfaceId};
pub(crate) use crate::window::*;

use std::collections::{HashMap, HashSet};

const SELECT_MENU_PAGE_SIZE: usize = 10;

fn select_menu_next_index(current: usize, len: usize, key: &str) -> Option<usize> {
    let last = len.checked_sub(1)?;
    Some(match key {
        "up" => {
            if current == 0 {
                last
            } else {
                current - 1
            }
        }
        "down" => {
            if current >= last {
                0
            } else {
                current + 1
            }
        }
        "home" => 0,
        "end" => last,
        "pageup" => current.saturating_sub(SELECT_MENU_PAGE_SIZE),
        "pagedown" => current.saturating_add(SELECT_MENU_PAGE_SIZE).min(last),
        _ => return None,
    })
}

/// Which Settings layer supplies a field's effective value. A thin display-
/// only mirror of `labonair_settings::SettingsLayer` — kept separate so this
/// crate never has to match on `SettingsLayer::Project(WorktreeId)` or other
/// payloads just to show where an override came from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SettingSource {
    Default,
    User,
    Project,
}

impl SettingSource {
    pub(crate) fn modified_in(self) -> Option<&'static str> {
        match self {
            Self::Default => None,
            Self::User => Some("user settings"),
            Self::Project => Some("project settings"),
        }
    }
}

/// The file a Settings UI edit targets. This is deliberately a scope selector,
/// not a second category system: the Settings owner still owns the seven value
/// areas and project writes are checked against its whitelist.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SettingsScope {
    #[default]
    User,
    Project,
}

impl SettingsScope {
    pub(crate) fn token(self) -> &'static str {
        match self {
            SettingsScope::User => "user",
            SettingsScope::Project => "project",
        }
    }

    pub(crate) fn from_token(token: &str) -> Option<Self> {
        match token {
            "user" => Some(Self::User),
            "project" => Some(Self::Project),
            _ => None,
        }
    }

    fn file_target(self) -> SettingsFileTarget {
        match self {
            SettingsScope::User => SettingsFileTarget::User,
            SettingsScope::Project => SettingsFileTarget::Project,
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub(crate) struct NavigationTarget {
    area: usize,
    section: Option<&'static str>,
}

impl NavigationTarget {
    fn area(area: usize) -> Self {
        Self {
            area,
            section: None,
        }
    }

    fn section(area: usize, section: &'static str) -> Self {
        Self {
            area,
            section: Some(section),
        }
    }
}

pub struct SettingsView {
    pub(crate) theme: Entity<ThemeStore>,
    pub(crate) services: SettingsServices,
    pub(crate) tokio: TokioHandle,
    pub(crate) open: bool,
    /// Index into `self.pages` — the active visible Settings category.
    pub(crate) active_area: usize,
    /// Index into `self.pages[active_area].sub_pages`, when a `SubPageLink`
    /// has been followed (rule 1).
    pub(crate) active_subpage: Option<usize>,
    /// Explicit persistence target shown in the header.
    pub(crate) scope: SettingsScope,
    pub(crate) search: String,
    /// The real UI-kit search input. `search` remains the view-level query so
    /// keyboard navigation and the fuzzy index keep their existing contract.
    pub(crate) search_input: Option<Entity<InputState>>,
    pub(crate) search_input_focused: bool,
    pub(crate) _search_input_subscription: Option<Subscription>,
    /// Native editors for every text setting. Entities stay stable while the
    /// virtualized content list mounts/unmounts rows, so tab focus, selection,
    /// clipboard, IME, and undo/redo do not depend on a row being rebuilt.
    pub(crate) text_inputs: HashMap<&'static str, Entity<InputState>>,
    pub(crate) text_input_key: Option<&'static str>,
    pub(crate) _text_input_subscriptions: Vec<Subscription>,
    /// Native editor for the one active numeric value, matching the text
    /// field lifecycle while keeping ordinary rows lightweight.
    pub(crate) number_input: Option<Entity<InputState>>,
    pub(crate) number_input_key: Option<&'static str>,
    pub(crate) _number_input_subscription: Option<Subscription>,
    /// `true` when this view is the root of its own OS window (T16-009); `false`
    /// for the legacy in-`AppShell` modal path (kept for tests only).
    pub(crate) windowed: bool,
    /// An open `Select` dropdown (json_path + anchor position + options),
    /// drawn as a deferred floating layer so it escapes the scroll clip.
    pub(crate) dropdown: Option<SelectMenu>,
    /// Virtualized options for the active select/font picker.
    pub(crate) select_list: ListState,
    /// Each select trigger's window-space bounds from the last paint, keyed by
    /// `json_path`. The open dropdown anchors to `bounds.bottom_left()` so it
    /// drops from the trigger, not from wherever inside it the click landed.
    pub(crate) select_bounds: HashMap<&'static str, Bounds<Pixels>>,
    /// Scanned system font family names for the `FontFamily` picker, loaded
    /// once asynchronously when the window opens.
    pub(crate) system_fonts: Vec<SharedString>,
    pub(crate) font_loading: bool,
    pub(crate) font_loaded: bool,
    pub(crate) font_error: Option<String>,
    pub(crate) focus: FocusHandle,
    // ── T19-004: generated settings UI ──────────────────────────────────
    /// Every generated field (`crate::schema::all_fields()`), computed once.
    pub(crate) all_fields: Vec<AnyField>,
    /// Visible field and capability pages composed from the owner registry.
    pub(crate) pages: Vec<SettingsPage>,
    /// Top-level sidebar rows whose sub-section list is expanded
    /// (`docs/architecture.md` §8.3 deviation). Toggled only by the row's
    /// disclosure chevron; selecting a category does not expand it.
    pub(crate) expanded_areas: HashSet<usize>,
    /// Stable focus handles for visible category and section rows. These
    /// allow tree-style arrow navigation to move native focus between rows.
    pub(crate) navigation_focus_handles: HashMap<NavigationTarget, FocusHandle>,
    /// A section label the sidebar asked to scroll the content area to,
    /// consumed by `render_generated_body` once the section rows are built.
    pub(crate) scroll_to_section: Option<&'static str>,
    /// Scroll position of the active generated page's content, tracked so
    /// the jump bar can scroll-to-section and highlight the section that is
    /// currently topmost (rule 1's scroll-spy).
    /// State for the virtualized variable-height content list.
    pub(crate) content_list: ListState,
    pub(crate) content_list_key: Option<String>,
    // ── T19-007: global settings search ─────────────────────────────────
    /// Every SettingsContent field, indexed once (`open()`) — never rebuilt
    /// per keystroke (task Warnung).
    pub(crate) search_index: SearchIndex,
    /// The current query's scored, category-grouped hits — recomputed
    /// whenever `search` changes, cached so `on_key`'s Up/Down/Enter can act
    /// on the same list the sidebar is showing.
    pub(crate) search_results: Vec<SearchRow>,
    /// Index into `search_results` for keyboard navigation.
    pub(crate) search_selected: usize,
    /// The setting path whose Labonair deep-link was most recently copied.
    pub(crate) last_copied_link_path: Option<&'static str>,
    /// A field's `json_path` currently pulsing (jumped-to via search),
    /// cleared by a short timer.
    pub(crate) highlight: Option<&'static str>,
    /// Bumped on every `set_highlight` call so a stale timer from an earlier
    /// jump can't clear a highlight set by a later one.
    pub(crate) highlight_token: u64,
    /// A field's `json_path` to scroll to once its (now-current) page has
    /// rendered its rows — set by a search jump, consumed by
    /// `render_generated_body`.
    pub(crate) pending_scroll: Option<&'static str>,
    /// Fingerprint of the settings diagnostics most recently published to the
    /// app-wide notification registry. This prevents watcher-triggered
    /// repaints from creating duplicate notifications while still allowing a
    /// changed problem to be reported again.
    pub(crate) published_diagnostics: Option<String>,
}

pub(crate) struct SelectMenu {
    pub(crate) key: &'static str,
    /// `(serialized token to store, human label to show)`.
    pub(crate) options: Vec<(SharedString, SharedString)>,
    pub(crate) at: Point<Pixels>,
    /// The `"(default)"` font entry — selecting it clears the pref to `""`.
    pub(crate) default_sentinel: Option<SharedString>,
    /// Keyboard highlight inside the open options list.
    pub(crate) highlighted: usize,
}

impl SettingsView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        theme: Entity<ThemeStore>,
        services: SettingsServices,
        tokio: TokioHandle,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        // The layered `SettingsStore` (T19-002/003) notifies on every write —
        // including ones this window did not make itself (e.g. a project
        // `.labonair/settings.json` edit) — so override sources / values stay
        // live without a bespoke observer list.
        if cx.has_global::<SettingsStore>() {
            cx.observe_global::<SettingsStore>(|this, cx| {
                this.publish_settings_diagnostics(cx);
                cx.notify();
            })
            .detach();
        }
        // Deep-link: jump to the requested area/section slug when another
        // part of the app asks for one while this window is open.
        cx.observe_global::<SettingsTarget>(|this, cx| {
            match cx
                .try_global::<SettingsTarget>()
                .cloned()
                .unwrap_or_default()
            {
                SettingsTarget::Home => {}
                SettingsTarget::Page(slug) => {
                    this.navigate_to_slug(slug);
                    this.search.clear();
                    cx.notify();
                }
                SettingsTarget::Field(path) => {
                    this.navigate_to_json_path(&path, cx);
                }
            }
        })
        .detach();
        let all_fields = all_fields();
        let surfaces = services.surfaces();
        let pages = pages(&surfaces);
        let search_index = SearchIndex::build(&all_fields, &pages, &surfaces);
        Self {
            theme,
            services,
            tokio,
            open: false,
            active_area: 0,
            active_subpage: None,
            scope: SettingsScope::User,
            search: String::new(),
            search_input: None,
            search_input_focused: false,
            _search_input_subscription: None,
            text_inputs: HashMap::new(),
            text_input_key: None,
            _text_input_subscriptions: Vec::new(),
            number_input: None,
            number_input_key: None,
            _number_input_subscription: None,
            windowed: false,
            dropdown: None,
            select_list: ListState::new(0, ListAlignment::Top, px(320.0)),
            select_bounds: HashMap::new(),
            system_fonts: Vec::new(),
            font_loading: false,
            font_loaded: false,
            font_error: None,
            focus: cx.focus_handle(),
            all_fields,
            pages,
            // The active category is visible immediately; other categories are
            // collapsed until disclosed, matching the dense tree navigation.
            expanded_areas: HashSet::new(),
            navigation_focus_handles: HashMap::new(),
            scroll_to_section: None,
            content_list: ListState::new(0, ListAlignment::Top, px(120.0)),
            content_list_key: None,
            search_index,
            search_results: Vec::new(),
            search_selected: 0,
            last_copied_link_path: None,
            highlight: None,
            highlight_token: 0,
            pending_scroll: None,
            published_diagnostics: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.cancel_text_input(cx);
        self.cancel_number_input(cx);
        self.scope = SettingsScope::User;
        self.search.clear();
        self.search_results.clear();
        self.search_selected = 0;
        self.highlight = None;
        self.pending_scroll = None;
        window.focus(&self.focus);
        self.publish_settings_diagnostics(cx);
        self.load_system_fonts(cx);
        cx.notify();
    }

    pub fn close(&mut self, cx: &mut Context<Self>) {
        self.open = false;
        self.cancel_text_input(cx);
        self.cancel_number_input(cx);
        cx.notify();
    }

    /// Close request from Esc / the header close button. In windowed mode this
    /// destroys the OS window (GPUI 0.2.2 has no per-window hide); the shared
    /// [`PreferencesStore`] keeps all persistent state so the next open is
    /// instant and lossless.
    pub(crate) fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.windowed {
            cx.set_global(SettingsWindowRef { handle: None });
            self.cancel_text_input(cx);
            self.cancel_number_input(cx);
            window.remove_window();
        } else {
            self.close(cx);
        }
    }

    pub fn toggle(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.open {
            self.close(cx);
        } else {
            self.open(window, cx);
        }
    }

    pub(crate) fn notify(&self, cx: &mut Context<Self>, n: Notification) {
        notification_center(cx).update(cx, |c, cx| {
            c.push(n, cx);
        });
    }

    pub(crate) fn notify_error(&self, cx: &mut Context<Self>, title: &'static str, body: String) {
        self.notify(cx, Notification::error(title, body));
    }

    /// Publish settings-file diagnostics through the app-wide notification
    /// registry. Settings owns parsing and validation, but it must not render
    /// a second, settings-specific error surface: the statusbar notification
    /// dropdown is the single user-facing destination for these findings.
    pub(crate) fn publish_settings_diagnostics(&mut self, cx: &mut Context<Self>) {
        let (fingerprint, notifications) = {
            let Some(store) = cx.try_global::<SettingsStore>() else {
                return;
            };
            let mut fingerprint = String::new();
            let mut notifications = Vec::new();

            if let Some(error) = store.user_json_error() {
                let details = error.to_string();
                fingerprint.push_str("user-json:");
                fingerprint.push_str(&details);
                notifications.push(
                    Notification::error(
                        "Settings file syntax error",
                        "config.json could not be parsed. Fix the file before editing settings.",
                    )
                    .source("settings")
                    .details(details),
                );
            }

            if let Some(error) = store.project_json_error() {
                let details = error.to_string();
                fingerprint.push_str("project-json:");
                fingerprint.push_str(&details);
                notifications.push(
                    Notification::error(
                        "Project settings syntax error",
                        "The project settings file could not be parsed. Fix it before editing project settings.",
                    )
                    .source("settings")
                    .details(details),
                );
            }

            for error in store.parse_errors() {
                let details = error.message.clone();
                fingerprint.push_str("parse:");
                fingerprint.push_str(error.area);
                fingerprint.push_str(&details);
                notifications.push(
                    Notification::error(
                        "Invalid settings section",
                        format!(
                            "The `{}` settings section is using its defaults.",
                            error.area
                        ),
                    )
                    .source("settings")
                    .details(details),
                );
            }

            for (layer, errors) in [
                ("user", store.schema_errors()),
                ("project", store.project_schema_errors()),
            ] {
                for error in errors {
                    let path = if error.json_path.is_empty() {
                        "the settings file"
                    } else {
                        error.json_path.as_str()
                    };
                    let details = error.to_string();
                    fingerprint.push_str(layer);
                    fingerprint.push_str(":schema:");
                    fingerprint.push_str(&details);
                    notifications.push(
                        Notification::error(
                            "Invalid setting",
                            format!("The {layer} setting `{path}` is using its default."),
                        )
                        .source("settings")
                        .details(details),
                    );
                }
            }

            for (layer, warnings) in [
                ("user", store.schema_warnings()),
                ("project", store.project_schema_warnings()),
            ] {
                for warning in warnings {
                    let path = if warning.json_path.is_empty() {
                        "the settings file"
                    } else {
                        warning.json_path.as_str()
                    };
                    let details = warning.to_string();
                    fingerprint.push_str(layer);
                    fingerprint.push_str(":warning:");
                    fingerprint.push_str(&details);
                    notifications.push(
                        Notification::warning(
                            "Unknown settings key",
                            format!("The {layer} key `{path}` was ignored."),
                        )
                        .source("settings")
                        .details(details),
                    );
                }
            }

            for key in store.project_rejected_keys() {
                fingerprint.push_str("project-rejected:");
                fingerprint.push_str(key);
                notifications.push(
                    Notification::warning(
                        "Project setting ignored",
                        format!("The project key `{key}` is not allowed."),
                    )
                    .source("settings")
                    .details("Project settings are limited to safe, non-network values."),
                );
            }

            (fingerprint, notifications)
        };

        if self.published_diagnostics.as_deref() == Some(fingerprint.as_str()) {
            return;
        }
        self.published_diagnostics = (!fingerprint.is_empty()).then_some(fingerprint);
        if notifications.is_empty() {
            return;
        }

        notification_center(cx).update(cx, |center, cx| {
            for notification in notifications {
                center.push(notification, cx);
            }
        });
    }

    // ── navigation (T19-004) ────────────────────────────────────────────

    /// Resolve a deep-link slug (`"terminal"`, `"terminal/advanced"`) to an
    /// area + optional sub-page and navigate there (rule 7).
    pub(crate) fn navigate_to_slug(&mut self, slug: &str) {
        let Some((area_idx, sub_idx)) = crate::pages::resolve_slug(&self.pages, slug) else {
            return;
        };
        self.active_area = area_idx;
        self.active_subpage = sub_idx;
        self.expanded_areas.insert(area_idx);
    }

    /// The active page's own body (main page, or the followed sub-page).
    pub(crate) fn active_body(&self) -> &PageBody {
        match self.active_subpage {
            Some(i) => &self.pages[self.active_area].sub_pages[i].body,
            None => &self.pages[self.active_area].body,
        }
    }

    pub(crate) fn go_to_area(&mut self, i: usize, cx: &mut Context<Self>) {
        self.active_area = i;
        self.active_subpage = None;
        // Root activation selects the page; only its disclosure control opens
        // the section anchors, as in the Settings navigation contract.
        self.search.clear();
        cx.notify();
    }

    /// Toggle a sidebar row's sub-section list without navigating
    /// (`docs/architecture.md` §8.3).
    pub(crate) fn toggle_area_expanded(&mut self, i: usize, cx: &mut Context<Self>) {
        if !self.expanded_areas.remove(&i) {
            self.expanded_areas.insert(i);
        }
        cx.notify();
    }

    fn navigation_focus_handle(
        &mut self,
        target: NavigationTarget,
        cx: &mut Context<Self>,
    ) -> FocusHandle {
        self.navigation_focus_handles
            .entry(target)
            .or_insert_with(|| cx.focus_handle())
            .clone()
    }

    fn visible_navigation_targets(&self) -> Vec<NavigationTarget> {
        let mut targets = Vec::new();
        for page in 0..self.pages.len() {
            targets.push(NavigationTarget::area(page));
            if self.expanded_areas.contains(&page) {
                targets.extend(
                    self.section_labels_for_area(page)
                        .into_iter()
                        .map(|section| NavigationTarget::section(page, section)),
                );
            }
        }
        targets
    }

    /// Handle the keyboard contract of a focused settings tree row. GPUI
    /// 0.2.2 does not expose ARIA tree roles, so the native fallback keeps the
    /// important behavior explicit: arrows move focus, Enter/Space activate,
    /// and Left/Right controls disclosure.
    pub(crate) fn handle_navigation_key(
        &mut self,
        target: NavigationTarget,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match (target.section, key) {
            (Some(label), "enter" | "space") => self.go_to_section(target.area, label, cx),
            (None, "enter" | "space") => self.go_to_area(target.area, cx),
            (None, "right") if !self.expanded_areas.contains(&target.area) => {
                self.expanded_areas.insert(target.area);
                cx.notify();
            }
            (None, "right") => {
                if let Some(section) = self.section_labels_for_area(target.area).first().copied() {
                    let focus = self.navigation_focus_handle(
                        NavigationTarget::section(target.area, section),
                        cx,
                    );
                    window.focus(&focus);
                }
            }
            (None, "left") if self.expanded_areas.contains(&target.area) => {
                self.expanded_areas.remove(&target.area);
                cx.notify();
            }
            (Some(_), "left") => {
                self.expanded_areas.remove(&target.area);
                let focus = self.navigation_focus_handle(NavigationTarget::area(target.area), cx);
                window.focus(&focus);
                cx.notify();
            }
            (_, "down" | "up") => {
                let targets = self.visible_navigation_targets();
                if let Some(index) = targets.iter().position(|candidate| *candidate == target) {
                    let next = if key == "down" {
                        (index + 1).min(targets.len().saturating_sub(1))
                    } else {
                        index.saturating_sub(1)
                    };
                    if let Some(target) = targets.get(next).copied() {
                        let focus = self.navigation_focus_handle(target, cx);
                        window.focus(&focus);
                    }
                }
            }
            _ => return false,
        }
        true
    }

    /// Navigate to `area` (if needed) and scroll its content to `label`'s
    /// section (`docs/architecture.md` §8.3 — sub-level sidebar entries are
    /// scroll anchors, not pages).
    pub(crate) fn go_to_section(
        &mut self,
        area: usize,
        label: &'static str,
        cx: &mut Context<Self>,
    ) {
        self.active_area = area;
        self.active_subpage = None;
        self.expanded_areas.insert(area);
        self.search.clear();
        self.scroll_to_section = Some(label);
        cx.notify();
    }

    /// Section labels shown under a top-level sidebar row: the curated
    /// section headers of the area's generated main page, plus a trailing
    /// "Other" when leftover fields exist.
    pub(crate) fn section_labels_for_area(&self, area_idx: usize) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        if let Some(page) = self.pages.get(area_idx) {
            let PageBody::Generated(items) = &page.body;
            for item in items {
                if let SettingsPageItem::SectionHeader(label) = item {
                    out.push(label);
                }
            }
        }
        if matches!(
            self.pages.get(area_idx).map(|p| &p.body),
            Some(PageBody::Generated(_))
        ) && !crate::pages::leftover_fields(area_idx, &self.pages, &self.all_fields).is_empty()
        {
            out.push("Other");
        }
        out
    }

    pub(crate) fn go_back_to_main_page(&mut self, cx: &mut Context<Self>) {
        self.active_subpage = None;
        cx.notify();
    }

    pub(crate) fn set_scope(&mut self, scope: SettingsScope, cx: &mut Context<Self>) {
        if scope == SettingsScope::Project
            && cx
                .try_global::<SettingsStore>()
                .and_then(|store| store.project_root())
                .is_none()
        {
            self.notify_error(
                cx,
                "Project settings unavailable",
                "Open a project before editing its settings.".to_string(),
            );
            return;
        }
        if scope == self.scope {
            return;
        }
        // Commit active editors before changing the write target. A keystroke
        // entered in User scope must never be saved into Project scope merely
        // because the user changed the selector before the blur event arrived.
        self.commit_active_text_input(cx);
        if let (Some(key), Some(input)) = (self.number_input_key, self.number_input.clone()) {
            self.commit_number_input(key, input.read(cx).value().to_string(), cx);
        }
        self.scope = scope;
        self.dropdown = None;
        cx.notify();
    }

    // ── global search (T19-007) ─────────────────────────────────────────

    /// Recompute `search_results` from the current query — cheap (index is
    /// ~200 entries), called every render so keyboard/mouse selection always
    /// acts on what's on screen. A no-op (empty results) when the query is
    /// empty, which is also how the sidebar knows to fall back to the normal
    /// category list.
    pub(crate) fn refresh_search_results(&mut self) {
        self.search_results = crate::search::search(&self.search_index, &self.search, 50);
        for row in &self.search_results {
            self.expanded_areas.insert(row.page_index);
        }
        if self.search_selected >= self.search_results.len() {
            self.search_selected = self.search_results.len().saturating_sub(1);
        }
    }

    /// Enter/click on a search result: navigate to its category (+ sub-page),
    /// clear the query, and schedule a scroll-to + highlight pulse once the
    /// target page has rendered (`render_generated_body` consumes
    /// `pending_scroll`).
    pub(crate) fn activate_search_hit(&mut self, target: SearchTarget, cx: &mut Context<Self>) {
        match target {
            SearchTarget::Field(idx) => {
                let Some(field) = self.all_fields.get(idx).copied() else {
                    return;
                };
                let Some(location) =
                    crate::pages::section_label_for_field(&self.pages, field.json_path)
                        .or_else(|| crate::pages::field_location(&self.pages, &field))
                else {
                    return;
                };
                self.active_area = location.page_index;
                self.active_subpage = location.sub_page_slug.and_then(|slug| {
                    self.pages[location.page_index]
                        .sub_pages
                        .iter()
                        .position(|page| page.slug == slug)
                });
                self.expanded_areas.insert(location.page_index);
                self.scroll_to_section = location.section;
                self.pending_scroll = Some(field.json_path);
                self.set_highlight(field.json_path, cx);
            }
            SearchTarget::OwnerSurface(surface) => {
                if let Err(error) = self.services.open_surface(surface, cx) {
                    tracing::error!("could not open Settings surface: {error}");
                }
            }
        }
        self.search.clear();
        self.search_results.clear();
        self.search_selected = 0;
        cx.notify();
    }

    pub(crate) fn navigate_to_json_path(&mut self, path: &str, cx: &mut Context<Self>) {
        let Some(index) = self
            .all_fields
            .iter()
            .position(|field| field.json_path == path)
        else {
            return;
        };
        self.activate_search_hit(SearchTarget::Field(index), cx);
    }

    pub(crate) fn copy_setting_link(&mut self, path: &'static str, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(format!(
            "labonair://settings/{path}"
        )));
        self.last_copied_link_path = Some(path);
        cx.notify();
    }

    /// Pulse `json_path`'s row for ~1s (task step 4). `highlight_token`
    /// guards against a stale timer from an earlier jump clearing a
    /// highlight set by a later one.
    pub(crate) fn set_highlight(&mut self, json_path: &'static str, cx: &mut Context<Self>) {
        self.highlight_token = self.highlight_token.wrapping_add(1);
        let token = self.highlight_token;
        self.highlight = Some(json_path);
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_millis(1000))
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.highlight_token == token {
                    this.highlight = None;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    // ── generic field read/write (T19-004) ──────────────────────────────

    pub(crate) fn field_by_path(&self, json_path: &str) -> Option<&AnyField> {
        self.all_fields.iter().find(|f| f.json_path == json_path)
    }

    /// The field's effective (merged) value, `None` if the layered store
    /// isn't published yet (headless/tests without `labonair_settings::init`).
    pub(crate) fn field_value(&self, field: &AnyField, cx: &App) -> Option<Value> {
        let store = cx.try_global::<SettingsStore>()?;
        (field.get)(store.merged())
    }

    /// Which layer supplies `field`'s effective value (rule 5).
    pub(crate) fn field_source(&self, field: &AnyField, cx: &App) -> SettingSource {
        match cx.try_global::<SettingsStore>() {
            None => SettingSource::Default,
            Some(store) => match store.source_of(field.json_path) {
                labonair_settings::SettingsLayer::Default => SettingSource::Default,
                labonair_settings::SettingsLayer::Project(_) => SettingSource::Project,
                _ => SettingSource::User,
            },
        }
    }

    /// Write a generated field's value through the explicitly selected layer.
    /// The Settings owner validates project scope again, so the UI never
    /// becomes the security boundary for `.labonair/settings.json`.
    pub(crate) fn set_field_value(
        &mut self,
        json_path: &'static str,
        value: Value,
        cx: &mut Context<Self>,
    ) {
        let Some(field) = self.field_by_path(json_path) else {
            return;
        };
        let set = field.set;
        if !cx.has_global::<SettingsStore>() {
            return;
        }
        let scope = self.scope;
        let result = match scope {
            SettingsScope::User => cx
                .global_mut::<SettingsStore>()
                .update_user_settings_deferred(move |c| {
                    (set)(c, value.clone());
                }),
            SettingsScope::Project => {
                if !labonair_settings::project::is_project_setting_allowed(json_path) {
                    Err(format!(
                        "`{json_path}` is not available in project settings"
                    ))
                } else {
                    cx.global_mut::<SettingsStore>()
                        .update_project_settings_deferred(move |c| {
                            (set)(c, value.clone());
                        })
                }
            }
        };
        match result {
            Err(err) => {
                self.notify_error(cx, "Could not save setting", err);
                return;
            }
            Ok(Some(request)) => self.queue_settings_write(request, cx),
            Ok(None) => {}
        }
        self.sync_theme_from_prefs(cx);
        cx.notify();
    }

    /// Clear the layer that currently supplies the field. Reset therefore
    /// reveals the next lower-precedence value instead of materializing a
    /// duplicate default in a higher layer.
    pub(crate) fn reset_field(&mut self, json_path: &'static str, cx: &mut Context<Self>) {
        let Some(field) = self.field_by_path(json_path) else {
            return;
        };
        if !cx.has_global::<SettingsStore>() {
            return;
        }
        let clear = field.clear;
        let scope = match self.field_source(field, cx) {
            SettingSource::Project => SettingsScope::Project,
            SettingSource::User => SettingsScope::User,
            SettingSource::Default => self.scope,
        };
        let result = match scope {
            SettingsScope::User => cx
                .global_mut::<SettingsStore>()
                .update_user_settings_deferred(clear),
            SettingsScope::Project => {
                if !labonair_settings::project::is_project_setting_allowed(json_path) {
                    self.notify_error(
                        cx,
                        "Could not reset setting",
                        format!("`{json_path}` is not available in project settings"),
                    );
                    return;
                }
                cx.global_mut::<SettingsStore>()
                    .update_project_settings_deferred(clear)
            }
        };
        match result {
            Ok(Some(request)) => self.queue_settings_write(request, cx),
            Ok(None) => {}
            Err(error) => self.notify_error(cx, "Could not reset setting", error),
        }
        self.sync_theme_from_prefs(cx);
        cx.notify();
    }

    fn queue_settings_write(
        &self,
        request: labonair_settings::UserSettingsWrite,
        cx: &mut Context<Self>,
    ) {
        match enqueue_user_settings_write(request) {
            Ok(receiver) => {
                cx.spawn(async move |this, cx| {
                    let result = cx
                        .background_executor()
                        .spawn(async move {
                            receiver.recv().unwrap_or_else(|_| {
                                Err("settings writer stopped before saving".to_string())
                            })
                        })
                        .await;
                    if let Err(error) = result {
                        let _ = this.update(cx, |this, cx| {
                            this.notify_error(cx, "Could not save setting", error);
                        });
                    }
                })
                .detach();
            }
            Err(error) => self.notify_error(cx, "Could not save setting", error),
        }
    }

    /// Re-derive the [`ThemeStore`] state (color mode, fonts, metrics, active
    /// theme + variant, icon theme) from the layered settings. The policy is
    /// owned by `labonair-theme-ui`; the Settings UI only asks for the refresh
    /// after it writes a value.
    pub(crate) fn sync_theme_from_prefs(&mut self, cx: &mut Context<Self>) {
        let theme = self.theme.clone();
        labonair_theme_ui::apply_prefs_to_theme(&theme, cx);
    }

    pub(crate) fn toggle_bool(&mut self, field: &AnyField, cx: &mut Context<Self>) {
        let cur = self
            .field_value(field, cx)
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        self.set_field_value(field.json_path, Value::Bool(!cur), cx);
    }

    /// Write an already-clamped `f64` (as produced by the shared
    /// [`labonair_ui_kit::NumberField`]) as a JSON number. T20-001 moved the
    /// clamping itself into the primitive, so this is only the store write.
    pub(crate) fn set_float_field(
        &mut self,
        json_path: &'static str,
        value: f64,
        cx: &mut Context<Self>,
    ) {
        let n = serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0));
        self.set_field_value(json_path, Value::Number(n), cx);
    }

    /// Create the real search input once the native window has a `Window`.
    /// `InputState` owns selection, clipboard, IME, and undo/redo behavior;
    /// the view still owns the query and fuzzy-result state.
    pub(crate) fn ensure_search_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.search_input.is_some() {
            return;
        }
        let input = cx.new(|cx| text_field(window, cx).placeholder("Search settings…"));
        let subscription =
            cx.subscribe(&input, |this, input, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    this.search = input.read(cx).value().to_string();
                    this.search_selected = 0;
                    this.refresh_search_results();
                    cx.notify();
                }
                InputEvent::Focus => {
                    this.search_input_focused = true;
                    cx.notify();
                }
                InputEvent::Blur => {
                    this.search_input_focused = false;
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    if let Some(row) = this.search_results.get(this.search_selected).copied() {
                        this.activate_search_hit(row.target, cx);
                        cx.stop_propagation();
                    }
                }
            });
        self.search_input = Some(input);
        self._search_input_subscription = Some(subscription);
    }

    /// Keep the view-level query and the visible `InputState` synchronized
    /// after keyboard navigation or a programmatic search clear.
    pub(crate) fn sync_search_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(input) = self.search_input.clone() else {
            return;
        };
        let current = input.read(cx).value().to_string();
        if current == self.search {
            return;
        }
        let query = self.search.clone();
        input.update(cx, |state, cx| state.set_value(query, window, cx));
    }

    /// Create one native UI-kit input per text setting. The entities are
    /// intentionally independent of the virtualized row tree: a row may be
    /// rebuilt as the user scrolls without losing its editing identity.
    pub(crate) fn ensure_text_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let fields: Vec<(&'static str, String)> = self
            .all_fields
            .iter()
            .filter(|field| matches!(field.control, FieldControl::Text))
            .map(|field| {
                let value = self
                    .field_value(field, cx)
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default();
                (field.json_path, value)
            })
            .collect();

        for (key, initial) in fields {
            if self.text_inputs.contains_key(key) {
                continue;
            }
            let input = cx.new(|cx| {
                let mut state = text_field(window, cx).placeholder("(default)");
                if !initial.is_empty() {
                    state.set_value(initial, window, cx);
                }
                state
            });
            let subscription =
                cx.subscribe(
                    &input,
                    move |this, input, event: &InputEvent, cx| match event {
                        InputEvent::Focus => {
                            if this.text_input_key != Some(key) {
                                this.commit_active_text_input(cx);
                                this.text_input_key = Some(key);
                            }
                            cx.notify();
                        }
                        InputEvent::PressEnter { .. } => {
                            this.commit_text_input(key, input.read(cx).value().to_string(), cx);
                            cx.stop_propagation();
                        }
                        InputEvent::Blur => {
                            this.commit_text_input(key, input.read(cx).value().to_string(), cx);
                        }
                        _ => {}
                    },
                );
            self.text_inputs.insert(key, input);
            self._text_input_subscriptions.push(subscription);
        }
    }

    /// Update only non-focused native text editors from the merged settings
    /// tree. A focused editor owns its in-progress text until Enter, Blur, or
    /// Escape, so external store notifications cannot overwrite user input.
    pub(crate) fn sync_text_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let values: Vec<(&'static str, String)> = self
            .all_fields
            .iter()
            .filter(|field| matches!(field.control, FieldControl::Text))
            .map(|field| {
                let value = self
                    .field_value(field, cx)
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_default();
                (field.json_path, value)
            })
            .collect();

        for (key, value) in values {
            if self.text_input_key == Some(key) {
                continue;
            }
            let Some(input) = self.text_inputs.get(key).cloned() else {
                continue;
            };
            if input.read(cx).value() != value {
                input.update(cx, |state, cx| state.set_value(value, window, cx));
            }
        }
    }

    /// Focus an existing native text editor. The editor's Focus event sets the
    /// active key as well, while this method keeps mouse activation immediate.
    pub(crate) fn begin_text_edit(
        &mut self,
        key: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_key == Some(key) {
            if let Some(input) = self.text_inputs.get(key) {
                input.update(cx, |state, cx| state.focus(window, cx));
            }
            return;
        }
        self.cancel_number_input(cx);
        self.commit_active_text_input(cx);
        let Some(input) = self.text_inputs.get(key).cloned() else {
            return;
        };
        self.text_input_key = Some(key);
        input.update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }

    pub(crate) fn commit_active_text_input(&mut self, cx: &mut Context<Self>) {
        let Some(key) = self.text_input_key else {
            return;
        };
        let value = self
            .text_inputs
            .get(key)
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default();
        self.commit_text_input(key, value, cx);
    }

    pub(crate) fn commit_text_input(
        &mut self,
        key: &'static str,
        value: String,
        cx: &mut Context<Self>,
    ) {
        if self.text_input_key != Some(key) {
            return;
        }
        self.text_input_key = None;
        self.set_field_value(key, Value::String(value.trim().to_owned()), cx);
    }

    pub(crate) fn cancel_text_input(&mut self, cx: &mut Context<Self>) {
        if self.text_input_key.is_none() {
            return;
        }
        self.text_input_key = None;
        cx.notify();
    }

    pub(crate) fn begin_number_edit(
        &mut self,
        key: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.number_input_key == Some(key) {
            if let Some(input) = &self.number_input {
                input.update(cx, |state, cx| state.focus(window, cx));
            }
            return;
        }
        let Some(field) = self.field_by_path(key).copied() else {
            return;
        };
        let decimals = match field.control {
            FieldControl::Int { .. } => 0,
            FieldControl::Float { .. } => 2,
            _ => return,
        };
        self.cancel_text_input(cx);
        self.cancel_number_input(cx);
        let initial = self
            .field_value(&field, cx)
            .and_then(|value| value.as_f64())
            .map(|value| format!("{value:.decimals$}"))
            .unwrap_or_default();
        let input = cx.new(|cx| {
            let mut state = text_field(window, cx).placeholder("(default)");
            state.set_value(initial, window, cx);
            state
        });
        let subscription =
            cx.subscribe(
                &input,
                move |this, input, event: &InputEvent, cx| match event {
                    InputEvent::PressEnter { .. } => {
                        this.commit_number_input(key, input.read(cx).value().to_string(), cx);
                        cx.stop_propagation();
                    }
                    InputEvent::Blur => {
                        this.commit_number_input(key, input.read(cx).value().to_string(), cx);
                    }
                    _ => {}
                },
            );
        input.update(cx, |state, cx| state.focus(window, cx));
        self.number_input = Some(input);
        self.number_input_key = Some(key);
        self._number_input_subscription = Some(subscription);
        cx.notify();
    }

    pub(crate) fn commit_number_input(
        &mut self,
        key: &'static str,
        raw: String,
        cx: &mut Context<Self>,
    ) {
        if self.number_input_key != Some(key) {
            return;
        }
        self.number_input = None;
        self.number_input_key = None;
        self._number_input_subscription = None;

        let Some(field) = self.field_by_path(key).copied() else {
            return;
        };
        let Ok(parsed) = raw.trim().parse::<f64>() else {
            self.notify_error(
                cx,
                "Invalid number",
                format!("Enter a numeric value for {}.", field.meta.title),
            );
            cx.notify();
            return;
        };
        let value = match field.control {
            FieldControl::Int { min, max, .. } => {
                Value::from(parsed.round().clamp(min as f64, max as f64) as i64)
            }
            FieldControl::Float {
                min_centi,
                max_centi,
                ..
            } => {
                let value = (parsed.clamp(min_centi as f64 / 100.0, max_centi as f64 / 100.0)
                    * 100.0)
                    .round()
                    / 100.0;
                Value::from(value)
            }
            _ => return,
        };
        self.set_field_value(key, value, cx);
    }

    pub(crate) fn cancel_number_input(&mut self, cx: &mut Context<Self>) {
        if self.number_input_key.is_none() {
            return;
        }
        self.number_input = None;
        self.number_input_key = None;
        self._number_input_subscription = None;
        cx.notify();
    }

    // ── key handling ──────────────────────────────────────────────────────

    pub(crate) fn on_key(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let ks = &ev.keystroke;
        let key = ks.key.as_str();
        if self.dropdown.is_some() {
            match key {
                "escape" => {
                    self.dropdown = None;
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "up" | "down" | "home" | "end" | "pageup" | "pagedown" => {
                    let mut highlighted = None;
                    if let Some(menu) = self.dropdown.as_mut() {
                        if let Some(index) =
                            select_menu_next_index(menu.highlighted, menu.options.len(), key)
                        {
                            menu.highlighted = index;
                            highlighted = Some(index);
                        }
                    }
                    if let Some(highlighted) = highlighted {
                        self.select_list.scroll_to_reveal_item(highlighted);
                    }
                    cx.stop_propagation();
                    cx.notify();
                    return;
                }
                "enter" | "space" => {
                    let Some(menu) = self.dropdown.take() else {
                        return;
                    };
                    let Some((token, _)) = menu.options.get(menu.highlighted).cloned() else {
                        cx.stop_propagation();
                        cx.notify();
                        return;
                    };
                    let value = if menu.default_sentinel.as_ref() == Some(&token) {
                        String::new()
                    } else {
                        token.to_string()
                    };
                    self.set_field_value(menu.key, Value::String(value), cx);
                    cx.stop_propagation();
                    return;
                }
                "tab" => {
                    self.dropdown = None;
                    cx.notify();
                    return;
                }
                _ => {
                    cx.stop_propagation();
                    return;
                }
            }
        }
        // A real text input owns character editing. The view only handles
        // Escape so an active edit can be cancelled without leaking the event
        // into Settings navigation.
        if self.text_input_key.is_some() {
            if key == "escape" {
                self.cancel_text_input(cx);
            }
            cx.stop_propagation();
            return;
        }
        if self.number_input_key.is_some() {
            if key == "escape" {
                self.cancel_number_input(cx);
            }
            cx.stop_propagation();
            return;
        }

        match key {
            // Esc clears an active query first (task step 5); only closes
            // the window once the query is already empty.
            "escape" => {
                if !self.search.is_empty() {
                    self.search.clear();
                    self.search_results.clear();
                    self.search_selected = 0;
                    cx.notify();
                } else {
                    self.request_close(window, cx);
                }
            }
            // The native InputState normally consumes this as an action before
            // this ancestor keydown handler runs. Keep the view-level fallback
            // for the keyboard-first path where the Settings card itself is
            // focused, and accept both deletion key names emitted by GPUI.
            "backspace" | "delete" => {
                self.search.pop();
                self.refresh_search_results();
                cx.notify();
            }
            "down" if !self.search_results.is_empty() => {
                self.search_selected = (self.search_selected + 1) % self.search_results.len();
                cx.notify();
            }
            "up" if !self.search_results.is_empty() => {
                self.search_selected = (self.search_selected + self.search_results.len() - 1)
                    % self.search_results.len();
                cx.notify();
            }
            "enter" => {
                if let Some(row) = self.search_results.get(self.search_selected).copied() {
                    self.activate_search_hit(row.target, cx);
                }
            }
            _ => {
                if self.search_input_focused
                    || ks.modifiers.platform
                    || ks.modifiers.control
                    || ks.modifiers.alt
                {
                    return;
                }
                if let Some(ch) = char_of(ks) {
                    self.search.push_str(&ch);
                    self.refresh_search_results();
                    cx.notify();
                }
            }
        }
        cx.stop_propagation();
    }
}

impl SettingsView {
    /// Render one category tree row plus its optional section scroll anchors.
    /// The settings window uses the shared UI-kit tree primitive so row density,
    /// selection, focus and disclosure affordances remain consistent with the
    /// rest of the native shell.
    fn render_area_navigation(
        &mut self,
        i: usize,
        page_key: &'static str,
        page_title: &'static str,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        let is_active = i == self.active_area;
        let expanded = self.expanded_areas.contains(&i);
        let sections = self.section_labels_for_area(i);
        let has_sections = !sections.is_empty();
        let view = cx.entity();
        let target = NavigationTarget::area(i);
        let focus_handle = self.navigation_focus_handle(target, cx);
        let state = TreeRowState {
            selected: is_active,
            ..Default::default()
        };
        let mut row = tree_row(
            SharedString::from(format!("settings-area-{}", page_key)),
            *c,
            page_title,
        )
        .label_tint(c.sidebar_fg)
        .state(state)
        .tab_index(0)
        .expanded(has_sections.then_some(expanded))
        .chevron(has_sections.then_some(if expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        }))
        .on_click({
            let view = view.clone();
            let focus_handle = focus_handle.clone();
            move |_event, window, app| {
                window.focus(&focus_handle);
                view.update(app, |this, cx| this.go_to_area(i, cx));
            }
        });
        if has_sections {
            row = row.on_chevron_click({
                let view = view.clone();
                move |_event, _window, app| {
                    view.update(app, |this, cx| this.toggle_area_expanded(i, cx));
                }
            });
        }
        row = row.extra({
            let view = view.clone();
            let focus_handle = focus_handle.clone();
            move |element| {
                element.track_focus(&focus_handle).on_key_down(
                    move |event: &KeyDownEvent, window, app| {
                        view.update(app, |this, cx| {
                            if this.handle_navigation_key(
                                target,
                                event.keystroke.key.as_str(),
                                window,
                                cx,
                            ) {
                                cx.stop_propagation();
                            }
                        });
                    },
                )
            }
        });

        let sub = if expanded && has_sections {
            let mut section_rows = Vec::new();
            for label in sections {
                let target = NavigationTarget::section(i, label);
                let focus_handle = self.navigation_focus_handle(target, cx);
                let view = cx.entity();
                let click_view = view.clone();
                section_rows.push(
                    tree_row(
                        SharedString::from(format!("settings-section-{i}-{label}")),
                        *c,
                        label,
                    )
                    .depth(1)
                    .tab_index(0)
                    .state(TreeRowState::default())
                    .on_click({
                        let focus_handle = focus_handle.clone();
                        move |_event, window, app| {
                            window.focus(&focus_handle);
                            click_view.update(app, |this, cx| this.go_to_section(i, label, cx));
                        }
                    })
                    .extra({
                        let view = view.clone();
                        move |element| {
                            element.track_focus(&focus_handle).on_key_down(
                                move |event: &KeyDownEvent, window, app| {
                                    view.update(app, |this, cx| {
                                        if this.handle_navigation_key(
                                            target,
                                            event.keystroke.key.as_str(),
                                            window,
                                            cx,
                                        ) {
                                            cx.stop_propagation();
                                        }
                                    });
                                },
                            )
                        }
                    }),
                );
            }
            Some(
                v_stack()
                    .gap(c.space(1.0))
                    .pb(c.space(4.0))
                    .children(section_rows),
            )
        } else {
            None
        };

        v_stack().child(row).children(sub).into_any_element()
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SettingsView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.windowed && !self.open {
            return div().into_any_element();
        }
        if self.scope == SettingsScope::Project
            && cx
                .try_global::<SettingsStore>()
                .and_then(|store| store.project_root())
                .is_none()
        {
            self.scope = SettingsScope::User;
            self.dropdown = None;
        }
        self.ensure_search_input(window, cx);
        self.sync_search_input(window, cx);
        self.ensure_text_inputs(window, cx);
        self.sync_text_inputs(window, cx);
        let c = Palette::from_theme(self.theme.read(cx));
        let searching = !self.search.trim().is_empty();
        // T19-007: recompute every render so Up/Down/Enter and mouse clicks
        // always act on what's currently on screen (cheap — ~200 entries).
        self.refresh_search_results();

        let search_editor = self
            .search_input
            .as_ref()
            .map(|input| search_input(input, c).into_any_element())
            .unwrap_or_else(|| div().into_any_element());
        let clear_button = if searching {
            Some(
                search_clear_button(
                    "settings-clear-search",
                    c,
                    cx.listener(|this, _: &ClickEvent, _window, cx| {
                        this.search.clear();
                        this.search_results.clear();
                        this.search_selected = 0;
                        cx.notify();
                    }),
                )
                .into_any_element(),
            )
        } else {
            None
        };
        let search_box = search_field(
            "settings-search",
            c,
            self.search_input_focused,
            search_editor,
            clear_button,
        )
        // The sidebar's 2-unit flex gap contributes to this separation, so a
        // 10-unit explicit margin keeps the total at 12 units.
        .mb(c.space(10.0));

        // Categories with no owner-backed values remain in the parity
        // crosswalk until their capability provides a real Settings surface.
        let sidebar_pages: Vec<_> = self
            .pages
            .iter()
            .map(|page| (page.key, page.title))
            .collect();
        let sidebar_items: Vec<_> = sidebar_pages
            .into_iter()
            .enumerate()
            .map(|(i, (key, title))| self.render_area_navigation(i, key, title, &c, cx))
            .collect();
        let sidebar_body = div()
            .id("settings-sidebar-list")
            .flex()
            .flex_col()
            .gap(c.space(2.0))
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .children(sidebar_items)
            .into_any_element();

        let sidebar = div()
            .id("settings-sidebar")
            .w(c.space(226.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap(c.space(2.0))
            .px(c.space(10.0))
            .pt(c.space(if self.windowed { 40.0 } else { 10.0 }))
            .pb(c.space(10.0))
            .overflow_hidden()
            // `docs/settings-guidelines.md`: the nav rail sits on its own
            // `--sidebar` surface, distinct from the `--card` content area.
            .bg(c.sidebar)
            .border_r_1()
            .border_color(c.sidebar_border)
            .child(search_box)
            .child(sidebar_body)
            .child(
                div()
                    .mt_auto()
                    .pt(c.space(8.0))
                    .border_t_1()
                    .border_color(c.sidebar_border)
                    .child(keybinding_hint("Navigate", ["↑", "↓", "Enter"], c)),
            );

        let body = self.render_body(&c, cx);
        let windowed = self.windowed;

        let header = self.render_header(&c, cx);

        // The navigation rail spans the full Settings surface. Scope and JSON
        // actions live in the content pane above its virtualized field list.
        // Keeping these as sibling columns matches the native Settings layout
        // and leaves the sidebar visually continuous beside both header/body.
        let content_pane = div()
            .flex_1()
            .min_w(px(400.0))
            .min_h_0()
            .flex()
            .flex_col()
            .child(header)
            .child(body);
        let content = div()
            .flex_1()
            .min_h_0()
            .flex()
            .child(sidebar)
            .child(content_pane);

        let card = div()
            .id("settings-card")
            .track_focus(&self.focus)
            .key_context("Settings")
            .flex()
            .bg(c.card)
            .text_color(c.fg)
            .on_key_down(cx.listener(Self::on_key))
            .child(content)
            .children(self.render_dropdown(&c, cx));

        if windowed {
            return card.size_full().into_any_element();
        }

        // Legacy in-`AppShell` modal path (kept for tests only).
        div()
            .id("settings-overlay")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(labonair_theme::modal_scrim())
            .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| this.close(cx)))
            .child(
                card.w(px(820.0))
                    .h(px(560.0))
                    .rounded_lg()
                    .border_1()
                    .border_color(c.border)
                    .overflow_hidden()
                    .on_click(|_, _w, cx| cx.stop_propagation()),
            )
            .into_any_element()
    }
}

impl SettingsView {
    /// Header: a thin chrome strip, not a toolbar. Left edge is padded clear
    /// of the OS traffic lights; it carries only the sub-page back-arrow +
    /// breadcrumb (when a `SubPageLink` was followed). The single action —
    /// "Edit in config.json" — sits at the right edge. No in-window close
    /// button: the traffic lights close the window
    /// (`docs/architecture.md` §8.3).
    pub(crate) fn render_header(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let page = &self.pages[self.active_area];
        let crumb: Option<gpui::AnyElement> = self.active_subpage.map(|i| {
            let sub_title = self.pages[self.active_area].sub_pages[i].title;
            div()
                .flex()
                .items_center()
                .gap_1()
                .text_color(c.fg)
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_size(px(12.5))
                .child(
                    icon_button_builder("settings-back", *c, IconName::ArrowLeft)
                        .variant(ButtonVariant::Subtle)
                        .size(ButtonSize::IconXs)
                        .shape(IconButtonShape::Square)
                        .tooltip("Back to settings category")
                        .render()
                        .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                            this.go_back_to_main_page(cx);
                        }))
                        .on_key_down(cx.listener(|this, event: &KeyDownEvent, _w, cx| {
                            if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                                this.go_back_to_main_page(cx);
                                cx.stop_propagation();
                            }
                        })),
                )
                .child(SharedString::from(format!(
                    "{} \u{203A} {}",
                    page.title, sub_title
                )))
                .into_any_element()
        });
        let has_project = cx
            .try_global::<SettingsStore>()
            .is_some_and(|store| store.project_root().is_some());
        let scope_segments = if has_project {
            segmented_control("settings-scope", *c, self.scope.token())
                .segment("user", "User")
                .segment("project", "Project")
        } else {
            segmented_control("settings-scope", *c, self.scope.token()).segment("user", "User")
        };
        let view = cx.entity();
        let scope_control = scope_segments
            .variant(SegmentVariant::Outline)
            .size(SegmentSize::Sm)
            .on_select(move |key, _window, app| {
                if let Some(scope) = SettingsScope::from_token(key.as_ref()) {
                    view.update(app, |this, cx| this.set_scope(scope, cx));
                }
            });
        let scope = self.scope;
        let json_label = match scope {
            SettingsScope::User => "Edit in config.json",
            SettingsScope::Project => "Edit in project settings.json",
        };
        let left = h_stack()
            .flex_1()
            .min_w_0()
            .items_center()
            .gap(c.space(12.0))
            .child(scope_control)
            .children(crumb);

        div()
            .h(c.space(44.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .pl(c.space(24.0))
            .pr(c.space(24.0))
            .child(left)
            .child(
                // T20-003: `button()` (`Outline`/`Xs`) — the one header action.
                button(
                    "settings-open-json",
                    *c,
                    ButtonVariant::Outlined,
                    ButtonSize::Sm,
                )
                .tab_index(0)
                .focus(|style| style.border_1().border_color(c.ring))
                .child(json_label)
                .on_click(cx.listener(move |this, _: &ClickEvent, window, cx| {
                    this.services
                        .open_settings_file(this.scope.file_target(), window, cx);
                }))
                .on_key_down(cx.listener(
                    move |this, event: &KeyDownEvent, window, cx| {
                        if matches!(event.keystroke.key.as_str(), "enter" | "space") {
                            this.services
                                .open_settings_file(this.scope.file_target(), window, cx);
                            cx.stop_propagation();
                        }
                    },
                )),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod select_menu_navigation_tests {
    use super::select_menu_next_index;

    #[test]
    fn selection_navigation_wraps_and_jumps_by_boundaries_and_pages() {
        assert_eq!(select_menu_next_index(0, 3, "up"), Some(2));
        assert_eq!(select_menu_next_index(2, 3, "down"), Some(0));
        assert_eq!(select_menu_next_index(2, 12, "home"), Some(0));
        assert_eq!(select_menu_next_index(2, 12, "end"), Some(11));
        assert_eq!(select_menu_next_index(8, 12, "pageup"), Some(0));
        assert_eq!(select_menu_next_index(3, 12, "pagedown"), Some(11));
        assert_eq!(select_menu_next_index(0, 0, "down"), None);
        assert_eq!(select_menu_next_index(0, 3, "left"), None);
    }
}

#[cfg(test)]
mod setting_source_tests {
    use super::SettingSource;

    #[test]
    fn only_overrides_have_a_modified_source_label() {
        assert_eq!(SettingSource::Default.modified_in(), None);
        assert_eq!(SettingSource::User.modified_in(), Some("user settings"));
        assert_eq!(
            SettingSource::Project.modified_in(),
            Some("project settings")
        );
    }
}
