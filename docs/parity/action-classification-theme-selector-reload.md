# Theme Selector Reload Action Classification

**Status:** Linux action declaration and Labonair ThemeStore route classified; pinned handler, action reachability, macOS registration, and native acceptance remain unresolved

**Baseline:** Zed commit `3569541038dd51524b03998ba4d38d253cb54f80`

**Owner:** Pinned ThemeSelector declares the action; the intended catalog owner is Theme. Labonair `labonair-theme` owns `ThemeRegistry` and `ThemeStore`; `labonair-theme-ui` owns theme selection surfaces and contributes palette behavior.

## Evidence boundary

The Linux runtime action inventory registers `theme_selector::Reload` with no arguments and the label `theme selector: reload`. Its source declaration documents the intent as “Reloads all themes from disk.” No default macOS, Linux, or Windows keymap binding or profile-layer entry was found. This is Linux registration evidence only; macOS registration has not been established.

The pinned ThemeSelector source declares the action but contains no matching handler or call site. Its initializer registers only the Theme and Icon Theme `Toggle` actions. No direct menu invocation was found; Command Palette discovery and programmatic invocation remain unverified. The declaration comment is intent evidence, not proof that invoking the action reloads anything.

The separate pinned `theme_settings::reload_theme` route resolves the currently selected theme from the already-loaded registry and refreshes windows. It is not evidence that `theme_selector::Reload` rescans theme files or replaces the full registry.

## Action classification

| Requirement | Pinned declaration and behavior | Labonair route and gap |
|---|---|---|
| [ACT-REGISTRY-LINUX-theme_selector-Reload](registered-actions-unbound.md) | No arguments. The action documentation says it should reload all themes from disk. No handler, caller, menu entry, default binding, or profile binding was found, so runtime effect, availability, failure behavior, and whether it includes icon themes remain unresolved. | Theme (`labonair-theme`) has `ThemeStore::reload_user_themes`, backed by `ThemeRegistry::load_user_themes`. Both are explicitly deferred under R08-011 and are not connected to a user surface or file watcher. The UI owner is `labonair-theme-ui`; no reload action or palette entry exists. |

## Labonair reload API and ownership

`ThemeRegistry::load_user_themes` replaces the non-built-in families from JSON files in a supplied directory. It skips unreadable or malformed files, returns warnings, and retains the built-in family. `ThemeStore::reload_user_themes` re-resolves the active family and falls back to the built-in theme if that family disappeared. These APIs are test-retained extension adapters, not a user workflow. Their directory scan and file reads are synchronous, so a future UI route must move filesystem work off the GPUI foreground thread.

The matching Labonair owner is Themes. Keep registry loading and active-theme resolution in `labonair-theme`; a future user entry point belongs in `labonair-theme-ui` and must use the existing Themes/Workspace composition. Settings stores the selected theme ID; the reload operation has no separate setting or persisted action state. Icon-theme reload is a distinct owner method and is not evidence that the pinned color Theme Selector action reloads icons.

## Acceptance still required

- Confirm whether the pinned macOS build registers this Linux-catalog action. Discover whether it has a reachable menu, palette, extension, or programmatic route.
- Attempt dispatch in the active ThemeSelector and outside it. Establish whether a handler exists at runtime and whether the documented reload operation changes the catalog, current preview, and active theme.
- Exercise added, edited, removed, unreadable, and malformed theme files; verify warning delivery, selected-theme fallback, selector refresh, and state after restart. Determine explicitly whether icon themes are in scope.
- Compare the pinned route with the local ThemeStore test API. Before adding a user route, define Settings persistence when the selected theme disappears and perform file I/O asynchronously.
- Capture paired native macOS evidence for action reachability, catalog refresh, active-theme recovery, and error states if the action is available.

## Source records inspected

- Pinned action declaration and ThemeSelector registration: `zed-refrence/zed/crates/theme_selector/src/theme_selector.rs`.
- Pinned current-theme reload and registry initialization: `zed-refrence/zed/crates/theme_settings/src/theme_settings.rs`.
- Pinned action inventory and keymap/profile assets: `docs/parity/registered-actions-unbound.md` and `zed-refrence/zed/assets/keymaps/`.
- Labonair registry/store: `crates/theme/src/registry.rs`, `crates/theme/src/store.rs`, and `crates/theme/src/import.rs`.
- Canonical ownership: [Theme mode and selector packet](action-classification-theme-mode-selector.md) and [`docs/capabilities.md`](../capabilities.md).
