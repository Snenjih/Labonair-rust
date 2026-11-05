//! Background-image layer (T02-006).
//!
//! Full parity with the reference `backgrounds` feature: the user imports an
//! image, picks it, and tunes opacity / blur / fit / tint; the image is then
//! painted behind the app and/or the terminal.
//!
//! [`BackgroundStore`] is a GPUI entity that owns the persisted
//! [`BackgroundSettings`] plus a
//! single decoded, downscaled and pre-blurred [`gpui::Image`]. The image is
//! rebuilt only when the selected file or the blur radius changes — never per
//! frame (see the task's performance warning) — and GPUI's own asset cache
//! keeps the GPU texture around after that.
//!
//! The image sits in an absolutely-positioned, non-interactive overlay beneath
//! the window chrome. While it covers the window the shell reports that to the
//! theme (`ThemeMetrics::background_active`) so the chrome, main and terminal
//! fills resolve translucent and the image shows through them; text stays
//! readable through the translucent fills' own alpha plus the tint clamp
//! ([`readable_tint_opacity`]). This crate owns values, editor, image storage,
//! decoding, and rendering.

mod settings_view;
mod storage;

use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc, Mutex,
};

use gpui::{
    div, img, prelude::*, AnyElement, Context, Entity, EventEmitter, Global, Image, ImageFormat,
    ObjectFit, PathPromptOptions, Styled,
};
use image::ImageEncoder;

pub use settings_view::BackgroundSettingsView;
pub use storage::{
    background_delete, background_import, background_settings_load, background_settings_save,
    backgrounds_dir, backgrounds_list, BackgroundInfo, BackgroundSettings,
};
pub use storage::{background_read_data_url, BackgroundFit, BackgroundTarget};

/// The presentation contract Workspace/Terminal consume instead of this
/// crate: `LayerScope` plus the injected [`BackgroundHost`] built by
/// [`host`]. Keeping this crate the sole owner of image storage, decoding,
/// and rendering policy while `workspace` never depends on it (B02).
pub use labonair_background_host::{BackgroundHost, BackgroundPulse, LayerScope};

/// Contribute the Background-owned image editor to Settings navigation.
///
/// The surface is inline (ADR 0006): the Settings window embeds the owner's
/// [`BackgroundSettingsView`] in the Themes page. `open_backgrounds` is the
/// fallback entry point for hosts that cannot embed (the composition root
/// opens Settings on the Themes page).
/// `report_error` receives the view's own filesystem failures (store failures
/// are reported app-wide by the shell); it should raise a notification.
pub fn register_settings_surfaces(
    registry: &mut labonair_settings::SettingsSurfaceRegistry,
    open_backgrounds: impl Fn(&mut gpui::App) + 'static,
    report_error: impl Fn(String, &mut gpui::App) + 'static,
) -> Result<(), String> {
    use labonair_settings::{
        SettingsSurface, SettingsSurfaceContribution, SettingsSurfaceId, SettingsSurfacePage,
    };

    type ErrorReporter = std::rc::Rc<dyn Fn(String, &mut gpui::App)>;
    let report_error: ErrorReporter = std::rc::Rc::new(report_error);
    registry.register(SettingsSurfaceContribution::inline(
        SettingsSurface::new(
            SettingsSurfaceId::new("backgrounds"),
            "Background image",
            "Choose a background image and adjust its appearance.",
            "Choose background…",
            SettingsSurfacePage::section("themes", "Themes", "Background"),
        ),
        open_backgrounds,
        move |window, cx| {
            let store = background_store(cx);
            let theme = labonair_theme::theme_store(cx);
            let report_error = report_error.clone();
            cx.new(|cx| {
                BackgroundSettingsView::new(store, theme, window, cx, move |message, app| {
                    report_error(message, app)
                })
            })
            .into()
        },
    ))
}

/// User-visible failures emitted by the Background owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BackgroundEvent {
    Error(String),
}

