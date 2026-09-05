//! `Dock` — one edge dock (left / right / bottom) that hosts several
//! registered panels with a single active one, plus open / size / zoom state
//! and its own persistence record.
//!
//! Reduced port of `struct Dock` from
//! `zed-refrence/zed/crates/workspace/src/dock.rs`. Kept: multi-panel storage
//! with an active index, `add_panel` / `remove_panel` / `activate_panel` /
//! `toggle_open` / `toggle_panel`, resize with a per-active-panel `min_size`
//! clamp, and a zoom flag. Omitted versus Zed: the `PanelButtons` dock-edge
//! button strip (`docs/architecture.md` §5 — Labonair toggles live in the
//! status bar, T18-003), the `Entity<Dock>` wrapper + `PanelEvent`
//! subscriptions (no panel subscribes to its dock yet; zoom/close are driven
//! from the shell), and the flexible-size / `initial_size_state` machinery.
//!
//! The `Dock` is a plain struct owned by [`crate::Workspace`]; the shell reads
//! its state to render each dock and to drive the resize handle.

use gpui::{px, App, Pixels};
use labonair_panel::{AnyPanelHandle, DockPosition};
use serde::{Deserialize, Serialize};

/// Thickness of the drag handle on a dock's resizable edge. Ported from Zed's
/// `RESIZE_HANDLE_SIZE` (`dock.rs`).
pub const RESIZE_HANDLE_SIZE: Pixels = px(6.0);

/// Default size (width for L/R, height for bottom) of a freshly-created dock,
/// before any persisted value or user resize.
pub fn default_size(position: DockPosition) -> Pixels {
    match position {
        DockPosition::Left => px(260.0),
        DockPosition::Right => px(380.0),
        DockPosition::Bottom => px(320.0),
    }
}

/// Lower bound the resize handle must respect even if the active panel reports
/// no `min_size`.
pub fn min_size(position: DockPosition) -> Pixels {
    match position {
        DockPosition::Left | DockPosition::Right => px(180.0),
        DockPosition::Bottom => px(120.0),
    }
}

/// Upper bound the resize handle must respect.
pub fn max_size(position: DockPosition) -> Pixels {
    match position {
        DockPosition::Left | DockPosition::Right => px(520.0),
        DockPosition::Bottom => px(600.0),
    }
}

/// Stable slug for a [`DockPosition`], used in the persisted [`DockData`].
/// `labonair-panel` deliberately keeps `DockPosition` free of `serde`, so the
/// mapping lives here in the persistence layer.
pub fn position_slug(position: DockPosition) -> &'static str {
    match position {
        DockPosition::Left => "left",
        DockPosition::Right => "right",
        DockPosition::Bottom => "bottom",
    }
}

/// Inverse of [`position_slug`].
pub fn position_from_slug(slug: &str) -> Option<DockPosition> {
    match slug {
        "left" => Some(DockPosition::Left),
        "right" => Some(DockPosition::Right),
        "bottom" => Some(DockPosition::Bottom),
        _ => None,
    }
}

/// One edge dock.
pub struct Dock {
    position: DockPosition,
    panels: Vec<AnyPanelHandle>,
    active: Option<usize>,
    open: bool,
    size: Pixels,
    zoomed: bool,
}

impl Dock {
    /// An empty, closed dock at `position` with the default size.
    pub fn new(position: DockPosition) -> Self {
        Self {
            position,
            panels: Vec::new(),
            active: None,
            open: false,
            size: default_size(position),
            zoomed: false,
        }
    }

