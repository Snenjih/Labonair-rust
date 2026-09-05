//! [`AppShell`] — the app's root coordinator, reduced to pure composition in
//! T17-006.
//!
//! `render` composes a small fixed set of children, top to bottom:
//!
//! * the [`Titlebar`] entity (tab strip + `＋` new-tab menu + one right-hand
//!   icon button; redesigned in T18-001);
//! * the [`Workspace`] entity — which now composes the three edge docks and the
//!   split-pane [`PaneGroup`](labonair_workspace::pane_group) itself;
//! * the [`StatusBar`] entity — renders purely from the workspace's
//!   [`StatusItemRegistry`](labonair_panel::StatusItemRegistry) (T17-003);
//! * the two overlay layers — [`ModalLayer`] + [`ToastLayer`] (T17-005).
//!
//! No feature logic lives in `render` anymore: there are no `drain_pending_*`
//! calls, no per-frame `sync_live_bridge`, no `build_palette_data`. The wiring
//! that used to sit in `AppShell::new` moved to [`crate::bootstrap`]; the menu /
//! command-palette action handlers moved to [`crate::actions`]. The concrete
//! panel + feature entities stay here (grouped in [`ShellPanels`]) because the
//! `labonair-panel-* → labonair-workspace` dependency edges make relocating
//! them into `Workspace` a crate cycle — see `docs/architecture.md` §8.4 / §8.9.
//!
//! Window geometry is still persisted every render via
//! [`Self::maybe_persist_geometry`] (throttled) — that is legitimate, not a
//! `drain_*`.

use std::time::{Duration, Instant};

use gpui::prelude::FluentBuilder;
use gpui::{
    div, px, App, Bounds, Context, Entity, FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement, Pixels, Render, Styled, Task, Window, WindowBounds,
};
use labonair_backend::App as Backend;
use labonair_notifications::NotificationCenter;
use tokio::runtime::Handle as TokioHandle;

use labonair_panel_ai::AiChatView;
use labonair_panel_explorer::{BookmarksView, ExplorerView};
use labonair_panel_scm::GitPanelView;
use labonair_panel_snippets::SnippetsView;
use labonair_settings_ui::PreferencesStore;
use labonair_workspace::live_bridge::WorkspaceLiveBridge;
use labonair_workspace::modal_layer::ModalLayer;
use labonair_workspace::status_bar::StatusBar;
use labonair_workspace::toast_layer::ToastLayer;

use crate::background::{BackgroundStore, LayerScope};
use crate::commands::CommandRegistry;
use crate::modals::ShellPalette;
use crate::theme::ThemeStore;
use crate::titlebar::Titlebar;
use crate::updater::UpdaterView;
use crate::window_state;
use crate::workspace::Workspace;

/// Minimum interval between window-geometry writes.
const SAVE_THROTTLE: Duration = Duration::from_millis(1000);

/// The concrete panel / feature entities the shell keeps direct handles to.
///
/// They live in `labonair-shell`, not `Workspace`, because
/// `labonair-panel-{explorer,scm,snippets,ai}` already depend on
/// `labonair-workspace` — storing their concrete `Entity<…>` on `Workspace`
/// would be a dependency cycle (`docs/architecture.md` §8.4). Grouped so
/// `AppShell` stays a thin composition root and the action handlers in
/// [`crate::actions`] have one field to reach through.
pub(crate) struct ShellPanels {
    pub(crate) explorer: Entity<ExplorerView>,
    pub(crate) bookmarks: Entity<BookmarksView>,
    pub(crate) git_panel: Entity<GitPanelView>,
    pub(crate) snippets: Entity<SnippetsView>,
    pub(crate) ai_chat: Entity<AiChatView>,
    pub(crate) updater: Entity<UpdaterView>,
    pub(crate) command_palette: Entity<ShellPalette>,
}

/// The root view: window chrome around the [`Workspace`].
pub struct AppShell {
    pub(crate) theme: Entity<ThemeStore>,
    pub(crate) background: Entity<BackgroundStore>,
    pub(crate) prefs: Entity<PreferencesStore>,
    pub(crate) workspace: Entity<Workspace>,
    pub(crate) titlebar: Entity<Titlebar>,
    pub(crate) panels: ShellPanels,
    pub(crate) status_bar: Entity<StatusBar>,
    /// Every menu / keybind / palette command. Single definition site:
    /// [`register_builtin_commands`](crate::commands::register_builtin_commands)
    /// (T17-007).
    pub(crate) command_registry: CommandRegistry,
    /// The app's single modal-overlay slot (T17-005).
    pub(crate) modal_layer: Entity<ModalLayer>,
    /// The stacked, non-blocking toast overlay (T17-005).
    pub(crate) toast_layer: Entity<ToastLayer<ThemeStore>>,
    /// Real `LiveBridge` for the AI agent — snapshot refreshed event-driven
    /// (T17-006), command queue drained by [`Self`]'s background task.
    pub(crate) live_bridge: WorkspaceLiveBridge,
    _live_drain: Task<()>,
    focus_handle: FocusHandle,
    last_saved: Option<(Bounds<Pixels>, Instant)>,
}

