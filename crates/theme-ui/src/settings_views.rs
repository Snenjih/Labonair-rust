//! Theme-owned views embedded inline on the Settings "Themes" page
//! (ADR 0006): the colour-theme grid and the icon-theme picker.
//!
//! Both are *row stacks*: the Settings page supplies the card, these views
//! contribute rows separated by hairlines. They read and write only the theme
//! owner's state (`ThemeStore` plus the persisted selection through this
//! crate's [`crate::activate_app_theme`] policy); Settings never touches it.

use std::path::PathBuf;

use gpui::prelude::FluentBuilder;
use gpui::{
    canvas, div, img, px, App, Bounds, ClickEvent, Context, Entity, InteractiveElement,
    IntoElement, ListAlignment, ListState, ParentElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Styled, Window,
};
use labonair_settings::{Settings as _, SettingsStore, ThemeSettings};
use labonair_theme::registry::Appearance;
use labonair_theme::{IconThemeMeta, Theme, ThemeStore};
use labonair_ui_kit::{
    button, divider, file_icon_path, folder_icon_path, preview_tile, preview_tile_group,
    select_popover, select_trigger, settings_row, svg_path, Axis, ButtonSize, ButtonVariant,
    Palette, PreviewKind, SelectOption, SelectPopoverAnchor, TileMock,
};

/// The persisted per-mode variant name chosen for `family`
/// (`appearance.themeVariantOverrides[family][mode]`), if any.
fn variant_override(overrides: &serde_json::Value, appearance: &str) -> Option<String> {
    overrides.get(appearance)?.as_str().map(str::to_owned)
}

/// The mini-window colours of `family` for one appearance, honouring the user's
/// variant override for that family.
fn family_mock(
    theme: &ThemeStore,
    family: &str,
    appearance: Appearance,
    overrides: &serde_json::Value,
) -> Option<TileMock> {
    let key = match appearance {
        Appearance::Light => "light",
        Appearance::Dark => "dark",
    };
    let (resolved, _warnings): (Theme, _) = theme
        .registry()
        .resolve_family_variant(
            family,
            appearance,
            variant_override(overrides, key).as_deref(),
        )
        .ok()?;
    Some(TileMock::from_theme(&resolved))
}

/// The colour-theme grid: one tile per theme family, each showing its light
/// and dark variants side by side. Selecting a tile activates the family; the
/// per-mode variant overrides still decide which variant a mode uses.
pub struct ColorThemeSettings {
    theme: Entity<ThemeStore>,
    themes_dir: PathBuf,
}

impl ColorThemeSettings {
    pub fn new(theme: Entity<ThemeStore>, themes_dir: PathBuf, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self { theme, themes_dir }
    }
}

impl Render for ColorThemeSettings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let overrides = ThemeSettings::try_get(cx)
            .map(|settings| settings.theme_variant_overrides())
            .unwrap_or_default();

        let (families, active, rejected) = {
            let store = self.theme.read(cx);
            let mut families: Vec<(String, String)> = Vec::new();
            for meta in store.list_themes() {
                if !families.iter().any(|(id, _)| *id == meta.family_id) {
                    families.push((meta.family_id.clone(), meta.family.clone()));
                }
            }
            let active = store
                .registry()
                .family_of(&store.active_theme_id())
                .unwrap_or_else(|| "default".to_owned());
            let rejected: Vec<(SharedString, SharedString)> = store
                .rejected_themes()
                .iter()
                .map(|rejected| {
                    let name = rejected
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| rejected.path.display().to_string());
                    (name.into(), rejected.reason.to_string().into())
                })
                .collect();
            (families, active, rejected)
        };

        let mut group = preview_tile_group("theme-grid", c, active);
        {
            let store = self.theme.read(cx);
            for (id, name) in &families {
                let family_overrides = overrides.get(id).cloned().unwrap_or_default();
                let tile = preview_tile(id.clone(), name.clone(), PreviewKind::Theme);
                let light = family_mock(store, id, Appearance::Light, &family_overrides);
                let dark = family_mock(store, id, Appearance::Dark, &family_overrides);
                group = group.tile(match (light, dark) {
                    (Some(light), Some(dark)) => tile.colors(light, dark),
                    _ => tile.colors(TileMock::from_palette(&c), TileMock::from_palette(&c)),
                });
            }
        }
        let theme = self.theme.clone();
        let group = group.on_select(move |family, _window, app| {
            crate::activate_app_theme(family, &theme, app);
        });

        let themes_dir = self.themes_dir.clone();
        let open_folder = button(
            "themes-open-folder",
            c,
            ButtonVariant::OutlinedGhost,
            ButtonSize::Xs,
        )
        .child("Open themes folder")
        .on_click(move |_: &ClickEvent, _window, app| reveal_dir(&themes_dir, app));

        let hairline = || divider(Axis::Horizontal, c.line_subtle);
        div()
            .id("settings-color-themes")
            .flex()
            .flex_col()
            .w_full()
            .child(
                div()
                    .px(c.control_space(16.0))
                    .py(c.control_space(16.0))
                    .child(group),
            )
            .child(hairline())
            .child(
                settings_row("themes-custom", c, "Custom themes")
                    .description(
                        "Drop theme files into the themes folder. Each file needs both a light and a dark variant.",
                    )
                    .control(open_folder),
            )
            .children(rejected.into_iter().enumerate().flat_map(|(index, (name, reason))| {
                [
                    hairline().into_any_element(),
                    settings_row(SharedString::from(format!("themes-rejected-{index}")), c, name)
                        .description(SharedString::from(format!("Ignored: {reason}")))
                        .into_any_element(),
                ]
            }))
    }
}

