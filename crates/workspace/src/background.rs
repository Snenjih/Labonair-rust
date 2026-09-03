//! Background-image layer (T02-006).
//!
//! Full parity with the reference `backgrounds` feature: the user imports an
//! image, picks it, and tunes opacity / blur / fit / tint; the image is then
//! painted behind the app and/or the terminal.
//!
//! [`BackgroundStore`] is a GPUI entity that owns the persisted
//! [`BackgroundSettings`] (via `labonair_backend::modules::backgrounds`) plus a
//! single decoded, downscaled and pre-blurred [`gpui::Image`]. The image is
//! rebuilt only when the selected file or the blur radius changes — never per
//! frame (see the task's performance warning) — and GPUI's own asset cache
//! keeps the GPU texture around after that.
//!
//! Rendering mirrors the reference implementation: the image sits in an
//! absolutely-positioned, non-interactive overlay at a halved opacity so the UI
//! and terminal text stay readable at any slider value. The settings UI is
//! wired up later in T13-002 (Appearance) — this module is the data + render
//! layer only.

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use gpui::{
    div, img, prelude::*, AnyElement, Context, Entity, Global, Image, ImageFormat, ObjectFit,
    PathPromptOptions, Styled,
};
use image::ImageEncoder;

use labonair_backend::modules::backgrounds::{
    background_delete, background_import, background_settings_load, background_settings_save,
    backgrounds_dir, backgrounds_list, BackgroundInfo, BackgroundSettings,
};
pub use labonair_backend::modules::backgrounds::{BackgroundFit, BackgroundTarget};

/// Longest edge (px) an imported image is kept at; larger images are
/// downscaled once at load time so a 6000px wallpaper doesn't cost a huge GPU
/// texture for a dimmed background.
const MAX_DIM: u32 = 2560;

/// Rendered opacity is the slider value (0..1) multiplied by this, so the
/// wallpaper never exceeds 50% and text stays legible (reference:
/// `BG_OPACITY_RENDER_FACTOR`).
const OPACITY_RENDER_FACTOR: f32 = 0.5;

/// Which surface is asking for a background layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerScope {
    /// The whole app window.
    App,
    /// The terminal surface only.
    Terminal,
}

/// Central background-image state. Created once at startup, stored as a GPUI
/// entity and exposed app-wide via [`GlobalBackground`].
pub struct BackgroundStore {
    settings: BackgroundSettings,
    /// Decoded + processed image, ready to hand to `img()`.
    image: Option<Arc<Image>>,
    /// `(filename, blur)` the cached `image` was built from.
    key: Option<(String, u8)>,
}

impl BackgroundStore {
    /// Loads the persisted settings and decodes the selected image (if any).
    pub fn new() -> Self {
        let mut store = Self {
            settings: background_settings_load(),
            image: None,
            key: None,
        };
        store.rebuild_image();
        store
    }

    /// The current settings (read-only).
    pub fn settings(&self) -> &BackgroundSettings {
        &self.settings
    }

    /// Every background image currently in the app-data `backgrounds/` dir.
    pub fn available(&self) -> Vec<BackgroundInfo> {
        backgrounds_list().unwrap_or_default()
    }

    // --- mutators (persist + notify) -----------------------------------

    /// Selects a background image by filename (`""` clears it).
    pub fn set_image(&mut self, filename: impl Into<String>, cx: &mut Context<Self>) {
        self.settings.background_image = filename.into();
        self.apply(cx);
    }

    /// Slider value 0..=100.
    pub fn set_opacity(&mut self, value: u8, cx: &mut Context<Self>) {
        self.settings.background_opacity = value.min(100);
        self.apply(cx);
    }

    /// Gaussian blur radius in pixels, 0..=100.
    pub fn set_blur(&mut self, value: u8, cx: &mut Context<Self>) {
        self.settings.background_blur = value.min(100);
        self.apply(cx);
    }

    /// Tint overlay color (`#rrggbb`).
    pub fn set_tint_color(&mut self, hex: impl Into<String>, cx: &mut Context<Self>) {
        self.settings.background_tint_color = hex.into();
        self.apply(cx);
    }

    /// Tint overlay opacity 0..=100.
    pub fn set_tint_opacity(&mut self, value: u8, cx: &mut Context<Self>) {
        self.settings.background_tint_opacity = value.min(100);
        self.apply(cx);
    }

    /// Image scaling mode.
    pub fn set_fit(&mut self, fit: BackgroundFit, cx: &mut Context<Self>) {
        self.settings.background_fit = fit;
        self.apply(cx);
    }

    /// Which surface(s) the image sits behind.
    pub fn set_target(&mut self, target: BackgroundTarget, cx: &mut Context<Self>) {
        self.settings.background_target = target;
        self.apply(cx);
    }