    pub fn position(&self) -> DockPosition {
        self.position
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn set_open(&mut self, open: bool) {
        self.open = open;
    }

    pub fn toggle_open(&mut self) {
        self.open = !self.open;
    }

    pub fn is_zoomed(&self) -> bool {
        self.zoomed
    }

    pub fn set_zoomed(&mut self, zoomed: bool) {
        self.zoomed = zoomed;
    }

    pub fn size(&self) -> Pixels {
        self.size
    }

    pub fn is_empty(&self) -> bool {
        self.panels.is_empty()
    }

    pub fn panels(&self) -> &[AnyPanelHandle] {
        &self.panels
    }

    /// Add `panel` to the tail of this dock (no-op if a panel with the same
    /// [`persistent_name`](labonair_panel::PanelHandle::persistent_name) is
    /// already present). The first panel added becomes the active one.
    pub fn add_panel(&mut self, panel: AnyPanelHandle) {
        let name = panel.persistent_name();
        if self.panels.iter().any(|p| p.persistent_name() == name) {
            return;
        }
        self.panels.push(panel);
        if self.active.is_none() {
            self.active = Some(0);
        }
    }

    /// Remove the panel named `name`, fixing up the active index. Returns the
    /// removed handle so a caller ([`crate::Workspace::move_panel`]) can move it
    /// to another dock.
    pub fn remove_panel(&mut self, name: &str) -> Option<AnyPanelHandle> {
        let idx = self
            .panels
            .iter()
            .position(|p| p.persistent_name() == name)?;
        let removed = self.panels.remove(idx);
        self.active = match self.active {
            _ if self.panels.is_empty() => None,
            Some(a) if a == idx => Some(a.min(self.panels.len() - 1)),
            Some(a) if a > idx => Some(a - 1),
            other => other,
        };
        Some(removed)
    }

    pub fn has_panel(&self, name: &str) -> bool {
        self.panels.iter().any(|p| p.persistent_name() == name)
    }

    pub fn active_panel(&self) -> Option<&AnyPanelHandle> {
        self.active.and_then(|i| self.panels.get(i))
    }

    pub fn active_name(&self) -> Option<&'static str> {
        self.active_panel().map(|p| p.persistent_name())
    }

    /// Make `name` the active panel. Returns `false` if this dock has no such
    /// panel (the active index is then left unchanged).
    pub fn activate_panel(&mut self, name: &str) -> bool {
        match self.panels.iter().position(|p| p.persistent_name() == name) {
            Some(i) => {
                self.active = Some(i);
                true
            }
            None => false,
        }
    }

    /// Status-bar-toggle semantics: clicking the already-active panel of an
    /// open dock closes the dock; anything else activates `name` and opens the
    /// dock.
    pub fn toggle_panel(&mut self, name: &str) {
        if self.open && self.active_name() == Some(name) {
            self.open = false;
        } else if self.activate_panel(name) {
            self.open = true;
        }
    }

    /// Resize the dock, clamping to `[max(min_size, active_floor), max_size]`.
    /// `active_floor` is the active panel's own `min_size`, resolved by the
    /// caller (it has the `App` the handle needs).
    pub fn set_size(&mut self, size: Pixels, active_floor: Option<Pixels>) {
        let floor = active_floor
            .unwrap_or(min_size(self.position))
            .max(min_size(self.position));
        self.size = size.max(floor).min(max_size(self.position));
    }

    /// Reorder the panels to match `order` (stable; names not in `order` keep
    /// their relative order and move to the tail, names in `order` but absent
    /// here are ignored). Used when restoring a persisted [`DockData`].
    pub fn apply_order(&mut self, order: &[String]) {
        let rank = |name: &str| order.iter().position(|n| n == name).unwrap_or(usize::MAX);
        let active_name = self.active_name();
        self.panels
            .sort_by_key(|p| (rank(p.persistent_name()), p.persistent_name()));
        if let Some(name) = active_name {
            self.activate_panel(name);
        }
    }

    /// Apply the persisted scalar state (open / size / zoom / active panel).
    /// Panel membership + order must already have been applied.
    pub fn apply_scalars(&mut self, data: &DockData) {
        self.open = data.open;
        self.zoomed = data.zoomed;
        self.set_size(px(data.size), None);
        if let Some(name) = &data.active_panel {
            self.activate_panel(name);
        }
    }

