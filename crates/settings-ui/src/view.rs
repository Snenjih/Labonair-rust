//! The `SettingsView` entity: its state struct, construction, lifecycle,
//! keyboard handling, the AREAS-driven navigation (T19-004: disclosure
//! sections + scroll-spy + sub-pages + custom top-level chrome, replacing the
//! old flat `CATEGORIES` sidebar), and the small render helpers + `Palette`.
//! The large per-pane render code lives in the sibling `panes/*` modules
//! (each a separate `impl SettingsView` block).

pub use gpui::prelude::FluentBuilder;
pub use gpui::{
    div, px, App, AppContext, ClickEvent, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, Pixels, Point, Render,
    ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
};
pub use serde_json::Value;
pub use tokio::runtime::Handle as TokioHandle;

pub use labonair_filesystem::paths::config_dir;
pub use labonair_notifications::{notification_center, Notification};
pub use labonair_settings::{Settings as _, SettingsStore};
pub use labonair_settings_content::areas::AREAS;
pub use labonair_theme::ThemeStore;
pub use labonair_ui_kit::{
    button, h_stack, list_header, list_separator, number_field, select_popover, select_trigger,
    v_stack, ButtonSize, ButtonVariant, IconName, ListItem, Palette, SelectOption, Switch,
};

pub(crate) use crate::apply::*;
pub(crate) use crate::pages::*;
pub(crate) use crate::schema::*;
pub(crate) use crate::search::{SearchIndex, SearchRow, SearchTarget};
pub(crate) use crate::services::{SettingsServices, SystemFontService};
pub(crate) use crate::window::*;

use std::collections::HashSet;

pub(crate) struct EditState {
    /// The field's `json_path` (e.g. `"terminal.terminalFontSize"`).
    pub(crate) key: String,
    pub(crate) buffer: String,
    pub(crate) numeric: bool,
}

/// One row in the command-palette theme list (built-in default + user themes).
pub(crate) struct ThemeEntry {
    /// Filename stem — `"default"` for the built-in.
    pub(crate) id: String,
    /// Display name from the theme file.
    pub(crate) name: String,
}

/// Which layer supplies a field's effective value, for the origin badge
/// (`docs/settings-guidelines.md` rule 5). A thin display-only mirror of
/// `labonair_settings::SettingsLayer` — kept separate so this crate never has
/// to match on `SettingsLayer::Project(WorktreeId)`/`Language(String)`'s
/// payloads just to render three words.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginBadge {
    Default,
    User,
    Project,
}

impl OriginBadge {
    pub(crate) fn label(self) -> &'static str {
        match self {
            OriginBadge::Default => "Default",
            OriginBadge::User => "User",
            OriginBadge::Project => "Project",
        }
    }
}