impl AppShell {
    pub fn new(
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        notifications: Entity<NotificationCenter>,
        backend: Backend,
        tokio: TokioHandle,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Re-render on theme changes; every other reactive edge is set up in
        // `bootstrap` (or self-managed by the child entities).
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();

        crate::bootstrap::bootstrap(theme, background, notifications, backend, tokio, window, cx)
    }

    /// The workspace view (for menu / command-palette wiring).
    pub fn workspace(&self) -> &Entity<Workspace> {
        &self.workspace
    }

    /// The central preferences store (T13-001).
    pub fn preferences(&self) -> &Entity<PreferencesStore> {
        &self.prefs
    }

    /// Assemble the shell from its already-built parts (called by
    /// [`crate::bootstrap`]).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_parts(
        theme: Entity<ThemeStore>,
        background: Entity<BackgroundStore>,
        prefs: Entity<PreferencesStore>,
        workspace: Entity<Workspace>,
        titlebar: Entity<Titlebar>,
        panels: ShellPanels,
        status_bar: Entity<StatusBar>,
        command_registry: CommandRegistry,
        modal_layer: Entity<ModalLayer>,
        toast_layer: Entity<ToastLayer<ThemeStore>>,
        live_bridge: WorkspaceLiveBridge,
        live_drain: Task<()>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            theme,
            background,
            prefs,
            workspace,
            titlebar,
            panels,
            status_bar,
            command_registry,
            modal_layer,
            toast_layer,
            live_bridge,
            _live_drain: live_drain,
            focus_handle: cx.focus_handle(),
            last_saved: None,
        }
    }

    /// Save the window geometry at most once per [`SAVE_THROTTLE`].
    fn maybe_persist_geometry(&mut self, window: &Window) {
        let WindowBounds::Windowed(bounds) = window.window_bounds() else {
            return;
        };
        let now = Instant::now();
        let stale = match self.last_saved {
            None => true,
            Some((last, at)) => {
                now.duration_since(at) >= SAVE_THROTTLE && bounds_differ(last, bounds)
            }
        };
        if stale {
            window_state::save(bounds);
            self.last_saved = Some((bounds, now));
        }
    }
}

impl Focusable for AppShell {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let _span =
            tracing::trace_span!(target: "labonair::perf", "render", view = "shell").entered();
        self.maybe_persist_geometry(window);
        // Mirror the async-driven updater dialog + the status-bar-toggled
        // bookmarks popover into the modal layer (both flip flags outside a
        // user action, so `render` is the one place with a `&mut Window`).
        self.sync_updater_modal(window, cx);
        self.sync_bookmarks_modal(window, cx);

        let (bg, ui_font, ui_font_size) = {
            let t = self.theme.read(cx);
            (t.background(), t.ui_font(), t.ui_font_size())
        };
        let background_layer = self.background.read(cx).layer(LayerScope::App);
        let show_statusbar = self.prefs.read(cx).get().zen_mode_show_statusbar;
        let can_split = self.workspace.read(cx).active_is_terminal(cx);
        let has_split = self.workspace.read(cx).active_has_split(cx);

        // The shell root carries only the three genuine window actions; every
        // other menu / keybind action is bridged to a `CommandId` and run
        // through `self.command_registry` (T17-007).
        let root = div()
            .track_focus(&self.focus_handle)
            .key_context("AppShell")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .bg(bg)
            .font(ui_font)
            .text_size(px(ui_font_size))
            .on_action(cx.listener(Self::act_toggle_fullscreen))
            .on_action(cx.listener(Self::act_minimize))
            .on_action(cx.listener(Self::act_zoom_window));
        let root = crate::commands::attach_action_handlers(root, can_split, has_split, cx);

        root.child(self.titlebar.clone())
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .child(self.workspace.clone()),
            )
            .when(show_statusbar, |d| d.child(self.status_bar.clone()))
            .children(background_layer)
            // Overlays: exactly two children (T17-005 / layout contract).
            .child(self.modal_layer.clone())
            .child(self.toast_layer.clone())
    }
}

fn bounds_differ(a: Bounds<Pixels>, b: Bounds<Pixels>) -> bool {
    let d = |x: Pixels, y: Pixels| (f32::from(x) - f32::from(y)).abs() > 2.0;
    d(a.origin.x, b.origin.x)
        || d(a.origin.y, b.origin.y)
        || d(a.size.width, b.size.width)
        || d(a.size.height, b.size.height)
}
