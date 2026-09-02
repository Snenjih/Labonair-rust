//! The unified bar-item ("unibar") model — port of
//! `reference-src/src/modules/settings/lib/barItems.ts` +
//! `barItemLayout.tsx`.
//!
//! One registry ([`BarItemId`]) + one placement store ([`Placements`]) drives
//! **both** the titlebar (`AppShell::render_header`) and the statusbar
//! (`AppShell::render_statusbar`): every item is individually placeable into
//! `{ bar: titlebar|statusbar, side: left|right, hidden }` through the shared
//! right-click menu and persisted via the backend
//! `settings_set_bar_item_placement`.
//!
//! This module is the *pure* half (enum, ordering, defaults, persistence
//! plumbing, divider rule). The rendering half — `render_bar_item` /
//! `build_bar_bucket` / the context menu — lives in `app_shell.rs` because it
//! needs `Context<AppShell>` for listeners and entity reads.

use std::collections::HashMap;

use gpui::Global;
use serde_json::{json, Value};

use crate::components::IconName;

/// Bumped whenever the settings window edits a bar-item placement so the live
/// `AppShell` bar re-reads the persisted blob (`cx.observe_global`).
#[derive(Default)]
pub struct BarLayoutTick(pub u64);

impl Global for BarLayoutTick {}

/// Every positionable titlebar/statusbar item. Order of variants is irrelevant;
/// [`BAR_ITEM_ORDER`] defines bucket iteration order (matches the reference
/// `BAR_ITEM_ORDER`, *not* declaration order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarItemId {
    // Titlebar badges
    Updater,
    Notifications,
    JumpHosts,
    AgentAccess,
    Transfers,
    Bookmarks,
    // Sidebar dock panel toggles
    ExplorerPanel,
    SnippetsPanel,
    SourceControlPanel,
    TabsPanel,
    // Statusbar info
    CwdBreadcrumb,
    CursorPosition,
    PreviewUrl,
    // AI cluster
    AiMini,
    AiPanel,
}

/// Stable registration order — bucket iteration uses this, 1:1 with the
/// reference `BAR_ITEM_ORDER`.
pub const BAR_ITEM_ORDER: [BarItemId; 15] = [
    BarItemId::Updater,
    BarItemId::Notifications,
    BarItemId::JumpHosts,
    BarItemId::AgentAccess,
    BarItemId::Transfers,
    BarItemId::Bookmarks,
    BarItemId::ExplorerPanel,
    BarItemId::SnippetsPanel,
    BarItemId::SourceControlPanel,
    BarItemId::TabsPanel,
    BarItemId::CwdBreadcrumb,
    BarItemId::CursorPosition,
    BarItemId::PreviewUrl,
    BarItemId::AiMini,
    BarItemId::AiPanel,
];