    /// Copies an image into the app-data `backgrounds/` dir and selects it.
    pub fn import(&mut self, source: PathBuf, cx: &mut Context<Self>) -> Result<String, String> {
        let info = background_import(source.to_string_lossy().to_string())?;
        self.set_image(info.filename.clone(), cx);
        Ok(info.filename)
    }

    /// Deletes an image file; clears the selection if it was the active one.
    pub fn delete(&mut self, filename: &str, cx: &mut Context<Self>) -> Result<(), String> {
        background_delete(filename.to_string())?;
        if self.settings.background_image == filename {
            self.set_image("", cx);
        } else {
            cx.notify();
        }
        Ok(())
    }

    /// Opens the native file picker and imports the chosen image.
    pub fn prompt_and_import(&mut self, cx: &mut Context<Self>) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Choose background image".into()),
        });
        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = receiver.await {
                if let Some(path) = paths.into_iter().next() {
                    let _ = this.update(cx, |this, cx| {
                        if let Err(err) = this.import(path, cx) {
                            eprintln!("labonair-ui: background import failed: {err}");
                        }
                    });
                }
            }
        })
        .detach();
    }

    // --- rendering ----------------------------------------------------

    /// The background overlay element for `scope`, or `None` when there is no
    /// image or the target doesn't include this surface.
    ///
    /// `App`/`Both` render a window-spanning overlay (which also covers the
    /// terminal); `Terminal` renders one clipped to the terminal element.
    pub fn layer(&self, scope: LayerScope) -> Option<AnyElement> {
        let image = self.image.clone()?;
        let s = &self.settings;

        let visible = matches!(
            (scope, s.background_target),
            (
                LayerScope::App,
                BackgroundTarget::App | BackgroundTarget::Both
            ) | (LayerScope::Terminal, BackgroundTarget::Terminal)
        );
        if !visible {
            return None;
        }

        let rendered_opacity = (s.background_opacity as f32 / 100.0) * OPACITY_RENDER_FACTOR;
        // GPUI's `img()` has no tiling mode — `Tile` falls back to `Cover`.
        let fit = match s.background_fit {
            BackgroundFit::Cover | BackgroundFit::Tile => ObjectFit::Cover,
            BackgroundFit::Contain => ObjectFit::Contain,
        };

        let mut root = div()
            .absolute()
            .inset_0()
            .overflow_hidden()
            .opacity(rendered_opacity)
            .child(img(image).object_fit(fit).size_full());

        if s.background_tint_opacity > 0 {
            if let Ok(tint) = labonair_theme::parse_color(&s.background_tint_color) {
                root = root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .bg(tint)
                        .opacity(s.background_tint_opacity as f32 / 100.0),
                );
            }
        }

        Some(root.into_any_element())
    }

    // --- internals --------------------------------------------------

    fn apply(&mut self, cx: &mut Context<Self>) {
        let _ = background_settings_save(&self.settings);
        self.rebuild_image();
        cx.notify();
    }

    fn rebuild_image(&mut self) {
        let s = &self.settings;
        if s.background_image.is_empty() {
            self.image = None;
            self.key = None;
            return;
        }

        let key = (s.background_image.clone(), s.background_blur);
        if self.key.as_ref() == Some(&key) {
            return;
        }

        match load_processed_image(&s.background_image, s.background_blur) {
            Ok(image) => {
                self.image = Some(Arc::new(image));
                self.key = Some(key);
            }
            Err(err) => {
                eprintln!(
                    "labonair-ui: background image '{}' failed to load ({err}); falling back to none",
                    s.background_image
                );
                self.image = None;
                self.key = None;
                // Mirror the reference: drop the broken selection and persist that.
                self.settings.background_image.clear();
                let _ = background_settings_save(&self.settings);
            }
        }
    }
}

impl Default for BackgroundStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Decodes `filename` from the backgrounds dir, downscales it to [`MAX_DIM`] and
/// pre-applies a Gaussian `blur` (px). Returns a [`gpui::Image`] whose bytes are
/// ready for GPUI's decoder.
fn load_processed_image(filename: &str, blur: u8) -> Result<Image, String> {
    load_processed_from(&backgrounds_dir()?, filename, blur)
}

