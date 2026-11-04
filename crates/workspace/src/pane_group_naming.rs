//! Label, icon and hover list of a tab that is a pane group (R10-002).
//!
//! Rules (plan "Tab-Name/Icon einer Gruppe"):
//!
//! * single pane: the pane's own title and icon;
//! * group: the user's `custom_title`, otherwise `"<active pane title> +N"`
//!   where `N` is the number of *other* panes;
//! * icon: the shared kind icon when every pane has the same [`PaneKind`],
//!   otherwise [`GENERIC_PANES_ICON`];
//! * hover list: one [`PaneListEntry`] per pane in visual order, the active
//!   one flagged.
//!
//! Deviation from a literal reading of the plan: a user-set `custom_title`
//! also wins for a *single* pane, because `Tab::label` has always let a rename
//! freeze the tab name and a renamed single terminal must not lose it.
//!
//! The functions are pure: they take the visual pane order and the
//! [`PaneMetaStore`] and never touch GPUI.

use labonair_ui_kit::IconName;

use crate::pane::PaneId;
use crate::pane_kind::{PaneKind, PaneMetaStore};

/// Icon of a group whose panes are of mixed kinds.
pub const GENERIC_PANES_ICON: IconName = IconName::Split;

/// One row of the hover popover.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneListEntry {
    pub id: PaneId,
    pub icon: IconName,
    pub title: String,
    pub active: bool,
}

/// What the tab strip shows for a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupLabel {
    pub title: String,
    pub icon: IconName,
    /// Number of panes the label is computed from.
    pub pane_count: usize,
}

/// The hover list, in `order` (visual leaf order). Panes unknown to `store`
/// are skipped. `active` falls back to the first listed pane when it is not
/// among them, so exactly one entry is flagged (unless the list is empty).
pub fn pane_list(
    order: &[PaneId],
    store: &PaneMetaStore,
    active: Option<PaneId>,
) -> Vec<PaneListEntry> {
    let known: Vec<PaneId> = order
        .iter()
        .copied()
        .filter(|id| store.contains(*id))
        .collect();
    let active = active
        .filter(|a| known.contains(a))
        .or_else(|| known.first().copied());
    known
        .into_iter()
        .filter_map(|id| {
            store.get(id).map(|m| PaneListEntry {
                id,
                icon: m.icon,
                title: m.display_title().to_string(),
                active: Some(id) == active,
            })
        })
        .collect()
}

/// The group's tab label and icon.
pub fn group_label(
    order: &[PaneId],
    store: &PaneMetaStore,
    active: Option<PaneId>,
    custom_title: Option<&str>,
) -> GroupLabel {
    let custom = custom_title.filter(|t| !t.is_empty());
    let list = pane_list(order, store, active);
    let pane_count = list.len();
    let Some(current) = list.iter().find(|e| e.active) else {
        return GroupLabel {
            title: custom.unwrap_or("Empty").to_string(),
            icon: GENERIC_PANES_ICON,
            pane_count: 0,
        };
    };

    let title = match custom {
        Some(t) => t.to_string(),
        None if pane_count == 1 => current.title.clone(),
        None => format!("{} +{}", current.title, pane_count - 1),
    };

    let kinds: Vec<PaneKind> = list.iter().filter_map(|e| store.kind(e.id)).collect();
    let uniform = kinds.windows(2).all(|w| w[0] == w[1]);
    let icon = if pane_count == 1 {
        // A single pane keeps its own icon, which an adapter may have refined.
        current.icon
    } else if uniform {
        kinds.first().map_or(GENERIC_PANES_ICON, |k| k.icon())
    } else {
        GENERIC_PANES_ICON
    };
    GroupLabel {
        title,
        icon,
        pane_count,
    }
}

/// Suppression conditions for the hover list (edge cases: no preview while a
/// context menu, a drag or another popover is open).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HoverGate {
    pub context_menu_open: bool,
    pub dragging: bool,
    pub popover_open: bool,
}