impl BarItemId {
    /// The serde string used in the persisted `barItemPlacements` blob and in
    /// the backend call — identical to the reference `BarItemId` union members.
    pub fn as_str(self) -> &'static str {
        match self {
            BarItemId::Updater => "updater",
            BarItemId::Notifications => "notifications",
            BarItemId::JumpHosts => "jumpHosts",
            BarItemId::AgentAccess => "agentAccess",
            BarItemId::Transfers => "transfers",
            BarItemId::Bookmarks => "bookmarks",
            BarItemId::ExplorerPanel => "explorerPanel",
            BarItemId::SnippetsPanel => "snippetsPanel",
            BarItemId::SourceControlPanel => "sourceControlPanel",
            BarItemId::TabsPanel => "tabsPanel",
            BarItemId::CwdBreadcrumb => "cwdBreadcrumb",
            BarItemId::CursorPosition => "cursorPosition",
            BarItemId::PreviewUrl => "previewUrl",
            BarItemId::AiMini => "aiMini",
            BarItemId::AiPanel => "aiPanel",
        }
    }

    /// Parse a persisted-blob key back to its variant.
    pub fn from_key(s: &str) -> Option<Self> {
        BAR_ITEM_ORDER.into_iter().find(|id| id.as_str() == s)
    }

    /// Divider-placement category (port of `BAR_ITEM_CATEGORY`): a divider only
    /// ever separates two clusters of *different* category.
    pub fn category(self) -> BarCategory {
        match self {
            BarItemId::Updater
            | BarItemId::Notifications
            | BarItemId::JumpHosts
            | BarItemId::AgentAccess
            | BarItemId::Transfers
            | BarItemId::Bookmarks => BarCategory::Badge,
            BarItemId::ExplorerPanel
            | BarItemId::SnippetsPanel
            | BarItemId::SourceControlPanel
            | BarItemId::TabsPanel => BarCategory::Panel,
            BarItemId::CwdBreadcrumb | BarItemId::CursorPosition | BarItemId::PreviewUrl => {
                BarCategory::Info
            }
            BarItemId::AiMini | BarItemId::AiPanel => BarCategory::Ai,
        }
    }

    /// Icon for the icon-only toggle items (panel toggles + AI toggles).
    pub fn icon(self) -> Option<IconName> {
        match self {
            BarItemId::ExplorerPanel => Some(IconName::FolderTree),
            BarItemId::SnippetsPanel => Some(IconName::Zap),
            BarItemId::SourceControlPanel => Some(IconName::GitBranch),
            BarItemId::TabsPanel => Some(IconName::PanelTop),
            BarItemId::AiMini => Some(IconName::MessageSquare),
            BarItemId::AiPanel => Some(IconName::PanelBottom),
            _ => None,
        }
    }

    /// `title=` tooltip text for the toggle items (port of `PANEL_TITLES`).
    pub fn toggle_title(self) -> &'static str {
        match self {
            BarItemId::ExplorerPanel => "Explorer (Cmd+B)",
            BarItemId::SnippetsPanel => "Snippets",
            BarItemId::SourceControlPanel => "Source Control",
            BarItemId::TabsPanel => "Tabs",
            BarItemId::AiMini => "Conversation",
            BarItemId::AiPanel => "AI Panel",
            _ => "",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarCategory {
    Badge,
    Panel,
    Info,
    Ai,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarLoc {
    Titlebar,
    Statusbar,
}

impl BarLoc {
    fn as_str(self) -> &'static str {
        match self {
            BarLoc::Titlebar => "titlebar",
            BarLoc::Statusbar => "statusbar",
        }
    }
    fn parse_str(s: &str) -> Option<Self> {
        match s {
            "titlebar" => Some(BarLoc::Titlebar),
            "statusbar" => Some(BarLoc::Statusbar),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BarSide {
    Left,
    Right,
}

impl BarSide {
    fn as_str(self) -> &'static str {
        match self {
            BarSide::Left => "left",
            BarSide::Right => "right",
        }
    }
    fn parse_str(s: &str) -> Option<Self> {
        match s {
            "left" => Some(BarSide::Left),
            "right" => Some(BarSide::Right),
            _ => None,
        }
    }
}

/// Which bar the trigger button renders in, which end of that bar, and whether
/// the button is hidden (the feature / shortcut / command stay live).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarItemPlacement {
    pub bar: BarLoc,
    pub side: BarSide,
    pub hidden: bool,
}

/// Fresh-install layout — reproduces `DEFAULT_BAR_ITEM_PLACEMENTS` exactly.
pub fn default_placement(id: BarItemId) -> BarItemPlacement {
    use BarItemId::*;
    use BarLoc::*;
    use BarSide::*;
    let (bar, side) = match id {
        Updater | Notifications | JumpHosts | AgentAccess | Transfers | Bookmarks => {
            (Titlebar, Right)
        }
        ExplorerPanel | SnippetsPanel | SourceControlPanel | TabsPanel | CwdBreadcrumb => {
            (Statusbar, Left)
        }
        CursorPosition | PreviewUrl | AiMini | AiPanel => (Statusbar, Right),
    };
    BarItemPlacement {
        bar,
        side,
        hidden: false,
    }
}

/// The live placement table: compiled-in defaults merged with whatever the
/// user has persisted.
#[derive(Debug, Clone)]
pub struct Placements {
    map: HashMap<BarItemId, BarItemPlacement>,
}

impl Default for Placements {
    fn default() -> Self {
        Self {
            map: BAR_ITEM_ORDER
                .into_iter()
                .map(|id| (id, default_placement(id)))
                .collect(),
        }
    }
}

impl Placements {
    /// Defaults overlaid with the persisted `barItemPlacements` blob
    /// (`{ itemId: { bar, side, hidden } }`, as produced by the backend).
    pub fn from_blob(blob: &serde_json::Map<String, Value>) -> Self {
        let mut me = Self::default();
        for (key, raw) in blob {
            let Some(id) = BarItemId::from_key(key) else {
                continue;
            };
            let Some(p) = me.map.get_mut(&id) else {
                continue;
            };
            if let Some(bar) = raw
                .get("bar")
                .and_then(Value::as_str)
                .and_then(BarLoc::parse_str)
            {
                p.bar = bar;
            }
            if let Some(side) = raw
                .get("side")
                .and_then(Value::as_str)
                .and_then(BarSide::parse_str)
            {
                p.side = side;
            }
            if let Some(hidden) = raw.get("hidden").and_then(Value::as_bool) {
                p.hidden = hidden;
            }
        }
        me
    }

    pub fn get(&self, id: BarItemId) -> BarItemPlacement {
        self.map
            .get(&id)
            .copied()
            .unwrap_or_else(|| default_placement(id))
    }

    pub fn set(&mut self, id: BarItemId, placement: BarItemPlacement) {
        self.map.insert(id, placement);
    }

    /// Ids visible in a given `(bar, side)` bucket, in [`BAR_ITEM_ORDER`].
    pub fn visible_items_for(&self, bar: BarLoc, side: BarSide) -> Vec<BarItemId> {
        BAR_ITEM_ORDER
            .into_iter()
            .filter(|id| {
                let p = self.get(*id);
                p.bar == bar && p.side == side && !p.hidden
            })
            .collect()
    }

    /// Which sidebar dock a panel-toggle opens into, driven by the toggle's own
    /// `side` (independent of which bar its button sits in) — port of the
    /// `useBarPanelSync` / `sidebarSlotLogic` rule.
    pub fn panel_dock_side(&self, id: BarItemId) -> BarSide {
        self.get(id).side
    }
}

/// JSON patch for one `settings_set_bar_item_placement` call.
pub fn placement_patch(p: BarItemPlacement) -> Value {
    json!({
        "bar": p.bar.as_str(),
        "side": p.side.as_str(),
        "hidden": p.hidden,
    })
}

/// The `withDividers` insertion rule: a divider is emitted only between two
/// adjacent items of *different* category, never leading/trailing. Returns the
/// indices *before which* a divider should be inserted.
pub fn divider_indices(categories: &[BarCategory]) -> Vec<usize> {
    let mut out = Vec::new();
    for i in 1..categories.len() {
        if categories[i] != categories[i - 1] {
            out.push(i);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn order_and_str_are_stable() {
        assert_eq!(BAR_ITEM_ORDER.len(), 15);
        assert_eq!(BAR_ITEM_ORDER[0].as_str(), "updater");
        assert_eq!(BAR_ITEM_ORDER[14].as_str(), "aiPanel");
        for id in BAR_ITEM_ORDER {
            assert_eq!(BarItemId::from_key(id.as_str()), Some(id));
        }
    }

    #[test]
    fn defaults_match_reference() {
        let p = Placements::default();
        // All badges → titlebar/right.
        for id in [
            BarItemId::Updater,
            BarItemId::AgentAccess,
            BarItemId::Bookmarks,
        ] {
            assert_eq!(p.get(id).bar, BarLoc::Titlebar);
            assert_eq!(p.get(id).side, BarSide::Right);
        }
        // Panel toggles + breadcrumb → statusbar/left.
        for id in [
            BarItemId::ExplorerPanel,
            BarItemId::TabsPanel,
            BarItemId::CwdBreadcrumb,
        ] {
            assert_eq!(p.get(id).bar, BarLoc::Statusbar);
            assert_eq!(p.get(id).side, BarSide::Left);
        }
        // cursor / preview / ai → statusbar/right.
        for id in [
            BarItemId::CursorPosition,
            BarItemId::PreviewUrl,
            BarItemId::AiMini,
            BarItemId::AiPanel,
        ] {
            assert_eq!(p.get(id).bar, BarLoc::Statusbar);
            assert_eq!(p.get(id).side, BarSide::Right);
        }
    }

    #[test]
    fn visible_items_for_respects_bucket_and_hidden() {
        let mut p = Placements::default();
        let tr = p.visible_items_for(BarLoc::Titlebar, BarSide::Right);
        assert_eq!(tr.first(), Some(&BarItemId::Updater));
        assert!(tr.contains(&BarItemId::Bookmarks));

        p.set(
            BarItemId::Updater,
            BarItemPlacement {
                bar: BarLoc::Titlebar,
                side: BarSide::Right,
                hidden: true,
            },
        );
        assert!(!p
            .visible_items_for(BarLoc::Titlebar, BarSide::Right)
            .contains(&BarItemId::Updater));

        p.set(
            BarItemId::Bookmarks,
            BarItemPlacement {
                bar: BarLoc::Statusbar,
                side: BarSide::Left,
                hidden: false,
            },
        );
        assert!(p
            .visible_items_for(BarLoc::Statusbar, BarSide::Left)
            .contains(&BarItemId::Bookmarks));
    }

    #[test]
    fn blob_round_trips_through_from_blob() {
        let mut p = Placements::default();
        p.set(
            BarItemId::AiPanel,
            BarItemPlacement {
                bar: BarLoc::Titlebar,
                side: BarSide::Left,
                hidden: true,
            },
        );
        let patch = placement_patch(p.get(BarItemId::AiPanel));
        let mut blob = serde_json::Map::new();
        blob.insert("aiPanel".into(), patch);
        blob.insert("bogusKey".into(), json!({ "bar": "x" })); // ignored

        let restored = Placements::from_blob(&blob);
        let ai = restored.get(BarItemId::AiPanel);
        assert_eq!(ai.bar, BarLoc::Titlebar);
        assert_eq!(ai.side, BarSide::Left);
        assert!(ai.hidden);
        // Untouched item keeps its default.
        assert_eq!(
            restored.get(BarItemId::Updater),
            default_placement(BarItemId::Updater)
        );
    }

    #[test]
    fn divider_rule_only_between_different_categories() {
        use BarCategory::*;
        // badge badge panel info info ai
        let cats = [Badge, Badge, Panel, Info, Info, Ai];
        assert_eq!(divider_indices(&cats), vec![2, 3, 5]);
        assert_eq!(divider_indices(&[Badge]), Vec::<usize>::new());
        assert_eq!(divider_indices(&[]), Vec::<usize>::new());
    }
}
