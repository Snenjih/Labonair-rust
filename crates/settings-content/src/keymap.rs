//! `keymap` — deliberately thin. Actual keybindings stay in their own
//! `keymap.json` (T19-008); `SettingsContent` only carries which base
//! keymap preset a fresh install (or a reset) starts from.

use crate::MergeFrom;
use serde::{Deserialize, Serialize};

/// A base keybinding preset new keymap overrides are seeded from.
#[derive(
    Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize, schemars::JsonSchema,
)]
#[serde(rename_all = "camelCase")]
pub enum BaseKeymap {
    /// Labonair's own defaults (`crates/backend`'s built-in keybind table).
    #[default]
    Native,
    VsCode,
    JetBrains,
}

impl MergeFrom for BaseKeymap {
    fn merge_from(&mut self, other: &Self) {
        *self = *other;
    }
}

#[derive(
    Clone, Debug, Default, PartialEq, Serialize, Deserialize, schemars::JsonSchema, MergeFrom,
)]
#[serde(default, rename_all = "camelCase")]
pub struct KeymapContent {
    pub base_keymap: Option<BaseKeymap>,
}

impl KeymapContent {
    pub fn defaults() -> Self {
        Self {
            base_keymap: Some(BaseKeymap::Native),
        }
    }
}
