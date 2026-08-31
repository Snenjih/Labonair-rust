use std::collections::HashMap;

use tauri::menu::MenuItem;

/// Handles for the native-menu items that mirror a customizable frontend
/// keyboard shortcut (see `src/modules/shortcuts/lib/nativeMenuSync.ts`),
/// captured once when `build_menu()` constructs the menu so this command
/// doesn't have to re-traverse every submenu on each sync call.
pub struct MenuItemRegistry(pub HashMap<String, MenuItem<tauri::Wry>>);

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MenuAccelUpdate {
    pub menu_item_ids: Vec<String>,
    pub accelerator: Option<String>,
}

#[tauri::command]
pub async fn menu_sync_accelerators(
    registry: tauri::State<'_, MenuItemRegistry>,
    updates: Vec<MenuAccelUpdate>,
) -> Result<(), String> {
    for update in updates {
        for id in &update.menu_item_ids {
            match registry.0.get(id) {
                Some(item) => item
                    .set_accelerator(update.accelerator.as_deref())
                    .map_err(|e| e.to_string())?,
                None => log::warn!("menu_sync_accelerators: unknown menu item id \"{id}\""),
            }
        }
    }
    Ok(())
}