    /// Snapshot for persistence.
    pub fn to_data(&self) -> DockData {
        DockData {
            position: position_slug(self.position).to_string(),
            open: self.open,
            size: f32::from(self.size),
            zoomed: self.zoomed,
            active_panel: self.active_name().map(str::to_string),
            panel_order: self
                .panels
                .iter()
                .map(|p| p.persistent_name().to_string())
                .collect(),
        }
    }

    /// Whether `panel` (already in this dock or not) may live at `to`.
    pub fn panel_allows(&self, name: &str, to: DockPosition, cx: &App) -> bool {
        self.panels
            .iter()
            .find(|p| p.persistent_name() == name)
            .map(|p| p.position_is_valid(to, cx))
            .unwrap_or(false)
    }

    /// Docks the panel `name` (which lives in this dock) may be moved to:
    /// every position except this dock's own and any the panel's
    /// [`position_is_valid`](labonair_panel::PanelHandle::position_is_valid)
    /// rejects. Drives the status-bar button's context menu so it can never
    /// present a move that [`crate::Workspace::move_panel`] would refuse.
    pub fn move_destinations(&self, name: &str, cx: &App) -> Vec<DockPosition> {
        DockPosition::ALL
            .into_iter()
            .filter(|d| *d != self.position)
            .filter(|d| self.panel_allows(name, *d, cx))
            .collect()
    }
}

