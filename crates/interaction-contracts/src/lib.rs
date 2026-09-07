//! Stable UI-free identities shared by interactive feature modules.
//!
//! This crate intentionally contains no keymap behavior and no GPUI types.
//! It exists so command metadata and keymap resolution can depend on the same
//! identity contract without depending on each other.

/// Stable identity of a rebindable keyboard shortcut.
///
/// The values are intentionally kept compatible with the existing keymap
/// registry. Presentation, default bindings, persistence, and conflict
/// handling remain owned by `labonair-keymap`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum ShortcutId {
    CommandPalette,
    ShortcutsOpen,
    TabNew,
    TabNewPreview,
    TabNewEditor,
    TabClose,
    TabNext,
    TabPrev,
    TabSelect1,
    TabSelect2,
    TabSelect3,
    TabSelect4,
    TabSelect5,
    TabSelect6,
    TabSelect7,
    TabSelect8,
    TabSelect9,
    PaneSplitRight,
    PaneSplitDown,
    PaneClose,
    PaneFocusNext,
    SearchFocus,
    SidebarToggle,
    ViewZenMode,
    ViewZoomIn,
    ViewZoomOut,
    ViewZoomReset,
}