/// Reveal `dir` in the platform file manager, creating it first so the first
/// drop of a theme file has somewhere to go.
fn reveal_dir(dir: &std::path::Path, app: &mut App) {
    if let Err(error) = std::fs::create_dir_all(dir) {
        tracing::warn!("could not create themes folder {dir:?}: {error}");
        return;
    }
    app.reveal_path(dir);
}

/// The icon-theme picker: a select over the registered icon themes plus a
/// small strip of sample file and folder glyphs from the active theme.
pub struct IconThemeSettings {
    theme: Entity<ThemeStore>,
    open: Option<Point<Pixels>>,
    trigger_bounds: Option<Bounds<Pixels>>,
    options: ListState,
}

impl IconThemeSettings {
    pub fn new(theme: Entity<ThemeStore>, cx: &mut Context<Self>) -> Self {
        cx.observe(&theme, |_, _, cx| cx.notify()).detach();
        Self {
            theme,
            open: None,
            trigger_bounds: None,
            options: ListState::new(0, ListAlignment::Top, px(320.0)),
        }
    }

    /// Persist and apply the chosen icon theme.
    fn choose(&mut self, id: &str, cx: &mut Context<Self>) {
        if cx.has_global::<SettingsStore>() {
            let persisted = id.to_owned();
            let _ = cx
                .global_mut::<SettingsStore>()
                .update_user_settings(move |content| {
                    content.appearance.icon_theme = Some(persisted)
                });
        }
        let id = id.to_owned();
        self.theme.update(cx, |theme, cx| {
            let _ = theme.set_active_icon_theme(id, cx);
        });
        self.open = None;
        cx.notify();
    }
}

impl Render for IconThemeSettings {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let c = Palette::from_theme(self.theme.read(cx));
        let (themes, active_id): (Vec<IconThemeMeta>, String) = {
            let store = self.theme.read(cx);
            (
                store.list_icon_themes(),
                store.active_icon_theme_id().to_owned(),
            )
        };
        let label = themes
            .iter()
            .find(|meta| meta.id == active_id)
            .map(|meta| meta.name.clone())
            .unwrap_or_else(|| active_id.clone());

        let weak = cx.weak_entity();
        let trigger = select_trigger("icon-theme-select", c, label, self.open.is_some())
            .relative()
            .child(
                canvas(
                    move |bounds, _window, app| {
                        let _ = weak.update(app, |this, cx| {
                            if this.trigger_bounds != Some(bounds) {
                                this.trigger_bounds = Some(bounds);
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full(),
            )
            .on_click(cx.listener(|this, event: &ClickEvent, _window, cx| {
                this.open = match this.open {
                    Some(_) => None,
                    None => Some(
                        this.trigger_bounds
                            .map(|bounds| bounds.bottom_left())
                            .unwrap_or_else(|| event.position()),
                    ),
                };
                cx.notify();
            }));

        let preview = {
            let store = self.theme.read(cx);
            let icons = store.icon_theme();
            let color = icons.color_icons;
            let glyph = |path: SharedString| {
                if color {
                    img(path).size(px(16.0)).flex_none().into_any_element()
                } else {
                    svg_path(path, c.fg).into_any_element()
                }
            };
            let mut strip = div().flex().items_center().gap(c.space(10.0));
            strip = strip.child(glyph(folder_icon_path(icons, "src", false)));
            for name in ["main.rs", "index.ts", "README.md", "Cargo.toml", "app.py"] {
                strip = strip.child(glyph(file_icon_path(icons, name)));
            }
            strip
        };

        let popover = self.open.map(|at| {
            let options: Vec<SelectOption> = themes
                .iter()
                .map(|meta| {
                    (
                        SharedString::from(meta.id.clone()),
                        SharedString::from(meta.name.clone()),
                    )
                })
                .collect();
            if self.options.item_count() != options.len() {
                self.options.reset(options.len());
            }
            let weak = cx.weak_entity();
            let dismiss_weak = weak.clone();
            select_popover(
                SelectPopoverAnchor::new("icon-theme-menu", at, self.options.clone()),
                c,
                &options,
                &active_id,
                move |_window, app| {
                    let _ = dismiss_weak.update(app, |this, cx| {
                        this.open = None;
                        cx.notify();
                    });
                },
                move |id, _window, app| {
                    let id = id.clone();
                    let _ = weak.update(app, |this, cx| this.choose(&id, cx));
                },
            )
        });

        div()
            .id("settings-icon-themes")
            .w_full()
            .child(
                settings_row("icon-theme", c, "Icon theme")
                    .description("The icon set used for files and folders.")
                    .control(
                        div()
                            .flex()
                            .items_center()
                            .gap(c.space(16.0))
                            .child(preview)
                            .child(trigger),
                    ),
            )
            .when_some(popover, |root, popover| root.child(popover))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variant_override_reads_the_requested_mode() {
        let overrides = serde_json::json!({ "light": "Day", "dark": "Night" });
        assert_eq!(
            variant_override(&overrides, "light").as_deref(),
            Some("Day")
        );
        assert_eq!(
            variant_override(&overrides, "dark").as_deref(),
            Some("Night")
        );
        assert_eq!(variant_override(&serde_json::Value::Null, "dark"), None);
    }
}
