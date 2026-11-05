# Theme Mode and Selector Action Classification

**Status:** Static action, keymap, Settings-effect, and route classification; native dispatch and paired macOS acceptance pending

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned Workspace handles `theme::ToggleMode`; theme_settings owns theme/icon-theme value conversion; ThemeSelector owns theme browsing, preview, confirmation, and dismissal. Labonair `labonair-theme` owns theme catalogs and `ThemeStore`, `labonair-theme-ui` applies/persists palette choices, Settings owns values, and Workspace/Shell compose palette and menu entry points.

## Evidence boundary

This packet classifies two default-macOS-bound actions. The exact platform bindings and contexts remain in [action-requirements.md](action-requirements.md): macOS `cmd-k cmd-shift-t` for ToggleMode and `cmd-k cmd-t` for ThemeSelector; Linux/Windows use the corresponding `ctrl-k` chords. No Atom, Vim/Helix, or other profile overlay entries were found for these actions. Static keymap presence does not establish registration or dispatch.

## Action semantics and routes

| Requirement | Pinned behavior | Labonair route and gap |
|---|---|---|
| [ACT-MACOS-theme-ToggleMode](action-requirements.md) | Switch a dynamic theme setting between Light and Dark; from System or unset, choose the mode opposite the currently resolved system appearance. `theme_settings::set_mode` also updates icon-theme mode state. For a static theme selection, that helper converts to dynamic System with the default One Light/One Dark pair instead of applying the computed mode; the pinned Workspace test asserts this persisted conversion. | Theme palette has explicit System, Light, and Dark choices routed through `general.theme`. No matching one-step ToggleMode action or default key binding exists. The color mode value and app theme ID are stored separately, and icon theme remains one selected ID. |
| [ACT-MACOS-theme_selector-Toggle](action-requirements.md) | Toggle the ThemeSelector modal in the active Workspace. Optional `themes_filter` narrows its catalog. Moving selection previews a theme through in-memory Settings; Confirm persists a static theme name or updates the selected light/dark slot and may change mode; cancel/dismiss restores the original Settings. The app Settings menu also exposes Select Theme. | The global titlebar Themes item opens the command palette Themes page, whose provider offers App Themes and Color Mode. Theme preview and activation exist through typed palette actions, but no `theme_selector::Toggle` action, `themes_filter` argument, or `cmd-k cmd-t` default binding was found. |

### ToggleMode Settings effects

`ThemeSettings::theme.mode()` supplies the current dynamic mode. With Light or Dark it chooses the other explicit mode; with System or no mode it chooses the opposite of the current effective appearance. The `set_mode` helper then applies the stored ThemeSelection shape. A static ThemeSelection becomes `{ mode: system, light: One Light, dark: One Dark }`; an existing dynamic selection keeps both names and changes its mode. The pinned Workspace test source checks this static-to-dynamic persisted result.

The same helper also updates icon-theme selection. A static icon theme becomes a dynamic selection with the same name in both light/dark slots and the requested mode. An existing dynamic icon selection changes mode while retaining both slots. If the value is absent, the helper adds the default static icon theme.

### ThemeSelector preview and confirmation

The selector opens as a Workspace modal and sorts dark themes before light themes. When the current theme is in the filtered catalog, it is the initial selection. It stores the original ThemeSettings and previews selected rows by replacing the in-memory global value. Confirm persists the selected static name or, for a dynamic selection, the chosen appearance slot while retaining the opposite slot. If System mode is active and the chosen theme matches current system appearance, System remains selected; otherwise the mode changes to the selected appearance. Dismissal and cancellation restore the original global settings. A filtered list may omit the currently active theme; no false active row is selected in that case.

## Labonair theme and Settings routes

Labonair's Theme command provider exposes App Themes and Color Mode palette submenus. The global titlebar Themes item opens the Themes page. `labonair-theme-ui` previews themes in ThemeStore and activates/persists the selected `appearance.app_theme` ID; its Color Mode handler persists `general.theme` as System, Light, or Dark and applies the preference. These are useful component routes, but neither is the pinned toggle action or ThemeSelector modal.

Local values have a different shape: `general.theme` stores one System/Light/Dark preference; `appearance.app_theme` stores one app theme ID; `appearance.icon_theme` stores one icon-theme ID. The local ThemeStore resolves the system preference and the selected theme through its registries. It has no matching dynamic `theme.mode/light/dark` or `icon_theme.mode/light/dark` settings object. The local palette route has no `themes_filter` action argument or pinned default shortcuts.

## Dynamic Settings effects