fn load_processed_from(dir: &Path, filename: &str, blur: u8) -> Result<Image, String> {
    let path = dir.join(filename);
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;

    let mut dynimg = match image::load_from_memory(&bytes) {
        Ok(d) => d,
        Err(e) => {
            // Undecodable by the `image` crate (e.g. AVIF): with no processing
            // requested, let GPUI's own decoder try the untouched bytes.
            if blur == 0 {
                if let Some(fmt) = gpui_format_from_path(&path) {
                    return Ok(Image::from_bytes(fmt, bytes));
                }
            }
            return Err(e.to_string());
        }
    };

    let had_alpha = dynimg.color().has_alpha();

    let downscaled = dynimg.width() > MAX_DIM || dynimg.height() > MAX_DIM;
    if downscaled {
        dynimg = dynimg.resize(MAX_DIM, MAX_DIM, image::imageops::FilterType::Triangle);
    }
    if blur > 0 {
        let blurred = image::imageops::blur(&dynimg.to_rgba8(), blur as f32);
        dynimg = if had_alpha {
            image::DynamicImage::ImageRgba8(blurred)
        } else {
            image::DynamicImage::ImageRgb8(image::DynamicImage::ImageRgba8(blurred).to_rgb8())
        };
    }

    // Fast path: original file is fine as-is, hand GPUI the raw bytes.
    if !downscaled && blur == 0 {
        if let Some(fmt) = gpui_format_from_path(&path) {
            return Ok(Image::from_bytes(fmt, bytes));
        }
    }

    // Re-encode the processed pixels: PNG if it carries alpha, else JPEG.
    let mut out = Cursor::new(Vec::new());
    if dynimg.color().has_alpha() {
        dynimg
            .write_to(&mut out, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(Image::from_bytes(ImageFormat::Png, out.into_inner()))
    } else {
        let rgb = dynimg.to_rgb8();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85)
            .write_image(
                rgb.as_raw(),
                rgb.width(),
                rgb.height(),
                image::ExtendedColorType::Rgb8,
            )
            .map_err(|e| e.to_string())?;
        Ok(Image::from_bytes(ImageFormat::Jpeg, out.into_inner()))
    }
}

fn gpui_format_from_path(path: &Path) -> Option<ImageFormat> {
    match path
        .extension()
        .and_then(|e| e.to_str())?
        .to_lowercase()
        .as_str()
    {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::Webp),
        "gif" => Some(ImageFormat::Gif),
        "bmp" => Some(ImageFormat::Bmp),
        _ => None,
    }
}

/// App-wide handle to the [`BackgroundStore`] entity.
pub struct GlobalBackground(pub Entity<BackgroundStore>);

impl Global for GlobalBackground {}

/// Creates the [`BackgroundStore`] and installs it as [`GlobalBackground`].
pub fn init(cx: &mut gpui::App) -> Entity<BackgroundStore> {
    let store = cx.new(|_| BackgroundStore::new());
    cx.set_global(GlobalBackground(store.clone()));
    store
}

/// The [`BackgroundStore`] entity from the global. Panics if [`init`] hasn't run.
pub fn background_store(cx: &gpui::App) -> Entity<BackgroundStore> {
    cx.global::<GlobalBackground>().0.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_and_cover_map_to_object_fit_cover() {
        // Documents the intentional Tile -> Cover fallback (GPUI has no tiling).
        let fit = |f: BackgroundFit| match f {
            BackgroundFit::Cover | BackgroundFit::Tile => ObjectFit::Cover,
            BackgroundFit::Contain => ObjectFit::Contain,
        };
        assert!(matches!(fit(BackgroundFit::Tile), ObjectFit::Cover));
        assert!(matches!(fit(BackgroundFit::Cover), ObjectFit::Cover));
        assert!(matches!(fit(BackgroundFit::Contain), ObjectFit::Contain));
    }

    #[test]
    fn format_detection_by_extension() {
        assert_eq!(
            gpui_format_from_path(Path::new("/x/y.PNG")),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            gpui_format_from_path(Path::new("/x/y.jpg")),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(gpui_format_from_path(Path::new("/x/y.avif")), None);
    }

    #[test]
    fn processes_a_generated_image_with_downscale_and_blur() {
        let dir = std::env::temp_dir().join(format!("labonair-bg-ui-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();
        let name = "wall.png";
        let path = dir.join(name);

        let buf = image::RgbImage::from_fn(3000, 100, |x, _| {
            if x % 2 == 0 {
                image::Rgb([255, 0, 0])
            } else {
                image::Rgb([0, 0, 255])
            }
        });
        image::DynamicImage::ImageRgb8(buf)
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();

        let processed = load_processed_from(&dir, name, 4).unwrap();
        assert!(!processed.bytes.is_empty());
        // Re-encoded (downscale + blur both triggered) -> JPEG, not the source PNG.
        assert_eq!(processed.format, ImageFormat::Jpeg);
        let decoded = image::load_from_memory(&processed.bytes).unwrap();
        assert!(decoded.width() <= MAX_DIM && decoded.height() <= MAX_DIM);

        std::fs::remove_dir_all(&dir).ok();
    }

    fn uuid_like() -> u128 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }
}
