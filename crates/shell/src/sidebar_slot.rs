//! Pure decision logic for a dock sidebar slot — a Rust port of the reference
//! `reference-src/src/modules/statusbar/lib/sidebarSlotLogic.ts` (+ its test
//! file). No GPUI rendering, no IO; unit-tested below.
//!
//! The app shell has two independent slots (left + right); each holds a
//! [`SidebarSlot`] and drives its toggle button / resize handle through these
//! functions.
//!
//! T17-001: the slot no longer stores a hard-coded `SidebarPanel` enum — it
//! stores the panel's [`persistent_name`](labonair_panel::Panel::persistent_name)
//! as a [`SharedString`], resolved against the `Workspace`'s `PanelRegistry`.
//! T17-002 replaces this ad-hoc per-side state with a real `Dock` model.

use gpui::SharedString;

/// The panel name a slot falls back to when nothing else resolves.
pub const DEFAULT_PANEL: &str = "explorer";

/// Below this fraction of the window width a slot counts as "collapsed" — not
/// just `<= 0` — because a resize observer can briefly report a tiny nonzero
/// width during layout churn. `minSize` (~180px) is comfortably above 1% on
/// any realistic window, so this can't misfire on a genuine small-but-real
/// open size. Mirrors `COLLAPSED_THRESHOLD_PCT` in the reference.
pub const COLLAPSED_THRESHOLD_PCT: f32 = 1.0;

/// `true` when `size_pct` (0..100) is below the collapsed threshold.
pub fn is_collapsed(size_pct: f32) -> bool {
    size_pct < COLLAPSED_THRESHOLD_PCT
}

/// What a toggle click should do to the slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToggleAction {
    /// Open the slot (it was collapsed) and show `next_panel`.
    Expand,
    /// Close the slot.
    Collapse,
    /// Already open — just switch the shown panel to `next_panel`.
    Switch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToggleResult {
    /// `None` when the action is [`ToggleAction::Collapse`].
    pub next_panel: Option<SharedString>,
    pub action: ToggleAction,
}

/// Clicking the already-active panel collapses the slot (or re-expands it if it
/// had been dragged to 0 without changing panel); clicking a different panel
/// switches to it, expanding the slot if it was collapsed. Port of
/// `resolveToggle`.
pub fn resolve_toggle(
    current_panel: &str,
    requested_panel: &str,
    current_size_pct: f32,
) -> ToggleResult {
    if current_panel == requested_panel {
        if is_collapsed(current_size_pct) {
            return ToggleResult {
                next_panel: Some(requested_panel.to_owned().into()),
                action: ToggleAction::Expand,
            };
        }
        return ToggleResult {
            next_panel: None,
            action: ToggleAction::Collapse,
        };
    }
    ToggleResult {
        next_panel: Some(requested_panel.to_owned().into()),
        action: if is_collapsed(current_size_pct) {
            ToggleAction::Expand
        } else {
            ToggleAction::Switch
        },
    }
}

/// What a manual drag of the resize handle should leave `active_panel` as —
/// collapsing clears it, dragging back open restores the last-active panel.
/// Port of `resolveResize`.
pub fn resolve_resize(
    size_pct: f32,
    current_panel: Option<&str>,
    last_active_panel: Option<&str>,
) -> Option<SharedString> {
    if is_collapsed(size_pct) {
        return None;
    }
    Some(
        current_panel
            .or(last_active_panel)
            .unwrap_or(DEFAULT_PANEL)
            .to_owned()
            .into(),
    )
}

/// One dock slot's live state (left or right edge). The two slots are fully
/// independent — both can be open at once, showing different panels.
#[derive(Debug, Clone)]
pub struct SidebarSlot {
    pub open: bool,
    /// Current width in px.
    pub width: f32,
    /// The [`persistent_name`](labonair_panel::Panel::persistent_name) of the
    /// panel this slot shows (kept even while collapsed, so re-expanding
    /// restores it).
    pub panel: SharedString,
    /// Width to restore to on `expand()` — tracked here rather than relying on
    /// a fragile pre-collapse memory.
    pub last_open_width: f32,
}

impl SidebarSlot {
    pub fn new(open: bool, width: f32, panel: impl Into<SharedString>) -> Self {
        Self {
            open,
            width,
            panel: panel.into(),
            last_open_width: width,
        }
    }

    /// Apply a toggle for `panel`. The `open` flag stands in for "not
    /// collapsed" (the % nuance in [`resolve_toggle`] only matters for a
    /// handle dragged to zero, which the resize path handles separately).
    pub fn toggle(&mut self, panel: impl Into<SharedString>) {
        let panel = panel.into();
        let pct = if self.open { 50.0 } else { 0.0 };
        match resolve_toggle(self.panel.as_ref(), panel.as_ref(), pct).action {
            ToggleAction::Collapse => self.open = false,
            ToggleAction::Expand | ToggleAction::Switch => {
                self.panel = panel;
                self.open = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPLORER: &str = "explorer";
    const SCM: &str = "source-control";

    // ── ported from sidebarSlotLogic.test.ts ──────────────────────────────

    #[test]
    fn is_collapsed_uses_the_one_percent_cutoff() {
        assert!(is_collapsed(0.0));
        assert!(is_collapsed(0.9));
        assert!(!is_collapsed(1.0));
        assert!(!is_collapsed(25.0));
    }

    #[test]
    fn toggle_same_panel_open_collapses() {
        let r = resolve_toggle(EXPLORER, EXPLORER, 25.0);
        assert_eq!(r.action, ToggleAction::Collapse);
        assert_eq!(r.next_panel, None);
    }

    #[test]
    fn toggle_same_panel_collapsed_reexpands() {
        let r = resolve_toggle(EXPLORER, EXPLORER, 0.0);
        assert_eq!(r.action, ToggleAction::Expand);
        assert_eq!(r.next_panel, Some(EXPLORER.into()));
    }

    #[test]
    fn toggle_other_panel_open_switches() {
        let r = resolve_toggle(EXPLORER, SCM, 25.0);
        assert_eq!(r.action, ToggleAction::Switch);
        assert_eq!(r.next_panel, Some(SCM.into()));
    }

    #[test]
    fn toggle_other_panel_collapsed_expands() {
        let r = resolve_toggle(EXPLORER, SCM, 0.5);
        assert_eq!(r.action, ToggleAction::Expand);
        assert_eq!(r.next_panel, Some(SCM.into()));
    }

    #[test]
    fn resize_below_threshold_clears_the_panel() {
        assert_eq!(resolve_resize(0.5, Some(EXPLORER), Some(SCM)), None);
    }

    #[test]
    fn resize_open_keeps_current_then_last_then_explorer() {
        assert_eq!(
            resolve_resize(25.0, Some(SCM), Some(EXPLORER)),
            Some(SCM.into())
        );
        assert_eq!(resolve_resize(25.0, None, Some(SCM)), Some(SCM.into()));
        assert_eq!(resolve_resize(25.0, None, None), Some(EXPLORER.into()));
    }

    #[test]
    fn slot_toggle_round_trips() {
        let mut s = SidebarSlot::new(true, 250.0, EXPLORER);
        s.toggle(EXPLORER); // same panel, open → collapse
        assert!(!s.open);
        s.toggle(EXPLORER); // same panel, collapsed → expand
        assert!(s.open && s.panel.as_ref() == EXPLORER);
        s.toggle(SCM); // other panel → switch
        assert!(s.open && s.panel.as_ref() == SCM);
    }
}
