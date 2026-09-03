//! Status-bar item contracts.
//!
//! Ported from `zed-refrence/zed/crates/workspace/src/status_bar.rs`
//! (`trait StatusItemView` + `struct HideStatusItem`). The new layout
//! (`docs/architecture.md` §4) drops Zed's titlebar scope, so an item only
//! chooses between the left and right side of the status bar and may describe
//! how it hides itself.

use std::sync::Arc;

use gpui::{AnyElement, AnyView, EntityId, Global};
use labonair_gpui_ext::prelude::*;

/// Which side of the status bar an item defaults to.
///
/// Zed's `barItemPlacements` schema had a titlebar side as well; the new
/// `statusBarItemPlacements` (`docs/architecture.md` §4 "What is removed") has
/// only left/right, so this enum has exactly two variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StatusSide {
    /// Left cluster — by default the panel toggles.
    Left,
    /// Right cluster — by default the info dropdowns.
    Right,
}

/// Describes that a status-bar item can be hidden by the user.
///
/// Port of `HideStatusItem` from
/// `zed-refrence/zed/crates/workspace/src/status_bar.rs`. Zed stores an
/// `Arc<dyn Fn(&mut SettingsContent)>` and persists the change through
/// `update_settings_file`. Labonair has no `SettingsContent` type yet
/// (`labonair-settings-content` is a later phase) and the task explicitly
/// forbids a `serde`/persistence dependency here, so for now this carries a
/// plain callback against the running `App`; T18-005 swaps the body for the
/// settings-file write without changing this type's shape.
#[derive(Clone)]
pub struct StatusItemHide {
    hide: Arc<dyn Fn(&mut App) + Send + Sync>,
}

impl StatusItemHide {
    /// Wrap a hide action.
    pub fn new(hide: impl Fn(&mut App) + Send + Sync + 'static) -> Self {
        Self {
            hide: Arc::new(hide),
        }
    }

    /// Run the hide action.
    pub fn apply(&self, cx: &mut App) {
        (self.hide)(cx)
    }
}

/// An item rendered into the status bar.
///
/// Reduced port of `trait StatusItemView` from
/// `zed-refrence/zed/crates/workspace/src/status_bar.rs`. Zed's
/// `set_active_pane_item` hook is omitted — Labonair status items observe the
/// workspace directly. `Render` stays a supertrait (as in Zed) so items are
/// ordinary GPUI views; the task's `render(&mut self, window, cx) ->
/// AnyElement` is exposed as [`StatusItem::render_status`] to avoid colliding
/// with `Render::render`.
pub trait StatusItem: Render {
    /// Stable id, used as the key in the placement/hidden settings map.
    fn id(&self) -> &'static str;

    /// Side the item shows on until the user moves it.
    fn default_side(&self) -> StatusSide;

    /// Render the item's content for the status bar row.
    fn render_status(&mut self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement;

    /// How this item hides itself, or `None` if it is inherently conditional on
    /// another user-visible setting (Zed's rule for returning `None`).
    fn hide(&self, cx: &App) -> Option<StatusItemHide> {
        let _ = cx;
        None
    }
}

/// Object-safe view of a [`StatusItem`] entity.
///
/// Analogue of `trait StatusItemViewHandle` in the same Zed file: the status
/// bar stores `Arc<dyn StatusItemHandle>` and forwards through each item's
/// `Entity`.
pub trait StatusItemHandle: Send + Sync {
    /// Entity id of the underlying item view.
    fn item_id(&self) -> EntityId;
    /// See [`StatusItem::id`].
    fn id(&self, cx: &App) -> &'static str;
    /// See [`StatusItem::default_side`].
    fn default_side(&self, cx: &App) -> StatusSide;
    /// See [`StatusItem::hide`].
    fn hide(&self, cx: &App) -> Option<StatusItemHide>;
    /// Type-erased view, for rendering inside the status bar.
    fn to_any(&self) -> AnyView;
}

impl<T: StatusItem + 'static> StatusItemHandle for Entity<T> {
    fn item_id(&self) -> EntityId {
        self.entity_id()
    }