/// Persisted layout of one dock. The equivalent of Zed's
/// `persistence::model::DockData`, reduced to Labonair's needs
/// (`panel_positions` is unnecessary — a moved panel is simply the one that
/// appears in another dock's `panel_order`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DockData {
    /// [`position_slug`] of the dock this record describes.
    pub position: String,
    pub open: bool,
    /// Width (L/R) or height (bottom) in logical pixels.
    pub size: f32,
    pub zoomed: bool,
    /// `persistent_name` of the active panel, if any.
    pub active_panel: Option<String>,
    /// `persistent_name`s of every panel in this dock, in tab order.
    pub panel_order: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AnyView, EntityId, FocusHandle};
    use labonair_panel::{PanelHandle, PanelIcon};
    use std::sync::Arc;

    /// Minimal `PanelHandle` stub — only the metadata the `Dock` bookkeeping
    /// touches is meaningful; the view-facing methods are never called here.
    struct StubPanel {
        name: &'static str,
        valid: &'static [DockPosition],
    }

    impl PanelHandle for StubPanel {
        fn panel_id(&self) -> EntityId {
            unreachable!()
        }
        fn persistent_name(&self) -> &'static str {
            self.name
        }
        fn title(&self, _cx: &App) -> gpui::SharedString {
            self.name.into()
        }
        fn icon(&self, _cx: &App) -> PanelIcon {
            PanelIcon::Explorer
        }
        fn position(&self, _cx: &App) -> DockPosition {
            DockPosition::Left
        }
        fn position_is_valid(&self, position: DockPosition, _cx: &App) -> bool {
            self.valid.contains(&position)
        }
        fn set_position(&self, _position: DockPosition, _window: &mut gpui::Window, _cx: &mut App) {
        }
        fn default_size(&self, _cx: &App) -> Pixels {
            px(200.0)
        }
        fn min_size(&self, _cx: &App) -> Option<Pixels> {
            None
        }
        fn focus_handle(&self, _cx: &App) -> FocusHandle {
            unreachable!()
        }
        fn to_any(&self) -> AnyView {
            unreachable!()
        }
    }

    fn stub(name: &'static str) -> AnyPanelHandle {
        Arc::new(StubPanel {
            name,
            valid: &[
                DockPosition::Left,
                DockPosition::Right,
                DockPosition::Bottom,
            ],
        })
    }

    #[test]
    fn add_sets_first_active_and_dedupes() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel(stub("explorer"));
        dock.add_panel(stub("scm"));
        dock.add_panel(stub("explorer")); // duplicate ignored
        assert_eq!(dock.panels().len(), 2);
        assert_eq!(dock.active_name(), Some("explorer"));
    }

    #[test]
    fn toggle_panel_opens_switches_and_closes() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel(stub("explorer"));
        dock.add_panel(stub("scm"));
        dock.set_open(false);

        dock.toggle_panel("scm"); // closed → open on scm
        assert!(dock.is_open());
        assert_eq!(dock.active_name(), Some("scm"));

        dock.toggle_panel("explorer"); // open, other panel → switch, stay open
        assert!(dock.is_open());
        assert_eq!(dock.active_name(), Some("explorer"));

        dock.toggle_panel("explorer"); // open, same panel → close
        assert!(!dock.is_open());
    }

    #[test]
    fn remove_fixes_active_index() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel(stub("a"));
        dock.add_panel(stub("b"));
        dock.add_panel(stub("c"));
        dock.activate_panel("c");
        dock.remove_panel("a");
        assert_eq!(dock.active_name(), Some("c"));
        dock.remove_panel("c");
        assert_eq!(dock.active_name(), Some("b"));
        dock.remove_panel("b");
        assert_eq!(dock.active_name(), None);
        assert!(dock.is_empty());
    }

    #[test]
    fn only_active_panel_of_open_dock_reads_as_toggled() {
        // The status-bar button's "active" predicate: dock open AND this is its
        // active panel. A closed dock keeps its active index but has no
        // toggled button.
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel(stub("explorer"));
        dock.add_panel(stub("scm"));
        dock.set_open(true);
        dock.activate_panel("explorer");

        let toggled = |d: &Dock, name: &str| d.is_open() && d.active_name() == Some(name);
        assert!(toggled(&dock, "explorer"));
        assert!(!toggled(&dock, "scm"));

        dock.set_open(false);
        assert!(!toggled(&dock, "explorer"));
        assert_eq!(
            dock.active_name(),
            Some("explorer"),
            "active index retained"
        );
    }

    #[gpui::test]
    fn move_destinations_excludes_current_and_invalid(cx: &mut gpui::TestAppContext) {
        // `explorer` may only live left or right → from the left dock the only
        // offered destination is Right (never Left = current, never Bottom =
        // invalid).
        let mut dock = Dock::new(DockPosition::Left);
        dock.add_panel(Arc::new(StubPanel {
            name: "explorer",
            valid: &[DockPosition::Left, DockPosition::Right],
        }));

        cx.update(|cx| {
            assert_eq!(
                dock.move_destinations("explorer", cx),
                vec![DockPosition::Right]
            );
        });
    }

    #[test]
    fn set_size_clamps_to_position_bounds() {
        let mut dock = Dock::new(DockPosition::Left);
        dock.set_size(px(10.0), None);
        assert_eq!(dock.size(), min_size(DockPosition::Left));
        dock.set_size(px(9999.0), None);
        assert_eq!(dock.size(), max_size(DockPosition::Left));
        dock.set_size(px(300.0), Some(px(320.0)));
        assert_eq!(dock.size(), px(320.0)); // active-panel floor wins
    }

    #[test]
    fn to_data_round_trips_through_apply() {
        let mut dock = Dock::new(DockPosition::Bottom);
        dock.add_panel(stub("git-graph"));
        dock.add_panel(stub("other"));
        dock.activate_panel("other");
        dock.set_open(true);
        dock.set_size(px(280.0), None);
        dock.set_zoomed(true);

        let data = dock.to_data();
        assert_eq!(data.position, "bottom");
        assert_eq!(data.panel_order, vec!["git-graph", "other"]);

        let mut restored = Dock::new(DockPosition::Bottom);
        restored.add_panel(stub("other"));
        restored.add_panel(stub("git-graph"));
        restored.apply_order(&data.panel_order);
        restored.apply_scalars(&data);
        assert_eq!(
            restored.panels()[0].persistent_name(),
            "git-graph",
            "order restored"
        );
        assert!(restored.is_open() && restored.is_zoomed());
        assert_eq!(restored.active_name(), Some("other"));
        assert_eq!(restored.size(), px(280.0));
    }
}