/// Longest edge (px) an imported image is kept at; larger images are
/// downscaled once at load time so a 6000px wallpaper doesn't cost a huge GPU
/// texture for a dimmed background.
const MAX_DIM: u32 = 2560;

/// Exponent of the slider-to-opacity curve (`rendered = slider^GAMMA`).
///
/// The reference halved the slider (`BG_OPACITY_RENDER_FACTOR = 0.5`) because
/// its chrome was transparent. Here the image sits *under* translucent chrome
/// fills (about 0.78 alpha, see `VibrancyLayer::opacity_in`), so only a fraction
/// of the image reaches the eye; halving it made the default 30% a ghost (~3%
/// of the image). A gamma below 1 lifts the low end so 30% renders at ~0.49
/// while 0% stays 0 and 100% stays fully opaque; legibility is the chrome
/// alpha's job, not the slider's.
const OPACITY_GAMMA: f32 = 0.6;

/// The image layer opacity (0..=1) for a slider value (0..=100).
fn rendered_opacity(slider: u8) -> f32 {
    (slider.min(100) as f32 / 100.0).powf(OPACITY_GAMMA)
}

/// Minimum WCAG contrast the tint colour must keep against the UI foreground
/// before it may be applied at full strength (large-text / UI-component floor).
const MIN_TINT_CONTRAST: f64 = 3.0;

/// Largest accepted source file. Bigger files are rejected (with a
/// notification) instead of being read into memory.
const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// Decoder guard: refuse images whose width or height exceeds this before any
/// pixel buffer is allocated (decompression bombs, 100k-pixel panoramas).
const MAX_SOURCE_DIM: u32 = 16_384;

/// Decoder guard: total allocation budget for decoding one image.
const MAX_DECODE_ALLOC: u64 = 768 * 1024 * 1024;

/// The tint opacity (0..=1) to actually paint: `requested`, scaled down when
/// the tint colour has too little contrast against the UI foreground `fg`, so
/// a white tint in a dark theme (or black in a light one) cannot wash text out.
/// A tint with at least [`MIN_TINT_CONTRAST`] against `fg` is unchanged.
pub fn readable_tint_opacity(tint: gpui::Hsla, fg: gpui::Hsla, requested: f32) -> f32 {
    let contrast = labonair_theme::contrast_ratio(labonair_theme::composite_over(tint, fg), fg);
    let scale = ((contrast - 1.0) / (MIN_TINT_CONTRAST - 1.0)).clamp(0.0, 1.0) as f32;
    (requested * scale).clamp(0.0, 1.0)
}

/// Central background-image state. Created once at startup, stored as a GPUI
/// entity and exposed app-wide via [`GlobalBackground`].
pub struct BackgroundStore {
    settings: BackgroundSettings,
    available_images: Vec<storage::BackgroundInfo>,
    /// Decoded + processed image, ready to hand to `img()`.
    image: Option<Arc<Image>>,
    /// `(filename, blur)` the cached `image` was built from.
    key: Option<(String, u8)>,
    pending_key: Option<(String, u8)>,
    image_epoch: u64,
    save_sequence: Arc<AtomicU64>,
    save_lock: Arc<Mutex<()>>,
    /// The UI foreground colour, pushed by the shell, that the tint is
    /// clamped against (see [`readable_tint_opacity`]).
    text_color: Option<gpui::Hsla>,
}

impl BackgroundStore {
    /// Loads the persisted settings and decodes the selected image (if any).
    pub fn new() -> Self {
        let mut store = Self::empty();
        store.settings = background_settings_load();
        store.available_images = backgrounds_list().unwrap_or_default();
        store.rebuild_image();
        store
    }

    fn empty() -> Self {
        Self {
            settings: BackgroundSettings::default(),
            available_images: Vec::new(),
            image: None,
            key: None,
            pending_key: None,
            image_epoch: 0,
            save_sequence: Arc::new(AtomicU64::new(0)),
            save_lock: Arc::new(Mutex::new(())),
            text_color: None,
        }
    }

    /// The current settings (read-only).
    pub fn settings(&self) -> &BackgroundSettings {
        &self.settings
    }