    fn id(&self, cx: &App) -> &'static str {
        self.read(cx).id()
    }

    fn default_side(&self, cx: &App) -> StatusSide {
        self.read(cx).default_side()
    }

    fn hide(&self, cx: &App) -> Option<StatusItemHide> {
        self.read(cx).hide(cx)
    }

    fn to_any(&self) -> AnyView {
        self.clone().into()
    }
}

/// Type-erased, cheaply cloned handle to a registered status item.
pub type AnyStatusItemHandle = Arc<dyn StatusItemHandle>;

/// Builds a fresh status-item view. Stored in [`StatusItemRegistry`].
pub type StatusItemConstructor =
    Arc<dyn Fn(&mut Window, &mut App) -> AnyStatusItemHandle + Send + Sync>;

/// One status-item type's registration record.
pub struct StatusItemRegistration {
    /// [`StatusItem::id`] of the registered type.
    pub id: &'static str,
    /// Side the item shows on before the user moves it.
    pub default_side: StatusSide,
    /// Constructor invoked lazily when the status bar is built.
    pub build: StatusItemConstructor,
}

/// The set of every status-item type known to the running app.
///
/// Same rationale as [`crate::PanelRegistry`]: `labonair-shell` declares the
/// items once, the workspace's status bar reads them back. Method surface is
/// frozen now so T17-003 can wire it without churn.
#[derive(Default)]
pub struct StatusItemRegistry {
    items: Vec<StatusItemRegistration>,
}

impl StatusItemRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a status-item type. A later registration with the same `id`
    /// replaces the earlier one.
    pub fn register(&mut self, registration: StatusItemRegistration) {
        if let Some(slot) = self.items.iter_mut().find(|i| i.id == registration.id) {
            *slot = registration;
        } else {
            self.items.push(registration);
        }
    }

    /// All registrations, in registration order.
    pub fn iter(&self) -> impl Iterator<Item = &StatusItemRegistration> {
        self.items.iter()
    }

    /// Registrations whose `default_side` equals `side`.
    pub fn for_side(&self, side: StatusSide) -> impl Iterator<Item = &StatusItemRegistration> {
        self.items.iter().filter(move |i| i.default_side == side)
    }

    /// Look up a registration by [`StatusItem::id`].
    pub fn get(&self, id: &str) -> Option<&StatusItemRegistration> {
        self.items.iter().find(|i| i.id == id)
    }

    /// Number of registered status-item types.
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Whether no status-item type is registered.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

impl Global for StatusItemRegistry {}

#[cfg(test)]
mod tests {
    use super::*;

    fn stub_item(id: &'static str, side: StatusSide) -> StatusItemRegistration {
        StatusItemRegistration {
            id,
            default_side: side,
            // Never invoked by the registry's bookkeeping methods.
            build: Arc::new(|_, _| unreachable!("stub constructor")),
        }
    }

    #[test]
    fn register_and_lookup() {
        let mut reg = StatusItemRegistry::new();
        assert!(reg.is_empty());
        reg.register(stub_item("notifications", StatusSide::Right));
        reg.register(stub_item("cwd", StatusSide::Right));
        assert_eq!(reg.len(), 2);
        assert!(reg.get("cwd").is_some());
        assert!(reg.get("missing").is_none());
    }

    #[test]
    fn register_replaces_duplicate_id() {
        let mut reg = StatusItemRegistry::new();
        reg.register(stub_item("updater", StatusSide::Left));
        reg.register(stub_item("updater", StatusSide::Right));
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get("updater").unwrap().default_side, StatusSide::Right);
    }

    #[test]
    fn for_side_filters() {
        let mut reg = StatusItemRegistry::new();
        reg.register(stub_item("toggles", StatusSide::Left));
        reg.register(stub_item("notifications", StatusSide::Right));
        reg.register(stub_item("cwd", StatusSide::Right));
        let right: Vec<_> = reg.for_side(StatusSide::Right).map(|i| i.id).collect();
        assert_eq!(right, ["notifications", "cwd"]);
        assert_eq!(reg.for_side(StatusSide::Left).count(), 1);
    }
}