| Requirement | Pinned effect | Labonair relationship |
|---|---|---|
| [SET-appearance-theme](settings-requirements.md) | Default is Dynamic System, One Light, and One Dark. ToggleMode changes its mode; ThemeSelector may persist a static or dynamic selection. | Color mode is `general.theme`; one active theme ID is `appearance.app_theme`. There is no paired dynamic Theme object. |
| [SET-appearance-theme-3eb68736](settings-requirements.md) | Static ThemeName selection, inactive under the packaged dynamic default. ThemeSelector updates a static selection; ToggleMode converts it to the default dynamic pair. | `appearance.app_theme` is a single ID without the pinned static/dynamic union. |
| [SET-appearance-theme-mode](settings-requirements.md) | Defaults to System. ToggleMode chooses Light/Dark from current mode/appearance, subject to the static-to-dynamic conversion above. | `general.theme` offers System/Light/Dark through the palette and Settings; no single ToggleMode action. |
| [SET-appearance-theme-light](settings-requirements.md) | Defaults to One Light. ThemeSelector updates this slot for a dynamic selection; static conversion seeds One Light. | No separate light slot; the active registry ID is one value. |
| [SET-appearance-theme-dark](settings-requirements.md) | Defaults to One Dark. ThemeSelector updates this slot for a dynamic selection; static conversion seeds One Dark. | No separate dark slot; the active registry ID is one value. |
| [SET-appearance-icon-theme](settings-requirements.md) | Defaults to static Zed (Default). ToggleMode can convert it to a dynamic selection. | `appearance.icon_theme` stores one icon-theme ID. |
| [SET-appearance-icon-theme-string](settings-requirements.md) | Derived selection name; a dynamic icon-theme mode can choose different light/dark names. | The selected icon-theme ID is one persisted value. |
| [SET-appearance-icon-theme-4a748f7a](settings-requirements.md) | Static/dynamic icon-theme selection branch. ToggleMode duplicates a static name into both slots or changes the mode of a dynamic selection. | No local static/dynamic icon-theme union or mode value. |
| [SET-appearance-icon-theme-light](settings-requirements.md) | Dynamic light icon-theme slot; ToggleMode retains the slot while changing mode, and static conversion copies the existing name into it. | No separate light icon-theme slot. |
| [SET-appearance-icon-theme-dark](settings-requirements.md) | Dynamic dark icon-theme slot; ToggleMode retains the slot while changing mode, and static conversion copies the existing name into it. | No separate dark icon-theme slot. |

All these values remain user Settings. Static defaults and handler source do not demonstrate Settings controls, reset, save/reload, scope, or live update.

## Acceptance evidence still required

- Invoke ToggleMode with explicit Light/Dark, System on light/dark OS appearance, dynamic custom light/dark names, a static custom theme, and absent settings. Verify the resulting active theme and persisted Theme and Icon Theme values.
- Exercise ThemeSelector filtering, empty matches, an active theme outside the filter, selection preview, Confirm, Escape, outside dismiss, modal toggling, and theme-load errors. Verify preview rollback and persisted value after reload.
- Compare local titlebar/palette/theme Settings routes with the pinned action contexts and chord precedence. Verify there is no hidden local default binding and record the palette's separate explicit mode/theme commands.
- Capture native macOS registration and dispatch, paired ThemeSelector/palette previews and cancel states, both system appearances, and narrow-window layout. Static source/tests inspected, not run, do not satisfy runtime acceptance.

## Source records inspected

- Pinned actions: `zed-refrence/zed/crates/workspace/src/workspace.rs`, `zed-refrence/zed/crates/theme_selector/src/theme_selector.rs`, `zed-refrence/zed/crates/theme_settings/src/settings.rs`, `zed-refrence/zed/crates/zed_actions/src/lib.rs`, and app menu registration.
- Pinned Settings and bindings: `zed-refrence/zed/assets/settings/default.json` and default macOS/Linux/Windows keymaps.
- Labonair theme state and values: `crates/theme/src/store.rs`, `crates/theme/src/command_provider.rs`, `crates/theme-ui/src/apply.rs`, `crates/theme-ui/src/command_provider.rs`, and Settings content/concrete types.
- Labonair user entry points: `crates/shell/src/titlebar.rs`, `crates/shell/src/settings_services.rs`, `crates/shell/src/actions.rs`, and Workspace command palette composition.
- Related registers: [action requirements](action-requirements.md), [Settings requirements](settings-requirements.md), and the [Appearance capability crosswalk](feature-crosswalk.md).