    /// Every background image currently in the app-data `backgrounds/` dir.
    pub fn available(&self) -> Vec<BackgroundInfo> {
        self.available_images.clone()
    }

    /// Whether a decoded image is painted behind the whole window (target
    /// `App`/`Both`). The shell mirrors this into `ThemeMetrics::background_active`
    /// so window surfaces turn translucent; a `Terminal`-only image does not
    /// touch the chrome.
    pub fn covers_window(&self) -> bool {
        self.image.is_some()
            && matches!(
                self.settings.background_target,
                BackgroundTarget::App | BackgroundTarget::Both
            )
    }

    /// Tell the store which foreground colour the UI draws text with, so the
    /// tint can be clamped to stay readable. Re-renders only on a change.
    pub fn set_text_color(&mut self, color: gpui::Hsla, cx: &mut Context<Self>) {
        if self.text_color != Some(color) {
            self.text_color = Some(color);
            cx.notify();
        }
    }

    /// Whether the selected image is currently being decoded or processed.
    pub fn is_loading(&self) -> bool {
        self.pending_key.is_some()
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
        Ok(self.select_imported(info, cx))
    }

    pub(crate) fn select_imported(
        &mut self,
        info: BackgroundInfo,
        cx: &mut Context<Self>,
    ) -> String {
        self.available_images
            .retain(|item| item.filename != info.filename);
        self.available_images.push(info.clone());
        self.available_images
            .sort_by(|left, right| left.filename.cmp(&right.filename));
        self.set_image(info.filename.clone(), cx);
        info.filename
    }

    /// Deletes an image file; clears the selection if it was the active one.
    pub fn delete(&mut self, filename: &str, cx: &mut Context<Self>) -> Result<(), String> {
        background_delete(filename.to_string())?;
        self.forget_deleted(filename, cx);
        Ok(())
    }