/// Whether the hover list may open for a group of `pane_count` panes.
/// Single-pane tabs have nothing to list.
pub fn hover_list_allowed(gate: HoverGate, pane_count: usize) -> bool {
    pane_count >= 2 && !gate.context_menu_open && !gate.dragging && !gate.popover_open
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pane_kind::PaneMeta;

    fn store(items: &[(PaneId, PaneKind, &str)]) -> PaneMetaStore {
        let mut s = PaneMetaStore::new();
        for (id, kind, title) in items {
            s.insert(*id, PaneMeta::new(*kind).with_title(*title));
        }
        s
    }

    #[test]
    fn single_pane_uses_its_own_label_and_icon() {
        let s = store(&[(1, PaneKind::Editor, "main.rs")]);
        let l = group_label(&[1], &s, Some(1), None);
        assert_eq!(l.title, "main.rs");
        assert_eq!(l.icon, PaneKind::Editor.icon());
        assert_eq!(l.pane_count, 1);
    }

    #[test]
    fn single_pane_keeps_a_refined_adapter_icon() {
        let mut s = store(&[(1, PaneKind::Editor, "a")]);
        s.set_icon(1, IconName::Globe);
        assert_eq!(group_label(&[1], &s, Some(1), None).icon, IconName::Globe);
    }

    #[test]
    fn group_title_is_active_label_plus_count() {
        let s = store(&[
            (1, PaneKind::Terminal, "zsh"),
            (2, PaneKind::Terminal, "htop"),
            (3, PaneKind::Terminal, "vim"),
        ]);
        let l = group_label(&[1, 2, 3], &s, Some(2), None);
        assert_eq!(l.title, "htop +2");
        // Same kind everywhere -> that kind's icon.
        assert_eq!(l.icon, PaneKind::Terminal.icon());
        assert_eq!(l.pane_count, 3);
    }

    #[test]
    fn mixed_kinds_get_the_generic_icon() {
        let s = store(&[
            (1, PaneKind::Terminal, "zsh"),
            (2, PaneKind::Editor, "a.rs"),
        ]);
        let l = group_label(&[1, 2], &s, Some(2), None);
        assert_eq!(l.title, "a.rs +1");
        assert_eq!(l.icon, GENERIC_PANES_ICON);
    }

    #[test]
    fn custom_title_wins_for_single_and_group() {
        let s = store(&[
            (1, PaneKind::Terminal, "zsh"),
            (2, PaneKind::Terminal, "vim"),
        ]);
        assert_eq!(
            group_label(&[1, 2], &s, Some(1), Some("build")).title,
            "build"
        );
        assert_eq!(group_label(&[1], &s, Some(1), Some("build")).title, "build");
        // An empty custom title is the same as none.
        assert_eq!(group_label(&[1, 2], &s, Some(1), Some("")).title, "zsh +1");
    }

    #[test]
    fn untitled_panes_fall_back_to_the_kind_label() {
        let mut s = PaneMetaStore::new();
        s.insert(1, PaneMeta::new(PaneKind::Terminal));
        s.insert(2, PaneMeta::new(PaneKind::Preview));
        assert_eq!(group_label(&[1, 2], &s, Some(1), None).title, "Terminal +1");
    }

    #[test]
    fn hover_list_follows_visual_order_and_flags_one_active() {
        let s = store(&[
            (1, PaneKind::Terminal, "zsh"),
            (2, PaneKind::Editor, "a.rs"),
            (3, PaneKind::Sftp, "host"),
        ]);
        let list = pane_list(&[3, 1, 2], &s, Some(1));
        assert_eq!(list.iter().map(|e| e.id).collect::<Vec<_>>(), vec![3, 1, 2]);
        assert_eq!(list.iter().filter(|e| e.active).count(), 1);
        assert!(list[1].active);
        assert_eq!(list[0].icon, PaneKind::Sftp.icon());
        assert_eq!(list[2].title, "a.rs");
    }

    #[test]
    fn stale_active_and_unknown_panes_are_tolerated() {
        let s = store(&[
            (1, PaneKind::Terminal, "zsh"),
            (2, PaneKind::Terminal, "vim"),
        ]);
        // Active pane is gone -> first listed; unknown id 9 is skipped.
        let list = pane_list(&[9, 1, 2], &s, Some(77));
        assert_eq!(list.len(), 2);
        assert!(list[0].active && !list[1].active);
        let empty = group_label(&[], &s, None, None);
        assert_eq!(empty.pane_count, 0);
        assert_eq!(empty.icon, GENERIC_PANES_ICON);
        assert_eq!(group_label(&[], &s, None, Some("x")).title, "x");
    }

    #[test]
    fn hover_list_is_suppressed_by_open_menus_drags_and_popovers() {
        let open = HoverGate::default();
        assert!(hover_list_allowed(open, 2));
        assert!(!hover_list_allowed(open, 1));
        for gate in [
            HoverGate {
                context_menu_open: true,
                ..open
            },
            HoverGate {
                dragging: true,
                ..open
            },
            HoverGate {
                popover_open: true,
                ..open
            },
        ] {
            assert!(!hover_list_allowed(gate, 3));
        }
    }
}
