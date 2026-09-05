//! The `SettingsView` entity: its state struct, construction, lifecycle,
//! keyboard handling, the AREAS-driven navigation (T19-004: disclosure
//! sections + scroll-spy + sub-pages + custom top-level chrome, replacing the
//! old flat `CATEGORIES` sidebar), and the small render helpers + `Palette`.
//! The large per-pane render code lives in the sibling `panes/*` modules
//! (each a separate `impl SettingsView` block).

pub use gpui::prelude::FluentBuilder;
pub use gpui::{
    div, px, App, AppContext, ClickEvent, ClipboardItem, Context, Entity, FocusHandle, Focusable,
    InteractiveElement, IntoElement, KeyDownEvent, ParentElement, PathPromptOptions, Pixels, Point,
    Render, ScrollHandle, SharedString, StatefulInteractiveElement, Styled, Window,
};
pub use serde_json::Value;
pub use std::fs;
pub use std::path::PathBuf;
pub use tokio::runtime::Handle as TokioHandle;

pub use labonair_backend::modules::fs::paths::config_dir;
pub use labonair_backend::modules::mcp::{
    mcp_get_status, mcp_regenerate_token, mcp_set_auto_revoke_minutes, mcp_set_enabled,
    mcp_set_max_command_timeout_secs, mcp_set_port,
};
pub use labonair_backend::modules::settings::mcp::{mcp_prefs_load, mcp_prefs_save, McpPrefs};
pub use labonair_backend::modules::settings::preferences::ThemePref;
pub use labonair_backend::App as Backend;
pub use labonair_command_palette::{
    effective_binding, shortcut, shortcut_slug, shortcuts, KeybindMap, ShortcutId,
};
pub use labonair_notifications::{notification_center, Notification};
pub use labonair_settings::SettingsStore;
pub use labonair_settings_content::areas::AREAS;
pub use labonair_theme::{ThemeFile, ThemePreference, ThemeStore};
pub use labonair_ui_kit::{
    banner, disclosure, h_stack, list_header, list_separator, number_field, segmented_control,
    select_popover, select_trigger, v_stack, ListItem, Palette, SelectOption, Severity,
};
pub use labonair_workspace::background::BackgroundStore;

pub(crate) use crate::apply::*;
pub(crate) use crate::pages::*;
pub(crate) use crate::schema::*;
pub(crate) use crate::search::{SearchIndex, SearchRow, SearchTarget};
pub(crate) use crate::store::*;
pub(crate) use crate::window::*;

use std::collections::HashSet;

pub(crate) struct EditState {
    /// The field's `json_path` (e.g. `"terminal.terminalFontSize"`), or a
    /// non-field synthetic key (`"provkey:<id>"` for AI provider API keys).
    pub(crate) key: String,
    pub(crate) buffer: String,
    pub(crate) numeric: bool,
}

/// One row in the Appearance theme list (built-in default + user themes).
pub(crate) struct ThemeEntry {
    /// Filename stem — `"default"` for the built-in.
    pub(crate) id: String,
    /// Display name from the theme file.
    pub(crate) name: String,
    /// Built-in themes can be activated/exported but never deleted.
    pub(crate) builtin: bool,
}

/// One entry of the community theme index (port of `RemoteTheme`).
#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteTheme {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) description: String,
    #[serde(default)]
    pub(crate) author: String,
    pub(crate) raw_url: String,
}