pub struct SettingsView {
    pub(crate) theme: Entity<ThemeStore>,
    pub(crate) font_service: std::sync::Arc<dyn SystemFontService>,
    pub(crate) tokio: TokioHandle,
    pub(crate) open: bool,
    /// Index into `AREAS` / `self.pages` — the active top-level category.
    pub(crate) active_area: usize,
    /// Index into `self.pages[active_area].sub_pages`, when a `SubPageLink`
    /// has been followed (rule 1).
    pub(crate) active_subpage: Option<usize>,
    pub(crate) search: String,
    pub(crate) editing: Option<EditState>,
    /// `true` when this view is the root of its own OS window (T16-009); `false`
    /// for the legacy in-`AppShell` modal path (kept for tests only).
    pub(crate) windowed: bool,
    /// An open `Select` dropdown (json_path + anchor position + options),
    /// drawn as a deferred floating layer so it escapes the scroll clip.
    pub(crate) dropdown: Option<SelectMenu>,
    /// Scanned system font family names for the `FontFamily` picker, loaded
    /// once asynchronously when the window opens.
    pub(crate) system_fonts: Vec<SharedString>,
    pub(crate) focus: FocusHandle,
    // ── T19-004: generated settings UI ──────────────────────────────────
    /// Every generated field (`crate::schema::all_fields()`), computed once.
    pub(crate) all_fields: Vec<AnyField>,
    /// Every top-level page (`crate::pages::pages()`), in `AREAS` order.
    pub(crate) pages: Vec<SettingsPage>,
    /// Top-level sidebar rows whose sub-section list is expanded
    /// (`docs/architecture.md` §8.3 deviation). The active area is expanded
    /// on navigation.
    pub(crate) expanded_areas: HashSet<usize>,
    /// A section label the sidebar asked to scroll the content area to,
    /// consumed by `render_generated_body` once the section rows are built.
    pub(crate) scroll_to_section: Option<&'static str>,
    /// Scroll position of the active generated page's content, tracked so
    /// the jump bar can scroll-to-section and highlight the section that is
    /// currently topmost (rule 1's scroll-spy).
    pub(crate) content_scroll: ScrollHandle,
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
        // `.labonair/settings.json` edit) — so origin badges / values stay
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
            if let Some(SettingsTarget(Some(slug))) = cx.try_global::<SettingsTarget>().copied() {
                this.navigate_to_slug(slug);
                this.search.clear();
                cx.notify();
            }
        })
        .detach();
        let all_fields = all_fields();
        let pages = pages();
        let search_index = SearchIndex::build(&all_fields);
        Self {
            theme,
            font_service: services.fonts,
            tokio,
            open: false,
            active_area: 0,
            active_subpage: None,
            search: String::new(),
            editing: None,
            windowed: false,
            dropdown: None,
            system_fonts: Vec::new(),
            focus: cx.focus_handle(),
            all_fields,
            pages,
            // Every sidebar category starts collapsed; `open()` re-clears this
            // so it holds on every reopen too.
            expanded_areas: HashSet::new(),
            scroll_to_section: None,
            content_scroll: ScrollHandle::new(),
            search_index,
            search_results: Vec::new(),
            search_selected: 0,
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
        self.editing = None;
        // Sidebar categories always start collapsed on (re)open.
        self.expanded_areas.clear();
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
        self.editing = None;
        cx.notify();
    }

    /// Close request from Esc / the header close button. In windowed mode this
    /// destroys the OS window (GPUI 0.2.2 has no per-window hide); the shared
    /// [`PreferencesStore`] keeps all persistent state so the next open is
    /// instant and lossless.
    pub(crate) fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.windowed {
            cx.set_global(SettingsWindowRef { handle: None });
            self.editing = None;
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
        self.expanded_areas.insert(i);
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
        let area = &AREAS[area_idx];
        if matches!(
            self.pages.get(area_idx).map(|p| &p.body),
            Some(PageBody::Generated(_))
        ) && !crate::pages::leftover_fields(area.target_module, &self.all_fields).is_empty()
        {
            out.push("Other");
        }
        out
    }

    pub(crate) fn go_back_to_main_page(&mut self, cx: &mut Context<Self>) {
        self.active_subpage = None;
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
        if self.search_selected >= self.search_results.len() {
            self.search_selected = self.search_results.len().saturating_sub(1);
        }
    }

    /// Enter/click on a search result: navigate to its area (+ sub-page),
    /// clear the query, and schedule a scroll-to + highlight pulse once the
    /// target page has rendered (`render_generated_body` consumes
    /// `pending_scroll`).
    pub(crate) fn activate_search_hit(&mut self, target: SearchTarget, cx: &mut Context<Self>) {
        match target {
            SearchTarget::Field(idx) => {
                let Some(field) = self.all_fields.get(idx).copied() else {
                    return;
                };
                let Some(area_index) = AREAS.iter().position(|a| a.target_module == field.area())
                else {
                    return;
                };
                let subpage_index = match section_label_for_field(field.area(), field.local_key()) {
                    Some(("", _)) => None,
                    Some((slug, _)) => self.pages[area_index]
                        .sub_pages
                        .iter()
                        .position(|sp| sp.slug == slug),
                    // Not placed by any curated group — falls through to
                    // the trailing "Other" section on the area's main page.
                    None => None,
                };
                self.active_area = area_index;
                self.active_subpage = subpage_index;
                self.expanded_areas.insert(area_index);
                self.pending_scroll = Some(field.json_path);
                self.set_highlight(field.json_path, cx);
            }
        }
        self.search.clear();
        self.search_results.clear();
        self.search_selected = 0;
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
    pub(crate) fn field_origin(&self, field: &AnyField, cx: &App) -> OriginBadge {
        match cx.try_global::<SettingsStore>() {
            None => OriginBadge::Default,
            Some(store) => match store.source_of(field.json_path) {
                labonair_settings::SettingsLayer::Default => OriginBadge::Default,
                labonair_settings::SettingsLayer::Project(_) => OriginBadge::Project,
                _ => OriginBadge::User,
            },
        }
    }

    /// Write a generated field's value through the layered `SettingsStore`
    /// (persists the `User` layer) — every consumer reads that store
    /// directly, so no separate bridge sync is needed.
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
        let result = cx.global_mut::<SettingsStore>().update_user(move |c| {
            (set)(c, value.clone());
        });
        if let Err(err) = result {
            // Blocked (invalid JSON on disk, T19-005) — surface it rather
            // than silently discarding the edit.
            self.notify_error(cx, "Could not save setting", err);
            return;
        }
        self.sync_theme_from_prefs(cx);
        cx.notify();
    }

    /// "Reset to default" (rule 5) — writes the field's `SettingsContent::
    /// defaults()` value back into the `User` layer. This is a pragmatic
    /// simplification of "clear the User-layer override": `SettingsStore`
    /// has no per-field unset, only whole-layer replacement, so a reset
    /// leaves the field explicitly set to its default rather than truly
    /// absent — visually and functionally identical (the field shows its
    /// default value and the origin badge would only differ from `Default`
    /// if a lower-priority… there is none below `Default`, so this is exact
    /// for the common case of no active project override).
    pub(crate) fn reset_field(&mut self, json_path: &'static str, cx: &mut Context<Self>) {
        let Some(field) = self.field_by_path(json_path) else {
            return;
        };
        let Some(default_value) =
            (field.get)(&labonair_settings_content::SettingsContent::defaults())
        else {
            return;
        };
        self.set_field_value(json_path, default_value, cx);
    }

    /// Re-derive the [`ThemeStore`] state (color mode, fonts, metrics, active
    /// theme + variant, icon theme) from the layered settings.
    pub(crate) fn sync_theme_from_prefs(&mut self, cx: &mut Context<Self>) {
        let theme = self.theme.clone();
        apply_prefs_to_theme(&theme, cx);
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

    pub(crate) fn begin_edit(&mut self, key: &str, numeric: bool, cx: &mut Context<Self>) {
        let buffer = self
            .field_by_path(key)
            .and_then(|f| self.field_value(f, cx))
            .map(|v| match v {
                Value::String(s) => s,
                other => other.to_string(),
            })
            .unwrap_or_default();
        self.editing = Some(EditState {
            key: key.to_string(),
            buffer,
            numeric,
        });
        cx.notify();
    }

    pub(crate) fn commit_edit(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.editing.take() else {
            return;
        };
        let Some(field) = self.field_by_path(&edit.key) else {
            return;
        };
        let json_path = field.json_path;
        let value = if edit.numeric {
            match edit.buffer.trim().parse::<i64>() {
                Ok(n) => Value::from(n),
                Err(_) => {
                    cx.notify();
                    return;
                }
            }
        } else {
            Value::String(edit.buffer.trim().to_string())
        };
        self.set_field_value(json_path, value, cx);
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
        if self.editing.is_some() {
            match key {
                "escape" => {
                    self.editing = None;
                    cx.notify();
                }
                "enter" => self.commit_edit(cx),
                "backspace" => {
                    if let Some(e) = self.editing.as_mut() {
                        e.buffer.pop();
                    }
                    cx.notify();
                }
                _ => {
                    if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
                        return;
                    }
                    if let Some(ch) = char_of(ks) {
                        if let Some(e) = self.editing.as_mut() {
                            e.buffer.push_str(&ch);
                        }
                        cx.notify();
                    }
                }
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
            "backspace" => {
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
                if ks.modifiers.platform || ks.modifiers.control || ks.modifiers.alt {
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

    /// The General page's own leading content: an About hero, above the
    /// generated field grid.
    pub(crate) fn render_about_hero(
        &self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        // A single raised info card: identity + "Check for updates" on the
        // left, a stacked list of icon links on the right.
        let os = match std::env::consts::OS {
            "macos" => "macOS",
            "linux" => "Linux",
            "windows" => "Windows",
            other => other,
        };

        // One right-hand link: icon + (title / subtitle), the whole row
        // clickable.
        let link = |id: &'static str,
                    icon: IconName,
                    title: &'static str,
                    subtitle: &'static str,
                    url: &'static str| {
            div()
                .id(id)
                .flex()
                .flex_row()
                .items_center()
                .gap_3()
                .cursor_pointer()
                .child(icon.svg(c.muted).size(px(18.0)))
                .child(
                    v_stack()
                        .child(div().text_size(px(13.0)).text_color(c.fg).child(title))
                        .child(
                            div()
                                .text_size(px(11.0))
                                .text_color(c.muted)
                                .child(subtitle),
                        ),
                )
                .on_click(cx.listener(move |_, _: &ClickEvent, _w, cx| {
                    cx.open_url(url);
                }))
        };

        div()
            .rounded_lg()
            .border_1()
            .border_color(c.border)
            .bg(c.card)
            .p(px(24.0))
            .mb_4()
            .child(
                h_stack()
                    .items_start()
                    .justify_between()
                    .gap_6()
                    .child(
                        v_stack()
                            .gap_3()
                            .child(
                                h_stack()
                                    .gap_3()
                                    .child(
                                        div()
                                            .size(px(52.0))
                                            .rounded_lg()
                                            .bg(c.bg)
                                            .border_1()
                                            .border_color(c.border)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .text_color(c.fg)
                                            .text_size(px(24.0))
                                            .font_weight(gpui::FontWeight::BOLD)
                                            .child("L"),
                                    )
                                    .child(
                                        v_stack()
                                            .gap_0p5()
                                            .child(
                                                div()
                                                    .text_size(px(22.0))
                                                    .font_weight(gpui::FontWeight::BOLD)
                                                    .text_color(c.fg)
                                                    .child("Labonair"),
                                            )
                                            .child(
                                                div()
                                                    .text_size(px(12.0))
                                                    .text_color(c.muted)
                                                    .child(SharedString::from(format!(
                                                        "{}  \u{2022}  {}  \u{2022}  v{}",
                                                        os,
                                                        std::env::consts::ARCH,
                                                        env!("CARGO_PKG_VERSION"),
                                                    ))),
                                            ),
                                    ),
                            )
                            .child(
                                button(
                                    "about-check-updates",
                                    *c,
                                    ButtonVariant::Default,
                                    ButtonSize::Sm,
                                )
                                .child("Check for updates")
                                .on_click(cx.listener(
                                    |_, _: &ClickEvent, _w, cx| {
                                        cx.open_url(
                                            "https://github.com/Snenjih/Labonair-rust/releases",
                                        );
                                    },
                                )),
                            ),
                    )
                    .child(
                        v_stack()
                            .gap_3()
                            .child(link(
                                "about-report",
                                IconName::Warning,
                                "Report a problem",
                                "Generate a pre-filled GitHub issue",
                                "https://github.com/Snenjih/Labonair-rust/issues/new",
                            ))
                            .child(link(
                                "about-github",
                                IconName::GitBranch,
                                "GitHub",
                                "Source code",
                                "https://github.com/Snenjih/Labonair-rust",
                            ))
                            .child(link(
                                "about-website",
                                IconName::Globe,
                                "Website",
                                "labonair.app",
                                "https://github.com/Snenjih/Labonair-rust",
                            )),
                    ),
            )
            .into_any_element()
    }

    /// The sidebar's content while a search query is active (T19-007 step
    /// 3): a flat, category-grouped list of `search_results` replacing the
    /// normal category nav. Selection follows keyboard Up/Down
    /// (`search_selected`); click/Enter jump to the field (`on_key`,
    /// `activate_search_hit`).
    pub(crate) fn render_search_results(
        &mut self,
        c: &Palette,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        if self.search_results.is_empty() {
            return div()
                .p_2()
                .text_size(px(11.0))
                .text_color(c.muted)
                .child(SharedString::from(format!(
                    "No setting found for \u{201C}{}\u{201D}.",
                    self.search.trim()
                )))
                .into_any_element();
        }
        // T20-001: one `ListHeader` per category + one `ListItem` per hit,
        // from the shared list primitives.
        let rows = self.search_results.clone();
        let selected = self.search_selected;
        let mut col = div().flex().flex_col().gap_0p5();
        let mut last_area: Option<&'static str> = None;
        for (i, row) in rows.into_iter().enumerate() {
            if last_area != Some(row.area_title) {
                if last_area.is_some() {
                    col = col.child(list_separator(c.border));
                }
                col = col.child(list_header(row.area_title, c.muted));
                last_area = Some(row.area_title);
            }
            let target = row.target;
            let subtitle = (!row.subtitle.is_empty()).then(|| {
                div()
                    .text_size(px(10.0))
                    .text_color(c.muted)
                    .child(SharedString::from(row.subtitle))
            });
            col = col.child(
                ListItem::new(
                    SharedString::from(format!("search-hit-{i}")),
                    c.fg,
                    c.muted,
                    c.accent,
                )
                .selected(i == selected)
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                    this.activate_search_hit(target, cx);
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_0p5()
                        .child(SharedString::from(row.title))
                        .children(subtitle),
                ),
            );
        }
        col.into_any_element()
    }
}

impl Focusable for SettingsView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if !self.windowed && !self.open {
            return div().into_any_element();
        }
        let c = Palette::from_theme(self.theme.read(cx));
        let active_area = self.active_area;
        let searching = !self.search.trim().is_empty();
        // T19-007: recompute every render so Up/Down/Enter and mouse clicks
        // always act on what's currently on screen (cheap — ~200 entries).
        self.refresh_search_results();

        let search_box = div()
            .mb_2()
            .px_2()
            .py(px(4.0))
            .rounded_sm()
            .border_1()
            .border_color(if searching { c.accent } else { c.border })
            .bg(c.bg)
            .text_size(px(11.5))
            .text_color(if self.search.is_empty() {
                c.muted
            } else {
                c.fg
            })
            .child(SharedString::from(if self.search.is_empty() {
                "Search settings\u{2026}".to_string()
            } else {
                self.search.clone()
            }));

        // Left: fixed-order top-level categories (rule 1), sourced from
        // `AREAS` contains only value-oriented settings categories. Capability
        // management surfaces are registered by their owning modules.
        //
        // `docs/architecture.md` §8.3 deviation: each top-level row carries a
        // disclosure chevron that reveals the page's section labels as
        // sub-level scroll anchors (not pages). Row click navigates + expands;
        // chevron click only toggles; sub-label click scrolls the content to
        // that section.
        let sidebar_body: gpui::AnyElement = if searching {
            self.render_search_results(&c, cx)
        } else {
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .children(AREAS.iter().enumerate().map(|(i, area)| {
                    let is_active = i == active_area;
                    let expanded = self.expanded_areas.contains(&i);
                    let sections = self.section_labels_for_area(i);
                    let has_sections = !sections.is_empty();
                    let chevron = if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    };
                    let toggle: gpui::AnyElement = if has_sections {
                        button(
                            SharedString::from(format!("area-tw-{}", area.key)),
                            c,
                            ButtonVariant::Ghost,
                            ButtonSize::IconXs,
                        )
                        .child(chevron.svg(c.sidebar_fg).size(px(12.0)))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.toggle_area_expanded(i, cx);
                        }))
                        .into_any_element()
                    } else {
                        div().w(px(20.0)).flex_shrink_0().into_any_element()
                    };
                    let row = h_stack().items_center().gap_0p5().child(toggle).child(
                        div().flex_1().min_w_0().child(
                            ListItem::new(
                                SharedString::from(area.key),
                                c.sidebar_fg,
                                c.muted,
                                c.accent,
                            )
                            .selected(is_active)
                            .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                this.go_to_area(i, cx);
                            }))
                            .child(SharedString::from(area.title)),
                        ),
                    );
                    let sub = (expanded && has_sections).then(|| {
                        v_stack()
                            .gap_0p5()
                            .pb_1()
                            .children(sections.into_iter().map(|label| {
                                div()
                                    .id(SharedString::from(format!("sec-{i}-{label}")))
                                    .pl(px(30.0))
                                    .pr_2()
                                    .py(px(3.0))
                                    .rounded_sm()
                                    .text_size(px(11.5))
                                    .text_color(c.muted)
                                    .cursor_pointer()
                                    .hover(|s| s.text_color(c.sidebar_fg))
                                    .child(SharedString::from(label))
                                    .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                                        this.go_to_section(i, label, cx);
                                    }))
                            }))
                    });
                    v_stack().child(row).children(sub)
                }))
                .into_any_element()
        };

        let sidebar = div()
            .id("settings-sidebar")
            .w(px(208.0))
            .flex_shrink_0()
            .flex()
            .flex_col()
            .gap_0p5()
            .p_2()
            .overflow_y_scroll()
            // `docs/settings-guidelines.md`: the nav rail sits on its own
            // `--sidebar` surface, distinct from the `--card` content area.
            .bg(c.sidebar)
            .border_r_1()
            .border_color(c.sidebar_border)
            .child(search_box)
            .child(sidebar_body);

        let body = self.render_body(&c, cx);
        let windowed = self.windowed;

        let header = self.render_header(&c, cx);

        let content = div().flex_1().min_h_0().flex().child(sidebar).child(
            div()
                .id("settings-scroll")
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .items_center()
                .p_4()
                .overflow_y_scroll()
                .track_scroll(&self.content_scroll)
                .child(
                    div()
                        .w_full()
                        .max_w(px(580.0))
                        .flex()
                        .flex_col()
                        .child(body),
                ),
        );

        let card = div()
            .id("settings-card")
            .track_focus(&self.focus)
            .key_context("Settings")
            .flex()
            .flex_col()
            .bg(c.card)
            .text_color(c.fg)
            .on_key_down(cx.listener(Self::on_key))
            .child(header)
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
        let area = &AREAS[self.active_area];
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
                    button(
                        "settings-back",
                        *c,
                        ButtonVariant::Ghost,
                        ButtonSize::IconXs,
                    )
                    .child("\u{2190}")
                    .on_click(cx.listener(|this, _: &ClickEvent, _w, cx| {
                        this.go_back_to_main_page(cx);
                    })),
                )
                .child(SharedString::from(format!(
                    "{} \u{203A} {}",
                    area.title, sub_title
                )))
                .into_any_element()
        });
        div()
            .h(px(44.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            // Clear the macOS traffic lights (positioned at x = 19).
            .pl(px(84.0))
            .pr_3()
            .border_b_1()
            .border_color(c.border)
            .child(div().flex_1().min_w_0().children(crumb))
            .child(
                // T20-003: `button()` (`Ghost`/`Xs`) — the one header action.
                button(
                    "settings-open-json",
                    *c,
                    ButtonVariant::Ghost,
                    ButtonSize::Xs,
                )
                .child("Edit in config.json")
                .on_click(cx.listener(|_, _: &ClickEvent, _w, cx| {
                    cx.reveal_path(&config_dir().join("config.json"));
                })),
            )
            .into_any_element()
    }
}