    pub(crate) fn forget_deleted(&mut self, filename: &str, cx: &mut Context<Self>) {
        self.available_images
            .retain(|item| item.filename != filename);
        if self.settings.background_image == filename {
            self.set_image("", cx);
        } else {
            cx.notify();
        }
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
                    let imported = cx
                        .background_executor()
                        .spawn(async move { background_import(path.to_string_lossy().to_string()) })
                        .await;
                    let _ = this.update(cx, |this, cx| match imported {
                        Ok(info) => {
                            this.select_imported(info, cx);
                        }
                        Err(message) => cx.emit(BackgroundEvent::Error(format!(
                            "Could not import background image: {message}"
                        ))),
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

        let rendered_opacity = rendered_opacity(s.background_opacity);
        // GPUI's `img()` has no tiling mode — `Tile` falls back to `Cover`.
        let fit = match s.background_fit {
            BackgroundFit::Cover | BackgroundFit::Tile => ObjectFit::Cover,
            BackgroundFit::Contain => ObjectFit::Contain,
        };

        // The image fades on its own; the tint is a sibling so its strength
        // is not multiplied by the image opacity.
        let mut root = div().absolute().inset_0().overflow_hidden().child(
            div()
                .absolute()
                .inset_0()
                .opacity(rendered_opacity)
                .child(img(image).object_fit(fit).size_full()),
        );

        if s.background_tint_opacity > 0 {
            if let Ok(tint) = labonair_theme::parse_color(&s.background_tint_color) {
                let requested = s.background_tint_opacity as f32 / 100.0;
                let tint_opacity = match self.text_color {
                    Some(fg) => readable_tint_opacity(tint, fg, requested),
                    None => requested,
                };
                root = root.child(div().absolute().inset_0().bg(tint).opacity(tint_opacity));
            }
        }

        Some(root.into_any_element())
    }

    // --- internals --------------------------------------------------

    fn apply(&mut self, cx: &mut Context<Self>) {
        self.persist_settings(cx);
        self.rebuild_image_async(cx);
        cx.notify();
    }

    fn persist_settings(&self, cx: &mut Context<Self>) {
        let settings = self.settings.clone();
        let sequence = self.save_sequence.fetch_add(1, Ordering::SeqCst) + 1;
        let save_sequence = self.save_sequence.clone();
        let save_lock = self.save_lock.clone();
        let save = cx.background_executor().spawn(async move {
            let _guard = save_lock
                .lock()
                .map_err(|_| "background settings save lock was poisoned".to_string())?;
            if save_sequence.load(Ordering::SeqCst) != sequence {
                return Ok(());
            }
            background_settings_save(&settings)
        });
        cx.spawn(async move |this, cx| {
            if let Err(message) = save.await {
                let _ = this.update(cx, |_, cx| {
                    cx.emit(BackgroundEvent::Error(format!(
                        "Could not save background settings: {message}"
                    )));
                });
            }
        })
        .detach();
    }

    fn rebuild_image_async(&mut self, cx: &mut Context<Self>) {
        if self.settings.background_image.is_empty() {
            self.image_epoch = self.image_epoch.wrapping_add(1);
            self.pending_key = None;
            self.image = None;
            self.key = None;
            return;
        }

        let key = (
            self.settings.background_image.clone(),
            self.settings.background_blur,
        );
        if self.key.as_ref() == Some(&key) || self.pending_key.as_ref() == Some(&key) {
            return;
        }

        self.image_epoch = self.image_epoch.wrapping_add(1);
        let epoch = self.image_epoch;
        self.pending_key = Some(key.clone());
        self.image = None;
        let job = cx
            .background_executor()
            .spawn(async move { load_processed_image(&key.0, key.1) });
        cx.spawn(async move |this, cx| {
            let result = job.await;
            let _ = this.update(cx, |this, cx| {
                if this.image_epoch != epoch {
                    return;
                }
                this.pending_key = None;
                match result {
                    Ok(image) => {
                        this.image = Some(Arc::new(image));
                        this.key = Some((
                            this.settings.background_image.clone(),
                            this.settings.background_blur,
                        ));
                    }
                    Err(message) => {
                        let filename = this.settings.background_image.clone();
                        this.image = None;
                        this.key = None;
                        this.settings.background_image.clear();
                        this.persist_settings(cx);
                        cx.emit(BackgroundEvent::Error(format!(
                            "Could not load background image '{filename}': {message}"
                        )));
                    }
                }
                cx.notify();
            });
        })
        .detach();
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

impl EventEmitter<BackgroundEvent> for BackgroundStore {}

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
    let size = std::fs::metadata(&path).map_err(|e| e.to_string())?.len();
    if size > MAX_FILE_BYTES {
        return Err(format!(
            "the file is {} MB; the limit is {} MB",
            size / (1024 * 1024),
            MAX_FILE_BYTES / (1024 * 1024)
        ));
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;

    // Every format GPUI can display is decodable here, so a decode failure
    // means a corrupt or unsupported file: report it instead of handing GPUI
    // bytes it will silently fail to draw.
    let mut dynimg = decode_limited(&bytes).map_err(|e| e.to_string())?;

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

/// Decode `bytes` with width/height and allocation limits so a corrupt or
/// hostile header cannot make the decoder allocate gigabytes.
fn decode_limited(bytes: &[u8]) -> image::ImageResult<image::DynamicImage> {
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SOURCE_DIM);
    limits.max_image_height = Some(MAX_SOURCE_DIM);
    limits.max_alloc = Some(MAX_DECODE_ALLOC);
    let mut reader = image::ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    reader.limits(limits);
    reader.decode()
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
    let store = cx.new(|_| BackgroundStore::empty());
    let load_store = store.clone();
    cx.spawn(async move |cx| {
        let (settings, images) = cx
            .background_executor()
            .spawn(async move {
                (
                    background_settings_load(),
                    backgrounds_list().unwrap_or_default(),
                )
            })
            .await;
        let _ = cx.update(|cx| {
            load_store.update(cx, |store, cx| {
                store.settings = settings;
                store.available_images = images;
                store.rebuild_image_async(cx);
                cx.notify();
            });
        });
    })
    .detach();
    cx.set_global(GlobalBackground(store.clone()));
    store
}

/// The [`BackgroundStore`] entity from the global. Panics if [`init`] hasn't run.
pub fn background_store(cx: &gpui::App) -> Entity<BackgroundStore> {
    cx.global::<GlobalBackground>().0.clone()
}

/// Build the narrow [`BackgroundHost`] presentation capability for `store`.
/// The composition root injects the result into Workspace/Terminal instead of
/// the concrete store, so those crates never depend on `labonair-background`.
pub fn host(store: &Entity<BackgroundStore>, cx: &mut gpui::App) -> BackgroundHost {
    let pulse = cx.new(|_| BackgroundPulse);
    cx.observe(store, {
        let pulse = pulse.clone();
        move |_, cx| {
            pulse.update(cx, |_, cx| cx.notify());
        }
    })
    .detach();
    let store = store.clone();
    BackgroundHost::new(pulse, move |cx, scope| store.read(cx).layer(scope))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_settings_surface_is_registered_by_its_owner() {
        let mut registry = labonair_settings::SettingsSurfaceRegistry::default();
        register_settings_surfaces(&mut registry, |_| {}, |_, _| {}).unwrap();
        let surface = registry
            .surface(labonair_settings::SettingsSurfaceId::new("backgrounds"))
            .unwrap();
        assert_eq!(surface.page.slug, "themes");
        assert_eq!(surface.page.section, "Background");
        assert_eq!(surface.mode, labonair_settings::SettingsSurfaceMode::Inline);
    }

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
    fn slider_curve_lifts_the_low_end_and_keeps_the_ends() {
        assert_eq!(rendered_opacity(0), 0.0);
        assert_eq!(rendered_opacity(100), 1.0);
        let default = rendered_opacity(30);
        assert!(default > 0.4 && default < 0.6, "{default}");
        assert!(rendered_opacity(29) < rendered_opacity(30));
    }

    #[test]
    fn tint_without_contrast_against_text_is_faded_out() {
        let white = labonair_theme::parse_color("#ffffff").unwrap();
        let black = labonair_theme::parse_color("#000000").unwrap();
        // White tint under white text would wash it out entirely.
        assert_eq!(readable_tint_opacity(white, white, 0.8), 0.0);
        // High-contrast tint is untouched.
        assert_eq!(readable_tint_opacity(black, white, 0.8), 0.8);
        // Never exceeds the request, never negative.
        let grey = labonair_theme::parse_color("#808080").unwrap();
        let faded = readable_tint_opacity(grey, white, 0.5);
        assert!((0.0..=0.5).contains(&faded));
    }

    #[test]
    fn corrupt_oversized_and_missing_images_are_errors_not_panics() {
        let dir = std::env::temp_dir().join(format!("labonair-bg-bad-{}", uuid_like()));
        std::fs::create_dir_all(&dir).unwrap();

        std::fs::write(dir.join("corrupt.png"), b"definitely not a png").unwrap();
        assert!(load_processed_from(&dir, "corrupt.png", 0).is_err());
        assert!(load_processed_from(&dir, "missing.png", 0).is_err());

        // A sparse file just over the size limit is rejected before reading.
        let big = std::fs::File::create(dir.join("huge.png")).unwrap();
        big.set_len(MAX_FILE_BYTES + 1).unwrap();
        let err = load_processed_from(&dir, "huge.png", 0).unwrap_err();
        assert!(err.contains("limit"), "{err}");

        // An image wider than the decoder limit is refused.
        let wide = image::RgbImage::from_pixel(MAX_SOURCE_DIM + 1, 1, image::Rgb([1, 2, 3]));
        image::DynamicImage::ImageRgb8(wide)
            .save_with_format(dir.join("wide.png"), image::ImageFormat::Png)
            .unwrap();
        assert!(load_processed_from(&dir, "wide.png", 0).is_err());

        std::fs::remove_dir_all(&dir).ok();
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