/// Inline agent/directive editor state — three keydown-buffer fields.
pub(crate) struct AiEditor {
    pub(crate) kind: AiEditKind,
    pub(crate) id: String,
    pub(crate) labels: [&'static str; 3],
    pub(crate) fields: [String; 3],
    pub(crate) focus_idx: usize,
    pub(crate) multiline_last: bool,
}

#[derive(PartialEq)]
pub(crate) enum AiEditKind {
    Agent,
    Directive,
}

pub(crate) const COMMUNITY_INDEX_URL: &str =
    "https://raw.githubusercontent.com/Snenjih/labonair-themes/main/index.json";

/// Fallback shown when the remote index cannot be fetched (port of
/// `MOCK_COMMUNITY_THEMES`).
pub(crate) fn mock_community_themes() -> Vec<RemoteTheme> {
    vec![
        RemoteTheme {
            id: "catppuccin".into(),
            name: "Catppuccin".into(),
            description: "Soothing pastel theme — Latte, Frappé, Macchiato, Mocha".into(),
            author: "Catppuccin".into(),
            raw_url:
                "https://raw.githubusercontent.com/Snenjih/labonair-themes/main/themes/catppuccin.json"
                    .into(),
        },
        RemoteTheme {
            id: "nord".into(),
            name: "Nord".into(),
            description: "An arctic, north-bluish color palette".into(),
            author: "arcticicestudio".into(),
            raw_url:
                "https://raw.githubusercontent.com/Snenjih/labonair-themes/main/themes/nord.json"
                    .into(),
        },
    ]
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
    pub(crate) prefs: Entity<PreferencesStore>,
    pub(crate) theme: Entity<ThemeStore>,
    /// Kept alive (and observed, in `new()`) so a live background change
    /// still repaints the settings window; the old bespoke background
    /// gallery picker (`render_appearance`) was dropped in T19-004 in favor
    /// of the generic field grid for `appearance.background*` — nothing
    /// reads this field directly any more, but the entity must stay held
    /// for its `cx.observe` subscription's lifetime.
    #[allow(dead_code)]
    pub(crate) background: Entity<BackgroundStore>,
    pub(crate) backend: Backend,
    pub(crate) tokio: TokioHandle,
    pub(crate) open: bool,
    /// Index into `AREAS` / `self.pages` — the active top-level category.
    pub(crate) active_area: usize,
    /// Index into `self.pages[active_area].sub_pages`, when a `SubPageLink`
    /// has been followed (rule 1).
    pub(crate) active_subpage: Option<usize>,
    pub(crate) search: String,
    pub(crate) editing: Option<EditState>,
    pub(crate) mcp: McpPrefs,
    pub(crate) mcp_token: Option<String>,
    /// Available themes for the Appearance pane, refreshed when the modal opens.
    pub(crate) theme_files: Vec<ThemeEntry>,
    /// Which listed theme is active (`None` = built-in light/dark, no override).
    pub(crate) active_theme_id: Option<String>,
    /// Themes pane: `false` = Installed tab, `true` = Community tab.
    pub(crate) themes_community_tab: bool,
    /// Community/marketplace theme index (mock fallback on fetch failure).
    pub(crate) community_themes: Vec<RemoteTheme>,
    pub(crate) community_error: Option<String>,
    pub(crate) community_loading: bool,
    /// Community theme ids currently being downloaded.
    pub(crate) installing_themes: std::collections::HashSet<String>,
    /// In-progress "New Theme…" name prompt.
    pub(crate) new_theme_prompt: Option<String>,
    pub(crate) new_theme_focus: FocusHandle,
    /// Shortcut currently capturing a new key combination (`Keyboard` pane).
    pub(crate) recording: Option<ShortcutId>,
    /// A captured combination that collides with another shortcut, awaiting
    /// the user's overwrite / cancel decision.
    pub(crate) kb_conflict: Option<KbConflict>,
    /// `true` when this view is the root of its own OS window (T16-009); `false`
    /// for the legacy in-`AppShell` modal path (kept for tests only).
    pub(crate) windowed: bool,
    /// An open `Select` dropdown (json_path + anchor position + options),
    /// drawn as a deferred floating layer so it escapes the scroll clip.
    pub(crate) dropdown: Option<SelectMenu>,
    /// AI provider instances + their keychain-backed API keys (T16-012).
    pub(crate) instances: labonair_ai::InstanceStore,
    pub(crate) secrets: std::sync::Arc<labonair_ai::KeyringSecretStore>,
    /// Scanned system font family names for the `FontFamily` picker, loaded
    /// once asynchronously when the window opens.
    pub(crate) system_fonts: Vec<SharedString>,
    /// AI agents + directives (T16-019) — loaded when the window opens.
    pub(crate) agents: Vec<labonair_backend::modules::agents::Agent>,
    pub(crate) active_agent_id: String,
    pub(crate) directives: Vec<labonair_backend::modules::directives::Directive>,
    /// Open inline agent/directive editor (keydown-buffer modal).
    pub(crate) ai_editor: Option<AiEditor>,
    pub(crate) ai_editor_focus: FocusHandle,
    pub(crate) focus: FocusHandle,
    /// The app's [`labonair_workspace::Workspace`] (T18-007) — backs the
    /// Personalization pane's statusbar-layout editor + panel-toggle
    /// visibility switches.
    pub(crate) workspace: Entity<labonair_workspace::Workspace>,
    /// The shared [`HostManagerView`] (T19-010) — embedded verbatim as the
    /// body of the Hosts custom category; the exact same entity
    /// `Workspace` uses for connecting / `known_hosts`, so an edit here is
    /// live everywhere immediately, with no separate sync path.
    pub(crate) host_manager: Entity<labonair_hosts_ui::HostManagerView>,
    // ── T19-004: generated settings UI ──────────────────────────────────
    /// Every generated field (`crate::schema::all_fields()`), computed once.
    pub(crate) all_fields: Vec<AnyField>,
    /// Every top-level page (`crate::pages::pages()`), in `AREAS` order.
    pub(crate) pages: Vec<SettingsPage>,
    /// Collapsed disclosure sections: `(area index, sub-page slug or "" for
    /// the main page, section label)`. Absent = open (rule 1: "Default: alle
    /// offen").
    pub(crate) collapsed_sections: HashSet<(usize, &'static str, &'static str)>,
    /// Scroll position of the active generated page's content, tracked so
    /// the jump bar can scroll-to-section and highlight the section that is
    /// currently topmost (rule 1's scroll-spy).
    pub(crate) content_scroll: ScrollHandle,
    // ── T19-007: global settings search ─────────────────────────────────
    /// Every field + custom pane, indexed once (`open()`) — never rebuilt per
    /// keystroke (task Warnung).
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
}

pub(crate) struct SelectMenu {
    pub(crate) key: &'static str,
    /// `(serialized token to store, human label to show)`.
    pub(crate) options: Vec<(SharedString, SharedString)>,
    pub(crate) at: Point<Pixels>,
    /// The `"(default)"` font entry — selecting it clears the pref to `""`.
    pub(crate) default_sentinel: Option<SharedString>,
}

pub(crate) struct KbConflict {
    pub(crate) id: ShortcutId,
    pub(crate) binding: String,
    pub(crate) other: ShortcutId,
}

impl SettingsView {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        prefs: Entity<PreferencesStore>,
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        backend: Backend,
        tokio: TokioHandle,
        workspace: Entity<labonair_workspace::Workspace>,
        host_manager: Entity<labonair_hosts_ui::HostManagerView>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.observe(&prefs, |_, _, cx| cx.notify()).detach();
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        cx.observe(&background, |_, _, cx| cx.notify()).detach();
        cx.observe(&workspace, |_, _, cx| cx.notify()).detach();
        cx.observe(&host_manager, |_, _, cx| cx.notify()).detach();
        // The statusbar layout (T18-005/T18-007) and panel-toggle visibility
        // both bump this global — reload-and-repaint so the Personalization
        // pane reflects the same live state as the in-app right-click menus.
        cx.observe_global::<labonair_workspace::status_placements::StatusBarLayoutTick>(|_, cx| {
            cx.notify()
        })
        .detach();
        // The layered `SettingsStore` (T19-002/003) notifies on every write —
        // including ones this window did not make itself (e.g. a project
        // `.labonair/settings.json` edit) — so origin badges / values stay
        // live without a bespoke observer list.
        if cx.has_global::<SettingsStore>() {
            cx.observe_global::<SettingsStore>(|_, cx| cx.notify())
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
        let search_index = SearchIndex::build(&all_fields, &pages);
        Self {
            prefs,
            theme,
            background,
            backend,
            tokio,
            open: false,
            active_area: 0,
            active_subpage: None,
            search: String::new(),
            editing: None,
            mcp: mcp_prefs_load(),
            mcp_token: None,
            theme_files: Vec::new(),
            active_theme_id: None,
            themes_community_tab: false,
            community_themes: Vec::new(),
            community_error: None,
            community_loading: false,
            installing_themes: std::collections::HashSet::new(),
            new_theme_prompt: None,
            new_theme_focus: cx.focus_handle(),
            recording: None,
            kb_conflict: None,
            windowed: false,
            dropdown: None,
            instances: labonair_ai::InstanceStore::open_default(),
            secrets: std::sync::Arc::new(labonair_ai::KeyringSecretStore),
            system_fonts: Vec::new(),
            agents: Vec::new(),
            active_agent_id: String::new(),
            directives: Vec::new(),
            ai_editor: None,
            ai_editor_focus: cx.focus_handle(),
            focus: cx.focus_handle(),
            workspace,
            host_manager,
            all_fields,
            pages,
            collapsed_sections: HashSet::new(),
            content_scroll: ScrollHandle::new(),
            search_index,
            search_results: Vec::new(),
            search_selected: 0,
            highlight: None,
            highlight_token: 0,
            pending_scroll: None,
        }
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.open = true;
        self.editing = None;
        self.recording = None;
        self.kb_conflict = None;
        self.search.clear();
        self.search_results.clear();
        self.search_selected = 0;
        self.highlight = None;
        self.pending_scroll = None;
        window.focus(&self.focus);
        self.refresh_mcp_status(cx);
        self.refresh_themes();
        if self.active_theme_id.is_none() {
            let stored = self.prefs.read(cx).get().app_theme.clone();
            if !stored.is_empty() && stored != "default" {
                self.active_theme_id = Some(stored);
            }
        }
        self.load_system_fonts(cx);
        self.refresh_agents_directives();
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

    pub(crate) fn refresh_mcp_status(&self, cx: &mut Context<Self>) {
        let app = self.backend.clone();
        let task = self
            .tokio
            .spawn(async move { mcp_get_status(app.clone(), &app.mcp, &app.secrets).await });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(status)) = task.await {
                let _ = this.update(cx, |this, cx| {
                    this.mcp_token = status.token;
                    this.mcp.bridge_port = status.port;
                    this.mcp.max_command_timeout_secs = status.max_command_timeout_secs;
                    this.mcp.auto_revoke_minutes = status.auto_revoke_minutes;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    pub(crate) fn notify(&self, cx: &mut Context<Self>, n: Notification) {
        notification_center(cx).update(cx, |c, cx| {
            c.push(n, cx);
        });
    }

    pub(crate) fn notify_error(&self, cx: &mut Context<Self>, title: &'static str, body: String) {
        self.notify(cx, Notification::error(title, body));
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

    /// The sub-page slug to key `collapsed_sections`/scroll state by (`""`
    /// for the main page — never a real slug, since every `AreaMeta::slug`
    /// / `SubPage::slug` is non-empty).
    pub(crate) fn active_subpage_slug(&self) -> &'static str {
        match self.active_subpage {
            Some(i) => self.pages[self.active_area].sub_pages[i].slug,
            None => "",
        }
    }

    pub(crate) fn go_to_area(&mut self, i: usize, cx: &mut Context<Self>) {
        self.active_area = i;
        self.active_subpage = None;
        self.search.clear();
        cx.notify();
    }

    pub(crate) fn go_to_subpage(&mut self, i: usize, cx: &mut Context<Self>) {
        self.active_subpage = Some(i);
        cx.notify();
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
    /// un-collapse the section it lives in, clear the query, and (for a
    /// field) schedule a scroll-to + highlight pulse once the target page has
    /// rendered (`render_generated_body` consumes `pending_scroll`).
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
                let (subpage_index, section) =
                    match section_label_for_field(field.area(), field.local_key()) {
                        Some(("", label)) => (None, Some(label)),
                        Some((slug, label)) => (
                            self.pages[area_index]
                                .sub_pages
                                .iter()
                                .position(|sp| sp.slug == slug),
                            Some(label),
                        ),
                        // Not placed by any curated group — falls through to
                        // the trailing "Other" section on the area's main page.
                        None => (None, Some("Other")),
                    };
                self.active_area = area_index;
                self.active_subpage = subpage_index;
                if let Some(label) = section {
                    let subpage_slug = self.active_subpage_slug();
                    self.collapsed_sections
                        .remove(&(area_index, subpage_slug, label));
                }
                self.pending_scroll = Some(field.json_path);
                self.set_highlight(field.json_path, cx);
            }
            SearchTarget::Pane {
                area_index,
                subpage_index,
            } => {
                self.active_area = area_index;
                self.active_subpage = subpage_index;
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

    pub(crate) fn toggle_section(&mut self, label: &'static str, cx: &mut Context<Self>) {
        let key = (self.active_area, self.active_subpage_slug(), label);
        if !self.collapsed_sections.remove(&key) {
            self.collapsed_sections.insert(key);
        }
        cx.notify();
    }

    pub(crate) fn section_collapsed(&self, label: &'static str) -> bool {
        self.collapsed_sections
            .contains(&(self.active_area, self.active_subpage_slug(), label))
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
    /// (persists the `User` layer), then refresh the `PreferencesStore` /
    /// `GlobalPreferences` bridge so not-yet-migrated consumers see it too
    /// (the task's warning).
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
        self.prefs.update(cx, |p, cx| p.reload_from_disk(cx));
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

    // ── legacy generic field mutation (PreferencesStore) ────────────────

    pub(crate) fn set_pref(&mut self, key: &str, value: Value, cx: &mut Context<Self>) {
        let key_owned = key.to_string();
        self.prefs
            .update(cx, |p, cx| p.set_value(&key_owned, value, cx));
        // Propagate the values modules can't observe generically.
        if key == "theme" {
            let pref = match self.prefs.read(cx).get().theme {
                ThemePref::System => ThemePreference::System,
                ThemePref::Light => ThemePreference::Light,
                ThemePref::Dark => ThemePreference::Dark,
            };
            self.theme.update(cx, |t, cx| t.set_preference(pref, cx));
        }
        // Keyboard shortcuts are no longer part of this generic `set_pref`
        // path (T19-008) — the Shortcuts pane writes `keymap.json` directly
        // (`crate::panes::shortcuts`) and re-applies via `apply_keymap_hook`.
        // The `Preferences` store already republishes `GlobalPreferences` on
        // every change (see `PreferencesStore::set_value`); terminal / editor /
        // workspace all `observe_global` / re-read it, so most keys propagate
        // for free — this is the port's generic `applySettingChange`. The rest
        // are the non-observable side effects (T16-012):
        match key {
            // Keep the AI chat's active model in sync with the settings pref.
            "defaultModelId" => {
                let v = self.prefs.read(cx).get().default_model_id.clone();
                if !v.is_empty() {
                    let _ = self.instances.set_active_model_ref(&v);
                }
            }
            // Reduce-motion and corner radius feed the theme/layout layer.
            "reduceMotion" | "appCornerRadius" | "appLineHeight" => {
                self.sync_theme_from_prefs(cx);
            }
            _ => {}
        }
        // Typography + editor syntax scheme are pushed into the ThemeStore so
        // open terminals / editors pick them up live (T13-003).
        self.sync_theme_from_prefs(cx);
        cx.notify();
    }

    // ── AI providers (T16-012) ───────────────────────────────────────────

    pub(crate) fn add_provider(
        &mut self,
        provider: labonair_ai::ProviderId,
        cx: &mut Context<Self>,
    ) {
        match self.instances.add(provider) {
            Ok(_) => cx.notify(),
            Err(e) => self.notify_error(cx, "Could not add provider", e),
        }
    }

    pub(crate) fn remove_provider(&mut self, id: String, cx: &mut Context<Self>) {
        if let Err(e) = self.instances.remove(&id) {
            self.notify_error(cx, "Could not remove provider", e);
        }
        let _ = labonair_ai::secret_store::clear_instance_key(&*self.secrets, &id);
        cx.notify();
    }

    /// Mirror the font / editor-theme preferences into the [`ThemeStore`].
    pub(crate) fn sync_theme_from_prefs(&mut self, cx: &mut Context<Self>) {
        let p = self.prefs.read(cx).get().clone();
        let theme = self.theme.clone();
        apply_prefs_to_theme(&p, &theme, cx);
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
        // Provider API keys are keychain-backed, never a preference key.
        if let Some(instance_id) = edit.key.strip_prefix("provkey:") {
            let trimmed = edit.buffer.trim();
            let res = if trimmed.is_empty() {
                labonair_ai::secret_store::clear_instance_key(&*self.secrets, instance_id)
            } else {
                labonair_ai::secret_store::set_instance_key(&*self.secrets, instance_id, trimmed)
            };
            match res {
                Ok(()) => self.notify(
                    cx,
                    Notification::success("API key saved", "Stored in the OS keychain."),
                ),
                Err(e) => self.notify_error(cx, "Could not save API key", e),
            }
            cx.notify();
            return;
        }
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
        if self.recording.is_some() {
            self.record_key(ev, window, cx);
            cx.stop_propagation();
            return;
        }
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
        let link = |id: &'static str, label: &'static str, url: &'static str| {
            div()
                .id(id)
                .px_2()
                .py(px(3.0))
                .rounded_sm()
                .border_1()
                .border_color(c.border)
                .text_size(px(11.5))
                .text_color(c.fg)
                .hover(|s| s.bg(c.border))
                .child(label)
                .on_click(cx.listener(move |_, _: &ClickEvent, _w, cx| {
                    cx.open_url(url);
                }))
        };
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_1()
            .py_4()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .size(px(56.0))
                    .rounded_lg()
                    .bg(c.accent)
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_color(c.bg)
                    .font_weight(gpui::FontWeight::BOLD)
                    .child("L"),
            )
            .child(
                div()
                    .text_color(c.fg)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .child("Labonair"),
            )
            .child(
                div()
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .child(SharedString::from(format!(
                        "Version {}  \u{2022}  {} {}",
                        env!("CARGO_PKG_VERSION"),
                        std::env::consts::OS,
                        std::env::consts::ARCH,
                    ))),
            )
            .child(
                div()
                    .mt_1()
                    .flex()
                    .gap_2()
                    .child(link(
                        "about-report",
                        "Report a problem",
                        "https://github.com/Snenjih/Labonair-rust/issues/new",
                    ))
                    .child(link(
                        "about-github",
                        "GitHub",
                        "https://github.com/Snenjih/Labonair-rust",
                    ))
                    .child(link(
                        "about-website",
                        "Website",
                        "https://github.com/Snenjih/Labonair-rust",
                    )),
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
        // `AREAS` — a Custom category (Themes, Hosts, Shortcuts, AI, MCP,
        // Personalization) is a normal entry here, exactly like a Generated
        // one (rule 4: "a custom pane may be registered as a top-level
        // category exactly like a field-based one").
        let sidebar_body: gpui::AnyElement = if searching {
            self.render_search_results(&c, cx)
        } else {
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .children(AREAS.iter().enumerate().map(|(i, area)| {
                    let is_active = i == active_area;
                    div()
                        .id(SharedString::from(area.key))
                        .px_2()
                        .py(px(5.0))
                        .rounded_sm()
                        .text_size(px(12.0))
                        .text_color(if is_active { c.fg } else { c.muted })
                        .when(is_active, |d| d.bg(c.accent))
                        .when(!is_active, |d| d.hover(|s| s.bg(c.border)))
                        .child(SharedString::from(area.title))
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            this.go_to_area(i, cx);
                        }))
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
            .border_r_1()
            .border_color(c.border)
            .child(search_box)
            .child(sidebar_body);

        let body = self.render_body(&c, cx);
        let windowed = self.windowed;

        let header = self.render_header(&c, cx);
        let json_error_banner = cx
            .try_global::<SettingsStore>()
            .and_then(|s| s.user_json_error())
            .map(|err| {
                // T20-001: the shared `Banner` primitive — this used to
                // hardcode `gpui::red()`, bypassing the theme's status tokens
                // (Critical Rule 3).
                banner(Severity::Error, c).child(SharedString::from(format!(
                    "labonair-settings.json has a syntax error ({err}) — fix it before \
                     changing settings here.",
                )))
            });

        // Schema-validation findings (T19-006): shown alongside (not instead
        // of) the syntax-error banner above — a file can be valid JSON but
        // still have a field with the wrong type/enum value, which is what
        // this banner reports (one line per finding, worst first: type/enum
        // errors before unknown-key warnings).
        let schema_banner = cx.try_global::<SettingsStore>().and_then(|s| {
            let errors: Vec<_> = s
                .schema_errors()
                .iter()
                .chain(s.project_schema_errors())
                .collect();
            let warnings: Vec<_> = s
                .schema_warnings()
                .iter()
                .chain(s.project_schema_warnings())
                .collect();
            if errors.is_empty() && warnings.is_empty() {
                return None;
            }
            let mut lines: Vec<SharedString> = errors
                .iter()
                .map(|e| SharedString::from(e.to_string()))
                .collect();
            lines.extend(
                warnings
                    .iter()
                    .map(|w| SharedString::from(format!("warning: {w}"))),
            );
            let severity = if errors.is_empty() {
                Severity::Warning
            } else {
                Severity::Error
            };
            Some(
                banner(severity, c)
                    .stacked(true)
                    .children(lines.into_iter().map(|line| div().child(line))),
            )
        });

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
            .children(json_error_banner)
            .children(schema_banner)
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
    /// Header: "Open settings.json" + breadcrumb (category, or "category >
    /// sub-page" with a back arrow when a `SubPageLink` was followed, rule
    /// 1) + close.
    pub(crate) fn render_header(&self, c: &Palette, cx: &mut Context<Self>) -> gpui::AnyElement {
        let area = &AREAS[self.active_area];
        let title: gpui::AnyElement = match self.active_subpage {
            Some(i) => {
                let sub_title = self.pages[self.active_area].sub_pages[i].title;
                div()
                    .flex()
                    .items_center()
                    .gap_1()
                    .child(
                        div()
                            .id("settings-back")
                            .text_color(c.muted)
                            .hover(|s| s.text_color(c.fg))
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
            }
            None => div()
                .child(SharedString::from(area.title))
                .into_any_element(),
        };
        div()
            .h(px(44.0))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_between()
            .px_3()
            .border_b_1()
            .border_color(c.border)
            .child(
                div()
                    .id("settings-open-json")
                    .text_size(px(11.0))
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child("Open settings.json")
                    .on_click(cx.listener(|_, _: &ClickEvent, _w, cx| {
                        cx.reveal_path(&config_dir().join("labonair-settings.json"));
                    })),
            )
            .child(
                div()
                    .text_color(c.fg)
                    .font_weight(gpui::FontWeight::SEMIBOLD)
                    .text_size(px(12.5))
                    .child(title),
            )
            .child(
                div()
                    .id("settings-close")
                    .text_color(c.muted)
                    .hover(|s| s.text_color(c.fg))
                    .child("\u{2715}")
                    .on_click(cx.listener(|this, _: &ClickEvent, window, cx| {
                        this.request_close(window, cx)
                    })),
            )
            .into_any_element()
    }
}

pub(crate) fn section_label(text: &'static str, c: &Palette) -> impl IntoElement {
    div()
        .pt_3()
        .pb_1()
        .text_size(px(11.0))
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(c.muted)
        .child(text)
}

pub(crate) fn bridge_switch_row(
    title: &'static str,
    desc: &'static str,
    on: bool,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, &mut Context<SettingsView>) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py_2()
        .border_b_1()
        .border_color(c.border)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_0p5()
                .flex_1()
                .min_w_0()
                .child(div().text_color(c.fg).child(title))
                .child(div().text_size(px(11.0)).text_color(c.muted).child(desc)),
        )
        .child(
            div()
                .id(SharedString::from(format!("mcp-sw-{title}")))
                .w(px(38.0))
                .h(px(20.0))
                .rounded_full()
                .flex()
                .items_center()
                .px(px(2.0))
                .bg(if on { c.accent } else { c.border })
                .child(
                    div()
                        .size(px(16.0))
                        .rounded_full()
                        .bg(c.bg)
                        .when(on, |d| d.ml(px(16.0))),
                )
                .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| f(this, cx))),
        )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bridge_int_row(
    title: &'static str,
    value: i64,
    min: i64,
    max: i64,
    step: i64,
    c: &Palette,
    cx: &mut Context<SettingsView>,
    f: impl Fn(&mut SettingsView, i64, &mut Context<SettingsView>) + Clone + 'static,
) -> impl IntoElement {
    let f_dec = f.clone();
    div()
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .py_2()
        .border_b_1()
        .border_color(c.border)
        .child(div().text_color(c.fg).flex_1().min_w_0().child(title))
        .child(
            div()
                .flex()
                .items_center()
                .gap_1()
                .child(
                    div()
                        .id(SharedString::from(format!("mcp-dec-{title}")))
                        .size(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(c.border)
                        .text_color(c.fg)
                        .hover(|s| s.bg(c.border))
                        .child("\u{2212}")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            f_dec(this, (value - step).clamp(min, max), cx)
                        })),
                )
                .child(
                    div()
                        .min_w(px(52.0))
                        .text_center()
                        .text_color(c.fg)
                        .child(SharedString::from(value.to_string())),
                )
                .child(
                    div()
                        .id(SharedString::from(format!("mcp-inc-{title}")))
                        .size(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_sm()
                        .border_1()
                        .border_color(c.border)
                        .text_color(c.fg)
                        .hover(|s| s.bg(c.border))
                        .child("+")
                        .on_click(cx.listener(move |this, _: &ClickEvent, _w, cx| {
                            f(this, (value + step).clamp(min, max), cx)
                        })),
                ),
        )
}
